// WP-66: Trust Layer Phase 2 — on-chain anchoring command layer.
//
// Thin session/role gating over `crate::anchoring::store`. Preparing an anchor
// and recording/verifying its txid are supervisory trust-layer actions, so they
// require the manage role (admin or supervisor), matching audit-checkpoint
// creation. Listing anchors is read-only for any authenticated user.
use tauri::State;

use crate::anchoring::{build_payload_preview, store, AnchorPayloadPreview};
use crate::auth as auth_service;
use crate::AppState;

const MANAGE_ONLY: &str = "Insufficient permissions — admin or supervisor role required.";

/// Preview the exact `OP_RETURN` bytes for a checkpoint's Merkle root without
/// writing anything. Handy for inspecting what would be published before
/// committing a `prepared` anchor row.
#[tauri::command]
pub fn preview_checkpoint_anchor_payload(
    state: State<AppState>,
    token: String,
    checkpoint_id: String,
    chain_name: Option<String>,
) -> Result<AnchorPayloadPreview, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err(MANAGE_ONLY.to_string());
    }
    let merkle_root: String = db
        .conn
        .query_row(
            "SELECT merkle_root FROM audit_checkpoints WHERE id = ?1",
            rusqlite::params![&checkpoint_id],
            |r| r.get(0),
        )
        .map_err(|_| format!("Checkpoint '{}' not found", checkpoint_id))?;
    build_payload_preview(&merkle_root, chain_name.as_deref().unwrap_or("dogecoin"))
}

/// Create a `prepared` anchor row for a checkpoint and return it (including the
/// canonical `OP_RETURN` scriptPubKey hex to broadcast externally).
#[tauri::command]
pub fn prepare_checkpoint_anchor(
    state: State<AppState>,
    token: String,
    checkpoint_id: String,
    chain_name: Option<String>,
) -> Result<store::CheckpointAnchor, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err(MANAGE_ONLY.to_string());
    }
    let anchor = store::prepare_anchor(
        &db.conn,
        &checkpoint_id,
        chain_name.as_deref().unwrap_or("dogecoin"),
        &user.id,
    )?;
    crate::db::queries::log_audit(
        &db.conn,
        Some(&user.id),
        "anchor_prepared",
        "checkpoint_anchor",
        Some(&anchor.id),
        None,
        Some(&anchor.merkle_root),
        Some(&format!("Prepared {} OP_RETURN anchor for checkpoint {}", anchor.chain_name, checkpoint_id)),
    )
    .ok();
    Ok(anchor)
}

/// Record the txid returned after the operator broadcast the anchor externally.
#[tauri::command]
pub fn record_checkpoint_anchor(
    state: State<AppState>,
    token: String,
    anchor_id: String,
    txid: String,
) -> Result<store::CheckpointAnchor, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err(MANAGE_ONLY.to_string());
    }
    let anchor = store::record_anchor_txid(&db.conn, &anchor_id, &txid)?;
    crate::db::queries::log_audit(
        &db.conn,
        Some(&user.id),
        "anchor_submitted",
        "checkpoint_anchor",
        Some(&anchor.id),
        None,
        anchor.txid.as_deref(),
        Some(&format!("Recorded broadcast txid for checkpoint {}", anchor.checkpoint_id)),
    )
    .ok();
    Ok(anchor)
}

/// Independently verify an anchor against on-chain `OP_RETURN` data the operator
/// copied from a block explorer. On a match the anchor becomes `confirmed`.
#[tauri::command]
pub fn verify_checkpoint_anchor(
    state: State<AppState>,
    token: String,
    anchor_id: String,
    op_return_hex: String,
) -> Result<store::AnchorVerifyResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err(MANAGE_ONLY.to_string());
    }
    let result = store::verify_anchor(&db.conn, &anchor_id, &op_return_hex)?;
    crate::db::queries::log_audit(
        &db.conn,
        Some(&user.id),
        if result.ok { "anchor_confirmed" } else { "anchor_verify_failed" },
        "checkpoint_anchor",
        Some(&anchor_id),
        None,
        result.found_root.as_deref(),
        Some(&result.message),
    )
    .ok();
    Ok(result)
}

/// List anchors, optionally scoped to a single checkpoint. Read-only.
#[tauri::command]
pub fn list_checkpoint_anchors(
    state: State<AppState>,
    token: String,
    checkpoint_id: Option<String>,
) -> Result<Vec<store::CheckpointAnchor>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::validate_session(&db, &token)?;
    store::list_anchors(&db.conn, checkpoint_id.as_deref())
}
