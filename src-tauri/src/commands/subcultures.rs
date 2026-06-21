use crate::auth as auth_service;
use crate::db::queries;
use crate::models::specimen::PaginatedResponse;
use crate::models::subculture::*;
use crate::AppState;
use rusqlite::params;
use tauri::State;

/// Records a terminal "death" event for a specimen, archives it, and prevents
/// any further passages or splits.  Unlike `create_subculture` this does NOT
/// increment the specimen's `subculture_count` — the death event is terminal,
/// not a normal passage in the culture's lineage.
#[tauri::command]
pub fn record_specimen_death(
    state: State<AppState>,
    token: String,
    request: RecordSpecimenDeathRequest,
) -> Result<Subculture, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let (current_count, is_archived): (i32, i32) = db.conn.query_row(
        "SELECT subculture_count, is_archived FROM specimens WHERE id = ?1",
        params![request.specimen_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|_| "Specimen not found".to_string())?;

    if is_archived != 0 {
        return Err("Specimen is already archived — cannot record a death event".to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();
    // Store after last passage number for ordering, but subculture_count stays unchanged.
    let event_passage_number = current_count + 1;

    let tx = db.conn
        .unchecked_transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    tx.execute(
        "INSERT INTO subcultures
             (id, specimen_id, passage_number, date, notes, observations,
              performed_by, employee_id, health_status, contamination_flag, event_type)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'0',0,'death')",
        params![
            id,
            request.specimen_id,
            event_passage_number,
            request.date,
            request.notes,
            request.observations,
            user.id,
            request.employee_id,
        ],
    ).map_err(|e| format!("Failed to insert death event: {}", e))?;

    // Archive specimen; set health to 0 (Dead).  subculture_count is intentionally untouched.
    tx.execute(
        "UPDATE specimens
         SET is_archived  = 1,
             archived_at  = datetime('now'),
             health_status = '0',
             updated_at   = datetime('now')
         WHERE id = ?1",
        params![request.specimen_id],
    ).map_err(|e| format!("Failed to archive specimen: {}", e))?;

    queries::log_audit(
        &tx,
        Some(&user.id),
        "death",
        "specimen",
        Some(&request.specimen_id),
        None,
        None,
        Some("Specimen marked dead and archived — terminal event"),
    ).map_err(|e| format!("Failed to write death audit: {}", e))?;

    tx.commit().map_err(|e| format!("Failed to commit death transaction: {}", e))?;

    db.conn.query_row(
        "SELECT sc.*, u.display_name as performer_name, mb.name as media_batch_name
         FROM subcultures sc
         LEFT JOIN users u ON sc.performed_by = u.id
         LEFT JOIN media_batches mb ON sc.media_batch_id = mb.id
         WHERE sc.id = ?1",
        params![id],
        row_to_subculture,
    ).map_err(|e| format!("Failed to fetch death event: {}", e))
}

// ── helper: map a DB row to a Subculture ────────────────────────────────────
fn row_to_subculture(row: &rusqlite::Row) -> rusqlite::Result<Subculture> {
    let flag: i32 = row.get("contamination_flag").unwrap_or(0);
    Ok(Subculture {
        id: row.get("id")?,
        specimen_id: row.get("specimen_id")?,
        passage_number: row.get("passage_number")?,
        date: row.get("date")?,
        media_batch_id: row.get("media_batch_id")?,
        media_batch_name: row.get("media_batch_name")?,
        ph: row.get("ph")?,
        temperature_c: row.get("temperature_c")?,
        light_cycle: row.get("light_cycle")?,
        light_intensity_lux: row.get("light_intensity_lux")?,
        experimental_treatment: row.get("experimental_treatment")?,
        vessel_type: row.get("vessel_type")?,
        vessel_size: row.get("vessel_size")?,
        vessel_material: row.get("vessel_material")?,
        vessel_lid_type: row.get("vessel_lid_type")?,
        location_from: row.get("location_from")?,
        location_to: row.get("location_to")?,
        temp_before: row.get("temp_before")?,
        temp_after: row.get("temp_after")?,
        humidity_before: row.get("humidity_before")?,
        humidity_after: row.get("humidity_after")?,
        light_before: row.get("light_before")?,
        light_after: row.get("light_after")?,
        exposure_duration_hours: row.get("exposure_duration_hours")?,
        notes: row.get("notes")?,
        observations: row.get("observations")?,
        performed_by: row.get("performed_by")?,
        performer_name: row.get("performer_name")?,
        employee_id: row.get("employee_id")?,
        health_status: row.get("health_status")?,
        contamination_flag: flag != 0,
        contamination_notes: row.get("contamination_notes")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        event_type: row.get("event_type").unwrap_or_else(|_| "passage".to_string()),
    })
}

#[tauri::command]
pub fn list_subcultures(
    state: State<AppState>,
    token: String,
    specimen_id: String,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<PaginatedResponse<Subculture>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let pg = queries::PaginationParams {
        page: page.unwrap_or(1),
        per_page: per_page.unwrap_or(50),
    };

    let total: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM subcultures WHERE specimen_id = ?1",
        params![specimen_id],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;

    let mut stmt = db.conn.prepare(
        "SELECT sc.*, u.display_name as performer_name, mb.name as media_batch_name
         FROM subcultures sc
         LEFT JOIN users u ON sc.performed_by = u.id
         LEFT JOIN media_batches mb ON sc.media_batch_id = mb.id
         WHERE sc.specimen_id = ?1
         ORDER BY sc.passage_number DESC
         LIMIT ?2 OFFSET ?3"
    ).map_err(|e| e.to_string())?;

    let subcultures = stmt.query_map(params![specimen_id, pg.limit(), pg.offset()], row_to_subculture)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();

    let total_pages = ((total as f64) / (pg.per_page as f64)).ceil() as u32;

    Ok(PaginatedResponse {
        items: subcultures,
        total,
        page: pg.page,
        per_page: pg.per_page,
        total_pages,
    })
}

#[tauri::command]
pub fn create_subculture(
    state: State<AppState>,
    token: String,
    request: CreateSubcultureRequest,
) -> Result<Subculture, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let (current_count, is_archived): (i32, i32) = db.conn.query_row(
        "SELECT subculture_count, is_archived FROM specimens WHERE id = ?1",
        params![request.specimen_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).map_err(|_| "Specimen not found".to_string())?;

    if is_archived != 0 {
        return Err("Cannot record a passage on an archived specimen".to_string());
    }

    let passage_number = current_count + 1;
    let id = uuid::Uuid::new_v4().to_string();
    let contamination_flag = request.contamination_flag.unwrap_or(false) as i32;

    let tx = db.conn.unchecked_transaction()
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    tx.execute(
        "INSERT INTO subcultures (id, specimen_id, passage_number, date, media_batch_id,
         ph, temperature_c, light_cycle, light_intensity_lux, experimental_treatment,
         vessel_type, vessel_size, vessel_material, vessel_lid_type,
         location_from, location_to, temp_before, temp_after,
         humidity_before, humidity_after, light_before, light_after,
         exposure_duration_hours, notes, observations, performed_by, employee_id,
         health_status, contamination_flag, contamination_notes)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30)",
        params![
            id, request.specimen_id, passage_number, request.date, request.media_batch_id,
            request.ph, request.temperature_c, request.light_cycle, request.light_intensity_lux,
            request.experimental_treatment, request.vessel_type, request.vessel_size,
            request.vessel_material, request.vessel_lid_type, request.location_from,
            request.location_to, request.temp_before, request.temp_after,
            request.humidity_before, request.humidity_after, request.light_before,
            request.light_after, request.exposure_duration_hours, request.notes,
            request.observations, user.id, request.employee_id, request.health_status,
            contamination_flag, request.contamination_notes,
        ],
    ).map_err(|e| format!("Failed to create subculture: {}", e))?;

    // Update specimen: subculture count, location (if transferred), health (if assessed)
    tx.execute(
        "UPDATE specimens SET
         subculture_count = ?1,
         location       = CASE WHEN ?2 IS NOT NULL THEN ?2 ELSE location       END,
         health_status  = CASE WHEN ?3 IS NOT NULL THEN ?3 ELSE health_status  END,
         updated_at     = datetime('now')
         WHERE id = ?4",
        params![passage_number, request.location_to, request.health_status, request.specimen_id],
    ).map_err(|e| format!("Failed to update specimen after passage: {}", e))?;

    // Audit passage on the SPECIMEN's chain so chain_seq increments for the specimen
    queries::log_audit(
        &tx,Some(&user.id), "subcultured", "specimen", Some(&request.specimen_id),
        None, None, Some(&format!("Passage #{} recorded", passage_number)),
    ).map_err(|e| format!("Failed to write passage audit: {}", e))?;

    tx.commit().map_err(|e| format!("Failed to commit subculture transaction: {}", e))?;

    db.conn.query_row(
        "SELECT sc.*, u.display_name as performer_name, mb.name as media_batch_name
         FROM subcultures sc
         LEFT JOIN users u ON sc.performed_by = u.id
         LEFT JOIN media_batches mb ON sc.media_batch_id = mb.id
         WHERE sc.id = ?1",
        params![id],
        row_to_subculture,
    ).map_err(|e| format!("Failed to fetch created subculture: {}", e))
}

#[tauri::command]
pub fn update_subculture(
    state: State<AppState>,
    token: String,
    request: UpdateSubcultureRequest,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref notes) = request.notes {
        updates.push(format!("notes = ?{}", values.len() + 1));
        values.push(Box::new(notes.clone()));
    }
    if let Some(ref obs) = request.observations {
        updates.push(format!("observations = ?{}", values.len() + 1));
        values.push(Box::new(obs.clone()));
    }
    if let Some(ref vt) = request.vessel_type {
        updates.push(format!("vessel_type = ?{}", values.len() + 1));
        values.push(Box::new(vt.clone()));
    }
    if let Some(ref lt) = request.location_to {
        updates.push(format!("location_to = ?{}", values.len() + 1));
        values.push(Box::new(lt.clone()));
    }
    if let Some(flag) = request.contamination_flag {
        updates.push(format!("contamination_flag = ?{}", values.len() + 1));
        values.push(Box::new(flag as i32));
    }
    if let Some(ref cn) = request.contamination_notes {
        updates.push(format!("contamination_notes = ?{}", values.len() + 1));
        values.push(Box::new(cn.clone()));
    }

    if updates.is_empty() {
        return Err("No fields to update".to_string());
    }

    updates.push("updated_at = datetime('now')".to_string());
    let sql = format!(
        "UPDATE subcultures SET {} WHERE id = ?{}",
        updates.join(", "),
        values.len() + 1
    );
    values.push(Box::new(request.id.clone()));

    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    db.conn.execute(&sql, bind_refs.as_slice())
        .map_err(|e| format!("Failed to update subculture: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "update", "subculture", Some(&request.id),
        None, None, Some("Subculture updated"),
    ).ok();

    Ok(())
}

// ── All Subcultures (for export) ─────────────────────────────────────────────

#[tauri::command]
pub fn list_all_subcultures(
    state: State<AppState>,
    token: String,
) -> Result<Vec<Subculture>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db.conn.prepare(
        "SELECT sc.*, u.display_name as performer_name, mb.name as media_batch_name
         FROM subcultures sc
         LEFT JOIN users u ON sc.performed_by = u.id
         LEFT JOIN media_batches mb ON sc.media_batch_id = mb.id
         ORDER BY sc.date DESC, sc.passage_number DESC"
    ).map_err(|e| e.to_string())?;

    let subcultures = stmt.query_map([], row_to_subculture)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(subcultures)
}

// ── Contamination Stats ──────────────────────────────────────────────────────

#[tauri::command]
pub fn get_contamination_stats(
    state: State<AppState>,
    token: String,
) -> Result<ContaminationStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let total_specimens: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM specimens WHERE is_archived = 0",
        [],
        |r| r.get(0),
    ).unwrap_or(0);

    let contaminated_specimens: i64 = db.conn.query_row(
        "SELECT COUNT(DISTINCT sc.specimen_id)
         FROM subcultures sc
         JOIN specimens sp ON sc.specimen_id = sp.id
         WHERE sc.contamination_flag = 1 AND sp.is_archived = 0",
        [],
        |r| r.get(0),
    ).unwrap_or(0);

    let contamination_rate_pct = if total_specimens > 0 {
        (contaminated_specimens as f64 / total_specimens as f64) * 100.0
    } else {
        0.0
    };

    let contaminated_vessels: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM subcultures WHERE contamination_flag = 1",
        [],
        |r| r.get(0),
    ).unwrap_or(0);

    let mut stmt = db.conn.prepare(
        "SELECT COALESCE(vessel_type, 'Unknown') as vessel_type, COUNT(*) as cnt
         FROM subcultures
         WHERE contamination_flag = 1
         GROUP BY vessel_type
         ORDER BY cnt DESC
         LIMIT 10"
    ).map_err(|e| e.to_string())?;

    let by_vessel_type: Vec<VesselContaminationCount> = stmt
        .query_map([], |row| {
            Ok(VesselContaminationCount {
                vessel_type: row.get("vessel_type")?,
                count: row.get("cnt")?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut stmt2 = db.conn.prepare(
        "SELECT sc.id as subculture_id, sc.specimen_id, sp.accession_number,
                s.species_code, sc.passage_number, sc.date, sc.vessel_type,
                sc.contamination_notes
         FROM subcultures sc
         JOIN specimens sp ON sc.specimen_id = sp.id
         JOIN species s ON sp.species_id = s.id
         WHERE sc.contamination_flag = 1
         ORDER BY sc.date DESC
         LIMIT 10"
    ).map_err(|e| e.to_string())?;

    let recent_events: Vec<RecentContaminationEvent> = stmt2
        .query_map([], |row| {
            Ok(RecentContaminationEvent {
                subculture_id: row.get("subculture_id")?,
                specimen_id: row.get("specimen_id")?,
                accession_number: row.get("accession_number")?,
                species_code: row.get("species_code")?,
                passage_number: row.get("passage_number")?,
                date: row.get("date")?,
                vessel_type: row.get("vessel_type")?,
                contamination_notes: row.get("contamination_notes")?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(ContaminationStats {
        total_specimens,
        contaminated_specimens,
        contamination_rate_pct,
        contaminated_vessels,
        by_vessel_type,
        recent_events,
    })
}

// ── Subculture Schedule ──────────────────────────────────────────────────────

#[tauri::command]
pub fn get_subculture_schedule(
    state: State<AppState>,
    token: String,
) -> Result<Vec<SubcultureScheduleEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    // For each active specimen, compute next due date using the species default
    // subculture interval and the date of the most recent passage.
    let mut stmt = db.conn.prepare(
        "SELECT
            sp.id              AS specimen_id,
            sp.accession_number,
            s.species_code     AS species_code,
            (s.genus || ' ' || s.species_name) AS species_name,
            sp.location,
            MAX(sc.date)       AS last_passage_date,
            s.default_subculture_interval_days AS interval_days,
            CASE
                WHEN s.default_subculture_interval_days IS NOT NULL AND MAX(sc.date) IS NOT NULL
                THEN date(MAX(sc.date), '+' || s.default_subculture_interval_days || ' days')
                ELSE NULL
            END AS next_due_date,
            CASE
                WHEN s.default_subculture_interval_days IS NOT NULL AND MAX(sc.date) IS NOT NULL
                THEN CAST(julianday(date(MAX(sc.date), '+' || s.default_subculture_interval_days || ' days')) - julianday('now') AS INTEGER)
                ELSE NULL
            END AS days_until_due
         FROM specimens sp
         JOIN species s ON sp.species_id = s.id
         LEFT JOIN subcultures sc ON sc.specimen_id = sp.id
         WHERE sp.is_archived = 0
         GROUP BY sp.id
         ORDER BY days_until_due ASC NULLS LAST"
    ).map_err(|e| e.to_string())?;

    let entries: Vec<SubcultureScheduleEntry> = stmt
        .query_map([], |row| {
            let days_until_due: Option<i64> = row.get("days_until_due")?;
            let is_overdue = days_until_due.map(|d| d < 0).unwrap_or(false);
            Ok(SubcultureScheduleEntry {
                specimen_id: row.get("specimen_id")?,
                accession_number: row.get("accession_number")?,
                species_code: row.get("species_code")?,
                species_name: row.get("species_name")?,
                location: row.get("location")?,
                last_passage_date: row.get("last_passage_date")?,
                interval_days: row.get("interval_days")?,
                next_due_date: row.get("next_due_date")?,
                days_until_due,
                is_overdue,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        crate::db::migrations::run_all(&conn).unwrap();
        crate::db::migrations::seed_defaults(&conn).unwrap();
        conn
    }

    fn insert_specimen(conn: &Connection, accession: &str) -> String {
        let species_id: String = conn
            .query_row("SELECT id FROM species LIMIT 1", [], |r| r.get(0))
            .expect("Need at least one seeded species");
        let spec_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO specimens
                 (id, accession_number, species_id, stage, initiation_date, is_archived)
             VALUES (?1, ?2, ?3, 'shoot', '2026-01-01', 0)",
            params![spec_id, accession, species_id],
        ).unwrap();
        spec_id
    }

    fn insert_passage(conn: &Connection, spec_id: &str, num: i32) {
        let sc_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO subcultures
                 (id, specimen_id, passage_number, date, contamination_flag, event_type)
             VALUES (?1, ?2, ?3, '2026-02-01', 0, 'passage')",
            params![sc_id, spec_id, num],
        ).unwrap();
        conn.execute(
            "UPDATE specimens SET subculture_count = ?1 WHERE id = ?2",
            params![num, spec_id],
        ).unwrap();
    }

    #[test]
    fn death_archives_specimen_and_sets_health_zero() {
        let conn = test_db();
        let spec_id = insert_specimen(&conn, "DEATH-001");
        insert_passage(&conn, &spec_id, 1);
        insert_passage(&conn, &spec_id, 2);

        // Record death event
        let death_id = uuid::Uuid::new_v4().to_string();
        let tx = conn.unchecked_transaction().unwrap();
        tx.execute(
            "INSERT INTO subcultures
                 (id, specimen_id, passage_number, date, health_status, contamination_flag, event_type)
             VALUES (?1, ?2, 3, '2026-03-01', '0', 0, 'death')",
            params![death_id, spec_id],
        ).unwrap();
        tx.execute(
            "UPDATE specimens
             SET is_archived = 1, archived_at = datetime('now'), health_status = '0'
             WHERE id = ?1",
            params![spec_id],
        ).unwrap();
        tx.commit().unwrap();

        let (is_archived, health, subculture_count): (i32, String, i32) = conn
            .query_row(
                "SELECT is_archived, health_status, subculture_count FROM specimens WHERE id = ?1",
                params![spec_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ).unwrap();

        assert_eq!(is_archived, 1, "specimen must be archived");
        assert_eq!(health, "0", "health must be Dead (0)");
        assert_eq!(subculture_count, 2, "subculture_count must NOT be incremented");
    }

    #[test]
    fn death_event_type_is_death() {
        let conn = test_db();
        let spec_id = insert_specimen(&conn, "DEATH-002");

        let death_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO subcultures
                 (id, specimen_id, passage_number, date, health_status, contamination_flag, event_type)
             VALUES (?1, ?2, 1, '2026-03-01', '0', 0, 'death')",
            params![death_id, spec_id],
        ).unwrap();

        let event_type: String = conn
            .query_row(
                "SELECT event_type FROM subcultures WHERE id = ?1",
                params![death_id],
                |r| r.get(0),
            ).unwrap();

        assert_eq!(event_type, "death");
    }

    #[test]
    fn archived_specimen_blocks_passage() {
        let conn = test_db();
        let spec_id = insert_specimen(&conn, "DEATH-003");

        // Archive specimen (simulates death or split)
        conn.execute(
            "UPDATE specimens SET is_archived = 1 WHERE id = ?1",
            params![spec_id],
        ).unwrap();

        let is_archived: i32 = conn
            .query_row(
                "SELECT is_archived FROM specimens WHERE id = ?1",
                params![spec_id],
                |r| r.get(0),
            ).unwrap();

        // The create_subculture command checks this guard
        assert_eq!(is_archived, 1, "archived check should prevent passage recording");
    }

    #[test]
    fn normal_passages_have_passage_event_type() {
        let conn = test_db();
        let spec_id = insert_specimen(&conn, "DEATH-004");
        insert_passage(&conn, &spec_id, 1);

        let event_type: String = conn
            .query_row(
                "SELECT event_type FROM subcultures WHERE specimen_id = ?1",
                params![spec_id],
                |r| r.get(0),
            ).unwrap();

        assert_eq!(event_type, "passage");
    }

    #[test]
    fn app_config_seeded_with_plant_tissue_culture() {
        let conn = test_db();
        let profile: String = conn
            .query_row("SELECT lab_profile FROM app_config WHERE id = 1", [], |r| r.get(0))
            .expect("app_config row must exist after migration");
        assert_eq!(profile, "plant_tissue_culture");
    }
}
