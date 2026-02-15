use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Species {
    pub id: String,
    pub genus: String,
    pub species_name: String,
    pub common_name: Option<String>,
    pub species_code: String,
    pub default_subculture_interval_days: Option<i32>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSpeciesRequest {
    pub genus: String,
    pub species_name: String,
    pub common_name: Option<String>,
    pub species_code: String,
    pub default_subculture_interval_days: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSpeciesRequest {
    pub id: String,
    pub genus: Option<String>,
    pub species_name: Option<String>,
    pub common_name: Option<String>,
    pub species_code: Option<String>,
    pub default_subculture_interval_days: Option<i32>,
    pub notes: Option<String>,
}
