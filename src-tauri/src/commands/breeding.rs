use crate::auth as auth_service;
use crate::db::queries;
use crate::models::breeding::{
    BreedingProgram, BreedingRecord, CreateBreedingProgramRequest, CreateBreedingRecordRequest,
    GenerationalSummary,
};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn create_breeding_program(
    state: State<AppState>,
    token: String,
    request: CreateBreedingProgramRequest,
) -> Result<BreedingProgram, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }
    let id = queries::create_breeding_program(&db.conn, &request, Some(&user.id))
        .map_err(|e| format!("Failed to create breeding program: {}", e))?;
    queries::log_audit(
        &db.conn, Some(&user.id), "create", "breeding_program", Some(&id),
        None, None, Some("Breeding program created"),
    ).ok();
    queries::get_breeding_program(&db.conn, &id)
        .map_err(|e| format!("Failed to retrieve breeding program: {}", e))
}

/// WP-55: masks `goal` and `target_traits` per the calling user's role.
fn apply_field_permissions(conn: &rusqlite::Connection, role: &str, mut program: BreedingProgram) -> BreedingProgram {
    program.goal = crate::db::permissions::mask_optional_field(conn, role, "breeding_program", "goal", program.goal);
    program.target_traits =
        crate::db::permissions::mask_optional_field(conn, role, "breeding_program", "target_traits", program.target_traits);
    program
}

#[tauri::command]
pub fn list_breeding_programs(
    state: State<AppState>,
    token: String,
) -> Result<Vec<BreedingProgram>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    let programs = queries::list_breeding_programs(&db.conn)
        .map_err(|e| format!("Failed to list breeding programs: {}", e))?;
    Ok(programs
        .into_iter()
        .map(|p| apply_field_permissions(&db.conn, user.role.as_str(), p))
        .collect())
}

#[tauri::command]
pub fn get_breeding_program(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<BreedingProgram, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    let program = queries::get_breeding_program(&db.conn, &id)
        .map_err(|e| format!("Failed to get breeding program: {}", e))?;
    Ok(apply_field_permissions(&db.conn, user.role.as_str(), program))
}

#[tauri::command]
pub fn add_breeding_record(
    state: State<AppState>,
    token: String,
    request: CreateBreedingRecordRequest,
) -> Result<BreedingRecord, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }
    let id = queries::add_breeding_record(&db.conn, &request, Some(&user.id))
        .map_err(|e| format!("Failed to add breeding record: {}", e))?;
    queries::log_audit(
        &db.conn, Some(&user.id), "create", "breeding_record", Some(&id),
        None, None, Some("Breeding record added"),
    ).ok();
    queries::get_breeding_record(&db.conn, &id)
        .map_err(|e| format!("Failed to retrieve breeding record: {}", e))
}

#[tauri::command]
pub fn list_breeding_records_for_program(
    state: State<AppState>,
    token: String,
    program_id: String,
) -> Result<Vec<BreedingRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    queries::list_breeding_records_for_program(&db.conn, &program_id)
        .map_err(|e| format!("Failed to list breeding records: {}", e))
}

#[tauri::command]
pub fn list_breeding_records_for_strain(
    state: State<AppState>,
    token: String,
    strain_id: String,
) -> Result<Vec<BreedingRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    queries::list_breeding_records_for_strain(&db.conn, &strain_id)
        .map_err(|e| format!("Failed to list breeding records for strain: {}", e))
}

#[tauri::command]
pub fn get_generational_summary(
    state: State<AppState>,
    token: String,
    program_id: String,
) -> Result<Vec<GenerationalSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    queries::get_generational_summary(&db.conn, &program_id)
        .map_err(|e| format!("Failed to get generational summary: {}", e))
}
