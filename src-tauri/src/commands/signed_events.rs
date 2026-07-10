// WP-67: Trust Layer Phase 3 — signed-event ledger command layer.
//
// Thin session/role gating over `crate::signed_ledger`. Recording an event signs
// it with the *acting user's* key, so any write-capable user may record one for
// their own action. Listing and verifying the ledger are read-only for any
// authenticated user.
use tauri::State;

use crate::auth as auth_service;
use crate::signed_ledger;
use crate::AppState;

/// The caller's Ed25519 public key (generating one on first use). Lets a user
/// publish the key others verify their signed events against.
#[tauri::command]
pub fn get_user_signing_public_key(state: State<AppState>, token: String) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    let (public_key, _) = signed_ledger::load_or_create_user_signing_key(&db.conn, &user.id)?;
    Ok(public_key)
}

/// Record a signed transaction for a lifecycle event, signed with the acting
/// user's key.
#[tauri::command]
pub fn record_signed_event(
    state: State<AppState>,
    token: String,
    event_type: String,
    entity_type: String,
    entity_id: Option<String>,
    payload: String,
) -> Result<signed_ledger::SignedEvent, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions — a write-capable role is required to sign an event.".to_string());
    }
    let event = signed_ledger::append_signed_event(
        &db.conn,
        &user.id,
        &event_type,
        &entity_type,
        entity_id.as_deref(),
        &payload,
    )?;
    crate::db::queries::log_audit(
        &db.conn,
        Some(&user.id),
        "sign_event",
        "signed_event",
        Some(&event.id),
        None,
        Some(&event.event_hash),
        Some(&format!("Signed ledger event #{} ({})", event.seq, event.event_type)),
    )
    .ok();
    Ok(event)
}

/// List signed events, optionally scoped to one entity. Read-only.
#[tauri::command]
pub fn list_signed_events(
    state: State<AppState>,
    token: String,
    entity_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<signed_ledger::SignedEvent>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    signed_ledger::list_signed_events(&db.conn, entity_id.as_deref(), limit.unwrap_or(100))
}

/// Verify the full signed-event ledger: hash chain + gapless sequence + every
/// signature. Read-only.
#[tauri::command]
pub fn verify_signed_event_ledger(
    state: State<AppState>,
    token: String,
) -> Result<signed_ledger::LedgerVerification, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    signed_ledger::verify_ledger(&db.conn)
}
