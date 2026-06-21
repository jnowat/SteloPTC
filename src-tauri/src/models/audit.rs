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
    /// WP-21: true when the checkpoint was created automatically.
    pub is_auto: bool,
    /// WP-21: "backup" | "entry_count" | None (manual).
    pub auto_source: Option<String>,
}

// ---------------------------------------------------------------------------
// WP-21 — Portable Merkle proofs & auto-checkpointing
// ---------------------------------------------------------------------------

/// One node in a Merkle inclusion path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerklePathNode {
    pub sibling_hash: String,
    /// Where the sibling sits relative to the current node.
    /// "right" → SHA256(current || sibling); "left" → SHA256(sibling || current).
    pub position: String,
}

/// One audit entry embedded in a portable proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofEntry {
    pub chain_seq: i64,
    /// Pipe-separated canonical form: lineage_id|chain_seq|timestamp|user_id|entity_type|entity_id|action|details
    pub canonical: String,
    pub prev_hash: String,
    pub entry_hash: String,
    /// Sibling-hash path from this leaf to the Merkle root.
    pub merkle_path: Vec<MerklePathNode>,
}

/// Checkpoint metadata embedded in an exported proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCheckpointMeta {
    pub id: String,
    pub lineage_id: String,
    pub start_seq: i64,
    pub end_seq: i64,
    pub entry_count: i64,
    pub merkle_root: String,
    pub created_at: String,
}

/// A self-contained, portable Merkle proof for a sealed range of one lineage's audit chain.
///
/// Contains all audit entries (with their canonical form and individual Merkle paths)
/// so the proof can be verified offline without the SteloPTC application or database.
/// See docs/merkle-proofs.md for the full specification and a standalone verifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableMerkleProof {
    /// Proof format version. Must be "1" for this implementation.
    pub version: String,
    /// ISO-8601 timestamp when this proof was exported.
    pub exported_at: String,
    pub checkpoint: ProofCheckpointMeta,
    /// Ordered by chain_seq ascending.
    pub entries: Vec<ProofEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyProofResult {
    pub ok: bool,
    pub message: String,
    pub entry_count: i64,
    pub merkle_root: String,
    /// Human-readable description of the specific failure, if any.
    pub failure_reason: Option<String>,
    /// chain_seq of the first entry where a failure was detected.
    pub failed_seq: Option<i64>,
}

/// Configuration for automatic checkpoint creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCheckpointConfig {
    pub enabled: bool,
    /// Minimum uncovered entries in a lineage before auto-checkpointing it.
    /// 0 means always checkpoint any lineage with uncovered entries.
    pub interval: i64,
    /// If true, auto-checkpoint all eligible lineages before each backup.
    pub on_backup: bool,
}

impl Default for AutoCheckpointConfig {
    fn default() -> Self {
        Self { enabled: true, interval: 100, on_backup: true }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AutoCheckpointResult {
    pub lineages_checked: usize,
    pub checkpoints_created: usize,
    pub details: Vec<String>,
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
