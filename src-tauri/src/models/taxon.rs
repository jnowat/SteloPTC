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
