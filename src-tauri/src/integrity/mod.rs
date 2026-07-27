// WP-76: Lab data-integrity self-check ("Health Report").
//
// A provenance app lives and dies on referential soundness — an orphaned
// specimen, a subculture pointing at a deleted parent, or a gap in an audit
// lineage all quietly corrupt the record the whole product is built to protect.
// SQLite enforces `PRAGMA foreign_keys=ON` for new writes, but does not
// retroactively catch rows that predate a constraint, arrive via an import, or
// survive a manual/out-of-band edit — and it never detects an audit-chain gap,
// which is exactly a deleted history row.
//
// This module runs a battery of read-only invariant checks over the live
// database and returns a structured report the operator (admin) can act on. It
// is pure (no I/O beyond the passed `&Connection`) and fully unit-testable under
// `--no-default-features`; the command layer is a thin admin-gated wrapper.

use rusqlite::Connection;
use serde::Serialize;

/// One failed integrity check.
#[derive(Debug, Clone, Serialize)]
pub struct IntegrityIssue {
    /// Stable machine id for the check.
    pub check: String,
    /// Human-readable description of what is wrong.
    pub title: String,
    /// `"critical"` (corrupts provenance) | `"high"` | `"normal"`.
    pub severity: String,
    /// Number of offending rows.
    pub count: i64,
    /// Up to 5 offending identifiers (accession numbers / ids / lineage ids),
    /// to make the problem actionable without dumping the whole table.
    pub examples: Vec<String>,
}

/// The full report of a self-check run.
#[derive(Debug, Serialize)]
pub struct IntegrityReport {
    /// True when no issue was found.
    pub ok: bool,
    /// Number of distinct checks executed.
    pub checks_run: i64,
    /// Issues found, most severe first.
    pub issues: Vec<IntegrityIssue>,
}

/// A single reference-orphan check: `count_sql` counts offenders,
/// `example_sql` returns up to 5 offending identifiers.
struct OrphanCheck {
    check: &'static str,
    title: &'static str,
    severity: &'static str,
    count_sql: &'static str,
    example_sql: &'static str,
}

fn run_orphan_check(conn: &Connection, spec: &OrphanCheck) -> Result<Option<IntegrityIssue>, String> {
    let count: i64 = conn
        .query_row(spec.count_sql, [], |r| r.get(0))
        .map_err(|e| format!("integrity check '{}' failed: {}", spec.check, e))?;
    if count == 0 {
        return Ok(None);
    }
    let mut stmt = conn.prepare(spec.example_sql).map_err(|e| e.to_string())?;
    let examples: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Some(IntegrityIssue {
        check: spec.check.to_string(),
        title: spec.title.to_string(),
        severity: spec.severity.to_string(),
        count,
        examples,
    }))
}

/// The catalogue of reference-orphan checks. Each verifies a foreign-key-like
/// relationship the schema intends but that may be violated by legacy rows,
/// imports, or out-of-band edits.
const ORPHAN_CHECKS: &[OrphanCheck] = &[
    OrphanCheck {
        // A specimen whose lab_profile is not one of the three known labs is
        // invisible everywhere: every read is scoped to the active profile, so
        // such a row belongs to no lab and silently vanishes from lists,
        // searches and dashboard counts without ever erroring. It cannot arise
        // through the command layer (migration 053 backfilled, and every write
        // path stamps a known value), but an import, a restored backup or an
        // out-of-band edit can produce one.
        check: "specimen_unknown_lab_profile",
        title: "Specimens filed under an unrecognised lab profile (invisible in every lab)",
        severity: "critical",
        count_sql: "SELECT COUNT(*) FROM specimens \
                    WHERE lab_profile NOT IN ('plant_tissue_culture','cell_culture','mycology')",
        example_sql: "SELECT accession_number FROM specimens \
                      WHERE lab_profile NOT IN ('plant_tissue_culture','cell_culture','mycology') LIMIT 5",
    },
    OrphanCheck {
        check: "specimen_missing_species",
        title: "Specimens referencing a species that no longer exists",
        severity: "critical",
        count_sql: "SELECT COUNT(*) FROM specimens WHERE species_id NOT IN (SELECT id FROM species)",
        example_sql: "SELECT accession_number FROM specimens WHERE species_id NOT IN (SELECT id FROM species) LIMIT 5",
    },
    OrphanCheck {
        check: "specimen_missing_strain",
        title: "Specimens bound to a strain that no longer exists",
        severity: "high",
        count_sql: "SELECT COUNT(*) FROM specimens WHERE strain_id IS NOT NULL AND strain_id NOT IN (SELECT id FROM strains)",
        example_sql: "SELECT accession_number FROM specimens WHERE strain_id IS NOT NULL AND strain_id NOT IN (SELECT id FROM strains) LIMIT 5",
    },
    OrphanCheck {
        check: "specimen_missing_parent",
        title: "Specimens whose parent specimen no longer exists",
        severity: "high",
        count_sql: "SELECT COUNT(*) FROM specimens WHERE parent_specimen_id IS NOT NULL AND parent_specimen_id NOT IN (SELECT id FROM specimens)",
        example_sql: "SELECT accession_number FROM specimens WHERE parent_specimen_id IS NOT NULL AND parent_specimen_id NOT IN (SELECT id FROM specimens) LIMIT 5",
    },
    OrphanCheck {
        check: "subculture_missing_specimen",
        title: "Passages/subcultures referencing a specimen that no longer exists",
        severity: "critical",
        count_sql: "SELECT COUNT(*) FROM subcultures WHERE specimen_id NOT IN (SELECT id FROM specimens)",
        example_sql: "SELECT id FROM subcultures WHERE specimen_id NOT IN (SELECT id FROM specimens) LIMIT 5",
    },
    OrphanCheck {
        check: "subculture_missing_media",
        title: "Passages referencing a media batch that no longer exists",
        severity: "normal",
        count_sql: "SELECT COUNT(*) FROM subcultures WHERE media_batch_id IS NOT NULL AND media_batch_id NOT IN (SELECT id FROM media_batches)",
        example_sql: "SELECT id FROM subcultures WHERE media_batch_id IS NOT NULL AND media_batch_id NOT IN (SELECT id FROM media_batches) LIMIT 5",
    },
    OrphanCheck {
        check: "strain_missing_species",
        title: "Strains referencing a species that no longer exists",
        severity: "critical",
        count_sql: "SELECT COUNT(*) FROM strains WHERE species_id NOT IN (SELECT id FROM species)",
        example_sql: "SELECT code FROM strains WHERE species_id NOT IN (SELECT id FROM species) LIMIT 5",
    },
    OrphanCheck {
        check: "duplicate_accession",
        title: "Accession numbers used by more than one specimen",
        severity: "critical",
        count_sql: "SELECT COUNT(*) FROM (SELECT accession_number FROM specimens GROUP BY accession_number HAVING COUNT(*) > 1)",
        example_sql: "SELECT accession_number FROM specimens GROUP BY accession_number HAVING COUNT(*) > 1 LIMIT 5",
    },
];

/// The audit-chain-gap check is special: it reasons about `chain_seq`
/// contiguity rather than a foreign key. A healthy lineage of N hash-chained
/// entries has genesis `chain_seq = 0` and max `chain_seq = N - 1`, so
/// `COUNT(*) = MAX(chain_seq) + 1`. Any mismatch means a history row was
/// removed — precisely the tamper the audit chain exists to make detectable.
fn run_chain_gap_check(conn: &Connection) -> Result<Option<IntegrityIssue>, String> {
    const COUNT_SQL: &str = "SELECT COUNT(*) FROM (\
        SELECT lineage_id FROM audit_log WHERE entry_hash IS NOT NULL \
        GROUP BY lineage_id HAVING COUNT(*) <> MAX(chain_seq) + 1)";
    const EXAMPLE_SQL: &str = "SELECT lineage_id FROM audit_log WHERE entry_hash IS NOT NULL \
        GROUP BY lineage_id HAVING COUNT(*) <> MAX(chain_seq) + 1 LIMIT 5";
    let count: i64 = conn
        .query_row(COUNT_SQL, [], |r| r.get(0))
        .map_err(|e| format!("integrity check 'audit_chain_gap' failed: {}", e))?;
    if count == 0 {
        return Ok(None);
    }
    let mut stmt = conn.prepare(EXAMPLE_SQL).map_err(|e| e.to_string())?;
    let examples: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Some(IntegrityIssue {
        check: "audit_chain_gap".to_string(),
        title: "Audit lineages with a sequence gap (a history row was removed)".to_string(),
        severity: "critical".to_string(),
        count,
        examples,
    }))
}

/// Verifies that the `specimens_fts` search index still agrees with the
/// `specimens` table it indexes.
///
/// `specimens_fts` is an FTS5 **external content** table (migration 054): it
/// stores only the index and resolves column values back through `rowid`. That
/// makes two silent-corruption modes possible, neither of which raises an error
/// at the time it happens:
///
///   * A migration that rebuilds `specimens` — the `specimens_v16` create/copy/
///     drop/rename pattern used several times in this schema — drops the
///     triggers with the old table and assigns new rowids. Search then returns
///     confidently wrong results.
///   * Any write that reaches the table without firing the triggers.
///
/// Two signals are used, because they catch different things and the cheap one
/// is also the one that can say *how many* rows are affected:
///
///   * `specimens_fts_docsize` holds one row per indexed document, so comparing
///     it against `specimens` detects rows missing from (or stale in) the index
///     and yields a count.
///   * FTS5's `('integrity-check', 1)` command compares the index against the
///     content table itself. The **rank=1 argument is required**: the plain
///     `('integrity-check')` form only verifies the index's internal
///     consistency and returns OK for an index that has silently stopped
///     tracking the table — verified empirically against SQLite 3.46.
///
/// Search returning wrong rows is a provenance problem in a regulated lab, so
/// this is `critical`.
fn run_search_index_check(conn: &Connection) -> Result<Option<IntegrityIssue>, String> {
    // The table only exists from migration 054 onward; a database mid-upgrade
    // is not an integrity failure.
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'specimens_fts'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if exists == 0 {
        return Ok(None);
    }

    let specimens: i64 = conn
        .query_row("SELECT COUNT(*) FROM specimens", [], |r| r.get(0))
        .map_err(|e| format!("integrity check 'search_index_out_of_sync' failed: {}", e))?;
    let indexed: i64 = conn
        .query_row("SELECT COUNT(*) FROM specimens_fts_docsize", [], |r| r.get(0))
        .unwrap_or(-1);

    let deep_check = conn.execute(
        "INSERT INTO specimens_fts (specimens_fts, rank) VALUES ('integrity-check', 1)",
        [],
    );

    if specimens == indexed && deep_check.is_ok() {
        return Ok(None);
    }

    let mut examples = vec![format!(
        "{} specimen(s) in the table, {} document(s) in the index",
        specimens, indexed
    )];
    if let Err(e) = deep_check {
        examples.push(e.to_string());
    }

    Ok(Some(IntegrityIssue {
        check: "search_index_out_of_sync".to_string(),
        title: "Search index disagrees with the specimen table — searches may return \
                wrong or incomplete results. Rebuild it from Admin."
            .to_string(),
        severity: "critical".to_string(),
        // The number of rows the index is missing, when that is knowable;
        // otherwise 1, meaning "the index is bad" without a row count.
        count: if indexed >= 0 { (specimens - indexed).abs().max(1) } else { 1 },
        examples,
    }))
}

/// Run every integrity check and return the aggregated report, issues sorted
/// most-severe first.
pub fn run_integrity_check(conn: &Connection) -> Result<IntegrityReport, String> {
    let mut issues: Vec<IntegrityIssue> = Vec::new();
    for spec in ORPHAN_CHECKS {
        if let Some(issue) = run_orphan_check(conn, spec)? {
            issues.push(issue);
        }
    }
    if let Some(issue) = run_chain_gap_check(conn)? {
        issues.push(issue);
    }
    if let Some(issue) = run_search_index_check(conn)? {
        issues.push(issue);
    }

    let rank = |s: &str| match s {
        "critical" => 0,
        "high" => 1,
        _ => 2,
    };
    issues.sort_by_key(|i| rank(&i.severity));

    // ORPHAN_CHECKS + the chain-gap check + the search-index check.
    let checks_run = ORPHAN_CHECKS.len() as i64 + 2;
    Ok(IntegrityReport {
        ok: issues.is_empty(),
        checks_run,
        issues,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_all;

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_all(&conn).unwrap();
        conn
    }

    fn seed_species(conn: &Connection, id: &str, code: &str) {
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) VALUES (?1, 'Genus', 'sp', ?2)",
            rusqlite::params![id, code],
        )
        .unwrap();
    }

    fn seed_specimen(conn: &Connection, id: &str, species_id: &str, accession: &str) {
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, stage, initiation_date) \
             VALUES (?1, ?2, ?3, 'explant', '2026-01-01')",
            rusqlite::params![id, accession, species_id],
        )
        .unwrap();
    }

    #[test]
    fn clean_db_reports_ok() {
        let conn = test_db();
        seed_species(&conn, "sp1", "AAA");
        seed_specimen(&conn, "s1", "sp1", "ACC-1");
        let report = run_integrity_check(&conn).unwrap();
        assert!(report.ok, "clean DB must report ok; issues: {:?}", report.issues);
        assert!(report.checks_run >= 8);
    }

    #[test]
    fn detects_orphaned_specimen_species() {
        let conn = test_db();
        // Insert a specimen whose species_id points nowhere (FKs are enforced on
        // the connection, so disable them just to plant the corrupt row — exactly
        // the "out-of-band edit / legacy import" the check exists to catch).
        conn.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
        seed_specimen(&conn, "s1", "ghost-species", "ACC-1");
        let report = run_integrity_check(&conn).unwrap();
        assert!(!report.ok);
        let issue = report.issues.iter().find(|i| i.check == "specimen_missing_species").unwrap();
        assert_eq!(issue.count, 1);
        assert_eq!(issue.examples, vec!["ACC-1".to_string()]);
        assert_eq!(issue.severity, "critical");
    }

    #[test]
    fn detects_orphaned_subculture() {
        let conn = test_db();
        conn.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
        conn.execute(
            "INSERT INTO subcultures (id, specimen_id, passage_number, date) VALUES ('sc1', 'ghost', 1, '2026-01-02')",
            [],
        )
        .unwrap();
        let report = run_integrity_check(&conn).unwrap();
        assert!(report.issues.iter().any(|i| i.check == "subculture_missing_specimen" && i.count == 1));
    }

    #[test]
    fn detects_orphaned_strain_binding() {
        let conn = test_db();
        seed_species(&conn, "sp1", "AAA");
        conn.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, strain_id, stage, initiation_date) \
             VALUES ('s1', 'ACC-1', 'sp1', 'ghost-strain', 'explant', '2026-01-01')",
            [],
        )
        .unwrap();
        let report = run_integrity_check(&conn).unwrap();
        assert!(report.issues.iter().any(|i| i.check == "specimen_missing_strain" && i.count == 1));
        // A valid species reference means that check stays clean.
        assert!(!report.issues.iter().any(|i| i.check == "specimen_missing_species"));
    }

    #[test]
    fn issues_are_sorted_most_severe_first() {
        let conn = test_db();
        conn.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
        // A 'normal'-severity orphan (subculture→media) plus a 'critical' one
        // (subculture→specimen); the critical must sort ahead.
        seed_species(&conn, "sp1", "AAA");
        seed_specimen(&conn, "s1", "sp1", "ACC-1");
        conn.execute("INSERT INTO subcultures (id, specimen_id, passage_number, date, media_batch_id) VALUES ('sc1','s1',1,'2026-01-02','ghost-media')", []).unwrap();
        conn.execute("INSERT INTO subcultures (id, specimen_id, passage_number, date) VALUES ('sc2','ghost-spec',1,'2026-01-02')", []).unwrap();
        let report = run_integrity_check(&conn).unwrap();
        assert!(report.issues.len() >= 2);
        assert_eq!(report.issues.first().unwrap().severity, "critical");
    }

    #[test]
    fn detects_audit_chain_gap() {
        let conn = test_db();
        // Build a 3-entry lineage 0,1,2 then delete the middle entry.
        for seq in 0..3 {
            conn.execute(
                "INSERT INTO audit_log (id, lineage_id, chain_seq, entry_hash, action, entity_type, created_at) \
                 VALUES (?1, 'lin1', ?2, ?3, 'x', 'specimen', '2026-01-01')",
                rusqlite::params![format!("e{seq}"), seq, format!("hash{seq}")],
            )
            .unwrap();
        }
        // Clean: no gap yet.
        assert!(run_chain_gap_check(&conn).unwrap().is_none());
        // Remove the middle entry → COUNT=2 but MAX(chain_seq)=2 → 2 != 3.
        conn.execute("DELETE FROM audit_log WHERE chain_seq = 1 AND lineage_id = 'lin1'", []).unwrap();
        let issue = run_chain_gap_check(&conn).unwrap().unwrap();
        assert_eq!(issue.count, 1);
        assert_eq!(issue.examples, vec!["lin1".to_string()]);
    }

    // ── Checks guarding the lab-isolation and search-index invariants ────────

    #[test]
    fn unknown_lab_profile_is_reported_as_critical() {
        // A specimen filed under no recognised lab is invisible in every lab —
        // it never appears and never errors, which is why it needs a check.
        let conn = test_db();
        seed_species(&conn, "sp1", "AAA");
        seed_specimen(&conn, "s1", "sp1", "ACC-001");
        conn.execute(
            "UPDATE specimens SET lab_profile = 'algae_culture' WHERE id = 's1'",
            [],
        )
        .unwrap();

        let report = run_integrity_check(&conn).unwrap();
        let issue = report
            .issues
            .iter()
            .find(|i| i.check == "specimen_unknown_lab_profile")
            .expect("an unrecognised lab profile must be reported");
        assert_eq!(issue.severity, "critical");
        assert_eq!(issue.count, 1);
        assert_eq!(issue.examples, vec!["ACC-001".to_string()]);
        assert!(!report.ok);
    }

    #[test]
    fn known_lab_profiles_are_not_reported() {
        let conn = test_db();
        seed_species(&conn, "sp1", "AAA");
        for (i, profile) in ["plant_tissue_culture", "cell_culture", "mycology"].iter().enumerate() {
            let id = format!("s{i}");
            seed_specimen(&conn, &id, "sp1", &format!("ACC-{i:03}"));
            conn.execute(
                "UPDATE specimens SET lab_profile = ?1 WHERE id = ?2",
                rusqlite::params![profile, id],
            )
            .unwrap();
        }
        let report = run_integrity_check(&conn).unwrap();
        assert!(
            !report.issues.iter().any(|i| i.check == "specimen_unknown_lab_profile"),
            "all three real lab profiles must pass"
        );
    }

    #[test]
    fn search_index_check_passes_on_a_healthy_database() {
        let conn = test_db();
        seed_species(&conn, "sp1", "AAA");
        seed_specimen(&conn, "s1", "sp1", "ACC-001");
        assert!(run_search_index_check(&conn).unwrap().is_none());
    }

    #[test]
    fn search_index_check_detects_an_index_that_stopped_tracking_the_table() {
        // Simulates what a future table-rebuild migration would do: the triggers
        // go away, writes stop reaching the index, and search silently returns
        // stale results. Nothing else in the system would notice.
        let conn = test_db();
        seed_species(&conn, "sp1", "AAA");
        seed_specimen(&conn, "s1", "sp1", "ACC-001");

        conn.execute_batch(
            "DROP TRIGGER specimens_fts_insert;
             DROP TRIGGER specimens_fts_update;
             DROP TRIGGER specimens_fts_delete;",
        )
        .unwrap();
        // This row now exists in `specimens` but not in the index.
        seed_specimen(&conn, "s2", "sp1", "ACC-002");

        let issue = run_search_index_check(&conn)
            .unwrap()
            .expect("a desynchronised search index must be reported");
        assert_eq!(issue.check, "search_index_out_of_sync");
        assert_eq!(issue.severity, "critical");

        let report = run_integrity_check(&conn).unwrap();
        assert!(!report.ok);
        assert!(report.issues.iter().any(|i| i.check == "search_index_out_of_sync"));
    }

    #[test]
    fn search_index_check_is_skipped_when_the_index_does_not_exist_yet() {
        // A database mid-upgrade (before migration 054) is not corrupt.
        let conn = test_db();
        conn.execute_batch(
            "DROP TRIGGER specimens_fts_insert;
             DROP TRIGGER specimens_fts_update;
             DROP TRIGGER specimens_fts_delete;
             DROP TABLE specimens_fts;",
        )
        .unwrap();
        assert!(run_search_index_check(&conn).unwrap().is_none());
    }

    #[test]
    fn checks_run_count_matches_the_checks_actually_executed() {
        // Guards against adding a check and forgetting to bump the count the
        // report advertises.
        let conn = test_db();
        let report = run_integrity_check(&conn).unwrap();
        assert_eq!(report.checks_run, ORPHAN_CHECKS.len() as i64 + 2);
    }
}
