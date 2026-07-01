// WP-63: deterministic large-scale fixture generator, shared by the
// Criterion benchmark suite (`benches/performance.rs`) and any test that
// needs at-scale data. Idempotent — if specimens already exist in the given
// connection, it returns the existing counts rather than doubling the
// dataset, so re-running a benchmark against a persistent DB file is safe.
use rusqlite::{params, Connection};

use super::DbResult;

/// Seeds `specimen_count` specimens (cycling through four stages) each with
/// `subcultures_per_specimen` passage records, all under one fixture species.
/// Returns `(specimen_count, subculture_count)` actually present after the
/// call (which may be larger than requested if a fixture was already seeded
/// with different parameters — this function never truncates existing data).
pub fn seed_large_fixture(
    conn: &Connection,
    specimen_count: i64,
    subcultures_per_specimen: i64,
) -> DbResult<(i64, i64)> {
    let existing: i64 = conn
        .query_row("SELECT COUNT(*) FROM specimens WHERE species_id = 'fx-sp1'", [], |r| r.get(0))
        .unwrap_or(0);
    if existing > 0 {
        let subs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM subcultures WHERE specimen_id LIKE 'fx-spec-%'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        return Ok((existing, subs));
    }

    conn.execute(
        "INSERT OR IGNORE INTO species (id, genus, species_name, species_code) \
         VALUES ('fx-sp1', 'Fixturia', 'generatus', 'FIX-001')",
        [],
    )?;

    let tx = conn.unchecked_transaction()?;
    for i in 0..specimen_count {
        let id = format!("fx-spec-{i}");
        let accession = format!("FIX-{i:08}");
        let stage = match i % 4 {
            0 => "explant",
            1 => "callus",
            2 => "shoot",
            _ => "plantlet",
        };
        tx.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, initiation_date, is_archived, created_at, updated_at) \
             VALUES (?1, ?2, 'fx-sp1', ?3, '2026-01-01', 0, datetime('now'), datetime('now'))",
            params![id, accession, stage],
        )?;
        for p in 0..subcultures_per_specimen {
            let sub_id = format!("fx-sub-{i}-{p}");
            tx.execute(
                "INSERT INTO subcultures \
                 (id, specimen_id, passage_number, date, event_type, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, date('now', '-' || ?3 || ' days'), 'passage', datetime('now'), datetime('now'))",
                params![sub_id, id, p + 1],
            )?;
        }
    }
    tx.commit()?;

    let specimens: i64 = conn.query_row("SELECT COUNT(*) FROM specimens", [], |r| r.get(0))?;
    let subcultures: i64 = conn.query_row("SELECT COUNT(*) FROM subcultures", [], |r| r.get(0))?;
    Ok((specimens, subcultures))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_all;

    #[test]
    fn seed_large_fixture_creates_expected_row_counts() {
        let conn = Connection::open_in_memory().unwrap();
        run_all(&conn).unwrap();
        let (specimens, subcultures) = seed_large_fixture(&conn, 50, 3).unwrap();
        assert_eq!(specimens, 50);
        assert_eq!(subcultures, 150);
    }

    #[test]
    fn seed_large_fixture_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_all(&conn).unwrap();
        seed_large_fixture(&conn, 20, 2).unwrap();
        let (specimens, subcultures) = seed_large_fixture(&conn, 20, 2).unwrap();
        // A second call must not double the dataset.
        assert_eq!(specimens, 20);
        assert_eq!(subcultures, 40);
    }
}
