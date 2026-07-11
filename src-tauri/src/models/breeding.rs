use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreedingProgram {
    pub id: String,
    pub name: String,
    pub goal: Option<String>,
    pub start_date: Option<String>,
    pub target_traits: Option<String>,
    pub founder_strain_ids: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateBreedingProgramRequest {
    pub name: String,
    pub goal: Option<String>,
    pub start_date: Option<String>,
    pub target_traits: Option<String>,
    pub founder_strain_ids: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreedingRecord {
    pub id: String,
    pub program_id: String,
    pub strain_id: String,
    pub generation_number: i32,
    pub selection_notes: Option<String>,
    pub fitness_score: Option<f64>,
    pub selection_date: Option<String>,
    pub selected_by: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    /// WP-72: the lab that authored this selection record. `None` for a locally
    /// authored record; a partner lab's name for a record merged in from a
    /// cross-lab coordination bundle.
    pub origin_lab: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateBreedingRecordRequest {
    pub program_id: String,
    pub strain_id: String,
    pub generation_number: i32,
    pub selection_notes: Option<String>,
    pub fitness_score: Option<f64>,
    pub selection_date: Option<String>,
    pub notes: Option<String>,
}

/// Aggregated statistics for a single generation within a breeding program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationalSummary {
    pub generation_number: i32,
    pub record_count: i32,
    pub avg_fitness: Option<f64>,
}
