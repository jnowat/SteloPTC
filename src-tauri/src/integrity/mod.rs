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

    let rank = |s: &str| match s {
        "critical" => 0,
        "high" => 1,
        _ => 2,
    };
    issues.sort_by_key(|i| rank(&i.severity));

    let checks_run = ORPHAN_CHECKS.len() as i64 + 1;
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
}
