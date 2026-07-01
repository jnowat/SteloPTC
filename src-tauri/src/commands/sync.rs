//! WP-51 — Tauri command surface for the LAN sync foundation.
//!
//! These commands expose change detection and conflict recording built on
//! the existing audit hash chain. There is no networking/transport layer yet
//! — `get_changes_since_cursor` and `apply_incoming_changes` are the API
//! surface a future LAN discovery/transport layer will call, but nothing in
//! this packet moves bytes over a network. `apply_incoming_changes`
//! deliberately does not write accepted changes into specimens/subcultures/
//! etc. — see `ApplyChangesResult::pending_manual_apply` doc comment.

use crate::auth as auth_service;
use crate::db::sync as sync_queries;
use crate::models::sync::{
    ApplyChangesRequest, ApplyChangesResult, ChangeSetResponse, SyncConflict, SyncCursor,
    SyncPeer, SyncStatusResponse,
};
use crate::AppState;
use tauri::State;

const DEFAULT_CHANGE_LIMIT: i64 = 500;

/// Any authenticated user may see a summary of sync state.
#[tauri::command]
pub fn get_sync_status(state: State<AppState>, token: String) -> Result<SyncStatusResponse, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    sync_queries::get_sync_status(&db.conn).map_err(|e| e.to_string())
}

/// Supervisor+: returns audit-chain entries newer than the supplied cursors.
/// This is the read side of the future sync transport's "what's new since I
/// last synced?" request.
#[tauri::command]
pub fn get_changes_since_cursor(
    state: State<AppState>,
    token: String,
    cursors: Vec<SyncCursor>,
    limit: Option<i64>,
) -> Result<ChangeSetResponse, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Insufficient permissions".to_string());
    }

    let limit = limit.unwrap_or(DEFAULT_CHANGE_LIMIT).clamp(1, 5000);
    // Ask for one extra row to cheaply detect whether more remain beyond this page.
    let mut changes = sync_queries::get_changes_since(&db.conn, &cursors, limit + 1)
        .map_err(|e| e.to_string())?;
    let has_more = changes.len() as i64 > limit;
    changes.truncate(limit as usize);

    Ok(ChangeSetResponse { changes, has_more })
}

/// Admin-only. Reconciles a batch of incoming changes against local state.
///
/// Duplicates (matching hash at the same position) are skipped. Conflicts
/// (differing hash at the same position) are durably recorded via
/// `sync_conflicts` — never silently discarded or auto-merged. Genuinely new
/// changes are counted in `pending_manual_apply`; writing them into
/// specimens/subcultures/etc. requires per-entity-type replay handlers that
/// are future work (see module doc comment).
#[tauri::command]
pub fn apply_incoming_changes(
    state: State<AppState>,
    token: String,
    request: ApplyChangesRequest,
) -> Result<ApplyChangesResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can submit incoming sync changes".to_string());
    }

    let detection = sync_queries::detect_sync_conflicts(
        &db.conn,
        &request.changes,
        &request.source_device_id,
    )
    .map_err(|e| e.to_string())?;

    let mut recorded_conflicts = Vec::with_capacity(detection.conflicts.len());
    for conflict in &detection.conflicts {
        sync_queries::record_sync_conflict(&db.conn, conflict).map_err(|e| e.to_string())?;
        recorded_conflicts.push(conflict.clone());
    }

    let result = ApplyChangesResult {
        applied: 0,
        skipped_duplicate: detection.duplicates.len(),
        pending_manual_apply: detection.new_changes.len(),
        conflicts: recorded_conflicts,
    };

    crate::db::queries::log_audit(
        &db.conn,
        Some(&user.id),
        "sync_submit",
        "sync_batch",
        None,
        None,
        None,
        Some(&format!(
            "Received {} changes from device '{}': {} duplicate, {} conflict, {} pending manual apply",
            request.changes.len(),
            request.source_device_id,
            result.skipped_duplicate,
            result.conflicts.len(),
            result.pending_manual_apply,
        )),
    )
    .ok();

    Ok(result)
}

/// Supervisor+: lists recorded sync conflicts.
#[tauri::command]
pub fn list_sync_conflicts(
    state: State<AppState>,
    token: String,
    unresolved_only: Option<bool>,
) -> Result<Vec<SyncConflict>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Insufficient permissions".to_string());
    }
    sync_queries::list_sync_conflicts(&db.conn, unresolved_only.unwrap_or(false))
        .map_err(|e| e.to_string())
}

/// Admin-only. Administratively closes a conflict record. Does not reconcile
/// the underlying data divergence — that requires a human decision about
/// which side's history is authoritative, recorded here as `resolution_note`.
#[tauri::command]
pub fn resolve_sync_conflict(
    state: State<AppState>,
    token: String,
    conflict_id: String,
    resolution_note: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can resolve sync conflicts".to_string());
    }
    if resolution_note.trim().is_empty() {
        return Err("A resolution note is required".to_string());
    }

    sync_queries::resolve_sync_conflict(&db.conn, &conflict_id, &user.id).map_err(|e| e.to_string())?;

    crate::db::queries::log_audit(
        &db.conn,
        Some(&user.id),
        "sync_conflict_resolve",
        "sync_conflict",
        Some(&conflict_id),
        None,
        None,
        Some(&resolution_note),
    )
    .ok();

    Ok(())
}

/// Admin-only. Registers (or updates) a trusted LAN peer device. There is no
/// automatic discovery yet — registration is a deliberate admin action.
#[tauri::command]
pub fn register_sync_peer(
    state: State<AppState>,
    token: String,
    device_id: String,
    device_name: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can register sync peers".to_string());
    }
    if device_id.trim().is_empty() || device_name.trim().is_empty() {
        return Err("Both device_id and device_name are required".to_string());
    }
    sync_queries::register_sync_peer(&db.conn, &device_id, &device_name).map_err(|e| e.to_string())
}

/// Supervisor+: lists known sync peers.
#[tauri::command]
pub fn list_sync_peers(state: State<AppState>, token: String) -> Result<Vec<SyncPeer>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Insufficient permissions".to_string());
    }
    sync_queries::list_sync_peers(&db.conn).map_err(|e| e.to_string())
}
