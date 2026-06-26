use crate::auth as auth_service;
use crate::db::queries;
use crate::models::fruiting::{CreateFruitingRecordRequest, FruitingRecord};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn create_fruiting_record(
    state: State<AppState>,
    token: String,
    request: CreateFruitingRecordRequest,
) -> Result<FruitingRecord, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }
    let id = queries::create_fruiting_record(&db.conn, &request, Some(&user.id))
        .map_err(|e| format!("Failed to create fruiting record: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "create", "fruiting_record", Some(&id),
        None, None, Some("Fruiting record added"),
    ).ok();

    queries::get_fruiting_record(&db.conn, &id)
        .map_err(|e| format!("Failed to retrieve fruiting record: {}", e))
}

#[tauri::command]
pub fn list_fruiting_records(
    state: State<AppState>,
    token: String,
    specimen_id: String,
) -> Result<Vec<FruitingRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    queries::list_fruiting_records(&db.conn, &specimen_id)
        .map_err(|e| format!("Failed to list fruiting records: {}", e))
}
