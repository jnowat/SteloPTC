// WP-76: Lab data-integrity self-check — command layer.
//
// Thin admin-gated wrapper over `crate::integrity`. The scan is read-only, but
// it can surface the identifiers of corrupt/orphaned rows, so it is restricted
// to administrators (the same audience as the audit-verification tools).
use tauri::State;

use crate::auth as auth_service;
use crate::integrity;
use crate::AppState;

/// Run every data-integrity check and return the report. Admin-only.
#[tauri::command]
pub fn run_data_integrity_check(
    state: State<AppState>,
    token: String,
) -> Result<integrity::IntegrityReport, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Data-integrity checks are restricted to administrators".to_string());
    }
    integrity::run_integrity_check(&db.conn)
}
