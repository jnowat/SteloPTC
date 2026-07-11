// WP-70: Federated identity & inter-lab specimen transfer — command layer.
//
// Thin session/role gating over `crate::passport::store`. Issuing and importing
// require a write-capable role (they produce a signed attestation / fold a
// foreign record into this lab's audit chain); viewing the lab identity,
// verifying, and listing are read-only for any authenticated user. Setting the
// lab name is a manage-only setting.
use tauri::State;

use crate::auth as auth_service;
use crate::passport::{store, IssuerIdentity, PassportVerification, SpecimenPassport};
use crate::AppState;

/// This lab's public issuer identity (name + Ed25519 public key). Shared
/// out-of-band so partner labs can verify the passports this lab issues.
#[tauri::command]
pub fn get_lab_identity(state: State<AppState>, token: String) -> Result<IssuerIdentity, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::get_lab_identity(&db.conn)
}

/// Set this lab's issuer name (appears in every passport it subsequently issues).
#[tauri::command]
pub fn set_lab_name(state: State<AppState>, token: String, name: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Insufficient permissions — admin or supervisor role required.".to_string());
    }
    store::set_lab_name(&db.conn, &name)?;
    crate::db::queries::log_audit(
        &db.conn,
        Some(&user.id),
        "update",
        "app_settings",
        Some("lab_name"),
        None,
        Some(name.trim()),
        Some("Updated the federation lab name."),
    )
    .ok();
    Ok(())
}

/// Issue a signed specimen passport for a local specimen and record it.
#[tauri::command]
pub fn issue_specimen_passport(
    state: State<AppState>,
    token: String,
    specimen_id: String,
) -> Result<SpecimenPassport, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions — a write-capable role is required to issue a passport.".to_string());
    }
    let passport = store::issue_passport(&db.conn, &specimen_id, Some(&user.id))?;
    crate::db::queries::log_audit(
        &db.conn,
        Some(&user.id),
        "issue",
        "specimen_passport",
        Some(&passport.passport_id),
        None,
        Some(&passport.content_hash),
        Some(&format!(
            "Issued specimen passport for accession {}.",
            passport.specimen.accession_number
        )),
    )
    .ok();
    Ok(passport)
}

/// Verify a passport JSON with no side effects (no import). Read-only.
#[tauri::command]
pub fn verify_specimen_passport(
    state: State<AppState>,
    token: String,
    passport_json: String,
) -> Result<PassportVerification, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::verify_passport_json(&passport_json)
}

/// Verify and import a received passport, folding it into this lab's audit chain.
#[tauri::command]
pub fn import_specimen_passport(
    state: State<AppState>,
    token: String,
    passport_json: String,
) -> Result<store::ImportPassportResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions — a write-capable role is required to import a passport.".to_string());
    }
    store::import_passport(&db.conn, &passport_json, Some(&user.id))
}

/// List passport register rows, optionally filtered by direction
/// (`issued`/`imported`). Read-only.
#[tauri::command]
pub fn list_specimen_passports(
    state: State<AppState>,
    token: String,
    direction: Option<String>,
) -> Result<Vec<store::PassportRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::list_passports(&db.conn, direction.as_deref())
}

/// Fetch a stored passport's full JSON for re-export. Read-only.
#[tauri::command]
pub fn get_specimen_passport_json(
    state: State<AppState>,
    token: String,
    row_id: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::get_passport_json(&db.conn, &row_id)
}
