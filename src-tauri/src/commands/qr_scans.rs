use crate::auth as auth_service;
use crate::AppState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct QrScan {
    pub id: String,
    pub raw_data: String,
    pub accession_number: Option<String>,
    pub scanned_by: Option<String>,
    pub scanned_at: String,
}

/// Store a QR scan event in the database.
#[tauri::command]
pub fn store_qr_scan(
    state: State<AppState>,
    token: String,
    raw_data: String,
    accession_number: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;

    let id = uuid::Uuid::new_v4().to_string();

    db.conn
        .execute(
            "INSERT INTO qr_scans (id, raw_data, accession_number, scanned_by)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, raw_data, accession_number, user.id],
        )
        .map_err(|e| format!("Failed to store QR scan: {}", e))?;

    Ok(())
}

/// List recent QR scan events (newest first, capped at 200).
#[tauri::command]
pub fn list_qr_scans(
    state: State<AppState>,
    token: String,
) -> Result<Vec<QrScan>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;

    let mut stmt = db
        .conn
        .prepare(
            "SELECT id, raw_data, accession_number, scanned_by, scanned_at
             FROM qr_scans
             ORDER BY scanned_at DESC
             LIMIT 200",
        )
        .map_err(|e| e.to_string())?;

    let scans = stmt
        .query_map([], |row| {
            Ok(QrScan {
                id: row.get(0)?,
                raw_data: row.get(1)?,
                accession_number: row.get(2)?,
                scanned_by: row.get(3)?,
                scanned_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(scans)
}
