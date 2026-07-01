use rusqlite::Connection;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::models::specimen::{SpeciesCount, SpecimenStats, StageCount};
use crate::models::subculture::{
    ContaminantTypeCount, ContaminationStats, CultureMaintenanceAlert, RecentContaminationEvent,
    SubcultureScheduleEntry, VesselContaminationCount, VialLineSummary,
};

/// WP-63: materialized dashboard cache. Holds the last-computed
/// `SpecimenStats` + `ContaminationStats` for a given profile, in memory only
/// (never persisted — recomputed fresh on every app start, per the ROADMAP
/// design). A `Mutex<Option<DashboardCacheEntry>>` field lives on `AppState`;
/// this module only depends on `&Mutex<..>` directly (not on the `AppState`
/// type itself) so the cache logic is unit-testable without a Tauri runtime.
#[derive(Clone)]
pub struct DashboardCacheEntry {
    pub profile: String,
    pub computed_at: Instant,
    pub specimen_stats: SpecimenStats,
    pub contamination_stats: ContaminationStats,
}

/// Default time-to-live for a cached dashboard snapshot before it is
/// recomputed on the next read. The ROADMAP default is 60 seconds; any write
/// that changes specimen/subculture counts should also call
/// `invalidate_dashboard_cache` directly so users never wait out the TTL to
/// see their own change reflected.
pub const DASHBOARD_CACHE_TTL: Duration = Duration::from_secs(60);

/// Returns the cached `(SpecimenStats, ContaminationStats)` for `profile` if
/// it is still fresh (within `ttl` and computed for the same profile),
/// otherwise recomputes both queries, stores the fresh snapshot, and returns it.
pub fn get_or_refresh_dashboard_cache(
    conn: &Connection,
    profile: &str,
    cache: &Mutex<Option<DashboardCacheEntry>>,
    ttl: Duration,
) -> Result<(SpecimenStats, ContaminationStats), String> {
    {
        let guard = cache.lock().map_err(|e| e.to_string())?;
        if let Some(entry) = guard.as_ref() {
            if entry.profile == profile && entry.computed_at.elapsed() < ttl {
                return Ok((entry.specimen_stats.clone(), entry.contamination_stats.clone()));
            }
        }
    }

    let specimen_stats = query_specimen_stats(conn, profile)?;
    let contamination_stats = query_contamination_stats(conn, profile)?;

    let mut guard = cache.lock().map_err(|e| e.to_string())?;
    *guard = Some(DashboardCacheEntry {
        profile: profile.to_string(),
        computed_at: Instant::now(),
        specimen_stats: specimen_stats.clone(),
        contamination_stats: contamination_stats.clone(),
    });

    Ok((specimen_stats, contamination_stats))
}

/// Drops the cached snapshot so the next read recomputes from scratch. Call
/// this from any write path that changes specimen or subculture counts
/// (create/update/delete/archive/split/passage/restore) so a cached dashboard
/// never contradicts a change the user just made.
pub fn invalidate_dashboard_cache(cache: &Mutex<Option<DashboardCacheEntry>>) {
    if let Ok(mut guard) = cache.lock() {
        *guard = None;
    }
}

/// Returns specimen statistics fully scoped to stages defined for `profile` in
/// the `stages` vocabulary table.  Every count — including the top-line
/// totals (total, active, quarantined, archived, recent subcultures) and the
/// per-species breakdown — uses an inner-join through `stages` so numbers
/// change when the active lab profile changes.
///
/// `by_stage` returns vocabulary **labels** (e.g. "Shoot Meristem") rather
/// than raw stage codes.
pub fn query_specimen_stats(conn: &Connection, profile: &str) -> Result<SpecimenStats, String> {
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM specimens sp \
             JOIN stages st ON sp.stage = st.code AND st.profile = ?1",
            [profile],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let active: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM specimens sp \
             JOIN stages st ON sp.stage = st.code AND st.profile = ?1 \
             WHERE sp.is_archived = 0",
            [profile],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let quarantined: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM specimens sp \
             JOIN stages st ON sp.stage = st.code AND st.profile = ?1 \
             WHERE sp.quarantine_flag = 1 AND sp.is_archived = 0",
            [profile],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let archived: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM specimens sp \
             JOIN stages st ON sp.stage = st.code AND st.profile = ?1 \
             WHERE sp.is_archived = 1",
            [profile],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Profile-aware: inner-join with stages so only stages defined for the
    // active profile are counted; return the vocabulary label for display.
    let mut stage_stmt = conn
        .prepare(
            "SELECT st.label, COUNT(*) AS cnt
             FROM specimens sp
             JOIN stages st ON sp.stage = st.code AND st.profile = ?1
             WHERE sp.is_archived = 0
             GROUP BY st.code
             ORDER BY cnt DESC, st.sort_order ASC",
        )
        .map_err(|e| e.to_string())?;
    let by_stage: Vec<StageCount> = stage_stmt
        .query_map([profile], |row| {
            Ok(StageCount {
                stage: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Profile-aware: only count specimens whose stage belongs to this profile.
    let mut species_stmt = conn
        .prepare(
            "SELECT sp_info.species_code, COUNT(*) \
             FROM specimens s \
             JOIN species sp_info ON s.species_id = sp_info.id \
             JOIN stages st ON s.stage = st.code AND st.profile = ?1 \
             WHERE s.is_archived = 0 \
             GROUP BY sp_info.species_code \
             ORDER BY COUNT(*) DESC",
        )
        .map_err(|e| e.to_string())?;
    let by_species: Vec<SpeciesCount> = species_stmt
        .query_map([profile], |row| {
            Ok(SpeciesCount {
                species_code: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Profile-aware: count only subcultures on specimens in this profile.
    let recent: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM subcultures sc \
             JOIN specimens sp ON sc.specimen_id = sp.id \
             JOIN stages st ON sp.stage = st.code AND st.profile = ?1 \
             WHERE sc.date >= date('now', '-7 days')",
            [profile],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(SpecimenStats {
        total_specimens: total,
        active_specimens: active,
        quarantined,
        archived,
        by_stage,
        by_species,
        recent_subcultures: recent,
    })
}

/// Returns contamination statistics restricted to specimens whose `stage` exists
/// in the `stages` vocabulary for `profile`.  Both the denominator
/// (`total_specimens`) and numerator (`contaminated_specimens`) are scoped to the
/// same profile so the resulting rate is internally consistent.
pub fn query_contamination_stats(
    conn: &Connection,
    profile: &str,
) -> Result<ContaminationStats, String> {
    let total_specimens: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM specimens sp
             JOIN stages st ON sp.stage = st.code AND st.profile = ?1
             WHERE sp.is_archived = 0",
            [profile],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let contaminated_specimens: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT sc.specimen_id)
             FROM subcultures sc
             JOIN specimens sp ON sc.specimen_id = sp.id
             JOIN stages st ON sp.stage = st.code AND st.profile = ?1
             WHERE sc.contamination_flag = 1 AND sp.is_archived = 0",
            [profile],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let contamination_rate_pct = if total_specimens > 0 {
        (contaminated_specimens as f64 / total_specimens as f64) * 100.0
    } else {
        0.0
    };

    let contaminated_vessels: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM subcultures sc
             JOIN specimens sp ON sc.specimen_id = sp.id
             JOIN stages st ON sp.stage = st.code AND st.profile = ?1
             WHERE sc.contamination_flag = 1",
            [profile],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let mut stmt = conn
        .prepare(
            "SELECT COALESCE(sc.vessel_type, 'Unknown') as vessel_type, \
                    COUNT(*) as cnt
             FROM subcultures sc
             JOIN specimens sp ON sc.specimen_id = sp.id
             JOIN stages st ON sp.stage = st.code AND st.profile = ?1
             WHERE sc.contamination_flag = 1
             GROUP BY sc.vessel_type
             ORDER BY cnt DESC
             LIMIT 10",
        )
        .map_err(|e| e.to_string())?;
    let by_vessel_type: Vec<VesselContaminationCount> = stmt
        .query_map([profile], |row| {
            Ok(VesselContaminationCount {
                vessel_type: row.get("vessel_type")?,
                count: row.get("cnt")?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut stmt_ct = conn
        .prepare(
            "SELECT COALESCE(sc.contaminant_type, 'Unknown') as contaminant_type, \
                    COUNT(*) as cnt
             FROM subcultures sc
             JOIN specimens sp ON sc.specimen_id = sp.id
             JOIN stages st ON sp.stage = st.code AND st.profile = ?1
             WHERE sc.contamination_flag = 1 AND sc.contaminant_type IS NOT NULL
             GROUP BY sc.contaminant_type
             ORDER BY cnt DESC
             LIMIT 10",
        )
        .map_err(|e| e.to_string())?;
    let by_ct_rows = stmt_ct
        .query_map([profile], |row| {
            Ok(ContaminantTypeCount {
                contaminant_type: row.get("contaminant_type")?,
                count: row.get("cnt")?,
            })
        })
        .map_err(|e| e.to_string())?;
    let by_contaminant_type: Vec<ContaminantTypeCount> =
        by_ct_rows.filter_map(|r| r.ok()).collect();

    let mut stmt2 = conn
        .prepare(
            "SELECT sc.id as subculture_id, sc.specimen_id, \
                    sp.accession_number, s.species_code, \
                    sc.passage_number, sc.date, sc.vessel_type, \
                    sc.contamination_notes, sc.contaminant_type
             FROM subcultures sc
             JOIN specimens sp ON sc.specimen_id = sp.id
             JOIN species s ON sp.species_id = s.id
             JOIN stages st ON sp.stage = st.code AND st.profile = ?1
             WHERE sc.contamination_flag = 1
             ORDER BY sc.date DESC
             LIMIT 10",
        )
        .map_err(|e| e.to_string())?;
    let stmt2_rows = stmt2
        .query_map([profile], |row| {
            Ok(RecentContaminationEvent {
                subculture_id: row.get("subculture_id")?,
                specimen_id: row.get("specimen_id")?,
                accession_number: row.get("accession_number")?,
                species_code: row.get("species_code")?,
                passage_number: row.get("passage_number")?,
                date: row.get("date")?,
                vessel_type: row.get("vessel_type")?,
                contamination_notes: row.get("contamination_notes")?,
                contaminant_type: row.get("contaminant_type")?,
            })
        })
        .map_err(|e| e.to_string())?;
    let recent_events: Vec<RecentContaminationEvent> =
        stmt2_rows.filter_map(|r| r.ok()).collect();

    Ok(ContaminationStats {
        total_specimens,
        contaminated_specimens,
        contamination_rate_pct,
        contaminated_vessels,
        by_vessel_type,
        by_contaminant_type,
        recent_events,
    })
}

/// Returns the subculture due-date schedule, restricted to specimens whose
/// `stage` exists in the `stages` vocabulary for `profile`.
pub fn query_subculture_schedule(
    conn: &Connection,
    profile: &str,
) -> Result<Vec<SubcultureScheduleEntry>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT
                sp.id              AS specimen_id,
                sp.accession_number,
                s.species_code     AS species_code,
                (s.genus || ' ' || s.species_name) AS species_name,
                sp.location,
                MAX(sc.date)       AS last_passage_date,
                s.default_subculture_interval_days AS interval_days,
                CASE
                    WHEN s.default_subculture_interval_days IS NOT NULL
                         AND MAX(sc.date) IS NOT NULL
                    THEN date(MAX(sc.date),
                              '+' || s.default_subculture_interval_days || ' days')
                    ELSE NULL
                END AS next_due_date,
                CASE
                    WHEN s.default_subculture_interval_days IS NOT NULL
                         AND MAX(sc.date) IS NOT NULL
                    THEN CAST(
                        julianday(date(MAX(sc.date),
                            '+' || s.default_subculture_interval_days || ' days'))
                        - julianday('now') AS INTEGER)
                    ELSE NULL
                END AS days_until_due
             FROM specimens sp
             JOIN species s ON sp.species_id = s.id
             JOIN stages st ON sp.stage = st.code AND st.profile = ?1
             LEFT JOIN subcultures sc ON sc.specimen_id = sp.id
             WHERE sp.is_archived = 0
             GROUP BY sp.id
             ORDER BY days_until_due ASC NULLS LAST",
        )
        .map_err(|e| e.to_string())?;

    let entries: Vec<SubcultureScheduleEntry> = stmt
        .query_map([profile], |row| {
            let days_until_due: Option<i64> = row.get("days_until_due")?;
            let is_overdue = days_until_due.map(|d| d < 0).unwrap_or(false);
            Ok(SubcultureScheduleEntry {
                specimen_id: row.get("specimen_id")?,
                accession_number: row.get("accession_number")?,
                species_code: row.get("species_code")?,
                species_name: row.get("species_name")?,
                location: row.get("location")?,
                last_passage_date: row.get("last_passage_date")?,
                interval_days: row.get("interval_days")?,
                next_due_date: row.get("next_due_date")?,
                days_until_due,
                is_overdue,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

/// Returns frozen-vial inventory aggregated by species/cell-line.
/// Only `active` lots are counted; `depleted` and `discarded` lots are excluded.
/// Results are ordered by total_vials ascending so low-stock lines appear first.
pub fn query_vial_summary_by_line(conn: &Connection) -> Result<Vec<VialLineSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT fv.species_id,
                    sp.species_code,
                    sp.genus || ' ' || sp.species_name AS species_name,
                    COUNT(*)          AS active_lots,
                    SUM(fv.vial_count) AS total_vials,
                    MIN(fv.vial_count) AS min_vials_in_lot
             FROM frozen_vials fv
             JOIN species sp ON fv.species_id = sp.id
             WHERE fv.status = 'active'
             GROUP BY fv.species_id
             ORDER BY total_vials ASC, sp.species_code ASC",
        )
        .map_err(|e| e.to_string())?;

    let items: Vec<VialLineSummary> = stmt
        .query_map([], |row| {
            Ok(VialLineSummary {
                species_id: row.get("species_id")?,
                species_code: row.get("species_code")?,
                species_name: row.get("species_name")?,
                active_lots: row.get("active_lots")?,
                total_vials: row.get("total_vials")?,
                min_vials_in_lot: row.get("min_vials_in_lot")?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(items)
}

/// Returns specimens in non-terminal, non-archived stages for `profile` that
/// have not had a recorded passage event in the last 7 days.  When no passage
/// exists the specimen's `created_at` is used as the reference date.
///
/// Results are ordered by `days_since_passage` descending so cultures that have
/// gone longest without attention appear first.  Capped at 20 rows.
pub fn query_culture_maintenance_alerts(
    conn: &Connection,
    profile: &str,
) -> Result<Vec<CultureMaintenanceAlert>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT
                 sp.id               AS specimen_id,
                 sp.accession_number,
                 s.species_code,
                 sp.stage,
                 st.label            AS stage_label,
                 MAX(sc.date)        AS last_passage_date,
                 CAST(julianday('now') - julianday(COALESCE(MAX(sc.date), sp.created_at))
                      AS INTEGER)    AS days_since_passage
             FROM specimens sp
             JOIN species s  ON sp.species_id = s.id
             JOIN stages  st ON sp.stage = st.code AND st.profile = ?1
             LEFT JOIN subcultures sc ON sc.specimen_id = sp.id
             WHERE sp.is_archived = 0 AND st.is_terminal = 0
             GROUP BY sp.id
             HAVING CAST(julianday('now') - julianday(COALESCE(MAX(sc.date), sp.created_at))
                         AS INTEGER) >= 7
             ORDER BY days_since_passage DESC
             LIMIT 20",
        )
        .map_err(|e| e.to_string())?;

    let items: Vec<CultureMaintenanceAlert> = stmt
        .query_map([profile], |row| {
            Ok(CultureMaintenanceAlert {
                specimen_id: row.get("specimen_id")?,
                accession_number: row.get("accession_number")?,
                species_code: row.get("species_code")?,
                stage: row.get("stage")?,
                stage_label: row.get("stage_label")?,
                last_passage_date: row.get("last_passage_date")?,
                days_since_passage: row.get("days_since_passage")?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(items)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    // ── Test helpers ──────────────────────────────────────────────────────────

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE stages (
                 id         INTEGER PRIMARY KEY AUTOINCREMENT,
                 profile    TEXT    NOT NULL,
                 code       TEXT    NOT NULL,
                 label      TEXT    NOT NULL,
                 sort_order INTEGER NOT NULL DEFAULT 0,
                 is_terminal INTEGER NOT NULL DEFAULT 0,
                 UNIQUE(profile, code)
             );
             CREATE TABLE species (
                 id         TEXT    PRIMARY KEY,
                 species_code TEXT  NOT NULL UNIQUE,
                 genus      TEXT    NOT NULL,
                 species_name TEXT  NOT NULL,
                 default_subculture_interval_days INTEGER
             );
             CREATE TABLE specimens (
                 id               TEXT    PRIMARY KEY,
                 accession_number TEXT    NOT NULL UNIQUE,
                 species_id       TEXT    NOT NULL,
                 stage            TEXT    NOT NULL DEFAULT 'explant',
                 quarantine_flag  INTEGER NOT NULL DEFAULT 0,
                 is_archived      INTEGER NOT NULL DEFAULT 0,
                 location         TEXT,
                 contamination_flag INTEGER NOT NULL DEFAULT 0,
                 created_at       TEXT    NOT NULL DEFAULT (datetime('now')),
                 updated_at       TEXT    NOT NULL DEFAULT (datetime('now'))
             );
             CREATE TABLE subcultures (
                 id                  TEXT    PRIMARY KEY,
                 specimen_id         TEXT    NOT NULL,
                 passage_number      INTEGER NOT NULL DEFAULT 1,
                 date                TEXT    NOT NULL,
                 vessel_type         TEXT,
                 contamination_flag  INTEGER NOT NULL DEFAULT 0,
                 contamination_notes TEXT,
                 contaminant_type    TEXT,
                 colonization_pct    REAL,
                 event_type          TEXT    NOT NULL DEFAULT 'passage',
                 created_at          TEXT    NOT NULL DEFAULT (datetime('now')),
                 updated_at          TEXT    NOT NULL DEFAULT (datetime('now'))
             );",
        )
        .unwrap();
        conn
    }

    fn seed_ptc_stages(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO stages (profile, code, label, sort_order) VALUES
                 ('plant_tissue_culture', 'explant',  'Explant',  1),
                 ('plant_tissue_culture', 'callus',   'Callus',   2),
                 ('plant_tissue_culture', 'shoot',    'Shoot',    5),
                 ('plant_tissue_culture', 'archived', 'Archived', 14);",
        )
        .unwrap();
    }

    fn seed_cell_culture_stages(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO stages (profile, code, label, sort_order) VALUES
                 ('cell_culture', 'adherent',   'Adherent',   1),
                 ('cell_culture', 'suspension', 'Suspension', 2);",
        )
        .unwrap();
    }

    fn insert_species(conn: &Connection, id: &str, code: &str, interval: Option<i64>) {
        conn.execute(
            "INSERT INTO species \
             (id, species_code, genus, species_name, default_subculture_interval_days) \
             VALUES (?1, ?2, 'Genus', 'species', ?3)",
            rusqlite::params![id, code, interval],
        )
        .unwrap();
    }

    fn insert_specimen(
        conn: &Connection,
        id: &str,
        accession: &str,
        species_id: &str,
        stage: &str,
    ) {
        conn.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))",
            rusqlite::params![id, accession, species_id, stage],
        )
        .unwrap();
    }

    fn insert_subculture(
        conn: &Connection,
        id: &str,
        specimen_id: &str,
        date: &str,
        contaminated: bool,
        vessel_type: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO subcultures \
             (id, specimen_id, passage_number, date, contamination_flag, vessel_type, \
              created_at, updated_at) \
             VALUES (?1, ?2, 1, ?3, ?4, ?5, datetime('now'), datetime('now'))",
            rusqlite::params![id, specimen_id, date, contaminated as i64, vessel_type],
        )
        .unwrap();
    }

    // ── Specimen stats ────────────────────────────────────────────────────────

    #[test]
    fn by_stage_returns_vocabulary_labels() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", None);
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "explant");
        insert_specimen(&conn, "s2", "ACC-002", "sp1", "callus");
        insert_specimen(&conn, "s3", "ACC-003", "sp1", "callus");

        let stats = query_specimen_stats(&conn, "plant_tissue_culture").unwrap();

        assert_eq!(stats.active_specimens, 3);
        assert_eq!(stats.by_stage.len(), 2);
        // Callus (count 2) first; Explant (count 1) second
        assert_eq!(stats.by_stage[0].stage, "Callus");
        assert_eq!(stats.by_stage[0].count, 2);
        assert_eq!(stats.by_stage[1].stage, "Explant");
        assert_eq!(stats.by_stage[1].count, 1);
    }

    #[test]
    fn by_stage_excludes_stages_not_in_active_profile() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        seed_cell_culture_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", None);
        // PTC specimen
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "explant");
        // Cell-culture specimen (stage 'adherent' not in PTC vocabulary)
        insert_specimen(&conn, "s2", "ACC-002", "sp1", "adherent");

        let ptc = query_specimen_stats(&conn, "plant_tissue_culture").unwrap();
        let cc = query_specimen_stats(&conn, "cell_culture").unwrap();

        // PTC sees only the explant specimen
        assert_eq!(ptc.by_stage.len(), 1);
        assert_eq!(ptc.by_stage[0].stage, "Explant");
        assert_eq!(ptc.by_stage[0].count, 1);

        // Cell culture sees only the adherent specimen
        assert_eq!(cc.by_stage.len(), 1);
        assert_eq!(cc.by_stage[0].stage, "Adherent");
        assert_eq!(cc.by_stage[0].count, 1);
    }

    #[test]
    fn by_stage_is_empty_for_profile_with_no_stages() {
        let conn = setup_db();
        // No stages seeded for 'mycology'
        insert_species(&conn, "sp1", "ARABTH", None);
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "explant");

        let stats = query_specimen_stats(&conn, "mycology").unwrap();
        assert!(stats.by_stage.is_empty());
    }

    #[test]
    fn aggregate_counts_are_profile_filtered() {
        // total/active/quarantined/archived must be scoped to stages that belong
        // to the requested profile — a PTC query must not count CC specimens.
        let conn = setup_db();
        seed_ptc_stages(&conn);
        seed_cell_culture_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", None);
        // PTC specimen (stage 'explant' is in the PTC vocabulary)
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "explant");
        // CC specimen (stage 'adherent' is in the CC vocabulary, not PTC)
        insert_specimen(&conn, "s2", "ACC-002", "sp1", "adherent");

        let ptc = query_specimen_stats(&conn, "plant_tissue_culture").unwrap();
        let cc = query_specimen_stats(&conn, "cell_culture").unwrap();

        // PTC profile sees only the PTC specimen
        assert_eq!(ptc.total_specimens, 1);
        assert_eq!(ptc.active_specimens, 1);

        // CC profile sees only the CC specimen
        assert_eq!(cc.total_specimens, 1);
        assert_eq!(cc.active_specimens, 1);
    }

    #[test]
    fn quarantined_and_archived_counts_are_profile_filtered() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        seed_cell_culture_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", None);

        // PTC: one quarantined specimen, one archived
        conn.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, quarantine_flag, is_archived, \
              created_at, updated_at) \
             VALUES ('s1', 'ACC-001', 'sp1', 'explant', 1, 0, \
              datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, quarantine_flag, is_archived, \
              created_at, updated_at) \
             VALUES ('s2', 'ACC-002', 'sp1', 'archived', 0, 1, \
              datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();

        // CC: one clean active specimen — should not appear in PTC counts
        conn.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, quarantine_flag, is_archived, \
              created_at, updated_at) \
             VALUES ('s3', 'ACC-003', 'sp1', 'adherent', 1, 0, \
              datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();

        let ptc = query_specimen_stats(&conn, "plant_tissue_culture").unwrap();
        // PTC: 1 quarantined (not archived), 1 archived, 0 of CC's quarantined
        assert_eq!(ptc.quarantined, 1);
        assert_eq!(ptc.archived, 1);

        let cc = query_specimen_stats(&conn, "cell_culture").unwrap();
        // CC: 1 quarantined, 0 archived
        assert_eq!(cc.quarantined, 1);
        assert_eq!(cc.archived, 0);
    }

    #[test]
    fn ptc_stage_sum_equals_active_when_all_stages_valid() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", None);
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "explant");
        insert_specimen(&conn, "s2", "ACC-002", "sp1", "callus");
        insert_specimen(&conn, "s3", "ACC-003", "sp1", "shoot");

        let stats = query_specimen_stats(&conn, "plant_tissue_culture").unwrap();
        let stage_total: i64 = stats.by_stage.iter().map(|s| s.count).sum();
        assert_eq!(stage_total, 3);
        assert_eq!(stage_total, stats.active_specimens);
    }

    // ── Contamination stats ───────────────────────────────────────────────────

    #[test]
    fn contamination_stats_scoped_to_active_profile() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        seed_cell_culture_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", None);
        insert_specimen(&conn, "ptc1", "ACC-001", "sp1", "explant");
        insert_specimen(&conn, "cc1",  "ACC-002", "sp1", "adherent");
        // Only the PTC specimen has a contaminated passage
        insert_subculture(&conn, "sc1", "ptc1", "2026-01-01", true, Some("flask"));

        let ptc = query_contamination_stats(&conn, "plant_tissue_culture").unwrap();
        let cc  = query_contamination_stats(&conn, "cell_culture").unwrap();

        assert_eq!(ptc.total_specimens, 1);
        assert_eq!(ptc.contaminated_specimens, 1);
        assert!((ptc.contamination_rate_pct - 100.0).abs() < 0.01);
        assert_eq!(ptc.contaminated_vessels, 1);

        assert_eq!(cc.total_specimens, 1);
        assert_eq!(cc.contaminated_specimens, 0);
        assert!((cc.contamination_rate_pct).abs() < 0.01);
        assert_eq!(cc.contaminated_vessels, 0);
    }

    #[test]
    fn contamination_rate_is_zero_when_no_specimens_for_profile() {
        let conn = setup_db();
        // No stages for 'mycology' — no specimens in scope
        insert_species(&conn, "sp1", "ARABTH", None);
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "explant");
        insert_subculture(&conn, "sc1", "s1", "2026-01-01", true, None);

        let stats = query_contamination_stats(&conn, "mycology").unwrap();
        assert_eq!(stats.total_specimens, 0);
        assert_eq!(stats.contaminated_specimens, 0);
        assert!((stats.contamination_rate_pct).abs() < 0.01);
    }

    #[test]
    fn contamination_by_vessel_type_scoped_to_profile() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", None);
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "explant");
        insert_subculture(&conn, "sc1", "s1", "2026-01-01", true, Some("jar"));
        insert_subculture(&conn, "sc2", "s1", "2026-01-02", true, Some("jar"));

        let stats = query_contamination_stats(&conn, "plant_tissue_culture").unwrap();
        assert_eq!(stats.by_vessel_type.len(), 1);
        assert_eq!(stats.by_vessel_type[0].vessel_type, "jar");
        assert_eq!(stats.by_vessel_type[0].count, 2);
    }

    #[test]
    fn contamination_by_contaminant_type_groups_correctly() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", None);
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "explant");
        // Two trich events, one wet_rot event
        conn.execute_batch(
            "INSERT INTO subcultures (id, specimen_id, passage_number, date,
                contamination_flag, contaminant_type, created_at, updated_at)
             VALUES
                ('sc-ct1', 's1', 1, '2026-01-01', 1, 'trich',   datetime('now'), datetime('now')),
                ('sc-ct2', 's1', 2, '2026-01-02', 1, 'trich',   datetime('now'), datetime('now')),
                ('sc-ct3', 's1', 3, '2026-01-03', 1, 'wet_rot', datetime('now'), datetime('now'));",
        )
        .unwrap();

        let stats = query_contamination_stats(&conn, "plant_tissue_culture").unwrap();
        assert_eq!(stats.by_contaminant_type.len(), 2);
        // trich should come first (highest count)
        assert_eq!(stats.by_contaminant_type[0].contaminant_type, "trich");
        assert_eq!(stats.by_contaminant_type[0].count, 2);
        assert_eq!(stats.by_contaminant_type[1].contaminant_type, "wet_rot");
        assert_eq!(stats.by_contaminant_type[1].count, 1);
    }

    #[test]
    fn contamination_by_contaminant_type_empty_when_no_type_set() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", None);
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "explant");
        // Contaminated but no contaminant_type set
        insert_subculture(&conn, "sc-nct", "s1", "2026-01-01", true, None);

        let stats = query_contamination_stats(&conn, "plant_tissue_culture").unwrap();
        assert_eq!(
            stats.by_contaminant_type.len(),
            0,
            "by_contaminant_type must be empty when contaminant_type is NULL"
        );
    }

    #[test]
    fn recent_events_include_contaminant_type() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", None);
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "explant");
        conn.execute_batch(
            "INSERT INTO subcultures (id, specimen_id, passage_number, date,
                contamination_flag, contaminant_type, created_at, updated_at)
             VALUES ('sc-re1', 's1', 1, '2026-01-01', 1, 'cobweb', datetime('now'), datetime('now'));",
        )
        .unwrap();

        let stats = query_contamination_stats(&conn, "plant_tissue_culture").unwrap();
        assert_eq!(stats.recent_events.len(), 1);
        assert_eq!(stats.recent_events[0].contaminant_type, Some("cobweb".to_string()));
    }

    // ── Subculture schedule ───────────────────────────────────────────────────

    #[test]
    fn schedule_scoped_to_active_profile() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        seed_cell_culture_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", Some(30));
        insert_specimen(&conn, "ptc1", "ACC-001", "sp1", "explant");
        insert_specimen(&conn, "cc1",  "ACC-002", "sp1", "adherent");
        insert_subculture(&conn, "sc1", "ptc1", "2025-12-01", false, None);
        insert_subculture(&conn, "sc2", "cc1",  "2025-12-01", false, None);

        let ptc_sched = query_subculture_schedule(&conn, "plant_tissue_culture").unwrap();
        let cc_sched  = query_subculture_schedule(&conn, "cell_culture").unwrap();

        assert_eq!(ptc_sched.len(), 1);
        assert_eq!(ptc_sched[0].accession_number, "ACC-001");

        assert_eq!(cc_sched.len(), 1);
        assert_eq!(cc_sched[0].accession_number, "ACC-002");
    }

    #[test]
    fn schedule_empty_for_profile_with_no_stages() {
        let conn = setup_db();
        insert_species(&conn, "sp1", "ARABTH", Some(30));
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "explant");
        insert_subculture(&conn, "sc1", "s1", "2025-12-01", false, None);

        let sched = query_subculture_schedule(&conn, "mycology").unwrap();
        assert!(sched.is_empty());
    }

    #[test]
    fn schedule_excludes_archived_specimens() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", Some(30));
        conn.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, is_archived, \
              created_at, updated_at) \
             VALUES ('s1', 'ACC-001', 'sp1', 'explant', 1, \
              datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();

        let sched = query_subculture_schedule(&conn, "plant_tissue_culture").unwrap();
        assert!(sched.is_empty());
    }

    // ── Vial summary by line ──────────────────────────────────────────────────

    fn setup_vial_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE species (
                 id           TEXT PRIMARY KEY,
                 species_code TEXT NOT NULL UNIQUE,
                 genus        TEXT NOT NULL,
                 species_name TEXT NOT NULL
             );
             CREATE TABLE frozen_vials (
                 id         TEXT    PRIMARY KEY,
                 species_id TEXT    NOT NULL REFERENCES species(id),
                 vial_count INTEGER NOT NULL DEFAULT 1,
                 status     TEXT    NOT NULL DEFAULT 'active'
             );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn vial_summary_groups_by_species() {
        let conn = setup_vial_db();
        conn.execute_batch(
            "INSERT INTO species (id, species_code, genus, species_name) VALUES
                 ('sp1', 'HEK', 'Homo', 'sapiens'),
                 ('sp2', 'CHO', 'Cricetulus', 'griseus');
             INSERT INTO frozen_vials (id, species_id, vial_count) VALUES
                 ('v1', 'sp1', 5), ('v2', 'sp1', 3), ('v3', 'sp2', 10);",
        )
        .unwrap();
        let summary = query_vial_summary_by_line(&conn).unwrap();
        assert_eq!(summary.len(), 2);
        // Ordered ascending by total_vials: HEK (total=8) before CHO (total=10)
        assert_eq!(summary[0].species_code, "HEK");
        assert_eq!(summary[0].active_lots, 2);
        assert_eq!(summary[0].total_vials, 8);
        assert_eq!(summary[0].min_vials_in_lot, 3);
        assert_eq!(summary[1].species_code, "CHO");
        assert_eq!(summary[1].total_vials, 10);
    }

    #[test]
    fn vial_summary_excludes_depleted_and_discarded() {
        let conn = setup_vial_db();
        conn.execute_batch(
            "INSERT INTO species (id, species_code, genus, species_name)
                 VALUES ('sp1', 'HEK', 'Homo', 'sapiens');
             INSERT INTO frozen_vials (id, species_id, vial_count, status) VALUES
                 ('v1', 'sp1', 5, 'active'),
                 ('v2', 'sp1', 0, 'depleted'),
                 ('v3', 'sp1', 3, 'discarded');",
        )
        .unwrap();
        let summary = query_vial_summary_by_line(&conn).unwrap();
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].active_lots, 1);
        assert_eq!(summary[0].total_vials, 5);
    }

    #[test]
    fn vial_summary_empty_when_no_active_vials() {
        let conn = setup_vial_db();
        conn.execute_batch(
            "INSERT INTO species (id, species_code, genus, species_name)
                 VALUES ('sp1', 'HEK', 'Homo', 'sapiens');
             INSERT INTO frozen_vials (id, species_id, vial_count, status)
                 VALUES ('v1', 'sp1', 0, 'depleted');",
        )
        .unwrap();
        let summary = query_vial_summary_by_line(&conn).unwrap();
        assert!(summary.is_empty());
    }

    #[test]
    fn vial_summary_min_reflects_smallest_active_lot() {
        let conn = setup_vial_db();
        conn.execute_batch(
            "INSERT INTO species (id, species_code, genus, species_name)
                 VALUES ('sp1', 'HEK', 'Homo', 'sapiens');
             INSERT INTO frozen_vials (id, species_id, vial_count, status) VALUES
                 ('v1', 'sp1', 10, 'active'),
                 ('v2', 'sp1', 2,  'active'),
                 ('v3', 'sp1', 7,  'active');",
        )
        .unwrap();
        let summary = query_vial_summary_by_line(&conn).unwrap();
        assert_eq!(summary[0].min_vials_in_lot, 2);
        assert_eq!(summary[0].total_vials, 19);
    }

    // ── Culture maintenance alerts ────────────────────────────────────────────

    #[test]
    fn maintenance_alerts_excludes_recently_passaged() {
        let conn = setup_db();
        seed_cell_culture_stages(&conn);
        insert_species(&conn, "sp1", "HEK", Some(7));
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "adherent");
        // Passage 1 day ago — below 7-day threshold, must NOT appear
        conn.execute(
            "INSERT INTO subcultures \
             (id, specimen_id, passage_number, date, created_at, updated_at) \
             VALUES ('sc1', 's1', 1, date('now', '-1 days'), \
             datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();
        let alerts = query_culture_maintenance_alerts(&conn, "cell_culture").unwrap();
        assert!(alerts.is_empty());
    }

    #[test]
    fn maintenance_alerts_includes_stale_specimen() {
        let conn = setup_db();
        seed_cell_culture_stages(&conn);
        insert_species(&conn, "sp1", "HEK", Some(7));
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "adherent");
        // Passage 10 days ago — above threshold, must appear
        conn.execute(
            "INSERT INTO subcultures \
             (id, specimen_id, passage_number, date, created_at, updated_at) \
             VALUES ('sc1', 's1', 1, date('now', '-10 days'), \
             datetime('now'), datetime('now'))",
            [],
        )
        .unwrap();
        let alerts = query_culture_maintenance_alerts(&conn, "cell_culture").unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].accession_number, "ACC-001");
        assert!(alerts[0].days_since_passage.unwrap() >= 7);
    }

    #[test]
    fn maintenance_alerts_scoped_to_profile() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        seed_cell_culture_stages(&conn);
        insert_species(&conn, "sp1", "HEK", None);
        // CC specimen created 30 days ago, never passaged
        conn.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, is_archived, \
              created_at, updated_at) \
             VALUES ('cc1', 'ACC-CC', 'sp1', 'adherent', 0, \
             date('now', '-30 days'), datetime('now'))",
            [],
        )
        .unwrap();
        // PTC specimen created 30 days ago, never passaged
        conn.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, is_archived, \
              created_at, updated_at) \
             VALUES ('ptc1', 'ACC-PTC', 'sp1', 'explant', 0, \
             date('now', '-30 days'), datetime('now'))",
            [],
        )
        .unwrap();

        let cc_alerts = query_culture_maintenance_alerts(&conn, "cell_culture").unwrap();
        let ptc_alerts = query_culture_maintenance_alerts(&conn, "plant_tissue_culture").unwrap();

        assert_eq!(cc_alerts.len(), 1);
        assert_eq!(cc_alerts[0].accession_number, "ACC-CC");
        assert_eq!(ptc_alerts.len(), 1);
        assert_eq!(ptc_alerts[0].accession_number, "ACC-PTC");
    }

    #[test]
    fn maintenance_alerts_excludes_terminal_stages() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO stages (profile, code, label, sort_order, is_terminal) \
             VALUES ('cell_culture', 'cryo_cc', 'Cryopreserved', 10, 1)",
            [],
        )
        .unwrap();
        insert_species(&conn, "sp1", "HEK", None);
        // Old specimen in a terminal stage — must NOT appear
        conn.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, is_archived, \
              created_at, updated_at) \
             VALUES ('s1', 'ACC-001', 'sp1', 'cryo_cc', 0, \
             date('now', '-30 days'), datetime('now'))",
            [],
        )
        .unwrap();
        let alerts = query_culture_maintenance_alerts(&conn, "cell_culture").unwrap();
        assert!(alerts.is_empty());
    }

    #[test]
    fn maintenance_alerts_uses_created_at_when_no_passage() {
        let conn = setup_db();
        seed_cell_culture_stages(&conn);
        insert_species(&conn, "sp1", "HEK", None);
        // Specimen created 14 days ago, never passaged
        conn.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, is_archived, \
              created_at, updated_at) \
             VALUES ('s1', 'ACC-001', 'sp1', 'adherent', 0, \
             date('now', '-14 days'), datetime('now'))",
            [],
        )
        .unwrap();
        let alerts = query_culture_maintenance_alerts(&conn, "cell_culture").unwrap();
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].days_since_passage.unwrap() >= 7);
        assert!(alerts[0].last_passage_date.is_none());
    }

    // ── WP-63: materialized dashboard cache ───────────────────────────────────

    #[test]
    fn dashboard_cache_matches_direct_query_and_is_reused() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", None);
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "explant");
        insert_specimen(&conn, "s2", "ACC-002", "sp1", "callus");

        let cache: Mutex<Option<DashboardCacheEntry>> = Mutex::new(None);
        let (stats, _contam) =
            get_or_refresh_dashboard_cache(&conn, "plant_tissue_culture", &cache, DASHBOARD_CACHE_TTL)
                .unwrap();
        let direct = query_specimen_stats(&conn, "plant_tissue_culture").unwrap();
        assert_eq!(stats.total_specimens, direct.total_specimens);
        assert_eq!(stats.active_specimens, 2);

        // A third specimen inserted directly (bypassing any invalidation call)
        // must NOT show up while the cache is still fresh — proves the second
        // call is served from cache, not recomputed.
        insert_specimen(&conn, "s3", "ACC-003", "sp1", "explant");
        let (cached_again, _) =
            get_or_refresh_dashboard_cache(&conn, "plant_tissue_culture", &cache, DASHBOARD_CACHE_TTL)
                .unwrap();
        assert_eq!(cached_again.active_specimens, 2, "cached snapshot must not see the new insert");
    }

    #[test]
    fn dashboard_cache_invalidation_forces_recompute() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", None);
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "explant");

        let cache: Mutex<Option<DashboardCacheEntry>> = Mutex::new(None);
        let (first, _) =
            get_or_refresh_dashboard_cache(&conn, "plant_tissue_culture", &cache, DASHBOARD_CACHE_TTL)
                .unwrap();
        assert_eq!(first.active_specimens, 1);

        insert_specimen(&conn, "s2", "ACC-002", "sp1", "explant");
        invalidate_dashboard_cache(&cache);

        let (after, _) =
            get_or_refresh_dashboard_cache(&conn, "plant_tissue_culture", &cache, DASHBOARD_CACHE_TTL)
                .unwrap();
        assert_eq!(after.active_specimens, 2, "invalidated cache must recompute and see the new insert");
    }

    #[test]
    fn dashboard_cache_recomputes_when_profile_changes() {
        let conn = setup_db();
        seed_ptc_stages(&conn);
        seed_cell_culture_stages(&conn);
        insert_species(&conn, "sp1", "ARABTH", None);
        insert_specimen(&conn, "s1", "ACC-001", "sp1", "explant");
        insert_specimen(&conn, "s2", "ACC-002", "sp1", "adherent");

        let cache: Mutex<Option<DashboardCacheEntry>> = Mutex::new(None);
        let (ptc, _) =
            get_or_refresh_dashboard_cache(&conn, "plant_tissue_culture", &cache, DASHBOARD_CACHE_TTL)
                .unwrap();
        assert_eq!(ptc.active_specimens, 1);

        // Switching profile must not return the PTC snapshot even though the
        // cache is still within its TTL — cache entries are profile-scoped.
        let (cc, _) =
            get_or_refresh_dashboard_cache(&conn, "cell_culture", &cache, DASHBOARD_CACHE_TTL).unwrap();
        assert_eq!(cc.active_specimens, 1);
        assert_eq!(cc.by_stage.first().map(|s| s.stage.as_str()), Some("Adherent"));
    }
}
