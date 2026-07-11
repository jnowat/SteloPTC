// WP-72: Cross-lab breeding program coordination — command layer.
//
// Thin session/role gating over `crate::coordination::store`. Exporting and
// importing require a write-capable role (they produce a signed attestation / fold
// a foreign lab's selection records into this lab's audit chain and breeding
// program); verifying, previewing, and listing are read-only for any authenticated
// user.
use tauri::State;

use crate::auth as auth_service;
use crate::coordination::store::{self, SelectionDecision};
use crate::coordination::{BundleVerification, CoordinationBundle};
use crate::AppState;

/// Export a signed coordination bundle for one breeding program and record it.
#[tauri::command]
pub fn export_coordination_bundle(
    state: State<AppState>,
    token: String,
    program_id: String,
) -> Result<CoordinationBundle, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions — a write-capable role is required to export a coordination bundle.".to_string());
    }
    let bundle = store::export_bundle(&db.conn, &program_id, Some(&user.id))?;
    crate::db::queries::log_audit(
        &db.conn,
        Some(&user.id),
        "export",
        "breeding_coordination",
        Some(&bundle.bundle_id),
        None,
        Some(&bundle.content_hash),
        Some(&format!(
            "Exported a signed breeding-coordination bundle for '{}' ({} records).",
            bundle.program.name,
            bundle.records.len()
        )),
    )
    .ok();
    Ok(bundle)
}

/// Verify a bundle JSON with no side effects (no import). Read-only.
#[tauri::command]
pub fn verify_coordination_bundle(
    state: State<AppState>,
    token: String,
    bundle_json: String,
) -> Result<BundleVerification, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::verify_bundle_json(&bundle_json)
}

/// Preview a bundle import: verify it and compute a per-record merge plan against
/// the local copy of the program. Read-only, no side effects.
#[tauri::command]
pub fn preview_coordination_import(
    state: State<AppState>,
    token: String,
    bundle_json: String,
) -> Result<store::BundleImportPreview, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::preview_import(&db.conn, &bundle_json)
}

/// Verify and import a received bundle, applying each record's disposition
/// (`accept`/`skip`) and folding the merge into this lab's audit chain.
#[tauri::command]
pub fn import_coordination_bundle(
    state: State<AppState>,
    token: String,
    bundle_json: String,
    decisions: Option<Vec<SelectionDecision>>,
) -> Result<store::BundleImportResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions — a write-capable role is required to import a coordination bundle.".to_string());
    }
    store::import_bundle(&db.conn, &bundle_json, &decisions.unwrap_or_default(), Some(&user.id))
}

/// List bundle register rows, optionally filtered by direction
/// (`issued`/`imported`). Read-only.
#[tauri::command]
pub fn list_coordination_bundles(
    state: State<AppState>,
    token: String,
    direction: Option<String>,
) -> Result<Vec<store::BundleRow>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::list_bundles(&db.conn, direction.as_deref())
}

/// Fetch a stored bundle's full JSON for re-export. Read-only.
#[tauri::command]
pub fn get_coordination_bundle_json(
    state: State<AppState>,
    token: String,
    row_id: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::get_bundle_json(&db.conn, &row_id)
}

/// Fetch the recorded per-record dispositions for one imported bundle. Read-only.
#[tauri::command]
pub fn list_coordination_dispositions(
    state: State<AppState>,
    token: String,
    bundle_row_id: String,
) -> Result<Vec<store::AppliedSelection>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::list_dispositions(&db.conn, &bundle_row_id)
}
