use crate::auth as auth_service;
use crate::AppState;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

// ── Request / Response types ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ImportPayload {
    pub specimens: Vec<Vec<String>>,
    pub subcultures: Vec<Vec<String>>,
    pub media: Vec<Vec<String>>,
    pub prepared_solutions: Vec<Vec<String>>,
    pub inventory: Vec<Vec<String>>,
    pub compliance: Vec<Vec<String>>,
}

#[derive(Serialize, Clone)]
pub struct RowError {
    pub sheet: String,
    pub row: usize,
    pub message: String,
}

#[derive(Serialize, Default, Clone)]
pub struct SheetStats {
    pub creates: u32,
    pub updates: u32,
    pub skips: u32,
}

#[derive(Serialize)]
pub struct ImportResult {
    pub specimens: SheetStats,
    pub subcultures: SheetStats,
    pub media: SheetStats,
    pub prepared_solutions: SheetStats,
    pub inventory: SheetStats,
    pub compliance: SheetStats,
    pub errors: Vec<RowError>,
    pub dry_run: bool,
}

// ── Small helpers ─────────────────────────────────────────────────────────────

fn opt(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn bool_from_str(s: &str) -> bool {
    matches!(s.trim().to_lowercase().as_str(), "yes" | "true" | "1")
}

fn col(row: &[String], idx: usize) -> &str {
    row.get(idx).map(|s| s.as_str()).unwrap_or("")
}

// ── Command ───────────────────────────────────────────────────────────────────

/// Import the six-sheet workbook produced by ExportManager.
/// When `dry_run` is true the transaction is rolled back so no data is changed;
/// the returned counts and error list reflect what a real import would do.
#[tauri::command]
pub fn import_xlsx(
    state: State<AppState>,
    token: String,
    payload: ImportPayload,
    dry_run: bool,
) -> Result<ImportResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Write permission required to import data".to_string());
    }

    let conn = &db.conn;
    let mut errors: Vec<RowError> = Vec::new();
    let mut spec_stats = SheetStats::default();
    let mut sub_stats = SheetStats::default();
    let mut media_stats = SheetStats::default();
    let mut ps_stats = SheetStats::default();
    let mut inv_stats = SheetStats::default();
    let mut comp_stats = SheetStats::default();

    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;

    // ── Specimens ─────────────────────────────────────────────────────────────
    // Columns: Accession(0) Species Code(1) Species(2) Stage(3) Provenance(4)
    //          Initiation Date(5) Location(6) Health Status(7) Quarantine(8)
    //          Subculture Count(9) Notes(10)
    for (i, row) in payload.specimens.iter().enumerate() {
        let row_num = i + 2;
        let accession = col(row, 0).trim();
        if accession.is_empty() {
            errors.push(RowError { sheet: "Specimens".into(), row: row_num, message: "Accession number is required".into() });
            spec_stats.skips += 1;
            continue;
        }

        let species_code = col(row, 1).trim();
        let species_name_full = col(row, 2).trim();

        // Resolve species_id; create a stub entry if the code is new.
        let species_id: Option<String> = if !species_code.is_empty() {
            match conn.query_row(
                "SELECT id FROM species WHERE species_code = ?1",
                params![species_code],
                |r| r.get::<_, String>(0),
            ) {
                Ok(id) => Some(id),
                Err(_) if !species_name_full.is_empty() => {
                    let parts: Vec<&str> = species_name_full.splitn(2, ' ').collect();
                    let genus = parts.first().copied().unwrap_or(species_code);
                    let sp_name = parts.get(1).copied().unwrap_or(species_code);
                    let sid = new_id();
                    let ts = now();
                    conn.execute(
                        "INSERT INTO species (id, genus, species_name, species_code, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                        params![sid, genus, sp_name, species_code, ts],
                    ).ok();
                    Some(sid)
                }
                Err(_) => None,
            }
        } else {
            None
        };

        let raw_stage = col(row, 3).trim();
        const VALID_STAGES: &[&str] = &[
            "explant","callus","suspension","protoplast","shoot","root",
            "embryogenic","plantlet","acclimatized","stock","archived","custom",
        ];
        let (stage, custom_stage): (String, Option<String>) =
            if VALID_STAGES.contains(&raw_stage) {
                (raw_stage.to_string(), None)
            } else if !raw_stage.is_empty() {
                ("custom".to_string(), Some(raw_stage.to_string()))
            } else {
                ("stock".to_string(), None)
            };

        let provenance = opt(col(row, 4));
        let initiation_date = opt(col(row, 5));
        let location = opt(col(row, 6));
        let health_status = opt(col(row, 7));
        let quarantine = bool_from_str(col(row, 8)) as i32;
        let subculture_count: i32 = col(row, 9).trim().parse().unwrap_or(0);
        let notes = opt(col(row, 10));
        let ts = now();

        match conn.query_row(
            "SELECT id FROM specimens WHERE accession_number = ?1",
            params![accession],
            |r| r.get::<_, String>(0),
        ) {
            Ok(id) => {
                if let Err(e) = conn.execute(
                    "UPDATE specimens SET species_id=?2, stage=?3, custom_stage=?4, provenance=?5,
                     initiation_date=?6, location=?7, health_status=?8, quarantine_flag=?9,
                     subculture_count=?10, notes=?11, updated_at=?12 WHERE id=?1",
                    params![id, species_id, stage, custom_stage, provenance,
                             initiation_date, location, health_status, quarantine,
                             subculture_count, notes, ts],
                ) {
                    errors.push(RowError { sheet: "Specimens".into(), row: row_num, message: e.to_string() });
                } else {
                    spec_stats.updates += 1;
                }
            }
            Err(_) => {
                let id = new_id();
                if let Err(e) = conn.execute(
                    "INSERT INTO specimens (id, accession_number, species_id, stage, custom_stage,
                     provenance, initiation_date, location, health_status, quarantine_flag,
                     subculture_count, notes, is_archived, created_by, created_at, updated_at)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,0,?13,?14,?14)",
                    params![id, accession, species_id, stage, custom_stage, provenance,
                             initiation_date, location, health_status, quarantine,
                             subculture_count, notes, user.id, ts],
                ) {
                    errors.push(RowError { sheet: "Specimens".into(), row: row_num, message: e.to_string() });
                } else {
                    spec_stats.creates += 1;
                }
            }
        }
    }

    // ── Media Batches ─────────────────────────────────────────────────────────
    // Columns: Name(0) Batch Code(1) Type(2) Prepared By(3) Date Prepared(4)
    //          Expiry Date(5) pH(6) Volume mL(7) Sterilization Method(8) Notes(9)
    for (i, row) in payload.media.iter().enumerate() {
        let row_num = i + 2;
        let name = col(row, 0).trim();
        if name.is_empty() {
            errors.push(RowError { sheet: "Media Batches".into(), row: row_num, message: "Name is required".into() });
            media_stats.skips += 1;
            continue;
        }
        let batch_code = opt(col(row, 1));
        let ph: Option<f64> = col(row, 6).trim().parse().ok();
        let volume: Option<f64> = col(row, 7).trim().parse().ok();
        let prep_date = opt(col(row, 4));
        let exp_date = opt(col(row, 5));
        let steril = opt(col(row, 8));
        let notes = opt(col(row, 9));
        let ts = now();

        let existing_id: Option<String> = if let Some(ref bc) = batch_code {
            conn.query_row(
                "SELECT id FROM media_batches WHERE batch_id = ?1",
                params![bc],
                |r| r.get(0),
            ).ok()
        } else {
            None
        }.or_else(|| {
            conn.query_row(
                "SELECT id FROM media_batches WHERE name = ?1",
                params![name],
                |r| r.get(0),
            ).ok()
        });

        match existing_id {
            Some(id) => {
                if let Err(e) = conn.execute(
                    "UPDATE media_batches SET name=?2, ph_before_autoclave=?3, volume_prepared_ml=?4,
                     preparation_date=?5, expiration_date=?6, sterilization_method=?7,
                     notes=?8, updated_at=?9 WHERE id=?1",
                    params![id, name, ph, volume, prep_date, exp_date, steril, notes, ts],
                ) {
                    errors.push(RowError { sheet: "Media Batches".into(), row: row_num, message: e.to_string() });
                } else {
                    media_stats.updates += 1;
                }
            }
            None => {
                let id = new_id();
                let bc = batch_code.unwrap_or_else(|| format!("IMP-{}", &id[..8].to_uppercase()));
                if let Err(e) = conn.execute(
                    "INSERT INTO media_batches (id, batch_id, name, ph_before_autoclave, volume_prepared_ml,
                     preparation_date, expiration_date, sterilization_method, notes,
                     created_by, created_at, updated_at)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?11)",
                    params![id, bc, name, ph, volume, prep_date, exp_date, steril, notes, user.id, ts],
                ) {
                    errors.push(RowError { sheet: "Media Batches".into(), row: row_num, message: e.to_string() });
                } else {
                    media_stats.creates += 1;
                }
            }
        }
    }

    // ── Prepared Solutions ────────────────────────────────────────────────────
    // Columns: Name(0) Concentration(1) Solvent(2) Prepared By(3) Date Prepared(4)
    //          Expiry Date(5) Volume mL(6) Storage Condition(7) Notes(8)
    for (i, row) in payload.prepared_solutions.iter().enumerate() {
        let row_num = i + 2;
        let name = col(row, 0).trim();
        if name.is_empty() {
            errors.push(RowError { sheet: "Prepared Solutions".into(), row: row_num, message: "Name is required".into() });
            ps_stats.skips += 1;
            continue;
        }
        let concentration = opt(col(row, 1));
        let solvent = opt(col(row, 2));
        let prep_date = opt(col(row, 4));
        let exp_date = opt(col(row, 5));
        let volume: Option<f64> = col(row, 6).trim().parse().ok();
        let storage = opt(col(row, 7));
        let notes = opt(col(row, 8));
        let ts = now();

        let existing_id: Option<String> = conn.query_row(
            "SELECT id FROM prepared_solutions WHERE name = ?1",
            params![name],
            |r| r.get(0),
        ).ok();

        match existing_id {
            Some(id) => {
                if let Err(e) = conn.execute(
                    "UPDATE prepared_solutions SET concentration=?2, solvent=?3, preparation_date=?4,
                     expiration_date=?5, volume_ml=?6, storage_conditions=?7, notes=?8,
                     updated_at=?9 WHERE id=?1",
                    params![id, concentration, solvent, prep_date, exp_date, volume, storage, notes, ts],
                ) {
                    errors.push(RowError { sheet: "Prepared Solutions".into(), row: row_num, message: e.to_string() });
                } else {
                    ps_stats.updates += 1;
                }
            }
            None => {
                let id = new_id();
                if let Err(e) = conn.execute(
                    "INSERT INTO prepared_solutions (id, name, concentration, solvent, preparation_date,
                     expiration_date, volume_ml, storage_conditions, notes,
                     prepared_by, created_at, updated_at)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?11)",
                    params![id, name, concentration, solvent, prep_date, exp_date,
                             volume, storage, notes, user.id, ts],
                ) {
                    errors.push(RowError { sheet: "Prepared Solutions".into(), row: row_num, message: e.to_string() });
                } else {
                    ps_stats.creates += 1;
                }
            }
        }
    }

    // ── Inventory ─────────────────────────────────────────────────────────────
    // Columns: Name(0) Category(1) Unit(2) Current Stock(3) Min Stock(4)
    //          Supplier(5) Catalog #(6) Location(7) Notes(8)
    for (i, row) in payload.inventory.iter().enumerate() {
        let row_num = i + 2;
        let name = col(row, 0).trim();
        if name.is_empty() {
            errors.push(RowError { sheet: "Inventory".into(), row: row_num, message: "Name is required".into() });
            inv_stats.skips += 1;
            continue;
        }
        let category = opt(col(row, 1));
        let unit = opt(col(row, 2));
        let current_stock: f64 = col(row, 3).trim().parse().unwrap_or(0.0);
        let min_stock: Option<f64> = col(row, 4).trim().parse().ok();
        let supplier = opt(col(row, 5));
        let catalog_number = opt(col(row, 6));
        let storage_location = opt(col(row, 7));
        let notes = opt(col(row, 8));
        let ts = now();

        let existing_id: Option<String> = conn.query_row(
            "SELECT id FROM inventory_items WHERE name = ?1",
            params![name],
            |r| r.get(0),
        ).ok();

        match existing_id {
            Some(id) => {
                if let Err(e) = conn.execute(
                    "UPDATE inventory_items SET category=?2, unit=?3, current_stock=?4, minimum_stock=?5,
                     supplier=?6, catalog_number=?7, storage_location=?8, notes=?9,
                     updated_at=?10 WHERE id=?1",
                    params![id, category, unit, current_stock, min_stock,
                             supplier, catalog_number, storage_location, notes, ts],
                ) {
                    errors.push(RowError { sheet: "Inventory".into(), row: row_num, message: e.to_string() });
                } else {
                    inv_stats.updates += 1;
                }
            }
            None => {
                let id = new_id();
                if let Err(e) = conn.execute(
                    "INSERT INTO inventory_items (id, name, category, unit, current_stock, minimum_stock,
                     supplier, catalog_number, storage_location, notes, created_at, updated_at)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?11)",
                    params![id, name, category, unit, current_stock, min_stock,
                             supplier, catalog_number, storage_location, notes, ts],
                ) {
                    errors.push(RowError { sheet: "Inventory".into(), row: row_num, message: e.to_string() });
                } else {
                    inv_stats.creates += 1;
                }
            }
        }
    }

    // ── Compliance ────────────────────────────────────────────────────────────
    // Columns: Specimen ID(0) Record Type(1) Status(2) Authority(3)
    //          Issue Date(4) Expiry Date(5) Notes(6)
    // Specimen ID is matched against specimens.id (UUID) then specimens.accession_number.
    for (i, row) in payload.compliance.iter().enumerate() {
        let row_num = i + 2;
        let specimen_ref = col(row, 0).trim();
        if specimen_ref.is_empty() {
            errors.push(RowError { sheet: "Compliance".into(), row: row_num, message: "Specimen ID is required".into() });
            comp_stats.skips += 1;
            continue;
        }

        let specimen_id: Option<String> = conn.query_row(
            "SELECT id FROM specimens WHERE id = ?1 OR accession_number = ?1",
            params![specimen_ref],
            |r| r.get(0),
        ).ok();

        let specimen_id = match specimen_id {
            Some(id) => id,
            None => {
                errors.push(RowError {
                    sheet: "Compliance".into(),
                    row: row_num,
                    message: format!("Specimen '{}' not found", specimen_ref),
                });
                comp_stats.skips += 1;
                continue;
            }
        };

        let record_type = opt(col(row, 1)).unwrap_or_else(|| "other".to_string());
        let status = opt(col(row, 2)).unwrap_or_else(|| "pending".to_string());
        let agency = opt(col(row, 3));
        let expiry_date = opt(col(row, 5));
        let notes = opt(col(row, 6));
        let ts = now();
        let id = new_id();

        if let Err(e) = conn.execute(
            "INSERT INTO compliance_records
             (id, specimen_id, record_type, status, agency, permit_expiry, notes,
              created_by, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)",
            params![id, specimen_id, record_type, status, agency, expiry_date, notes, user.id, ts],
        ) {
            errors.push(RowError { sheet: "Compliance".into(), row: row_num, message: e.to_string() });
        } else {
            comp_stats.creates += 1;
        }
    }

    // ── Subcultures ───────────────────────────────────────────────────────────
    // Columns: Specimen ID(0) Passage #(1) Date(2) Media Batch(3) Vessel Type(4)
    //          Vessel Size(5) Health Status(6) Contamination(7) Contamination Notes(8)
    //          pH(9) Temp °C(10) Light Cycle(11) Performed By(12) Notes(13) Observations(14)
    // Specimen ID is matched against specimens.id (UUID) then specimens.accession_number.
    for (i, row) in payload.subcultures.iter().enumerate() {
        let row_num = i + 2;
        let specimen_ref = col(row, 0).trim();
        if specimen_ref.is_empty() {
            errors.push(RowError { sheet: "Subcultures".into(), row: row_num, message: "Specimen ID is required".into() });
            sub_stats.skips += 1;
            continue;
        }

        let specimen_id: Option<String> = conn.query_row(
            "SELECT id FROM specimens WHERE id = ?1 OR accession_number = ?1",
            params![specimen_ref],
            |r| r.get(0),
        ).ok();

        let specimen_id = match specimen_id {
            Some(id) => id,
            None => {
                errors.push(RowError {
                    sheet: "Subcultures".into(),
                    row: row_num,
                    message: format!(
                        "Specimen '{}' not found — import specimens first, or use accession number in this column",
                        specimen_ref
                    ),
                });
                sub_stats.skips += 1;
                continue;
            }
        };

        let passage: i32 = col(row, 1).trim().parse().unwrap_or(1);
        let date = opt(col(row, 2));
        let media_batch_name = col(row, 3).trim();

        let media_batch_id: Option<String> = if !media_batch_name.is_empty() {
            conn.query_row(
                "SELECT id FROM media_batches WHERE name = ?1 OR batch_id = ?1",
                params![media_batch_name],
                |r| r.get(0),
            ).ok()
        } else {
            None
        };

        let vessel_type = opt(col(row, 4));
        let vessel_size = opt(col(row, 5));
        let health_status = opt(col(row, 6));
        let contamination = bool_from_str(col(row, 7)) as i32;
        let contamination_notes = opt(col(row, 8));
        let ph: Option<f64> = col(row, 9).trim().parse().ok();
        let temp: Option<f64> = col(row, 10).trim().parse().ok();
        let light_cycle = opt(col(row, 11));
        let performer_name = opt(col(row, 12));
        let notes = opt(col(row, 13));
        let observations = opt(col(row, 14));
        let ts = now();

        let existing_id: Option<String> = conn.query_row(
            "SELECT id FROM subcultures WHERE specimen_id = ?1 AND passage_number = ?2",
            params![specimen_id, passage],
            |r| r.get(0),
        ).ok();

        match existing_id {
            Some(id) => {
                if let Err(e) = conn.execute(
                    "UPDATE subcultures SET date=?2, media_batch_id=?3, vessel_type=?4, vessel_size=?5,
                     health_status=?6, contamination_flag=?7, contamination_notes=?8, ph=?9,
                     temperature_c=?10, light_cycle=?11, performer_name=?12,
                     notes=?13, observations=?14, updated_at=?15 WHERE id=?1",
                    params![id, date, media_batch_id, vessel_type, vessel_size, health_status,
                             contamination, contamination_notes, ph, temp, light_cycle,
                             performer_name, notes, observations, ts],
                ) {
                    errors.push(RowError { sheet: "Subcultures".into(), row: row_num, message: e.to_string() });
                } else {
                    sub_stats.updates += 1;
                }
            }
            None => {
                let id = new_id();
                if let Err(e) = conn.execute(
                    "INSERT INTO subcultures
                     (id, specimen_id, passage_number, date, media_batch_id,
                      vessel_type, vessel_size, health_status, contamination_flag, contamination_notes,
                      ph, temperature_c, light_cycle, performer_name, notes, observations,
                      performed_by, created_at, updated_at)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?18)",
                    params![id, specimen_id, passage, date, media_batch_id, vessel_type, vessel_size,
                             health_status, contamination, contamination_notes, ph, temp,
                             light_cycle, performer_name, notes, observations, user.id, ts],
                ) {
                    errors.push(RowError { sheet: "Subcultures".into(), row: row_num, message: e.to_string() });
                } else {
                    sub_stats.creates += 1;
                }
            }
        }
    }

    // ── Commit or roll back ───────────────────────────────────────────────────
    if dry_run {
        conn.execute_batch("ROLLBACK").map_err(|e| e.to_string())?;
    } else {
        conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
    }

    Ok(ImportResult {
        specimens: spec_stats,
        subcultures: sub_stats,
        media: media_stats,
        prepared_solutions: ps_stats,
        inventory: inv_stats,
        compliance: comp_stats,
        errors,
        dry_run,
    })
}
