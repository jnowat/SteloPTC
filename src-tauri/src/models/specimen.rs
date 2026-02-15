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
    pub parent_specimen_id: Option<String>,
    pub qr_code_data: Option<String>,
    pub notes: Option<String>,
    pub is_archived: bool,
    pub archived_at: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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
