use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strain {
    pub id: String,
    pub species_id: String,
    pub name: String,
    pub code: String,
    pub strain_type: String,
    /// One of: "unverified", "claimed", "confirmed_manual", "confirmed_genomic"
    pub status: String,
    pub claimed_by: Option<String>,
    pub claimed_at: Option<String>,
    pub confirmation_basis: Option<String>,
    pub genomic_fingerprint: Option<String>,
    pub is_hybrid: bool,
    pub is_archived: bool,
    pub archived_at: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Populated only in list queries (list_strains_by_species).
    pub specimen_count: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateStrainRequest {
    pub species_id: String,
    pub name: String,
    pub code: String,
    pub strain_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStrainRequest {
    pub id: String,
    pub name: Option<String>,
    pub code: Option<String>,
    pub strain_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStrainStatusRequest {
    pub id: String,
    pub status: String,
    pub claimed_by: Option<String>,
    pub claimed_at: Option<String>,
    pub confirmation_basis: Option<String>,
    pub genomic_fingerprint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateHybridizationEventRequest {
    pub parent_a_id: String,
    pub parent_b_id: String,
    pub name: String,
    pub code: String,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HybridizationResult {
    pub hybrid_strain_id: String,
    pub event_id: String,
}
