//! WP-51 — LAN sync foundation: change detection and conflict recording.
//!
//! This module reuses the existing per-lineage hash chain (`lineage_id`,
//! `chain_seq`, `prev_hash`, `entry_hash` on `audit_log`) as the change-vector
//! for sync, rather than introducing a parallel change-tracking mechanism.
//! It provides the data model and command structure for a future LAN
//! networking layer — actual peer discovery, transport, and cross-device
//! write-back of accepted changes are explicitly out of scope for this
//! packet (see ROADMAP.md WP-51 "Not yet implemented").

use super::DbResult;
use crate::models::sync::{ChangeRecord, SyncConflict, SyncCursor, SyncPeer, SyncStatusResponse};
use rusqlite::{params, Connection};

fn row_to_change_record(row: &rusqlite::Row) -> rusqlite::Result<ChangeRecord> {
    Ok(ChangeRecord {
        lineage_id: row.get("lineage_id")?,
        chain_seq: row.get("chain_seq")?,
        entity_type: row.get("entity_type")?,
        entity_id: row.get("entity_id")?,
        action: row.get("action")?,
        old_value: row.get("old_value")?,
        new_value: row.get("new_value")?,
        details: row.get("details")?,
        prev_hash: row.get("prev_hash")?,
        entry_hash: row.get("entry_hash")?,
        created_at: row.get("created_at")?,
    })
}

const CHANGE_RECORD_COLUMNS: &str =
    "lineage_id, chain_seq, entity_type, entity_id, action, old_value, new_value, \
     details, prev_hash, entry_hash, created_at";

/// Returns audit-chain entries newer than each cursor's `last_seen_chain_seq`,
/// merged and sorted by `(lineage_id, chain_seq)`, capped at `limit`.
///
/// An empty `cursors` slice means "the requesting peer has nothing yet" — every
/// syncable entry (i.e. one with both `lineage_id` and `chain_seq` populated)
/// is returned, subject to `limit`.
pub fn get_changes_since(
    conn: &Connection,
    cursors: &[SyncCursor],
    limit: i64,
) -> DbResult<Vec<ChangeRecord>> {
    let mut changes: Vec<ChangeRecord> = Vec::new();

    if cursors.is_empty() {
        let sql = format!(
            "SELECT {} FROM audit_log \
             WHERE lineage_id IS NOT NULL AND chain_seq IS NOT NULL \
             ORDER BY lineage_id, chain_seq LIMIT ?1",
            CHANGE_RECORD_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![limit], row_to_change_record)?;
        for row in rows {
            changes.push(row?);
        }
        return Ok(changes);
    }

    let sql = format!(
        "SELECT {} FROM audit_log \
         WHERE lineage_id = ?1 AND chain_seq > ?2 AND chain_seq IS NOT NULL \
         ORDER BY chain_seq",
        CHANGE_RECORD_COLUMNS
    );
    for cursor in cursors {
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(
            params![cursor.lineage_id, cursor.last_seen_chain_seq],
            row_to_change_record,
        )?;
        for row in rows {
            changes.push(row?);
        }
    }

    changes.sort_by(|a, b| a.lineage_id.cmp(&b.lineage_id).then(a.chain_seq.cmp(&b.chain_seq)));
    changes.truncate(limit.max(0) as usize);
    Ok(changes)
}

/// The outcome of reconciling a batch of incoming changes against local state.
pub struct ConflictDetectionResult {
    /// No local entry exists at this position — genuinely new to this database.
    pub new_changes: Vec<ChangeRecord>,
    /// A local entry already exists at this position with a matching hash.
    pub duplicates: Vec<ChangeRecord>,
    /// A local entry exists at this position with a *different* hash — a fork.
    pub conflicts: Vec<SyncConflict>,
}

/// Classifies each incoming change as new, a duplicate of what's already
/// local, or a genuine conflict (local and incoming disagree on `entry_hash`
/// at the same `(lineage_id, chain_seq)` position). Never silently merges a
/// conflict — every one is returned so the caller can record it durably.
pub fn detect_sync_conflicts(
    conn: &Connection,
    incoming: &[ChangeRecord],
    source_device_id: &str,
) -> DbResult<ConflictDetectionResult> {
    let mut new_changes = Vec::new();
    let mut duplicates = Vec::new();
    let mut conflicts = Vec::new();

    for change in incoming {
        let local_hash: Option<String> = conn
            .query_row(
                "SELECT entry_hash FROM audit_log WHERE lineage_id = ?1 AND chain_seq = ?2",
                params![change.lineage_id, change.chain_seq],
                |r| r.get(0),
            )
            .ok();

        match local_hash {
            None => new_changes.push(change.clone()),
            Some(local) if Some(&local) == change.entry_hash.as_ref() => {
                duplicates.push(change.clone());
            }
            Some(local) => {
                conflicts.push(SyncConflict {
                    id: uuid::Uuid::new_v4().to_string(),
                    lineage_id: change.lineage_id.clone(),
                    chain_seq: change.chain_seq,
                    local_entry_hash: Some(local),
                    incoming_entry_hash: change.entry_hash.clone(),
                    incoming_source_device_id: Some(source_device_id.to_string()),
                    reason: format!(
                        "Local and incoming entries disagree at lineage {} chain_seq {}: \
                         this lineage has forked between devices.",
                        change.lineage_id, change.chain_seq
                    ),
                    resolved: false,
                    resolved_by: None,
                    resolved_at: None,
                    detected_at: String::new(), // set by the DB default on insert
                });
            }
        }
    }

    Ok(ConflictDetectionResult { new_changes, duplicates, conflicts })
}

pub fn record_sync_conflict(conn: &Connection, conflict: &SyncConflict) -> DbResult<String> {
    let id = if conflict.id.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        conflict.id.clone()
    };
    conn.execute(
        "INSERT INTO sync_conflicts \
         (id, lineage_id, chain_seq, local_entry_hash, incoming_entry_hash, \
          incoming_source_device_id, reason) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            conflict.lineage_id,
            conflict.chain_seq,
            conflict.local_entry_hash,
            conflict.incoming_entry_hash,
            conflict.incoming_source_device_id,
            conflict.reason,
        ],
    )?;
    Ok(id)
}

fn row_to_sync_conflict(row: &rusqlite::Row) -> rusqlite::Result<SyncConflict> {
    Ok(SyncConflict {
        id: row.get("id")?,
        lineage_id: row.get("lineage_id")?,
        chain_seq: row.get("chain_seq")?,
        local_entry_hash: row.get("local_entry_hash")?,
        incoming_entry_hash: row.get("incoming_entry_hash")?,
        incoming_source_device_id: row.get("incoming_source_device_id")?,
        reason: row.get("reason")?,
        resolved: row.get::<_, i64>("resolved")? != 0,
        resolved_by: row.get("resolved_by")?,
        resolved_at: row.get("resolved_at")?,
        detected_at: row.get("detected_at")?,
    })
}

pub fn list_sync_conflicts(conn: &Connection, unresolved_only: bool) -> DbResult<Vec<SyncConflict>> {
    let sql = if unresolved_only {
        "SELECT * FROM sync_conflicts WHERE resolved = 0 ORDER BY detected_at DESC"
    } else {
        "SELECT * FROM sync_conflicts ORDER BY detected_at DESC"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], row_to_sync_conflict)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn resolve_sync_conflict(
    conn: &Connection,
    conflict_id: &str,
    resolved_by: &str,
) -> DbResult<()> {
    let affected = conn.execute(
        "UPDATE sync_conflicts \
         SET resolved = 1, resolved_by = ?1, resolved_at = datetime('now') \
         WHERE id = ?2",
        params![resolved_by, conflict_id],
    )?;
    if affected == 0 {
        return Err(super::DbError::NotFound(format!(
            "No sync conflict found with id '{}'",
            conflict_id
        )));
    }
    Ok(())
}

/// Upserts a peer by `device_id`. Registering a peer is a deliberate admin
/// action in this packet — there is no automatic LAN discovery yet.
pub fn register_sync_peer(conn: &Connection, device_id: &str, device_name: &str) -> DbResult<String> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM sync_peers WHERE device_id = ?1",
            params![device_id],
            |r| r.get(0),
        )
        .ok();

    if let Some(id) = existing {
        conn.execute(
            "UPDATE sync_peers SET device_name = ?1, last_seen_at = datetime('now') WHERE id = ?2",
            params![device_name, id],
        )?;
        Ok(id)
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO sync_peers (id, device_id, device_name, last_seen_at) \
             VALUES (?1, ?2, ?3, datetime('now'))",
            params![id, device_id, device_name],
        )?;
        Ok(id)
    }
}

pub fn list_sync_peers(conn: &Connection) -> DbResult<Vec<SyncPeer>> {
    let mut stmt = conn.prepare("SELECT * FROM sync_peers ORDER BY device_name")?;
    let rows = stmt.query_map([], |row| {
        Ok(SyncPeer {
            id: row.get("id")?,
            device_id: row.get("device_id")?,
            device_name: row.get("device_name")?,
            last_seen_at: row.get("last_seen_at")?,
            last_sync_at: row.get("last_sync_at")?,
            created_at: row.get("created_at")?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn get_sync_status(conn: &Connection) -> DbResult<SyncStatusResponse> {
    let lineages_tracked: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT lineage_id) FROM audit_log WHERE lineage_id IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let max_chain_seq_overall: i64 = conn
        .query_row("SELECT COALESCE(MAX(chain_seq), 0) FROM audit_log", [], |r| r.get(0))
        .unwrap_or(0);
    let unresolved_conflicts: i64 = conn
        .query_row("SELECT COUNT(*) FROM sync_conflicts WHERE resolved = 0", [], |r| r.get(0))
        .unwrap_or(0);
    let known_peers: i64 = conn
        .query_row("SELECT COUNT(*) FROM sync_peers", [], |r| r.get(0))
        .unwrap_or(0);

    Ok(SyncStatusResponse {
        lineages_tracked,
        max_chain_seq_overall,
        unresolved_conflicts,
        known_peers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_all;

    fn migrated_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        run_all(&conn).expect("all migrations must succeed on a fresh in-memory DB");
        conn
    }

    fn insert_audit_row(
        conn: &Connection,
        lineage_id: &str,
        chain_seq: i64,
        entry_hash: &str,
        prev_hash: &str,
    ) {
        conn.execute(
            "INSERT INTO audit_log \
             (id, action, entity_type, entity_id, lineage_id, chain_seq, prev_hash, entry_hash) \
             VALUES (?1, 'create', 'specimen', ?2, ?2, ?3, ?4, ?5)",
            params![uuid::Uuid::new_v4().to_string(), lineage_id, chain_seq, prev_hash, entry_hash],
        )
        .unwrap();
    }

    #[test]
    fn get_changes_since_empty_cursors_returns_all_syncable_entries() {
        let conn = migrated_db();
        insert_audit_row(&conn, "sp-1", 0, "h0", "0000");
        insert_audit_row(&conn, "sp-1", 1, "h1", "h0");
        insert_audit_row(&conn, "sp-2", 0, "h2", "0000");

        let changes = get_changes_since(&conn, &[], 100).unwrap();
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn get_changes_since_cursor_returns_only_newer_entries() {
        let conn = migrated_db();
        insert_audit_row(&conn, "sp-1", 0, "h0", "0000");
        insert_audit_row(&conn, "sp-1", 1, "h1", "h0");
        insert_audit_row(&conn, "sp-1", 2, "h2", "h1");

        let cursors = vec![SyncCursor { lineage_id: "sp-1".to_string(), last_seen_chain_seq: 0 }];
        let changes = get_changes_since(&conn, &cursors, 100).unwrap();
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].chain_seq, 1);
        assert_eq!(changes[1].chain_seq, 2);
    }

    #[test]
    fn get_changes_since_respects_limit() {
        let conn = migrated_db();
        for i in 0..5 {
            insert_audit_row(&conn, "sp-1", i, &format!("h{}", i), "prev");
        }
        let changes = get_changes_since(&conn, &[], 2).unwrap();
        assert_eq!(changes.len(), 2);
    }

    #[test]
    fn get_changes_since_multiple_cursors_merged_and_sorted() {
        let conn = migrated_db();
        insert_audit_row(&conn, "sp-1", 0, "a0", "0000");
        insert_audit_row(&conn, "sp-1", 1, "a1", "a0");
        insert_audit_row(&conn, "sp-2", 0, "b0", "0000");
        insert_audit_row(&conn, "sp-2", 1, "b1", "b0");

        let cursors = vec![
            SyncCursor { lineage_id: "sp-1".to_string(), last_seen_chain_seq: -1 },
            SyncCursor { lineage_id: "sp-2".to_string(), last_seen_chain_seq: 0 },
        ];
        let changes = get_changes_since(&conn, &cursors, 100).unwrap();
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0].lineage_id, "sp-1");
        assert_eq!(changes[1].lineage_id, "sp-1");
        assert_eq!(changes[2].lineage_id, "sp-2");
    }

    fn sample_change(lineage_id: &str, chain_seq: i64, entry_hash: &str) -> ChangeRecord {
        ChangeRecord {
            lineage_id: lineage_id.to_string(),
            chain_seq,
            entity_type: "specimen".to_string(),
            entity_id: Some(lineage_id.to_string()),
            action: "create".to_string(),
            old_value: None,
            new_value: None,
            details: None,
            prev_hash: Some("prev".to_string()),
            entry_hash: Some(entry_hash.to_string()),
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
        }
    }

    #[test]
    fn detect_sync_conflicts_classifies_new_entry() {
        let conn = migrated_db();
        let incoming = vec![sample_change("sp-1", 0, "h0")];
        let result = detect_sync_conflicts(&conn, &incoming, "device-a").unwrap();
        assert_eq!(result.new_changes.len(), 1);
        assert_eq!(result.duplicates.len(), 0);
        assert_eq!(result.conflicts.len(), 0);
    }

    #[test]
    fn detect_sync_conflicts_classifies_duplicate() {
        let conn = migrated_db();
        insert_audit_row(&conn, "sp-1", 0, "h0", "0000");
        let incoming = vec![sample_change("sp-1", 0, "h0")];
        let result = detect_sync_conflicts(&conn, &incoming, "device-a").unwrap();
        assert_eq!(result.new_changes.len(), 0);
        assert_eq!(result.duplicates.len(), 1);
        assert_eq!(result.conflicts.len(), 0);
    }

    #[test]
    fn detect_sync_conflicts_classifies_genuine_conflict() {
        let conn = migrated_db();
        insert_audit_row(&conn, "sp-1", 0, "h0-local", "0000");
        let incoming = vec![sample_change("sp-1", 0, "h0-incoming")];
        let result = detect_sync_conflicts(&conn, &incoming, "device-a").unwrap();
        assert_eq!(result.new_changes.len(), 0);
        assert_eq!(result.duplicates.len(), 0);
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].local_entry_hash.as_deref(), Some("h0-local"));
        assert_eq!(result.conflicts[0].incoming_entry_hash.as_deref(), Some("h0-incoming"));
        assert_eq!(result.conflicts[0].incoming_source_device_id.as_deref(), Some("device-a"));
    }

    #[test]
    fn detect_sync_conflicts_mixed_batch() {
        let conn = migrated_db();
        insert_audit_row(&conn, "sp-1", 0, "dup-hash", "0000");
        insert_audit_row(&conn, "sp-2", 0, "local-hash", "0000");
        let incoming = vec![
            sample_change("sp-1", 0, "dup-hash"),   // duplicate
            sample_change("sp-2", 0, "other-hash"), // conflict
            sample_change("sp-3", 0, "new-hash"),   // new
        ];
        let result = detect_sync_conflicts(&conn, &incoming, "device-a").unwrap();
        assert_eq!(result.new_changes.len(), 1);
        assert_eq!(result.duplicates.len(), 1);
        assert_eq!(result.conflicts.len(), 1);
    }

    #[test]
    fn record_and_list_sync_conflict_round_trip() {
        let conn = migrated_db();
        let conflict = SyncConflict {
            id: String::new(),
            lineage_id: "sp-1".to_string(),
            chain_seq: 0,
            local_entry_hash: Some("a".to_string()),
            incoming_entry_hash: Some("b".to_string()),
            incoming_source_device_id: Some("device-a".to_string()),
            reason: "fork".to_string(),
            resolved: false,
            resolved_by: None,
            resolved_at: None,
            detected_at: String::new(),
        };
        let id = record_sync_conflict(&conn, &conflict).unwrap();
        let listed = list_sync_conflicts(&conn, false).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, id);
        assert!(!listed[0].resolved);
    }

    #[test]
    fn resolve_sync_conflict_marks_resolved() {
        let conn = migrated_db();
        let conflict = SyncConflict {
            id: String::new(),
            lineage_id: "sp-1".to_string(),
            chain_seq: 0,
            local_entry_hash: Some("a".to_string()),
            incoming_entry_hash: Some("b".to_string()),
            incoming_source_device_id: None,
            reason: "fork".to_string(),
            resolved: false,
            resolved_by: None,
            resolved_at: None,
            detected_at: String::new(),
        };
        let id = record_sync_conflict(&conn, &conflict).unwrap();
        resolve_sync_conflict(&conn, &id, "admin-user").unwrap();

        let listed = list_sync_conflicts(&conn, true).unwrap();
        assert_eq!(listed.len(), 0, "resolved conflicts must not appear in the unresolved-only list");

        let all = list_sync_conflicts(&conn, false).unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].resolved);
        assert_eq!(all[0].resolved_by.as_deref(), Some("admin-user"));
    }

    #[test]
    fn resolve_sync_conflict_unknown_id_returns_not_found() {
        let conn = migrated_db();
        let result = resolve_sync_conflict(&conn, "does-not-exist", "admin-user");
        assert!(result.is_err());
    }

    #[test]
    fn register_sync_peer_creates_new_row() {
        let conn = migrated_db();
        let id = register_sync_peer(&conn, "dev-1", "Lab PC 1").unwrap();
        let peers = list_sync_peers(&conn).unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].id, id);
        assert_eq!(peers[0].device_name, "Lab PC 1");
    }

    #[test]
    fn register_sync_peer_upserts_existing_device() {
        let conn = migrated_db();
        let id1 = register_sync_peer(&conn, "dev-1", "Lab PC 1").unwrap();
        let id2 = register_sync_peer(&conn, "dev-1", "Lab PC 1 (renamed)").unwrap();
        assert_eq!(id1, id2, "same device_id must update the existing row, not create a new one");

        let peers = list_sync_peers(&conn).unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].device_name, "Lab PC 1 (renamed)");
    }

    #[test]
    fn get_sync_status_reports_correct_aggregates() {
        let conn = migrated_db();
        insert_audit_row(&conn, "sp-1", 0, "h0", "0000");
        insert_audit_row(&conn, "sp-1", 1, "h1", "h0");
        insert_audit_row(&conn, "sp-2", 0, "h2", "0000");
        register_sync_peer(&conn, "dev-1", "Lab PC 1").unwrap();
        let conflict = SyncConflict {
            id: String::new(),
            lineage_id: "sp-1".to_string(),
            chain_seq: 0,
            local_entry_hash: Some("a".to_string()),
            incoming_entry_hash: Some("b".to_string()),
            incoming_source_device_id: None,
            reason: "fork".to_string(),
            resolved: false,
            resolved_by: None,
            resolved_at: None,
            detected_at: String::new(),
        };
        record_sync_conflict(&conn, &conflict).unwrap();

        let status = get_sync_status(&conn).unwrap();
        assert_eq!(status.lineages_tracked, 2);
        assert_eq!(status.max_chain_seq_overall, 1);
        assert_eq!(status.unresolved_conflicts, 1);
        assert_eq!(status.known_peers, 1);
    }
}
