use serde::{Deserialize, Serialize};

/// WP-51 — what a peer has already synced for one lineage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCursor {
    pub lineage_id: String,
    pub last_seen_chain_seq: i64,
}

/// One change, shaped from an `audit_log` row. Reuses the existing hash-chain
/// columns (lineage_id, chain_seq, prev_hash, entry_hash) as the change
/// identity and ordering — no parallel change-tracking mechanism is introduced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub lineage_id: String,
    pub chain_seq: i64,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub action: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub details: Option<String>,
    pub prev_hash: Option<String>,
    pub entry_hash: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeSetResponse {
    pub changes: Vec<ChangeRecord>,
    /// True when more changes exist beyond the returned page (caller hit `limit`).
    pub has_more: bool,
}

/// A durable record of a genuine fork: a local entry and an incoming entry
/// disagree on `entry_hash` at the same `(lineage_id, chain_seq)` position.
/// Never silently discarded or auto-merged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    pub id: String,
    pub lineage_id: String,
    pub chain_seq: i64,
    pub local_entry_hash: Option<String>,
    pub incoming_entry_hash: Option<String>,
    pub incoming_source_device_id: Option<String>,
    pub reason: String,
    pub resolved: bool,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<String>,
    pub detected_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplyChangesRequest {
    pub changes: Vec<ChangeRecord>,
    pub source_device_id: String,
}

/// Result of submitting incoming changes for reconciliation.
///
/// `pending_manual_apply` counts changes that are genuinely new (no local
/// entry exists at that position) but were **not** written into
/// specimens/subcultures/etc. — replaying a generic change record into the
/// correct domain table requires per-entity-type handlers, which is future
/// work once the networking transport layer exists (see ROADMAP.md WP-51
/// "Not yet implemented"). This packet's `apply_incoming_changes` command
/// detects and durably records conflicts/duplicates; it does not yet perform
/// the write-back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyChangesResult {
    pub applied: usize,
    pub skipped_duplicate: usize,
    pub pending_manual_apply: usize,
    pub conflicts: Vec<SyncConflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPeer {
    pub id: String,
    pub device_id: String,
    pub device_name: String,
    pub last_seen_at: Option<String>,
    pub last_sync_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatusResponse {
    pub lineages_tracked: i64,
    pub max_chain_seq_overall: i64,
    pub unresolved_conflicts: i64,
    pub known_peers: i64,
}
