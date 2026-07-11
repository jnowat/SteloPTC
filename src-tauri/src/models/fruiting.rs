use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FruitingRecord {
    pub id: String,
    pub specimen_id: String,
    pub flush_number: i32,
    pub harvest_date: String,
    pub fresh_weight_g: Option<f64>,
    pub dry_weight_g: Option<f64>,
    pub fruiting_temp_c: Option<f64>,
    pub fruiting_rh_percent: Option<f64>,
    pub fae_rate: Option<f64>,
    pub light_hours_per_day: Option<f64>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// A fruiting record joined with its parent specimen's identity, for the
/// cross-specimen Fruiting overview (mycology). The extra fields are read-only
/// display context; the record fields mirror `FruitingRecord` exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FruitingRecordWithSpecimen {
    pub id: String,
    pub specimen_id: String,
    pub specimen_accession: String,
    /// "Genus species (common)" — best-effort label; empty when the species row
    /// is missing (a specimen always has a species FK in practice).
    pub species_label: String,
    pub flush_number: i32,
    pub harvest_date: String,
    pub fresh_weight_g: Option<f64>,
    pub dry_weight_g: Option<f64>,
    pub fruiting_temp_c: Option<f64>,
    pub fruiting_rh_percent: Option<f64>,
    pub fae_rate: Option<f64>,
    pub light_hours_per_day: Option<f64>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateFruitingRecordRequest {
    pub specimen_id: String,
    pub flush_number: i32,
    pub harvest_date: String,
    pub fresh_weight_g: Option<f64>,
    pub dry_weight_g: Option<f64>,
    pub fruiting_temp_c: Option<f64>,
    pub fruiting_rh_percent: Option<f64>,
    pub fae_rate: Option<f64>,
    pub light_hours_per_day: Option<f64>,
    pub notes: Option<String>,
}
