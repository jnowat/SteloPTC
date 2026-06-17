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
