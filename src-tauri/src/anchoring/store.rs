// WP-66: connection-level anchor lifecycle. Pure functions over a rusqlite
// `Connection` (no Tauri) so the full prepare → record → verify flow is
// unit-testable against an in-memory migrated database, exactly as the
// compliance_export::bundle helpers are. The thin `commands::anchoring` layer
// only adds session/role gating on top of these.
use rusqlite::{params, Connection};
use serde::Serialize;

use super::{build_op_return_script_hex, extract_root_from_hex, op_return_matches_root};

#[derive(Debug, Serialize)]
pub struct CheckpointAnchor {
    pub id: String,
    pub checkpoint_id: String,
    pub chain_name: String,
    pub merkle_root: String,
    pub op_return_hex: String,
    pub txid: Option<String>,
    pub status: String,
    pub verified_at: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct AnchorVerifyResult {
    pub anchor_id: String,
    pub ok: bool,
    pub expected_root: String,
    pub found_root: Option<String>,
    pub message: String,
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

const ANCHOR_COLS: &str = "id, checkpoint_id, chain_name, merkle_root, op_return_hex, \
                           txid, status, verified_at, created_by, created_at, updated_at";

fn map_anchor(r: &rusqlite::Row) -> rusqlite::Result<CheckpointAnchor> {
    Ok(CheckpointAnchor {
        id: r.get(0)?,
        checkpoint_id: r.get(1)?,
        chain_name: r.get(2)?,
        merkle_root: r.get(3)?,
        op_return_hex: r.get(4)?,
        txid: r.get(5)?,
        status: r.get(6)?,
        verified_at: r.get(7)?,
        created_by: r.get(8)?,
        created_at: r.get(9)?,
        updated_at: r.get(10)?,
    })
}

/// Load a single anchor by id.
pub fn get_anchor(conn: &Connection, anchor_id: &str) -> Result<CheckpointAnchor, String> {
    conn.query_row(
        &format!("SELECT {} FROM checkpoint_anchors WHERE id = ?1", ANCHOR_COLS),
        params![anchor_id],
        map_anchor,
    )
    .map_err(|_| format!("Anchor '{}' not found", anchor_id))
}

/// List anchors, newest first, optionally scoped to one checkpoint.
pub fn list_anchors(conn: &Connection, checkpoint_id: Option<&str>) -> Result<Vec<CheckpointAnchor>, String> {
    match checkpoint_id {
        Some(cid) => {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT {} FROM checkpoint_anchors WHERE checkpoint_id = ?1 ORDER BY created_at DESC",
                    ANCHOR_COLS
                ))
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![cid], map_anchor)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(rows)
        }
        None => {
            let mut stmt = conn
                .prepare(&format!("SELECT {} FROM checkpoint_anchors ORDER BY created_at DESC", ANCHOR_COLS))
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], map_anchor)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(rows)
        }
    }
}

/// Prepare an anchor for a checkpoint: snapshot its Merkle root, build the
/// canonical `OP_RETURN` scriptPubKey the operator will broadcast, and record a
/// `prepared` row. Fails cleanly if the checkpoint is missing or seals an
/// all-zero (empty) root that must not be anchored.
pub fn prepare_anchor(
    conn: &Connection,
    checkpoint_id: &str,
    chain_name: &str,
    user_id: &str,
) -> Result<CheckpointAnchor, String> {
    let merkle_root: String = conn
        .query_row(
            "SELECT merkle_root FROM audit_checkpoints WHERE id = ?1",
            params![checkpoint_id],
            |r| r.get(0),
        )
        .map_err(|_| format!("Checkpoint '{}' not found", checkpoint_id))?;

    // build_op_return_script_hex rejects an all-zero / malformed root, so an
    // un-anchorable checkpoint surfaces a clear error here rather than storing a
    // meaningless payload.
    let op_return_hex = build_op_return_script_hex(&merkle_root)?;

    let chain = if chain_name.trim().is_empty() { "dogecoin" } else { chain_name.trim() };
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_iso();
    conn.execute(
        "INSERT INTO checkpoint_anchors \
         (id, checkpoint_id, chain_name, merkle_root, op_return_hex, status, created_by, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, 'prepared', ?6, ?7, ?7)",
        params![id, checkpoint_id, chain, merkle_root, op_return_hex, user_id, now],
    )
    .map_err(|e| e.to_string())?;

    get_anchor(conn, &id)
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Record the transaction id the operator got back after broadcasting the
/// prepared `OP_RETURN` externally. Advances the anchor to `submitted` and writes
/// the txid back onto the covering checkpoint's `anchored_txid` column (the
/// Phase-2 hook reserved since migration 013).
pub fn record_anchor_txid(conn: &Connection, anchor_id: &str, txid: &str) -> Result<CheckpointAnchor, String> {
    let anchor = get_anchor(conn, anchor_id)?;
    if anchor.status == "confirmed" {
        return Err("Anchor is already confirmed; its txid cannot be changed".to_string());
    }
    let txid = txid.trim();
    if !is_hex64(txid) {
        return Err("A Dogecoin transaction id must be 64 hexadecimal characters".to_string());
    }
    let now = now_iso();
    conn.execute(
        "UPDATE checkpoint_anchors SET txid = ?1, status = 'submitted', updated_at = ?2 WHERE id = ?3",
        params![txid, now, anchor_id],
    )
    .map_err(|e| e.to_string())?;
    // Surface the anchor on the checkpoint itself.
    conn.execute(
        "UPDATE audit_checkpoints SET anchored_txid = ?1 WHERE id = ?2",
        params![txid, anchor.checkpoint_id],
    )
    .map_err(|e| e.to_string())?;
    get_anchor(conn, anchor_id)
}

/// Independently verify that the `OP_RETURN` data an operator copied from a block
/// explorer (for the recorded txid) commits to exactly this anchor's Merkle root.
/// Trusts nothing but the two inputs. On a match, advances the anchor to
/// `confirmed` and stamps `verified_at`.
pub fn verify_anchor(conn: &Connection, anchor_id: &str, op_return_hex: &str) -> Result<AnchorVerifyResult, String> {
    let anchor = get_anchor(conn, anchor_id)?;
    let found_root = extract_root_from_hex(op_return_hex).ok();
    let matches = op_return_matches_root(op_return_hex, &anchor.merkle_root)?;

    if matches {
        let now = now_iso();
        conn.execute(
            "UPDATE checkpoint_anchors SET status = 'confirmed', verified_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, anchor_id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(AnchorVerifyResult {
        anchor_id: anchor_id.to_string(),
        ok: matches,
        expected_root: anchor.merkle_root,
        found_root,
        message: if matches {
            "On-chain OP_RETURN data commits to this checkpoint's Merkle root — anchor confirmed.".to_string()
        } else {
            "The provided OP_RETURN data does NOT commit to this checkpoint's Merkle root.".to_string()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_all;
    use crate::db::queries::build_merkle_root;

    fn seed_checkpoint(conn: &Connection, id: &str, root: &str) {
        conn.execute(
            "INSERT INTO audit_checkpoints (id, lineage_id, start_seq, end_seq, entry_count, merkle_root, created_at, is_auto) \
             VALUES (?1, 'lin1', 0, 2, 3, ?2, '2026-01-01T00:00:00Z', 0)",
            params![id, root],
        )
        .unwrap();
    }

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_all(&conn).unwrap();
        // created_by references users(id) (FK enforced after migrations run).
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, role) \
             VALUES ('user1', 'u1', 'x', 'User One', 'admin')",
            [],
        )
        .unwrap();
        conn
    }

    fn sample_root() -> String {
        build_merkle_root(&["aa".repeat(32), "bb".repeat(32), "cc".repeat(32)])
    }

    #[test]
    fn prepare_creates_a_prepared_anchor_with_op_return_hex() {
        let conn = test_db();
        let root = sample_root();
        seed_checkpoint(&conn, "cp1", &root);
        let anchor = prepare_anchor(&conn, "cp1", "dogecoin", "user1").unwrap();
        assert_eq!(anchor.status, "prepared");
        assert_eq!(anchor.merkle_root, root);
        assert!(anchor.op_return_hex.starts_with("6a25"));
        assert!(anchor.txid.is_none());
    }

    #[test]
    fn prepare_rejects_missing_checkpoint() {
        let conn = test_db();
        assert!(prepare_anchor(&conn, "nope", "dogecoin", "user1").is_err());
    }

    #[test]
    fn record_txid_advances_to_submitted_and_stamps_checkpoint() {
        let conn = test_db();
        let root = sample_root();
        seed_checkpoint(&conn, "cp1", &root);
        let anchor = prepare_anchor(&conn, "cp1", "dogecoin", "user1").unwrap();
        let txid = "d4".repeat(32); // 64 hex chars
        let updated = record_anchor_txid(&conn, &anchor.id, &txid).unwrap();
        assert_eq!(updated.status, "submitted");
        assert_eq!(updated.txid.as_deref(), Some(txid.as_str()));

        let stamped: Option<String> = conn
            .query_row("SELECT anchored_txid FROM audit_checkpoints WHERE id = 'cp1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(stamped.as_deref(), Some(txid.as_str()));
    }

    #[test]
    fn record_txid_rejects_malformed_txid() {
        let conn = test_db();
        let root = sample_root();
        seed_checkpoint(&conn, "cp1", &root);
        let anchor = prepare_anchor(&conn, "cp1", "dogecoin", "user1").unwrap();
        assert!(record_anchor_txid(&conn, &anchor.id, "too-short").is_err());
    }

    #[test]
    fn verify_confirms_when_on_chain_data_matches() {
        let conn = test_db();
        let root = sample_root();
        seed_checkpoint(&conn, "cp1", &root);
        let anchor = prepare_anchor(&conn, "cp1", "dogecoin", "user1").unwrap();
        // Simulate the operator pasting back exactly the OP_RETURN we prepared.
        let result = verify_anchor(&conn, &anchor.id, &anchor.op_return_hex).unwrap();
        assert!(result.ok);
        assert_eq!(result.found_root.as_deref(), Some(root.as_str()));

        let reloaded = get_anchor(&conn, &anchor.id).unwrap();
        assert_eq!(reloaded.status, "confirmed");
        assert!(reloaded.verified_at.is_some());
    }

    #[test]
    fn verify_fails_for_a_different_root_and_leaves_status_unchanged() {
        let conn = test_db();
        let root = sample_root();
        seed_checkpoint(&conn, "cp1", &root);
        let anchor = prepare_anchor(&conn, "cp1", "dogecoin", "user1").unwrap();
        // OP_RETURN for a different root.
        let other = super::super::build_op_return_script_hex(&build_merkle_root(&["ff".repeat(32)])).unwrap();
        let result = verify_anchor(&conn, &anchor.id, &other).unwrap();
        assert!(!result.ok);
        assert_eq!(get_anchor(&conn, &anchor.id).unwrap().status, "prepared");
    }

    #[test]
    fn list_anchors_scopes_by_checkpoint() {
        let conn = test_db();
        let root = sample_root();
        seed_checkpoint(&conn, "cp1", &root);
        seed_checkpoint(&conn, "cp2", &root);
        prepare_anchor(&conn, "cp1", "dogecoin", "user1").unwrap();
        prepare_anchor(&conn, "cp2", "dogecoin", "user1").unwrap();
        assert_eq!(list_anchors(&conn, Some("cp1")).unwrap().len(), 1);
        assert_eq!(list_anchors(&conn, None).unwrap().len(), 2);
    }
}
