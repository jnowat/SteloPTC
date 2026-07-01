/// Wires `tests/fixtures/seed_large.rs` into cargo's integration-test
/// discovery (Cargo only auto-discovers top-level files under `tests/`, not
/// nested subdirectories) and exercises it at a scale big enough to be a
/// meaningful smoke test without slowing down the mandated `cargo test` run.
#[path = "fixtures/seed_large.rs"]
mod seed_large;

use rusqlite::Connection;
use seed_large::seed_large_fixture;

#[test]
fn seed_large_fixture_smoke_test_at_moderate_scale() {
    let conn = Connection::open_in_memory().unwrap();
    stelo_ptc_lib::db::migrations::run_all(&conn).unwrap();
    let (specimens, subcultures) = seed_large_fixture(&conn, 200, 5).unwrap();
    assert_eq!(specimens, 200);
    assert_eq!(subcultures, 1000);
}
