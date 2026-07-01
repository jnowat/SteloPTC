//! WP-54 — Environmental sensor integration command surface.

use crate::auth as auth_service;
use crate::db::sensors as sensor_queries;
use crate::models::sensors::{CreateEnvironmentalReadingRequest, EnvironmentalAlert, EnvironmentalReading};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn create_environmental_reading(
    state: State<AppState>,
    token: String,
    request: CreateEnvironmentalReadingRequest,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let id = sensor_queries::create_environmental_reading(&db.conn, &request, Some(&user.id))?;

    crate::db::queries::log_audit(
        &db.conn,
        Some(&user.id),
        "create",
        "environmental_reading",
        Some(&id),
        None,
        None,
        Some(&format!(
            "{} = {} ({})",
            request.reading_type,
            request.value,
            request.source.as_deref().unwrap_or("manual")
        )),
    )
    .ok();

    Ok(id)
}

/// Parses a raw sensor payload (serial line or JSON) and records one reading
/// per recognized field. This is the transport-agnostic entry point a future
/// USB/BLE/MQTT listener would call for each incoming message — see
/// `db::sensors` module docs for what is and isn't wired to real hardware
/// in this packet.
#[tauri::command]
pub fn ingest_sensor_payload(
    state: State<AppState>,
    token: String,
    specimen_id: Option<String>,
    subculture_id: Option<String>,
    source: String,
    raw_payload: String,
) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let parsed = sensor_queries::parse_sensor_payload(&raw_payload)?;
    let mut ids = Vec::with_capacity(parsed.len());
    for reading in parsed {
        let req = CreateEnvironmentalReadingRequest {
            specimen_id: specimen_id.clone(),
            subculture_id: subculture_id.clone(),
            reading_type: reading.reading_type,
            value: reading.value,
            unit: None,
            source: Some(source.clone()),
            recorded_at: None,
            notes: None,
        };
        ids.push(sensor_queries::create_environmental_reading(&db.conn, &req, Some(&user.id))?);
    }
    Ok(ids)
}

#[tauri::command]
pub fn list_environmental_readings(
    state: State<AppState>,
    token: String,
    specimen_id: String,
    limit: Option<i64>,
) -> Result<Vec<EnvironmentalReading>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    sensor_queries::list_environmental_readings(&db.conn, Some(&specimen_id), limit.unwrap_or(100))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_environmental_alerts(state: State<AppState>, token: String) -> Result<Vec<EnvironmentalAlert>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    sensor_queries::get_environmental_alerts(&db.conn).map_err(|e| e.to_string())
}
