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
    // WP-55 defense-in-depth: never let the literal "[RESTRICTED]" marker be
    // persisted into a masked field, even on create. There is no
    // update_breeding_program path today (so no read-masked value can
    // round-trip here the way it could for strains), but applying the same
    // guard the strain write path uses means a future edit path — or a
    // client that echoes a masked value straight back — can never corrupt
    // these fields with the placeholder string. Cheap, uniform, and matches
    // the pattern already established in `strains::update_strain_status`.
    crate::db::permissions::reject_if_restricted_marker(request.goal.as_deref(), "Breeding program goal")?;
    crate::db::permissions::reject_if_restricted_marker(request.target_traits.as_deref(), "Breeding program target traits")?;
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
/// Takes a pre-loaded [`crate::db::permissions::FieldPermissionSet`] rather
/// than querying per call — see the matching comment in `commands::strains`
/// for why (fixes an N+1 query pattern across list results).
///
/// There is currently no `update_breeding_program` command, so unlike
/// `strains::update_strain_status` there is no write path that could
/// round-trip a masked "[RESTRICTED]" value back into these fields.
/// `create_breeding_program` nonetheless already rejects the marker on both
/// fields as defense-in-depth (see its body). If an update command is added
/// later, it **must** apply the same
/// `crate::db::permissions::reject_if_restricted_marker` guard before
/// persisting — see the WP-55 write-path-guard doc comment in
/// `db::permissions` for why.
fn apply_field_permissions(perms: &crate::db::permissions::FieldPermissionSet, mut program: BreedingProgram) -> BreedingProgram {
    program.goal = perms.mask_optional_field("breeding_program", "goal", program.goal);
    program.target_traits = perms.mask_optional_field("breeding_program", "target_traits", program.target_traits);
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
    let perms = crate::db::permissions::FieldPermissionSet::load(&db.conn, user.role.as_str())
        .map_err(|e| e.to_string())?;
    Ok(programs
        .into_iter()
        .map(|p| apply_field_permissions(&perms, p))
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
    let perms = crate::db::permissions::FieldPermissionSet::load(&db.conn, user.role.as_str())
        .map_err(|e| e.to_string())?;
    Ok(apply_field_permissions(&perms, program))
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
