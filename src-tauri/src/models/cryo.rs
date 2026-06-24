use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrozenVial {
    pub id: String,
    pub specimen_id: Option<String>,
    pub species_id: String,
    pub species_code: Option<String>,
    pub species_name: Option<String>,
    pub passage_number: i32,
    pub cumulative_pdl: Option<f64>,
    pub vial_count: i32,
    pub freeze_date: String,
    pub freeze_medium: String,
    pub location: Option<String>,
    pub location_freezer: Option<String>,
    pub location_tower: Option<String>,
    pub location_box: Option<String>,
    pub location_position: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateFrozenVialRequest {
    /// Optional source specimen that was frozen (for traceability).
    pub specimen_id: Option<String>,
    pub species_id: String,
    /// Passage number / lineage_passage_offset at the time of freezing.
    pub passage_number: i32,
    /// Cumulative PDL at the time of freezing (carries forward WP-31 data).
    pub cumulative_pdl: Option<f64>,
    /// Total number of vials placed in storage.
    pub vial_count: i32,
    pub freeze_date: String,
    /// E.g. "10% DMSO in complete medium".
    pub freeze_medium: String,
    pub location_freezer: Option<String>,
    pub location_tower: Option<String>,
    pub location_box: Option<String>,
    pub location_position: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListFrozenVialsParams {
    pub species_id: Option<String>,
    pub specimen_id: Option<String>,
    /// Filter by status: "active" | "depleted" | "discarded".  None = all.
    pub status: Option<String>,
    pub location_freezer: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ThawVialRequest {
    pub vial_id: String,
    pub thaw_date: String,
    /// How many vials to remove from inventory (default: 1).
    pub vials_to_thaw: Option<i32>,
    /// Location for the newly created specimen.
    pub location: Option<String>,
    pub notes: Option<String>,
    pub employee_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ThawVialResult {
    pub updated_vial: FrozenVial,
    pub new_specimen_id: String,
    pub new_specimen_accession: String,
}

#[derive(Debug, Deserialize)]
pub struct DiscardFrozenVialRequest {
    pub vial_id: String,
    pub notes: Option<String>,
}
