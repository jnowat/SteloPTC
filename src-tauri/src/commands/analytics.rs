// WP-58: Advanced analytics & reporting dashboards — Tauri command layer.
// Every command here is a thin wrapper around `db::analytics`; role gating
// beyond "must be an authenticated user" is applied only to the Technician
// Activity report (supervisor/admin only), matching the ROADMAP's framing of
// that report as workload/capacity visibility rather than a general metric.
use tauri::State;

use crate::auth as auth_service;
use crate::db::analytics::{self, TimeRange};
use crate::AppState;

#[tauri::command]
pub fn get_specimen_growth_rate(
    state: State<AppState>,
    token: String,
    time_range: String,
) -> Result<Vec<analytics::TimeSeriesPoint>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    analytics::specimen_growth_rate(&db.conn, TimeRange::parse(&time_range)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_subculture_frequency_trend(
    state: State<AppState>,
    token: String,
    time_range: String,
    species_id: Option<String>,
) -> Result<Vec<analytics::TimeSeriesPoint>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    analytics::subculture_frequency_trend(&db.conn, TimeRange::parse(&time_range), species_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_contamination_rate_trend(
    state: State<AppState>,
    token: String,
    time_range: String,
) -> Result<Vec<analytics::TimeSeriesPoint>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    analytics::contamination_rate_trend(&db.conn, TimeRange::parse(&time_range)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_passage_success_rate(
    state: State<AppState>,
    token: String,
    time_range: String,
) -> Result<analytics::PassageSuccessRate, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    analytics::passage_success_rate(&db.conn, TimeRange::parse(&time_range)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_batch_efficiency(
    state: State<AppState>,
    token: String,
    time_range: String,
) -> Result<Vec<analytics::MediaBatchEfficiency>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    analytics::media_batch_efficiency(&db.conn, TimeRange::parse(&time_range)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_strain_performance(
    state: State<AppState>,
    token: String,
    species_id: String,
) -> Result<Vec<analytics::StrainPerformance>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    analytics::strain_performance(&db.conn, &species_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_cryo_utilization(
    state: State<AppState>,
    token: String,
) -> Result<Vec<analytics::CryoUtilization>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    analytics::cryo_utilization(&db.conn).map_err(|e| e.to_string())
}

/// Supervisor/admin only — see ROADMAP.md WP-58 on why this report is
/// role-gated while the others are not.
#[tauri::command]
pub fn get_technician_activity(
    state: State<AppState>,
    token: String,
    time_range: String,
) -> Result<Vec<analytics::TechnicianActivity>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Only supervisors and admins can view technician activity".to_string());
    }
    analytics::technician_activity(&db.conn, TimeRange::parse(&time_range)).map_err(|e| e.to_string())
}

/// KPI summary strip: total active specimens, passages this week,
/// contamination rate this month, pending work-queue items, throughput
/// (passages per active specimen), and a month-over-month growth indicator.
#[derive(serde::Serialize)]
pub struct AnalyticsKpiSummary {
    pub total_active_specimens: i64,
    pub passages_this_week: i64,
    pub contamination_rate_this_month_pct: f64,
    pub pending_work_queue_items: i64,
    pub passages_per_active_specimen: f64,
    pub new_specimens_this_month: i64,
    pub new_specimens_last_month: i64,
}

#[tauri::command]
pub fn get_analytics_kpi_summary(state: State<AppState>, token: String) -> Result<AnalyticsKpiSummary, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let profile = crate::db::vocabulary::active_profile(&db.conn);
    let total_active_specimens = crate::db::dashboard::get_or_refresh_dashboard_cache(
        &db.conn,
        &profile,
        &state.dashboard_cache,
        crate::db::dashboard::DASHBOARD_CACHE_TTL,
    )
    .map(|(stats, _)| stats.active_specimens)
    .unwrap_or(0);

    let passages_this_week: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM subcultures WHERE date >= date('now', '-7 days')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let (month_total, month_contaminated): (i64, i64) = db
        .conn
        .query_row(
            "SELECT COUNT(*), SUM(contamination_flag) FROM subcultures WHERE date >= date('now', '-30 days')",
            [],
            |r| Ok((r.get(0)?, r.get::<_, Option<i64>>(1)?.unwrap_or(0))),
        )
        .unwrap_or((0, 0));
    let contamination_rate_this_month_pct =
        if month_total > 0 { 100.0 * month_contaminated as f64 / month_total as f64 } else { 0.0 };

    let pending_work_queue_items = crate::db::work_queue::compute_work_queue_items(&db.conn)
        .map(|items| items.len() as i64)
        .unwrap_or(0);

    let passages_per_active_specimen = if total_active_specimens > 0 {
        let total_passages: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM subcultures", [], |r| r.get(0))
            .unwrap_or(0);
        total_passages as f64 / total_active_specimens as f64
    } else {
        0.0
    };

    let new_specimens_this_month: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM specimens WHERE created_at >= date('now', 'start of month')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let new_specimens_last_month: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM specimens \
             WHERE created_at >= date('now', 'start of month', '-1 months') \
               AND created_at < date('now', 'start of month')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    Ok(AnalyticsKpiSummary {
        total_active_specimens,
        passages_this_week,
        contamination_rate_this_month_pct,
        pending_work_queue_items,
        passages_per_active_specimen,
        new_specimens_this_month,
        new_specimens_last_month,
    })
}

/// Reads the user's saved Analytics panel visibility configuration (which
/// panels are toggled on/off), persisted as a JSON blob in `app_settings`.
/// Returns `"{}"` (empty object) when nothing has been saved yet — the
/// frontend treats every panel as visible-by-default in that case.
#[tauri::command]
pub fn get_analytics_panel_config(state: State<AppState>, token: String) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    Ok(crate::db::queries::read_setting(&db.conn, "analytics_panel_config", "{}"))
}

#[tauri::command]
pub fn set_analytics_panel_config(state: State<AppState>, token: String, config_json: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    // Cheap validity check — reject non-JSON before persisting.
    serde_json::from_str::<serde_json::Value>(&config_json)
        .map_err(|e| format!("Invalid panel config JSON: {}", e))?;
    db.conn.execute(
        "INSERT INTO app_settings (key, value, updated_at) VALUES ('analytics_panel_config', ?1, datetime('now')) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        [config_json],
    ).map_err(|e| e.to_string())?;
    Ok(())
}
