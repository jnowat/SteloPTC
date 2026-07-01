use crate::auth as auth_service;
use crate::db::queries;
use crate::models::cryo::{
    CreateFrozenVialRequest, DiscardFrozenVialRequest, FrozenVial, ListFrozenVialsParams,
    ThawVialRequest, ThawVialResult,
};
use crate::models::subculture::VialLineSummary;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn create_frozen_vial(
    state: State<AppState>,
    token: String,
    request: CreateFrozenVialRequest,
) -> Result<FrozenVial, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }
    let id = queries::create_frozen_vial(&db.conn, &request, Some(&user.id))
        .map_err(|e| format!("Failed to create frozen vial: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "create", "frozen_vial", Some(&id),
        None, None, Some("Frozen vial lot recorded"),
    ).ok();

    queries::get_frozen_vial(&db.conn, &id)
        .map_err(|e| format!("Failed to retrieve frozen vial: {}", e))
}

#[tauri::command]
pub fn list_frozen_vials(
    state: State<AppState>,
    token: String,
    params: Option<ListFrozenVialsParams>,
) -> Result<Vec<FrozenVial>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let p = params.unwrap_or(ListFrozenVialsParams {
        species_id: None,
        specimen_id: None,
        status: None,
        location_freezer: None,
    });
    queries::list_frozen_vials(&db.conn, &p)
        .map_err(|e| format!("Failed to list frozen vials: {}", e))
}

#[tauri::command]
pub fn get_frozen_vial(
    state: State<AppState>,
    token: String,
    id: String,
) -> Result<FrozenVial, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    queries::get_frozen_vial(&db.conn, &id)
        .map_err(|e| format!("Frozen vial not found: {}", e))
}

#[tauri::command]
pub fn thaw_vial(
    state: State<AppState>,
    token: String,
    request: ThawVialRequest,
) -> Result<ThawVialResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let vials_to_thaw = request.vials_to_thaw.unwrap_or(1);
    let (specimen_id, accession) = queries::thaw_frozen_vial(
        &db.conn,
        &request.vial_id,
        &request.thaw_date,
        vials_to_thaw,
        request.location.as_deref(),
        request.notes.as_deref(),
        request.employee_id.as_deref(),
        Some(&user.id),
    ).map_err(|e| format!("Thaw failed: {}", e))?;

    // WP-63: thawing a vial inserts a brand-new, non-archived specimen
    // (stage 'thaw_recovery'), which changes total/active/by-stage/by-species
    // dashboard counts. Every other specimen-creating path invalidates the
    // materialized dashboard cache; this one was missed, so a thawed specimen
    // would not appear on the dashboard for up to the 60s TTL. Invalidate here
    // to match the create_specimen / split_specimen / import paths.
    crate::db::dashboard::invalidate_dashboard_cache(&state.dashboard_cache);

    let updated_vial = queries::get_frozen_vial(&db.conn, &request.vial_id)
        .map_err(|e| format!("Failed to retrieve updated vial: {}", e))?;

    Ok(ThawVialResult {
        updated_vial,
        new_specimen_id: specimen_id,
        new_specimen_accession: accession,
    })
}

#[tauri::command]
pub fn discard_frozen_vial(
    state: State<AppState>,
    token: String,
    request: DiscardFrozenVialRequest,
) -> Result<FrozenVial, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }
    queries::discard_frozen_vial(&db.conn, &request.vial_id, request.notes.as_deref())
        .map_err(|e| format!("Discard failed: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "discard", "frozen_vial", Some(&request.vial_id),
        Some("active"), Some("discarded"), request.notes.as_deref(),
    ).ok();

    queries::get_frozen_vial(&db.conn, &request.vial_id)
        .map_err(|e| format!("Failed to retrieve vial: {}", e))
}

#[tauri::command]
pub fn get_vial_summary_by_line(
    state: State<AppState>,
    token: String,
) -> Result<Vec<VialLineSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    crate::db::dashboard::query_vial_summary_by_line(&db.conn)
}
