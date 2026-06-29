use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Taxon {
    pub id: String,
    pub rank: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub ncbi_taxon_id: Option<i64>,
    pub ncbi_updated_at: Option<String>,
    pub local_override: bool,
    pub taxon_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaxonRequest {
    pub rank: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub ncbi_taxon_id: Option<i64>,
    pub local_override: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaxonRequest {
    pub id: String,
    pub name: Option<String>,
    pub parent_id: Option<String>,
    pub ncbi_taxon_id: Option<i64>,
    pub ncbi_updated_at: Option<String>,
    pub local_override: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesNodeSummary {
    pub id: String,
    pub genus: String,
    pub species_name: String,
    pub common_name: Option<String>,
    pub species_code: String,
    pub strain_count: i64,
    pub specimen_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonNode {
    pub taxon: Taxon,
    pub strain_count: i64,
    pub specimen_count: i64,
    pub species: Vec<SpeciesNodeSummary>,
    pub children: Vec<TaxonNode>,
}

// ── WP-36: NCBI Taxonomy sync models ──────────────────────────────────────────

/// A single taxon record sourced from NCBI Taxonomy.
/// The caller (frontend) is responsible for fetching data from the NCBI API
/// and passing the normalized records here for import/sync processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NcbiTaxonRecord {
    pub ncbi_taxon_id: i64,
    pub name: String,
    /// Taxonomic rank as returned by NCBI (may need normalization to our internal ranks).
    pub rank: String,
    pub parent_ncbi_id: Option<i64>,
}

/// A row from the `ncbi_sync_log` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NcbiSyncLog {
    pub id: String,
    pub sync_type: String,
    pub taxon_id: Option<String>,
    pub ncbi_taxon_id: Option<i64>,
    /// JSON object describing field-level differences (name, rank) between local and NCBI data.
    pub conflict_details: Option<String>,
    pub resolved_at: Option<String>,
    pub resolved_by: Option<String>,
    pub resolution: Option<String>,
    pub created_at: String,
}

/// Request payload for `import_ncbi_taxonomy`.
#[derive(Debug, Deserialize)]
pub struct ImportNcbiTaxonomyRequest {
    pub taxa: Vec<NcbiTaxonRecord>,
    /// When true, computes and returns what would happen without writing to the DB.
    pub dry_run: bool,
}

/// Summary of a single detected conflict, returned as part of `ImportNcbiTaxonomyResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NcbiConflictSummary {
    /// ID of the `ncbi_sync_log` row written for this conflict (None in dry-run).
    pub sync_log_id: Option<String>,
    /// The local taxon ID that conflicts with the NCBI record.
    pub taxon_id: Option<String>,
    pub ncbi_taxon_id: i64,
    /// The local taxon name for display purposes.
    pub local_name: Option<String>,
    /// The NCBI name that differs from the local value.
    pub ncbi_name: String,
    /// JSON string describing which fields differ and their respective values.
    pub conflict_details: String,
}

/// Result returned by `import_ncbi_taxonomy` (dry-run or real).
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportNcbiTaxonomyResult {
    pub imported: i64,
    pub updated: i64,
    pub skipped_overrides: i64,
    pub conflicts: Vec<NcbiConflictSummary>,
    pub dry_run: bool,
}

// ── WP-39: Advanced taxonomy navigator models ──────────────────────────────────

/// Flat taxon summary with aggregated descendant strain and specimen counts.
/// Returned by `get_taxon_column` for lazy column loading in the navigator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonColumnItem {
    pub id: String,
    pub rank: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub ncbi_taxon_id: Option<i64>,
    pub local_override: bool,
    pub strain_count: i64,
    pub specimen_count: i64,
}

/// A single hit from the global taxonomy search.
/// Contains enough context for the frontend to navigate to the item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomySearchResult {
    /// One of: "taxon" | "species" | "strain" | "specimen"
    pub result_type: String,
    pub id: String,
    pub display_name: String,
    /// Secondary label: rank for taxa, species_code for species,
    /// strain code for strains, stage for specimens.
    pub secondary: String,
    /// Ordered taxon IDs from kingdom down to genus (parsed from taxon_path).
    pub taxon_ids: Vec<String>,
    pub species_id: Option<String>,
    pub strain_id: Option<String>,
}

/// Request payload for `resolve_ncbi_conflict`.
#[derive(Debug, Deserialize)]
pub struct ResolveNcbiConflictRequest {
    pub sync_log_id: String,
    /// One of: "kept_local" | "accepted_ncbi" | "merged"
    pub resolution: String,
}

// ── WP-49: Custom/provisional taxa & Darwin Core export ───────────────────────

/// Request to create a provisional (lab-internal) taxon.
#[derive(Debug, Deserialize)]
pub struct CreateProvisionalTaxonRequest {
    pub rank: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub provisional_notes: Option<String>,
}

/// A mapping from a provisional taxon to an accepted NCBI taxon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonMapping {
    pub id: String,
    pub provisional_taxon_id: String,
    pub accepted_taxon_id: Option<String>,
    pub accepted_ncbi_id: Option<i64>,
    pub accepted_name: Option<String>,
    pub notes: Option<String>,
    pub mapped_by: Option<String>,
    pub mapped_at: String,
}

/// Request to map a provisional taxon to an accepted taxon.
#[derive(Debug, Deserialize)]
pub struct CreateTaxonMappingRequest {
    pub provisional_taxon_id: String,
    pub accepted_taxon_id: Option<String>,
    pub accepted_ncbi_id: Option<i64>,
    pub accepted_name: Option<String>,
    pub notes: Option<String>,
}

/// A single Darwin Core record for a taxon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarwinCoreRecord {
    #[serde(rename = "taxonID")]
    pub taxon_id: String,
    #[serde(rename = "scientificName")]
    pub scientific_name: String,
    #[serde(rename = "taxonRank")]
    pub taxon_rank: String,
    #[serde(rename = "parentNameUsageID")]
    pub parent_name_usage_id: Option<String>,
    #[serde(rename = "taxonomicStatus")]
    pub taxonomic_status: String,
    #[serde(rename = "nameAccordingTo")]
    pub name_according_to: Option<String>,
    pub remarks: Option<String>,
}

/// Result of a Darwin Core export operation.
#[derive(Debug, Serialize, Deserialize)]
pub struct DarwinCoreExport {
    pub record_count: usize,
    pub records: Vec<DarwinCoreRecord>,
}
