use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Specimen {
    pub id: String,
    pub accession_number: String,
    pub species_id: String,
    pub species_code: Option<String>,
    pub species_name: Option<String>,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub stage: String,
    pub custom_stage: Option<String>,
    pub provenance: Option<String>,
    pub source_plant: Option<String>,
    pub initiation_date: String,
    pub location: Option<String>,
    pub location_details: Option<String>,
    pub propagation_method: Option<String>,
    pub acclimatization_status: Option<String>,
    pub health_status: Option<String>,
    pub disease_status: Option<String>,
    pub quarantine_flag: bool,
    pub quarantine_release_date: Option<String>,
    pub permit_number: Option<String>,
    pub permit_expiry: Option<String>,
    pub ip_flag: bool,
    pub ip_notes: Option<String>,
    pub environmental_notes: Option<String>,
    pub subculture_count: i32,
    pub generation: i32,
    pub lineage_passage_offset: i32,
    pub root_specimen_id: Option<String>,
    pub parent_specimen_id: Option<String>,
    pub qr_code_data: Option<String>,
    pub notes: Option<String>,
    pub employee_id: Option<String>,
    pub is_archived: bool,
    pub archived_at: Option<String>,
    /// Contamination flag set at archive time (e.g. during a split).
    /// Distinct from `has_contamination`, which aggregates across subculture records.
    pub contamination_flag: bool,
    pub contamination_notes: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub has_contamination: bool,
    pub strain_id: Option<String>,
    pub strain_chain_seq: Option<i64>,
    /// Cumulative population doubling level for this specimen, including PDL
    /// inherited from ancestors and PDL gained through its own passages.
    /// NULL when no cell-count data has ever been recorded on this lineage.
    pub cumulative_pdl: Option<f64>,
    /// Biosafety containment level for cell culture lines (BSL-1 through BSL-3).
    /// NULL means unclassified.
    pub biosafety_level: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSpecimenRequest {
    pub species_id: String,
    pub project_id: Option<String>,
    pub stage: String,
    pub custom_stage: Option<String>,
    pub provenance: Option<String>,
    pub source_plant: Option<String>,
    pub initiation_date: String,
    pub location: Option<String>,
    pub location_details: Option<String>,
    pub propagation_method: Option<String>,
    pub acclimatization_status: Option<String>,
    pub health_status: Option<String>,
    pub disease_status: Option<String>,
    pub quarantine_flag: Option<bool>,
    pub permit_number: Option<String>,
    pub permit_expiry: Option<String>,
    pub ip_flag: Option<bool>,
    pub ip_notes: Option<String>,
    pub environmental_notes: Option<String>,
    pub parent_specimen_id: Option<String>,
    pub notes: Option<String>,
    pub employee_id: Option<String>,
    pub strain_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSpecimenRequest {
    pub id: String,
    pub stage: Option<String>,
    pub custom_stage: Option<String>,
    pub location: Option<String>,
    pub location_details: Option<String>,
    pub propagation_method: Option<String>,
    pub acclimatization_status: Option<String>,
    pub health_status: Option<String>,
    pub disease_status: Option<String>,
    pub quarantine_flag: Option<bool>,
    pub quarantine_release_date: Option<String>,
    pub permit_number: Option<String>,
    pub permit_expiry: Option<String>,
    pub ip_flag: Option<bool>,
    pub ip_notes: Option<String>,
    pub environmental_notes: Option<String>,
    pub notes: Option<String>,
    pub biosafety_level: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SpecimenSearchParams {
    pub query: Option<String>,
    pub species_id: Option<String>,
    pub stage: Option<String>,
    pub project_id: Option<String>,
    pub quarantine_only: Option<bool>,
    pub archived: Option<bool>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct SpecimenStats {
    pub total_specimens: i64,
    pub active_specimens: i64,
    pub quarantined: i64,
    pub archived: i64,
    pub by_stage: Vec<StageCount>,
    pub by_species: Vec<SpeciesCount>,
    pub recent_subcultures: i64,
}

#[derive(Debug, Serialize)]
pub struct StageCount {
    pub stage: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct SpeciesCount {
    pub species_code: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

/// Per-child configuration for a split operation.
#[derive(Debug, Deserialize)]
pub struct SplitChild {
    /// User-specified accession number; auto-generated from parent + letter suffix if None/empty.
    pub accession_number: Option<String>,
    pub location: Option<String>,
    pub media_batch_id: Option<String>,
    pub vessel_type: Option<String>,
    pub notes: Option<String>,
    pub health_status: Option<String>,
    /// Override the stage for this child (inherits parent stage when None).
    pub stage: Option<String>,
    /// If > 0, create a check-in reminder this many days after the split date.
    pub reminder_days: Option<i64>,
}

/// Request payload for splitting a specimen into N children.
#[derive(Debug, Deserialize)]
pub struct SplitSpecimenRequest {
    pub parent_specimen_id: String,
    pub date: String,
    pub children: Vec<SplitChild>,
    pub observations: Option<String>,
    pub notes: Option<String>,
    pub employee_id: Option<String>,
    pub health_status: Option<String>,
    pub contamination_flag: Option<bool>,
    pub contamination_notes: Option<String>,
    pub temperature_c: Option<f64>,
    pub ph: Option<f64>,
    pub light_cycle: Option<String>,
}

/// Lightweight summary of a specimen used in family-tree queries.
#[derive(Debug, Serialize)]
pub struct FamilyMember {
    pub id: String,
    pub accession_number: String,
    pub generation: i32,
    pub lineage_passage_offset: i32,
    pub subculture_count: i32,
    pub is_archived: bool,
    pub parent_specimen_id: Option<String>,
    pub root_specimen_id: Option<String>,
    pub health_status: Option<String>,
    pub location: Option<String>,
    pub species_code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SplitResult {
    pub archived_parent_id: String,
    pub children: Vec<SplitChildResult>,
}

#[derive(Debug, Serialize)]
pub struct SplitChildResult {
    pub id: String,
    pub accession_number: String,
}
