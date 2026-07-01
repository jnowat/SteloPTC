//! WP-55 — Field-level permissions command surface.

use crate::auth as auth_service;
use crate::db::permissions;
use crate::models::permissions::{FieldPermission, SetFieldPermissionRequest};
use crate::AppState;
use tauri::State;

/// Admin-only: full matrix for the PermissionsEditor UI.
#[tauri::command]
pub fn list_field_permissions(state: State<AppState>, token: String) -> Result<Vec<FieldPermission>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    permissions::validate_admin_role(user.role.as_str())?;
    permissions::list_field_permissions(&db.conn).map_err(|e| e.to_string())
}

/// Admin-only: toggle visibility for one (role, entity_type, field_name) triple.
/// Takes effect immediately — every read command queries `field_permissions`
/// live, there is nothing to invalidate or restart.
#[tauri::command]
pub fn set_field_permission(
    state: State<AppState>,
    token: String,
    request: SetFieldPermissionRequest,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    permissions::validate_admin_role(user.role.as_str())?;

    permissions::set_field_permission(
        &db.conn,
        &request.role,
        &request.entity_type,
        &request.field_name,
        request.visible,
    )
    .map_err(|e| e.to_string())?;

    crate::db::queries::log_audit(
        &db.conn,
        Some(&user.id),
        "update",
        "field_permission",
        None,
        None,
        None,
        Some(&format!(
            "{}.{} visibility for role '{}' set to {}",
            request.entity_type, request.field_name, request.role, request.visible
        )),
    )
    .ok();

    Ok(())
}
