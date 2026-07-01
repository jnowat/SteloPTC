// WP-58: Advanced analytics & reporting dashboards — pure, testable query
// functions. Every function here takes a `&Connection` and (where relevant) a
// `time_range` selector and returns plain data (no Tauri types), so the whole
// module is unit-testable without a Tauri runtime; `commands::analytics` is a
// thin pass-through wrapper around these functions.
use rusqlite::Connection;
use serde::Serialize;

use super::DbResult;

/// The four time-range presets exposed in the Analytics UI's global selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeRange {
    Days30,
    Days90,
    Year1,
    All,
}

impl TimeRange {
    pub fn parse(s: &str) -> TimeRange {
        match s {
            "30d" => TimeRange::Days30,
            "90d" => TimeRange::Days90,
            "1y" => TimeRange::Year1,
            _ => TimeRange::All,
        }
    }

    /// The earliest date (`YYYY-MM-DD`) included in this range, or `None` for
    /// "all time" (in which case callers omit the lower-bound filter).
    fn since(self, conn: &Connection) -> Option<String> {
        let offset = match self {
            TimeRange::Days30 => "-30 days",
            TimeRange::Days90 => "-90 days",
            TimeRange::Year1 => "-365 days",
            TimeRange::All => return None,
        };
        conn.query_row("SELECT date('now', ?1)", [offset], |r| r.get::<_, String>(0)).ok()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeSeriesPoint {
    pub bucket: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PassageSuccessRate {
    pub total_passages: i64,
    pub successful_passages: i64,
    pub success_rate_pct: f64,
    /// Positive = improving (later half of the range has a higher success
    /// rate than the earlier half), negative = declining, 0.0 = flat/no data.
    pub trend_delta_pct: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaBatchEfficiency {
    pub batch_id: String,
    pub name: String,
    pub specimens_supported: i64,
    pub waste_rate_pct: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrainPerformance {
    pub strain_id: String,
    pub strain_name: String,
    pub mean_health: Option<f64>,
    pub total_specimens: i64,
    pub avg_days_between_passages: Option<f64>,
    pub best_performer_rate_pct: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CryoUtilization {
    pub species_id: String,
    pub species_code: String,
    pub vials_active: i64,
    pub vials_depleted_or_discarded: i64,
    pub utilization_rate_pct: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TechnicianActivity {
    pub user_id: String,
    pub display_name: String,
    pub passages_recorded: i64,
    pub contamination_events: i64,
}

/// New specimens created per week within `time_range`. One point per ISO
/// week (`%Y-%W`) that actually has at least one specimen — weeks with zero
/// creations are simply absent, matching the "friendly empty state, not a
/// zero-filled blank panel" acceptance criterion for a fully-empty range.
pub fn specimen_growth_rate(conn: &Connection, time_range: TimeRange) -> DbResult<Vec<TimeSeriesPoint>> {
    let since = time_range.since(conn);
    let sql = match &since {
        Some(_) => "SELECT strftime('%Y-%W', created_at) AS bucket, COUNT(*) AS cnt \
                     FROM specimens WHERE created_at >= ?1 GROUP BY bucket ORDER BY bucket ASC",
        None => "SELECT strftime('%Y-%W', created_at) AS bucket, COUNT(*) AS cnt \
                  FROM specimens GROUP BY bucket ORDER BY bucket ASC",
    };
    let mut stmt = conn.prepare(sql)?;
    let map_row = |r: &rusqlite::Row| -> rusqlite::Result<TimeSeriesPoint> {
        Ok(TimeSeriesPoint { bucket: r.get(0)?, value: r.get::<_, i64>(1)? as f64 })
    };
    let rows = match &since {
        Some(s) => stmt.query_map([s], map_row)?.filter_map(|r| r.ok()).collect(),
        None => stmt.query_map([], map_row)?.filter_map(|r| r.ok()).collect(),
    };
    Ok(rows)
}

/// Subculture (passage) frequency per week within `time_range`, optionally
/// restricted to one species.
pub fn subculture_frequency_trend(
    conn: &Connection,
    time_range: TimeRange,
    species_id: Option<&str>,
) -> DbResult<Vec<TimeSeriesPoint>> {
    let since = time_range.since(conn);
    let species_filter = species_id
        .map(|_| "AND specimen_id IN (SELECT id FROM specimens WHERE species_id = ?2)")
        .unwrap_or("");
    let date_filter = if since.is_some() { "date >= ?1" } else { "1=1" };
    let sql = format!(
        "SELECT strftime('%Y-%W', date) AS bucket, COUNT(*) AS cnt \
         FROM subcultures WHERE {date_filter} {species_filter} \
         GROUP BY bucket ORDER BY bucket ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let map_row = |r: &rusqlite::Row| -> rusqlite::Result<TimeSeriesPoint> {
        Ok(TimeSeriesPoint { bucket: r.get(0)?, value: r.get::<_, i64>(1)? as f64 })
    };
    let since_placeholder = since.clone().unwrap_or_default();
    let rows = match (since.is_some(), species_id) {
        (true, Some(sp)) => stmt.query_map(rusqlite::params![since_placeholder, sp], map_row)?.filter_map(|r| r.ok()).collect(),
        (true, None) => stmt.query_map(rusqlite::params![since_placeholder], map_row)?.filter_map(|r| r.ok()).collect(),
        (false, Some(sp)) => stmt.query_map(rusqlite::params![sp], map_row)?.filter_map(|r| r.ok()).collect(),
        (false, None) => stmt.query_map([], map_row)?.filter_map(|r| r.ok()).collect(),
    };
    Ok(rows)
}

/// Contamination rate (%) per week within `time_range`: contaminated
/// passages / total passages recorded that week.
pub fn contamination_rate_trend(conn: &Connection, time_range: TimeRange) -> DbResult<Vec<TimeSeriesPoint>> {
    let since = time_range.since(conn);
    let sql = match &since {
        Some(_) => "SELECT strftime('%Y-%W', date) AS bucket, \
                            100.0 * SUM(contamination_flag) / COUNT(*) AS rate \
                     FROM subcultures WHERE date >= ?1 GROUP BY bucket ORDER BY bucket ASC",
        None => "SELECT strftime('%Y-%W', date) AS bucket, \
                         100.0 * SUM(contamination_flag) / COUNT(*) AS rate \
                  FROM subcultures GROUP BY bucket ORDER BY bucket ASC",
    };
    let mut stmt = conn.prepare(sql)?;
    let map_row = |r: &rusqlite::Row| -> rusqlite::Result<TimeSeriesPoint> {
        Ok(TimeSeriesPoint { bucket: r.get(0)?, value: r.get(1)? })
    };
    let rows = match &since {
        Some(s) => stmt.query_map([s], map_row)?.filter_map(|r| r.ok()).collect(),
        None => stmt.query_map([], map_row)?.filter_map(|r| r.ok()).collect(),
    };
    Ok(rows)
}

/// Overall passage success rate over `time_range`, plus a trend delta
/// comparing the first half of the range to the second half. A passage is
/// "successful" if it was not flagged for contamination.
pub fn passage_success_rate(conn: &Connection, time_range: TimeRange) -> DbResult<PassageSuccessRate> {
    let since = time_range.since(conn);

    let (total, successful): (i64, i64) = match &since {
        Some(s) => conn.query_row(
            "SELECT COUNT(*), SUM(CASE WHEN contamination_flag = 0 THEN 1 ELSE 0 END) \
             FROM subcultures WHERE date >= ?1",
            [s],
            |r| Ok((r.get(0)?, r.get::<_, Option<i64>>(1)?.unwrap_or(0))),
        )?,
        None => conn.query_row(
            "SELECT COUNT(*), SUM(CASE WHEN contamination_flag = 0 THEN 1 ELSE 0 END) FROM subcultures",
            [],
            |r| Ok((r.get(0)?, r.get::<_, Option<i64>>(1)?.unwrap_or(0))),
        )?,
    };

    let success_rate_pct = if total > 0 { 100.0 * successful as f64 / total as f64 } else { 0.0 };

    // Trend: split the range at its midpoint date and compare success rates
    // of the two halves.
    let range_start: Option<String> = match &since {
        Some(s) => Some(s.clone()),
        None => conn
            .query_row("SELECT MIN(date) FROM subcultures", [], |r| r.get::<_, Option<String>>(0))
            .ok()
            .flatten(),
    };
    let range_end: Option<String> = match &since {
        Some(s) => conn
            .query_row("SELECT MAX(date) FROM subcultures WHERE date >= ?1", [s], |r| r.get::<_, Option<String>>(0))
            .ok()
            .flatten(),
        None => conn
            .query_row("SELECT MAX(date) FROM subcultures", [], |r| r.get::<_, Option<String>>(0))
            .ok()
            .flatten(),
    };

    let trend_delta_pct = match (range_start, range_end) {
        (Some(start), Some(end)) if start < end => {
            let midpoint: String = conn
                .query_row(
                    "SELECT date(?1, '+' || (CAST(julianday(?2) - julianday(?1) AS INTEGER) / 2) || ' days')",
                    [&start, &end],
                    |r| r.get(0),
                )
                .unwrap_or_else(|_| end.clone());

            let rate_in = |lo: &str, hi_exclusive: Option<&str>| -> f64 {
                let result: rusqlite::Result<(i64, Option<i64>)> = match hi_exclusive {
                    Some(hi) => conn.query_row(
                        "SELECT COUNT(*), SUM(CASE WHEN contamination_flag = 0 THEN 1 ELSE 0 END) \
                         FROM subcultures WHERE date >= ?1 AND date < ?2",
                        rusqlite::params![lo, hi],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    ),
                    None => conn.query_row(
                        "SELECT COUNT(*), SUM(CASE WHEN contamination_flag = 0 THEN 1 ELSE 0 END) \
                         FROM subcultures WHERE date >= ?1",
                        [lo],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    ),
                };
                match result {
                    Ok((t, s)) if t > 0 => 100.0 * s.unwrap_or(0) as f64 / t as f64,
                    _ => 0.0,
                }
            };

            rate_in(&midpoint, None) - rate_in(&start, Some(&midpoint))
        }
        _ => 0.0,
    };

    Ok(PassageSuccessRate {
        total_passages: total,
        successful_passages: successful,
        success_rate_pct,
        trend_delta_pct,
    })
}

/// Per-batch efficiency: how many distinct specimens were ever passaged
/// using this batch, and the batch's recorded waste rate (unused volume /
/// prepared volume). Batches with no `volume_prepared_ml` recorded report a
/// waste rate of 0.0 rather than dividing by zero.
pub fn media_batch_efficiency(conn: &Connection, time_range: TimeRange) -> DbResult<Vec<MediaBatchEfficiency>> {
    let since = time_range.since(conn);
    let sql = match &since {
        Some(_) => "SELECT mb.batch_id, mb.name, \
                            COUNT(DISTINCT sc.specimen_id) AS specimens_supported, \
                            CASE WHEN mb.volume_prepared_ml IS NOT NULL AND mb.volume_prepared_ml > 0 \
                                 THEN 100.0 * (mb.volume_prepared_ml - COALESCE(mb.volume_used_ml, 0)) / mb.volume_prepared_ml \
                                 ELSE 0.0 END AS waste_rate \
                     FROM media_batches mb \
                     LEFT JOIN subcultures sc ON sc.media_batch_id = mb.id \
                     WHERE mb.preparation_date >= ?1 \
                     GROUP BY mb.id ORDER BY specimens_supported DESC",
        None => "SELECT mb.batch_id, mb.name, \
                         COUNT(DISTINCT sc.specimen_id) AS specimens_supported, \
                         CASE WHEN mb.volume_prepared_ml IS NOT NULL AND mb.volume_prepared_ml > 0 \
                              THEN 100.0 * (mb.volume_prepared_ml - COALESCE(mb.volume_used_ml, 0)) / mb.volume_prepared_ml \
                              ELSE 0.0 END AS waste_rate \
                  FROM media_batches mb \
                  LEFT JOIN subcultures sc ON sc.media_batch_id = mb.id \
                  GROUP BY mb.id ORDER BY specimens_supported DESC",
    };
    let mut stmt = conn.prepare(sql)?;
    let map_row = |r: &rusqlite::Row| -> rusqlite::Result<MediaBatchEfficiency> {
        Ok(MediaBatchEfficiency {
            batch_id: r.get(0)?,
            name: r.get(1)?,
            specimens_supported: r.get(2)?,
            waste_rate_pct: r.get(3)?,
        })
    };
    let rows = match &since {
        Some(s) => stmt.query_map([s], map_row)?.filter_map(|r| r.ok()).collect(),
        None => stmt.query_map([], map_row)?.filter_map(|r| r.ok()).collect(),
    };
    Ok(rows)
}

/// Strain performance comparison within one species: mean specimen health
/// (health_status is stored as a text-encoded scale, '-1' = unknown and
/// excluded), average days between passages, total specimens ever bound to
/// the strain, and the fraction flagged `is_best_performer`.
pub fn strain_performance(conn: &Connection, species_id: &str) -> DbResult<Vec<StrainPerformance>> {
    let mut stmt = conn.prepare(
        "SELECT st.id, st.name, \
                (SELECT AVG(CAST(sp.health_status AS REAL)) FROM specimens sp \
                  WHERE sp.strain_id = st.id AND sp.health_status IS NOT NULL AND sp.health_status != '-1') AS mean_health, \
                (SELECT COUNT(*) FROM specimens sp WHERE sp.strain_id = st.id) AS total_specimens, \
                (SELECT AVG(gap) FROM ( \
                    SELECT julianday(sc.date) - julianday(LAG(sc.date) OVER (PARTITION BY sc.specimen_id ORDER BY sc.date)) AS gap \
                    FROM subcultures sc JOIN specimens sp2 ON sc.specimen_id = sp2.id \
                    WHERE sp2.strain_id = st.id \
                 ) WHERE gap IS NOT NULL) AS avg_gap, \
                (SELECT CASE WHEN COUNT(*) = 0 THEN 0.0 \
                             ELSE 100.0 * SUM(CASE WHEN sp.is_best_performer = 1 THEN 1 ELSE 0 END) / COUNT(*) END \
                 FROM specimens sp WHERE sp.strain_id = st.id) AS best_rate \
         FROM strains st \
         WHERE st.species_id = ?1 AND st.is_archived = 0 \
         ORDER BY mean_health DESC NULLS LAST",
    )?;
    let rows = stmt
        .query_map([species_id], |r| {
            Ok(StrainPerformance {
                strain_id: r.get(0)?,
                strain_name: r.get(1)?,
                mean_health: r.get(2)?,
                total_specimens: r.get(3)?,
                avg_days_between_passages: r.get(4)?,
                best_performer_rate_pct: r.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

/// Cryopreservation utilization by species/line: active vs. depleted+discarded
/// vial counts. There is no dedicated "thaw outcome" field in the current
/// `frozen_vials` schema (see WP-32) — vial depletion only records that the
/// last vial was thawed, not whether the resulting culture recovered — so
/// this reports vial-count utilization rather than a true thaw *success*
/// rate. A thaw-outcome column is a natural follow-up to make this exact.
pub fn cryo_utilization(conn: &Connection) -> DbResult<Vec<CryoUtilization>> {
    let mut stmt = conn.prepare(
        "SELECT fv.species_id, sp.species_code, \
                SUM(CASE WHEN fv.status = 'active' THEN fv.vial_count ELSE 0 END) AS active, \
                SUM(CASE WHEN fv.status != 'active' THEN fv.vial_count ELSE 0 END) AS inactive \
         FROM frozen_vials fv \
         JOIN species sp ON fv.species_id = sp.id \
         GROUP BY fv.species_id \
         ORDER BY sp.species_code ASC",
    )?;
    let rows = stmt
        .query_map([], |r| {
            let active: i64 = r.get(2)?;
            let inactive: i64 = r.get(3)?;
            let total = active + inactive;
            Ok(CryoUtilization {
                species_id: r.get(0)?,
                species_code: r.get(1)?,
                vials_active: active,
                vials_depleted_or_discarded: inactive,
                utilization_rate_pct: if total > 0 { 100.0 * inactive as f64 / total as f64 } else { 0.0 },
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

/// Per-technician activity within `time_range`: passages recorded and
/// contamination events flagged. Framed as workload/capacity visibility, not
/// a performance-review tool — see ROADMAP.md WP-58.
pub fn technician_activity(conn: &Connection, time_range: TimeRange) -> DbResult<Vec<TechnicianActivity>> {
    let since = time_range.since(conn);
    let sql = match &since {
        Some(_) => "SELECT u.id, u.display_name, COUNT(*) AS passages, \
                            SUM(sc.contamination_flag) AS contamination_events \
                     FROM subcultures sc JOIN users u ON sc.performed_by = u.id \
                     WHERE sc.date >= ?1 GROUP BY u.id ORDER BY passages DESC",
        None => "SELECT u.id, u.display_name, COUNT(*) AS passages, \
                         SUM(sc.contamination_flag) AS contamination_events \
                  FROM subcultures sc JOIN users u ON sc.performed_by = u.id \
                  GROUP BY u.id ORDER BY passages DESC",
    };
    let mut stmt = conn.prepare(sql)?;
    let map_row = |r: &rusqlite::Row| -> rusqlite::Result<TechnicianActivity> {
        Ok(TechnicianActivity {
            user_id: r.get(0)?,
            display_name: r.get(1)?,
            passages_recorded: r.get(2)?,
            contamination_events: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
        })
    };
    let rows = match &since {
        Some(s) => stmt.query_map([s], map_row)?.filter_map(|r| r.ok()).collect(),
        None => stmt.query_map([], map_row)?.filter_map(|r| r.ok()).collect(),
    };
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_all;

    fn analytics_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_all(&conn).unwrap();
        conn
    }

    fn seed_species(conn: &Connection, id: &str, code: &str) {
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) VALUES (?1, 'Genus', 'species', ?2)",
            [id, code],
        )
        .unwrap();
    }

    fn seed_specimen(conn: &Connection, id: &str, species_id: &str, created_at: &str, health: &str, strain_id: Option<&str>) {
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, strain_id, initiation_date, health_status, created_at, updated_at) \
             VALUES (?1, ?1, ?2, ?3, '2026-01-01', ?4, ?5, ?5)",
            rusqlite::params![id, species_id, strain_id, health, created_at],
        )
        .unwrap();
    }

    fn seed_subculture(conn: &Connection, id: &str, specimen_id: &str, date: &str, contaminated: bool, performer: Option<&str>) {
        conn.execute(
            "INSERT INTO subcultures (id, specimen_id, passage_number, date, contamination_flag, performed_by, created_at, updated_at) \
             VALUES (?1, ?2, 1, ?3, ?4, ?5, ?3, ?3)",
            rusqlite::params![id, specimen_id, date, contaminated as i64, performer],
        )
        .unwrap();
    }

    #[test]
    fn specimen_growth_rate_empty_when_no_specimens() {
        let conn = analytics_db();
        let points = specimen_growth_rate(&conn, TimeRange::All).unwrap();
        assert!(points.is_empty());
    }

    #[test]
    fn specimen_growth_rate_counts_per_week() {
        let conn = analytics_db();
        seed_species(&conn, "sp1", "ARABTH");
        seed_specimen(&conn, "s1", "sp1", "2026-01-05 10:00:00", "4", None);
        seed_specimen(&conn, "s2", "sp1", "2026-01-06 10:00:00", "4", None);
        seed_specimen(&conn, "s3", "sp1", "2026-02-10 10:00:00", "4", None);
        let points = specimen_growth_rate(&conn, TimeRange::All).unwrap();
        assert_eq!(points.len(), 2, "two distinct weeks");
        assert_eq!(points[0].value, 2.0);
        assert_eq!(points[1].value, 1.0);
    }

    #[test]
    fn specimen_growth_rate_respects_time_range_filter() {
        let conn = analytics_db();
        seed_species(&conn, "sp1", "ARABTH");
        // Far in the past — must be excluded by a 30-day window.
        seed_specimen(&conn, "s1", "sp1", "2020-01-01 10:00:00", "4", None);
        let points = specimen_growth_rate(&conn, TimeRange::Days30).unwrap();
        assert!(points.is_empty(), "old specimen must not appear in a 30-day window");
    }

    #[test]
    fn contamination_rate_trend_computes_percentage() {
        let conn = analytics_db();
        seed_species(&conn, "sp1", "ARABTH");
        seed_specimen(&conn, "s1", "sp1", "2026-01-05 10:00:00", "4", None);
        seed_subculture(&conn, "sc1", "s1", "2026-01-05", true, None);
        seed_subculture(&conn, "sc2", "s1", "2026-01-06", false, None);
        let points = contamination_rate_trend(&conn, TimeRange::All).unwrap();
        assert_eq!(points.len(), 1);
        assert!((points[0].value - 50.0).abs() < 0.01);
    }

    #[test]
    fn passage_success_rate_zero_total_is_handled_without_panic() {
        let conn = analytics_db();
        let result = passage_success_rate(&conn, TimeRange::All).unwrap();
        assert_eq!(result.total_passages, 0);
        assert_eq!(result.success_rate_pct, 0.0);
        assert_eq!(result.trend_delta_pct, 0.0);
    }

    #[test]
    fn passage_success_rate_computes_overall_and_trend() {
        let conn = analytics_db();
        seed_species(&conn, "sp1", "ARABTH");
        seed_specimen(&conn, "s1", "sp1", "2026-01-01 10:00:00", "4", None);
        // First half: all contaminated. Second half: all clean → improving trend.
        seed_subculture(&conn, "sc1", "s1", "2026-01-01", true, None);
        seed_subculture(&conn, "sc2", "s1", "2026-01-02", true, None);
        seed_subculture(&conn, "sc3", "s1", "2026-01-09", false, None);
        seed_subculture(&conn, "sc4", "s1", "2026-01-10", false, None);
        let result = passage_success_rate(&conn, TimeRange::All).unwrap();
        assert_eq!(result.total_passages, 4);
        assert_eq!(result.successful_passages, 2);
        assert!((result.success_rate_pct - 50.0).abs() < 0.01);
        assert!(result.trend_delta_pct > 0.0, "success rate improved in the second half");
    }

    #[test]
    fn media_batch_efficiency_computes_waste_rate_and_specimen_count() {
        let conn = analytics_db();
        seed_species(&conn, "sp1", "ARABTH");
        seed_specimen(&conn, "s1", "sp1", "2026-01-01 10:00:00", "4", None);
        seed_specimen(&conn, "s2", "sp1", "2026-01-01 10:00:00", "4", None);
        conn.execute(
            "INSERT INTO media_batches (id, batch_id, name, preparation_date, volume_prepared_ml, volume_used_ml) \
             VALUES ('mb1', 'MB-001', 'MS Basic', '2026-01-01', 1000, 400)",
            [],
        )
        .unwrap();
        seed_subculture(&conn, "sc1", "s1", "2026-01-05", false, None);
        conn.execute("UPDATE subcultures SET media_batch_id = 'mb1' WHERE id = 'sc1'", []).unwrap();
        seed_subculture(&conn, "sc2", "s2", "2026-01-06", false, None);
        conn.execute("UPDATE subcultures SET media_batch_id = 'mb1' WHERE id = 'sc2'", []).unwrap();

        let rows = media_batch_efficiency(&conn, TimeRange::All).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].specimens_supported, 2);
        assert!((rows[0].waste_rate_pct - 60.0).abs() < 0.01);
    }

    #[test]
    fn strain_performance_ranks_by_mean_health_and_excludes_unknown() {
        let conn = analytics_db();
        seed_species(&conn, "sp1", "ARABTH");
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code) VALUES ('st1', 'sp1', 'Alpha', 'A')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code) VALUES ('st2', 'sp1', 'Beta', 'B')",
            [],
        )
        .unwrap();
        seed_specimen(&conn, "s1", "sp1", "2026-01-01 10:00:00", "4", Some("st1"));
        seed_specimen(&conn, "s2", "sp1", "2026-01-01 10:00:00", "2", Some("st2"));
        // Unknown health must be excluded from the mean, not treated as -1.
        seed_specimen(&conn, "s3", "sp1", "2026-01-01 10:00:00", "-1", Some("st2"));

        let rows = strain_performance(&conn, "sp1").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].strain_id, "st1", "Alpha (health=4) ranks above Beta (health=2)");
        assert_eq!(rows[0].mean_health, Some(4.0));
        let beta = rows.iter().find(|r| r.strain_id == "st2").unwrap();
        assert_eq!(beta.mean_health, Some(2.0), "unknown (-1) reading excluded from mean");
        assert_eq!(beta.total_specimens, 2, "total_specimens still counts the unknown-health specimen");
    }

    #[test]
    fn cryo_utilization_computes_active_vs_inactive() {
        let conn = analytics_db();
        seed_species(&conn, "sp1", "HEK");
        conn.execute_batch(
            "INSERT INTO frozen_vials (id, species_id, vial_count, freeze_date, freeze_medium, status) VALUES
                ('v1', 'sp1', 5, '2026-01-01', 'DMSO', 'active'),
                ('v2', 'sp1', 3, '2026-01-01', 'DMSO', 'depleted');",
        )
        .unwrap();
        let rows = cryo_utilization(&conn).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].vials_active, 5);
        assert_eq!(rows[0].vials_depleted_or_discarded, 3);
        assert!((rows[0].utilization_rate_pct - 37.5).abs() < 0.01);
    }

    #[test]
    fn technician_activity_groups_by_user_and_counts_contamination() {
        let conn = analytics_db();
        seed_species(&conn, "sp1", "ARABTH");
        seed_specimen(&conn, "s1", "sp1", "2026-01-01 10:00:00", "4", None);
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, role) \
             VALUES ('u1', 'tech1', 'x', 'Tech One', 'tech')",
            [],
        )
        .unwrap();
        seed_subculture(&conn, "sc1", "s1", "2026-01-05", true, Some("u1"));
        seed_subculture(&conn, "sc2", "s1", "2026-01-06", false, Some("u1"));
        let rows = technician_activity(&conn, TimeRange::All).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].passages_recorded, 2);
        assert_eq!(rows[0].contamination_events, 1);
    }
}
