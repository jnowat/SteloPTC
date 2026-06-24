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

// ── Pedigree types (WP-37) ────────────────────────────────────────────────────

/// Lightweight strain summary used in pedigree and specimen tree views.
#[derive(Debug, Clone, Serialize)]
pub struct StrainSummary {
    pub id: String,
    pub name: String,
    pub code: String,
    pub strain_type: String,
    pub status: String,
    pub is_hybrid: bool,
    pub is_archived: bool,
    pub specimen_count: i64,
}

/// Directed hybridization edge (parent → hybrid) with provenance metadata.
#[derive(Debug, Clone, Serialize)]
pub struct PedigreeEdge {
    pub parent_strain_id: String,
    pub parent_role: Option<String>,
    pub parent_chain_seq_at_creation: Option<i64>,
    pub event_id: Option<String>,
    pub event_notes: Option<String>,
}

/// One node in the multi-generational strain pedigree.
///
/// Ancestry view: `parents` populated, `children` empty.
/// Descendant view: `children` populated, `parents` empty.
/// Root node has `edge = None`.
#[derive(Debug, Clone, Serialize)]
pub struct PedigreeNode {
    pub strain: StrainSummary,
    pub depth: u32,
    pub edge: Option<PedigreeEdge>,
    pub parents: Vec<PedigreeNode>,
    pub children: Vec<PedigreeNode>,
}

/// Lightweight specimen entry for the strain specimen tree.
#[derive(Debug, Clone, Serialize)]
pub struct SpecimenSummary {
    pub id: String,
    pub accession_number: String,
    pub stage: String,
    pub location: Option<String>,
    pub is_archived: bool,
    pub strain_id: String,
    pub created_at: String,
}

/// A strain with its live specimens, and optionally the same for descendant strains.
#[derive(Debug, Serialize)]
pub struct StrainSpecimenTree {
    pub strain: StrainSummary,
    pub specimens: Vec<SpecimenSummary>,
    pub descendant_trees: Vec<StrainSpecimenTree>,
}

/// Single hybridization event record included in pedigree exports.
#[derive(Debug, Clone, Serialize)]
pub struct HybridizationEventRecord {
    pub id: String,
    pub hybrid_strain_id: String,
    pub parent_a_strain_id: String,
    pub parent_b_strain_id: String,
    pub parent_a_chain_seq: i64,
    pub parent_b_chain_seq: i64,
    pub notes: Option<String>,
    pub created_at: String,
}

/// Portable pedigree export bundle: all reachable strains + hybridization events.
#[derive(Debug, Serialize)]
pub struct PedigreeExport {
    pub root_strain_id: String,
    pub exported_at: String,
    pub strains: Vec<StrainSummary>,
    pub hybridization_events: Vec<HybridizationEventRecord>,
}
