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
    provenance: Option<String>,
    initiation_date: String,
    location: Option<String>,
    health_status: Option<String>,
    quarantine_flag: bool,
    subculture_count: i32,
    notes: Option<String>,
}

#[tauri::command]
pub fn export_specimens_csv(state: State<AppState>, token: String) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db.conn.prepare(
        "SELECT s.accession_number, sp.species_code,
                sp.genus || ' ' || sp.species_name as species_name,
                s.stage, s.provenance, s.initiation_date, s.location,
                s.health_status, s.quarantine_flag, s.subculture_count, s.notes
         FROM specimens s
         LEFT JOIN species sp ON s.species_id = sp.id
         WHERE s.is_archived = 0
         ORDER BY s.accession_number"
    ).map_err(|e| e.to_string())?;

    let mut csv = String::from("Accession,Species Code,Species,Stage,Provenance,Initiation Date,Location,Health Status,Quarantine,Subculture Count,Notes\n");

    let rows = stmt.query_map([], |row| {
        let qf: i32 = row.get(8)?;
        Ok(format!(
            "{},{},{},{},{},{},{},{},{},{},{}",
            escape_csv(&row.get::<_, String>(0)?),
            escape_csv(&row.get::<_, Option<String>>(1)?.unwrap_or_default()),
            escape_csv(&row.get::<_, Option<String>>(2)?.unwrap_or_default()),
            escape_csv(&row.get::<_, String>(3)?),
            escape_csv(&row.get::<_, Option<String>>(4)?.unwrap_or_default()),
            escape_csv(&row.get::<_, String>(5)?),
            escape_csv(&row.get::<_, Option<String>>(6)?.unwrap_or_default()),
            escape_csv(&row.get::<_, Option<String>>(7)?.unwrap_or_default()),
            if qf != 0 { "Yes" } else { "No" },
            row.get::<_, i32>(9)?,
            escape_csv(&row.get::<_, Option<String>>(10)?.unwrap_or_default()),
        ))
    }).map_err(|e| e.to_string())?;

    for row in rows {
        if let Ok(line) = row {
            csv.push_str(&line);
            csv.push('\n');
        }
    }

    Ok(csv)
}

#[tauri::command]
pub fn export_specimens_json(state: State<AppState>, token: String) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db.conn.prepare(
        "SELECT s.accession_number, sp.species_code,
                sp.genus || ' ' || sp.species_name as species_name,
                s.stage, s.provenance, s.initiation_date, s.location,
                s.health_status, s.quarantine_flag, s.subculture_count, s.notes
         FROM specimens s
         LEFT JOIN species sp ON s.species_id = sp.id
         WHERE s.is_archived = 0
         ORDER BY s.accession_number"
    ).map_err(|e| e.to_string())?;

    let specimens: Vec<ExportSpecimen> = stmt.query_map([], |row| {
        Ok(ExportSpecimen {
            accession_number: row.get(0)?,
            species_code: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            species_name: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            stage: row.get(3)?,
            provenance: row.get(4)?,
            initiation_date: row.get(5)?,
            location: row.get(6)?,
            health_status: row.get(7)?,
            quarantine_flag: row.get::<_, i32>(8)? != 0,
            subculture_count: row.get(9)?,
            notes: row.get(10)?,
        })
    }).map_err(|e| e.to_string())?
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
