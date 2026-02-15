use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaBatch {
    pub id: String,
    pub batch_id: String,
    pub name: String,
    pub preparation_date: String,
    pub expiration_date: Option<String>,
    pub basal_salts: Option<String>,
    pub basal_salts_concentration: Option<f64>,
    pub vitamins: Option<String>,
    pub sucrose_g_per_l: Option<f64>,
    pub agar_g_per_l: Option<f64>,
    pub gelling_agent: Option<String>,
    pub ph_before_autoclave: Option<f64>,
    pub ph_after_autoclave: Option<f64>,
    pub sterilization_method: Option<String>,
    pub volume_prepared_ml: Option<f64>,
    pub volume_used_ml: Option<f64>,
    pub volume_remaining_ml: Option<f64>,
    pub storage_conditions: Option<String>,
    pub qc_notes: Option<String>,
    pub supplier_info: Option<String>,
    pub cost_per_batch: Option<f64>,
    pub osmolarity: Option<f64>,
    pub conductivity: Option<f64>,
    pub is_custom: bool,
    pub needs_review: bool,
    pub notes: Option<String>,
    pub hormones: Vec<MediaHormone>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaHormone {
    pub id: String,
    pub hormone_name: String,
    pub hormone_type: Option<String>,
    pub concentration_mg_per_l: f64,
    pub supplier: Option<String>,
    pub lot_number: Option<String>,
    pub reagent_batch_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMediaBatchRequest {
    pub name: String,
    pub preparation_date: String,
    pub expiration_date: Option<String>,
    pub basal_salts: Option<String>,
    pub basal_salts_concentration: Option<f64>,
    pub vitamins: Option<String>,
    pub sucrose_g_per_l: Option<f64>,
    pub agar_g_per_l: Option<f64>,
    pub gelling_agent: Option<String>,
    pub ph_before_autoclave: Option<f64>,
    pub ph_after_autoclave: Option<f64>,
    pub sterilization_method: Option<String>,
    pub volume_prepared_ml: Option<f64>,
    pub storage_conditions: Option<String>,
    pub qc_notes: Option<String>,
    pub supplier_info: Option<String>,
    pub cost_per_batch: Option<f64>,
    pub osmolarity: Option<f64>,
    pub conductivity: Option<f64>,
    pub is_custom: Option<bool>,
    pub notes: Option<String>,
    pub hormones: Option<Vec<CreateMediaHormoneRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMediaHormoneRequest {
    pub hormone_name: String,
    pub hormone_type: Option<String>,
    pub concentration_mg_per_l: f64,
    pub supplier: Option<String>,
    pub lot_number: Option<String>,
    pub reagent_batch_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMediaBatchRequest {
    pub id: String,
    pub name: Option<String>,
    pub expiration_date: Option<String>,
    pub volume_used_ml: Option<f64>,
    pub volume_remaining_ml: Option<f64>,
    pub storage_conditions: Option<String>,
    pub qc_notes: Option<String>,
    pub needs_review: Option<bool>,
    pub notes: Option<String>,
}
