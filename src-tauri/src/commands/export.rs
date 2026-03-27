use crate::auth as auth_service;
use crate::AppState;
use tauri::State;
use serde::Serialize;

#[derive(Serialize)]
struct ExportSpecimen {
    accession_number: String,
    species_code: String,
    species_name: String,
    stage: String,
    custom_stage: Option<String>,
    provenance: Option<String>,
    source_plant: Option<String>,
    initiation_date: String,
    location: Option<String>,
    location_details: Option<String>,
    propagation_method: Option<String>,
    acclimatization_status: Option<String>,
    health_status: Option<String>,
    disease_status: Option<String>,
    quarantine_flag: bool,
    quarantine_release_date: Option<String>,
    permit_number: Option<String>,
    permit_expiry: Option<String>,
    ip_flag: bool,
    ip_notes: Option<String>,
    environmental_notes: Option<String>,
    subculture_count: i32,
    parent_specimen_id: Option<String>,
    notes: Option<String>,
    employee_id: Option<String>,
    created_by: Option<String>,
    created_at: String,
    updated_at: String,
}

const EXPORT_SQL: &str =
    "SELECT s.accession_number, sp.species_code,
            sp.genus || ' ' || sp.species_name as species_name,
            s.stage, s.custom_stage, s.provenance, s.source_plant,
            s.initiation_date, s.location, s.location_details,
            s.propagation_method, s.acclimatization_status,
            s.health_status, s.disease_status,
            s.quarantine_flag, s.quarantine_release_date,
            s.permit_number, s.permit_expiry,
            s.ip_flag, s.ip_notes, s.environmental_notes,
            s.subculture_count, s.parent_specimen_id,
            s.notes, s.employee_id, s.created_by, s.created_at, s.updated_at
     FROM specimens s
     LEFT JOIN species sp ON s.species_id = sp.id
     WHERE s.is_archived = 0
     ORDER BY s.accession_number";

fn map_export_row(row: &rusqlite::Row) -> rusqlite::Result<ExportSpecimen> {
    Ok(ExportSpecimen {
        accession_number: row.get(0)?,
        species_code: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        species_name: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        stage: row.get(3)?,
        custom_stage: row.get(4)?,
        provenance: row.get(5)?,
        source_plant: row.get(6)?,
        initiation_date: row.get(7)?,
        location: row.get(8)?,
        location_details: row.get(9)?,
        propagation_method: row.get(10)?,
        acclimatization_status: row.get(11)?,
        health_status: row.get(12)?,
        disease_status: row.get(13)?,
        quarantine_flag: row.get::<_, i32>(14)? != 0,
        quarantine_release_date: row.get(15)?,
        permit_number: row.get(16)?,
        permit_expiry: row.get(17)?,
        ip_flag: row.get::<_, i32>(18)? != 0,
        ip_notes: row.get(19)?,
        environmental_notes: row.get(20)?,
        subculture_count: row.get(21)?,
        parent_specimen_id: row.get(22)?,
        notes: row.get(23)?,
        employee_id: row.get(24)?,
        created_by: row.get(25)?,
        created_at: row.get(26)?,
        updated_at: row.get(27)?,
    })
}

#[tauri::command]
pub fn export_specimens_csv(state: State<AppState>, token: String) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db.conn.prepare(EXPORT_SQL).map_err(|e| e.to_string())?;

    let header = "Accession,Species Code,Species,Stage,Custom Stage,Provenance,Source Plant,\
Initiation Date,Location,Location Details,Propagation Method,Acclimatization Status,\
Health Status,Disease Status,Quarantine,Quarantine Release,Permit Number,Permit Expiry,\
IP Flag,IP Notes,Environmental Notes,Subculture Count,Parent Specimen,\
Notes,Employee ID,Created By,Created At,Updated At\n";

    let mut csv = String::from(header);

    let rows = stmt.query_map([], map_export_row).map_err(|e| e.to_string())?;

    for row in rows {
        if let Ok(s) = row {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                escape_csv(&s.accession_number),
                escape_csv(&s.species_code),
                escape_csv(&s.species_name),
                escape_csv(&s.stage),
                escape_csv(&s.custom_stage.unwrap_or_default()),
                escape_csv(&s.provenance.unwrap_or_default()),
                escape_csv(&s.source_plant.unwrap_or_default()),
                escape_csv(&s.initiation_date),
                escape_csv(&s.location.unwrap_or_default()),
                escape_csv(&s.location_details.unwrap_or_default()),
                escape_csv(&s.propagation_method.unwrap_or_default()),
                escape_csv(&s.acclimatization_status.unwrap_or_default()),
                escape_csv(&s.health_status.unwrap_or_default()),
                escape_csv(&s.disease_status.unwrap_or_default()),
                if s.quarantine_flag { "Yes" } else { "No" },
                escape_csv(&s.quarantine_release_date.unwrap_or_default()),
                escape_csv(&s.permit_number.unwrap_or_default()),
                escape_csv(&s.permit_expiry.unwrap_or_default()),
                if s.ip_flag { "Yes" } else { "No" },
                escape_csv(&s.ip_notes.unwrap_or_default()),
                escape_csv(&s.environmental_notes.unwrap_or_default()),
                s.subculture_count,
                escape_csv(&s.parent_specimen_id.unwrap_or_default()),
                escape_csv(&s.notes.unwrap_or_default()),
                escape_csv(&s.employee_id.unwrap_or_default()),
                escape_csv(&s.created_by.unwrap_or_default()),
                escape_csv(&s.created_at),
                escape_csv(&s.updated_at),
            ));
        }
    }

    Ok(csv)
}

#[tauri::command]
pub fn export_specimens_json(state: State<AppState>, token: String) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db.conn.prepare(EXPORT_SQL).map_err(|e| e.to_string())?;

    let specimens: Vec<ExportSpecimen> = stmt
        .query_map([], map_export_row)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    serde_json::to_string_pretty(&specimens).map_err(|e| e.to_string())
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
