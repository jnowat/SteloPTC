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
