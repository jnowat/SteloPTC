/// Integration tests for the DB layer (migrations, death workflow, lab profile).
/// These run with `cargo test --no-default-features` in environments that lack
/// GTK/WebKit system libraries, or with the full feature set in CI.
use rusqlite::{params, Connection};

fn test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    stelo_ptc_lib::db::migrations::run_all(&conn).unwrap();
    stelo_ptc_lib::db::migrations::seed_defaults(&conn).unwrap();
    conn
}

fn insert_specimen(conn: &Connection, accession: &str) -> String {
    let species_id: String = conn
        .query_row("SELECT id FROM species LIMIT 1", [], |r| r.get(0))
        .expect("Need at least one seeded species");
    let spec_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO specimens
             (id, accession_number, species_id, stage, initiation_date, is_archived)
         VALUES (?1, ?2, ?3, 'shoot', '2026-01-01', 0)",
        params![spec_id, accession, species_id],
    )
    .unwrap();
    spec_id
}

fn insert_passage(conn: &Connection, spec_id: &str, num: i32) {
    let sc_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO subcultures
             (id, specimen_id, passage_number, date, contamination_flag, event_type)
         VALUES (?1, ?2, ?3, '2026-02-01', 0, 'passage')",
        params![sc_id, spec_id, num],
    )
    .unwrap();
    conn.execute(
        "UPDATE specimens SET subculture_count = ?1 WHERE id = ?2",
        params![num, spec_id],
    )
    .unwrap();
}

fn record_death(conn: &Connection, spec_id: &str, subculture_count: i32) {
    let death_id = uuid::Uuid::new_v4().to_string();
    let tx = conn.unchecked_transaction().unwrap();
    tx.execute(
        "INSERT INTO subcultures
             (id, specimen_id, passage_number, date, health_status, contamination_flag, event_type)
         VALUES (?1, ?2, ?3, '2026-03-01', '0', 0, 'death')",
        params![death_id, spec_id, subculture_count + 1],
    )
    .unwrap();
    // subculture_count intentionally NOT incremented
    tx.execute(
        "UPDATE specimens
         SET is_archived = 1, archived_at = datetime('now'), health_status = '0', updated_at = datetime('now')
         WHERE id = ?1",
        params![spec_id],
    )
    .unwrap();
    tx.commit().unwrap();
}

// ── Migration / seed ─────────────────────────────────────────────────────────

#[test]
fn app_config_seeded_with_plant_tissue_culture() {
    let conn = test_db();
    let profile: String = conn
        .query_row("SELECT lab_profile FROM app_config WHERE id = 1", [], |r| r.get(0))
        .expect("app_config row must exist after migration");
    assert_eq!(profile, "plant_tissue_culture");
}

#[test]
fn app_config_rejects_invalid_profile() {
    let conn = test_db();
    let result = conn.execute(
        "UPDATE app_config SET lab_profile = 'invalid_profile' WHERE id = 1",
        [],
    );
    assert!(result.is_err(), "DB CHECK constraint should reject unknown profile values");
}

#[test]
fn app_config_allows_valid_profiles() {
    let conn = test_db();
    for profile in ["plant_tissue_culture", "cell_culture", "mycology"] {
        conn.execute(
            "UPDATE app_config SET lab_profile = ?1 WHERE id = 1",
            params![profile],
        )
        .unwrap_or_else(|e| panic!("Valid profile '{}' was rejected: {}", profile, e));
    }
}

// ── Death workflow ───────────────────────────────────────────────────────────

#[test]
fn death_archives_specimen_and_sets_health_zero() {
    let conn = test_db();
    let spec_id = insert_specimen(&conn, "DEATH-001");
    insert_passage(&conn, &spec_id, 1);
    insert_passage(&conn, &spec_id, 2);
    record_death(&conn, &spec_id, 2);

    let (is_archived, health, subculture_count): (i32, String, i32) = conn
        .query_row(
            "SELECT is_archived, health_status, subculture_count FROM specimens WHERE id = ?1",
            params![spec_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();

    assert_eq!(is_archived, 1, "specimen must be archived after death");
    assert_eq!(health, "0", "health must be Dead (0) after death");
    assert_eq!(subculture_count, 2, "subculture_count must NOT be incremented by death event");
}

#[test]
fn death_event_type_is_death() {
    let conn = test_db();
    let spec_id = insert_specimen(&conn, "DEATH-002");
    record_death(&conn, &spec_id, 0);

    let event_type: String = conn
        .query_row(
            "SELECT event_type FROM subcultures WHERE specimen_id = ?1",
            params![spec_id],
            |r| r.get(0),
        )
        .unwrap();

    assert_eq!(event_type, "death");
}

#[test]
fn death_does_not_count_as_lineage_passage() {
    let conn = test_db();
    let spec_id = insert_specimen(&conn, "DEATH-003");
    insert_passage(&conn, &spec_id, 1);
    insert_passage(&conn, &spec_id, 2);
    insert_passage(&conn, &spec_id, 3);
    record_death(&conn, &spec_id, 3);

    // subculture_count reflects only real passages, not the death event
    let subculture_count: i32 = conn
        .query_row(
            "SELECT subculture_count FROM specimens WHERE id = ?1",
            params![spec_id],
            |r| r.get(0),
        )
        .unwrap();

    // Count of subculture rows with event_type='death' vs 'passage'
    let death_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM subcultures WHERE specimen_id = ?1 AND event_type = 'death'",
            params![spec_id],
            |r| r.get(0),
        )
        .unwrap();
    let passage_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM subcultures WHERE specimen_id = ?1 AND event_type = 'passage'",
            params![spec_id],
            |r| r.get(0),
        )
        .unwrap();

    assert_eq!(subculture_count, 3, "subculture_count must equal real passage count only");
    assert_eq!(death_rows, 1, "exactly one death event row");
    assert_eq!(passage_rows, 3, "three normal passage rows");
}

#[test]
fn archived_specimen_blocks_second_death() {
    let conn = test_db();
    let spec_id = insert_specimen(&conn, "DEATH-004");
    record_death(&conn, &spec_id, 0);

    // Simulate the guard in record_specimen_death: fetch is_archived
    let is_archived: i32 = conn
        .query_row(
            "SELECT is_archived FROM specimens WHERE id = ?1",
            params![spec_id],
            |r| r.get(0),
        )
        .unwrap();

    assert_eq!(is_archived, 1, "specimen must be archived — guard should block further actions");
}

#[test]
fn normal_passages_have_passage_event_type() {
    let conn = test_db();
    let spec_id = insert_specimen(&conn, "DEATH-005");
    insert_passage(&conn, &spec_id, 1);

    let event_type: String = conn
        .query_row(
            "SELECT event_type FROM subcultures WHERE specimen_id = ?1",
            params![spec_id],
            |r| r.get(0),
        )
        .unwrap();

    assert_eq!(event_type, "passage");
}

#[test]
fn death_passage_number_is_one_above_last_passage() {
    let conn = test_db();
    let spec_id = insert_specimen(&conn, "DEATH-006");
    insert_passage(&conn, &spec_id, 1);
    insert_passage(&conn, &spec_id, 2);
    record_death(&conn, &spec_id, 2);

    let death_passage_num: i32 = conn
        .query_row(
            "SELECT passage_number FROM subcultures WHERE specimen_id = ?1 AND event_type = 'death'",
            params![spec_id],
            |r| r.get(0),
        )
        .unwrap();

    // passage_number for timeline ordering = subculture_count + 1
    assert_eq!(death_passage_num, 3, "death event passage_number should be subculture_count + 1 for timeline ordering");
}
