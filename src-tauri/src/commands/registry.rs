// WP-71: Shared taxonomy registry — command layer.
//
// Thin session/role gating over `crate::registry::store`. Exporting and importing
// require a write-capable role (they produce a signed attestation / fold a foreign
// record into this lab's audit chain and reference tables); verifying, previewing,
// and listing are read-only for any authenticated user.
use tauri::State;

use crate::auth as auth_service;
use crate::registry::store::{self, RecordDecision};
use crate::registry::{RegistryVerification, TaxonomyRegistry};
use crate::AppState;

/// Export a signed taxonomy registry for this lab and record it.
#[tauri::command]
pub fn export_taxonomy_registry(state: State<AppState>, token: String) -> Result<TaxonomyRegistry, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions — a write-capable role is required to export a registry.".to_string());
    }
    let registry = store::export_registry(&db.conn, Some(&user.id))?;
    crate::db::queries::log_audit(
        &db.conn,
        Some(&user.id),
        "export",
        "taxonomy_registry",
        Some(&registry.registry_id),
        None,
        Some(&registry.content_hash),
        Some(&format!(
            "Exported a signed taxonomy registry ({} records).",
            registry.records.len()
        )),
    )
    .ok();
    Ok(registry)
}

/// Verify a registry JSON with no side effects (no import). Read-only.
#[tauri::command]
pub fn verify_taxonomy_registry(
    state: State<AppState>,
    token: String,
    registry_json: String,
) -> Result<RegistryVerification, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::verify_registry_json(&registry_json)
}

/// Preview a registry import: verify it and compute a per-record reconciliation
/// plan against the local database. Read-only, no side effects.
#[tauri::command]
pub fn preview_taxonomy_registry_import(
    state: State<AppState>,
    token: String,
    registry_json: String,
) -> Result<store::RegistryImportPreview, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::preview_import(&db.conn, &registry_json)
}

/// Verify and import a received registry, applying each record's disposition
/// (`accept`/`override`/`fork`) and folding the merge into this lab's audit chain.
#[tauri::command]
pub fn import_taxonomy_registry(
    state: State<AppState>,
    token: String,
    registry_json: String,
    decisions: Option<Vec<RecordDecision>>,
) -> Result<store::RegistryImportResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions — a write-capable role is required to import a registry.".to_string());
    }
    store::import_registry(&db.conn, &registry_json, &decisions.unwrap_or_default(), Some(&user.id))
}

/// List registry register rows, optionally filtered by direction
/// (`issued`/`imported`). Read-only.
#[tauri::command]
pub fn list_taxonomy_registries(
    state: State<AppState>,
    token: String,
    direction: Option<String>,
) -> Result<Vec<store::RegistryRecordRow>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::list_registries(&db.conn, direction.as_deref())
}

/// Fetch a stored registry's full JSON for re-export. Read-only.
#[tauri::command]
pub fn get_taxonomy_registry_json(
    state: State<AppState>,
    token: String,
    row_id: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::get_registry_json(&db.conn, &row_id)
}

/// Fetch the recorded per-record dispositions for one imported registry. Read-only.
#[tauri::command]
pub fn list_registry_dispositions(
    state: State<AppState>,
    token: String,
    registry_row_id: String,
) -> Result<Vec<store::AppliedRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::list_dispositions(&db.conn, &registry_row_id)
}
