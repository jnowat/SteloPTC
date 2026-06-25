use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subculture {
    pub id: String,
    pub specimen_id: String,
    pub passage_number: i32,
    pub date: String,
    pub media_batch_id: Option<String>,
    pub media_batch_name: Option<String>,
    pub ph: Option<f64>,
    pub temperature_c: Option<f64>,
    pub light_cycle: Option<String>,
    pub light_intensity_lux: Option<f64>,
    pub experimental_treatment: Option<String>,
    pub vessel_type: Option<String>,
    pub vessel_size: Option<String>,
    pub vessel_material: Option<String>,
    pub vessel_lid_type: Option<String>,
    pub location_from: Option<String>,
    pub location_to: Option<String>,
    pub temp_before: Option<f64>,
    pub temp_after: Option<f64>,
    pub humidity_before: Option<f64>,
    pub humidity_after: Option<f64>,
    pub light_before: Option<String>,
    pub light_after: Option<String>,
    pub exposure_duration_hours: Option<f64>,
    pub notes: Option<String>,
    pub observations: Option<String>,
    pub performed_by: Option<String>,
    pub performer_name: Option<String>,
    pub employee_id: Option<String>,
    pub health_status: Option<String>,
    pub contamination_flag: bool,
    pub contamination_notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// "passage" for normal subculture events; "death" for terminal death recordings.
    pub event_type: String,
    // ── WP-31: passage lineage & doubling time ───────────────────────────────
    /// Number of cells seeded at the start of this passage (for PDL / DT calc).
    pub seed_cell_count: Option<f64>,
    /// Number of cells harvested at the end of this passage (for PDL / DT calc).
    pub harvest_cell_count: Option<f64>,
    /// Split ratio used when cell counts are unavailable (e.g. 4.0 for a 1:4 split).
    pub split_ratio: Option<f64>,
    /// Population doublings gained during this passage (log₂(harvest/seed) or log₂(ratio)).
    pub pdl_gained: Option<f64>,
    /// Doubling time in hours, calculated from seed/harvest counts and elapsed time.
    pub doubling_time_hours: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSubcultureRequest {
    pub specimen_id: String,
    pub date: String,
    pub media_batch_id: Option<String>,
    pub ph: Option<f64>,
    pub temperature_c: Option<f64>,
    pub light_cycle: Option<String>,
    pub light_intensity_lux: Option<f64>,
    pub experimental_treatment: Option<String>,
    pub vessel_type: Option<String>,
    pub vessel_size: Option<String>,
    pub vessel_material: Option<String>,
    pub vessel_lid_type: Option<String>,
    pub location_from: Option<String>,
    pub location_to: Option<String>,
    pub temp_before: Option<f64>,
    pub temp_after: Option<f64>,
    pub humidity_before: Option<f64>,
    pub humidity_after: Option<f64>,
    pub light_before: Option<String>,
    pub light_after: Option<String>,
    pub exposure_duration_hours: Option<f64>,
    pub notes: Option<String>,
    pub observations: Option<String>,
    pub employee_id: Option<String>,
    pub health_status: Option<String>,
    pub contamination_flag: Option<bool>,
    pub contamination_notes: Option<String>,
    // ── WP-31: passage lineage & doubling time ───────────────────────────────
    pub seed_cell_count: Option<f64>,
    pub harvest_cell_count: Option<f64>,
    pub split_ratio: Option<f64>,
}

/// Payload for the "Record Death & Archive" terminal event.
#[derive(Debug, Deserialize)]
pub struct RecordSpecimenDeathRequest {
    pub specimen_id: String,
    pub date: String,
    pub observations: Option<String>,
    pub notes: Option<String>,
    pub employee_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSubcultureRequest {
    pub id: String,
    pub notes: Option<String>,
    pub observations: Option<String>,
    pub vessel_type: Option<String>,
    pub location_to: Option<String>,
    pub contamination_flag: Option<bool>,
    pub contamination_notes: Option<String>,
}

/// Lab-wide contamination statistics.
#[derive(Debug, Serialize)]
pub struct ContaminationStats {
    /// Total active (non-archived) specimens.
    pub total_specimens: i64,
    /// Specimens that have at least one contaminated vessel.
    pub contaminated_specimens: i64,
    /// Contamination rate as a percentage (0–100).
    pub contamination_rate_pct: f64,
    /// Total contaminated vessel events.
    pub contaminated_vessels: i64,
    /// Breakdown of contaminated events by vessel type.
    pub by_vessel_type: Vec<VesselContaminationCount>,
    /// The 10 most recent contamination events.
    pub recent_events: Vec<RecentContaminationEvent>,
}

#[derive(Debug, Serialize)]
pub struct VesselContaminationCount {
    pub vessel_type: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct RecentContaminationEvent {
    pub subculture_id: String,
    pub specimen_id: String,
    pub accession_number: String,
    pub species_code: String,
    pub passage_number: i32,
    pub date: String,
    pub vessel_type: Option<String>,
    pub contamination_notes: Option<String>,
}

/// Per-cell-line frozen vial inventory summary for the dashboard.
#[derive(Debug, Serialize)]
pub struct VialLineSummary {
    pub species_id: String,
    pub species_code: String,
    pub species_name: String,
    /// Number of active (non-depleted, non-discarded) frozen lots for this line.
    pub active_lots: i64,
    /// Total vials across all active lots.
    pub total_vials: i64,
    /// Vial count in the smallest active lot (low-stock risk indicator).
    pub min_vials_in_lot: i64,
}

/// A specimen in a non-terminal, non-archived profile stage that has not had a
/// recorded passage event in the last 7 days. Useful as a routine maintenance
/// alert in cell culture (cells may be approaching confluence).
#[derive(Debug, Serialize)]
pub struct CultureMaintenanceAlert {
    pub specimen_id: String,
    pub accession_number: String,
    pub species_code: String,
    /// Raw stage code (e.g. `"adherent"`).
    pub stage: String,
    /// Human-readable stage label from the vocabulary table (e.g. `"Adherent"`).
    pub stage_label: String,
    /// Date of the most recent recorded passage (ISO format); `None` if never passaged.
    pub last_passage_date: Option<String>,
    /// Days since the last passage; falls back to days since `created_at` when no passage exists.
    pub days_since_passage: Option<i64>,
}

/// One row of the subculture due-date schedule.
#[derive(Debug, Serialize)]
pub struct SubcultureScheduleEntry {
    pub specimen_id: String,
    pub accession_number: String,
    pub species_code: String,
    pub species_name: String,
    pub location: Option<String>,
    pub last_passage_date: Option<String>,
    pub interval_days: Option<i64>,
    /// ISO date string of when the next subculture is due (null if no interval set).
    pub next_due_date: Option<String>,
    /// Positive = days until due; negative = days overdue.
    pub days_until_due: Option<i64>,
    pub is_overdue: bool,
}
