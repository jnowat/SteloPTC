use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub details: Option<String>,
    pub created_at: String,
    // Hash-chain fields (None for rows written before v1.5.0)
    pub lineage_id: Option<String>,
    pub chain_seq: Option<i64>,
    pub prev_hash: Option<String>,
    pub entry_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyEntryResult {
    pub entry_id: String,
    pub ok: bool,
    pub message: String,
    pub stored_hash: Option<String>,
    pub computed_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyChainResult {
    pub lineage_id: String,
    pub ok: bool,
    pub checked: usize,
    pub first_break_seq: Option<i64>,
    pub message: String,
}

/// A stored Merkle checkpoint over a contiguous range of one lineage's audit chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditCheckpoint {
    pub id: String,
    pub lineage_id: String,
    pub start_seq: i64,
    pub end_seq: i64,
    pub entry_count: i64,
    pub merkle_root: String,
    pub created_at: String,
    pub created_by: Option<String>,
    /// Nullable Phase-2 hook: Dogecoin txid once the root is anchored on-chain.
    pub anchored_txid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCheckpointResult {
    pub checkpoint_id: String,
    pub lineage_id: String,
    pub start_seq: i64,
    pub end_seq: i64,
    pub entry_count: i64,
    pub merkle_root: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyCheckpointResult {
    pub checkpoint_id: String,
    pub lineage_id: String,
    pub ok: bool,
    pub expected_count: i64,
    pub actual_count: i64,
    /// First chain_seq where tampering was detected, if pinpointable.
    pub tampered_seq: Option<i64>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct AuditSearchParams {
    pub user_id: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub action: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}
