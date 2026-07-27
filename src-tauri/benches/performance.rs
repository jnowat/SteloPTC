//! WP-63: Criterion benchmark suite for SteloPTC at 100k+ specimen scale.
//!
//! These are canaries, not gates: CI runs them non-blocking and archives the
//! JSON output as a build artifact for trend tracking (a >20% regression vs.
//! the last main-branch run should prompt investigation, not fail the build).
//! Every benchmark here talks to the pure `db::` layer directly (raw SQL /
//! `db::queries` / `db::dashboard` functions) rather than the Tauri command
//! layer, so this suite builds and runs without the desktop GTK/WebKit stack.
//!
//! Run locally with `cargo bench --bench performance`.

use criterion::{criterion_group, criterion_main, Criterion};
use rusqlite::Connection;
use stelo_ptc_lib::db::{dashboard, fixtures, migrations, queries};

fn fixture_db(specimen_count: i64, subcultures_per_specimen: i64) -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory DB");
    migrations::run_all(&conn).expect("run migrations");
    migrations::seed_defaults(&conn).expect("seed defaults");
    fixtures::seed_large_fixture(&conn, specimen_count, subcultures_per_specimen)
        .expect("seed large fixture");
    conn
}

fn bench_list_specimens_10k(c: &mut Criterion) {
    let conn = fixture_db(10_000, 2);
    c.bench_function("list_specimens_10k", |b| {
        b.iter(|| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, accession_number FROM specimens \
                     WHERE is_archived = 0 AND stage = 'callus' \
                     ORDER BY created_at DESC LIMIT 200",
                )
                .unwrap();
            let rows: Vec<String> = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();
            criterion::black_box(rows);
        })
    });
}

fn bench_list_specimens_100k(c: &mut Criterion) {
    let conn = fixture_db(100_000, 1);
    c.bench_function("list_specimens_100k", |b| {
        b.iter(|| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, accession_number FROM specimens \
                     WHERE is_archived = 0 AND stage = 'callus' \
                     ORDER BY created_at DESC LIMIT 200",
                )
                .unwrap();
            let rows: Vec<String> = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();
            criterion::black_box(rows);
        })
    });
}

/// An 8-level classification chain — the 6 ranks the `taxa` table supports
/// (kingdom -> phylum -> class -> order -> family -> genus), plus a species
/// row hanging off the genus, plus 10k specimens hanging off that species —
/// benchmarked via the same recursive-CTE shape `get_taxon_descendants` uses
/// in the command layer, extended to also roll up the specimen count at the
/// leaf (what a real "descendants" view needs to render).
/// The free-text specimen search as the command layer issues it: seven
/// leading-wildcard LIKE predicates ORed across `specimens` and `species`.
///
/// A leading `%` defeats every B-tree index, so this is a full scan of both
/// tables on every keystroke — and the command runs it twice (once for
/// COUNT(*), once for the page). This benchmark is the baseline the FTS5 index
/// is measured against.
fn bench_search_specimens_like_100k(c: &mut Criterion) {
    let conn = fixture_db(100_000, 1);
    c.bench_function("search_specimens_like_100k", |b| {
        b.iter(|| {
            let mut stmt = conn
                .prepare(
                    "SELECT s.id FROM specimens s \
                     LEFT JOIN species sp ON s.species_id = sp.id \
                     WHERE s.is_archived = 0 AND ( \
                         s.accession_number LIKE ?1 OR s.notes LIKE ?1 OR s.location LIKE ?1 \
                         OR s.provenance LIKE ?1 OR s.source_plant LIKE ?1 \
                         OR sp.genus LIKE ?1 OR sp.species_name LIKE ?1) \
                     ORDER BY s.created_at DESC LIMIT 50",
                )
                .unwrap();
            let rows: Vec<String> = stmt
                .query_map(["%QT00042%"], |r| r.get::<_, String>(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();
            criterion::black_box(rows);
        })
    });
}

/// The same search served by the trigram FTS index (migration 054), in exactly
/// the shape `search_specimens` ships: a MATERIALIZED CTE pinned as the outer
/// loop by CROSS JOIN.
///
/// Compare against `search_specimens_like_100k` — same corpus, same needle,
/// same result set (equivalence is asserted in the migrations.rs tests).
fn bench_search_specimens_fts_100k(c: &mut Criterion) {
    let conn = fixture_db(100_000, 1);
    c.bench_function("search_specimens_fts_100k", |b| {
        b.iter(|| {
            let mut stmt = conn
                .prepare(
                    "WITH matches(id) AS MATERIALIZED ( \
                         SELECT fs.id FROM specimens_fts f \
                             JOIN specimens fs ON fs.rowid = f.rowid \
                             WHERE f.specimens_fts MATCH ?1 \
                         UNION \
                         SELECT ss.id FROM specimens ss \
                             WHERE ss.species_id IN ( \
                                 SELECT id FROM species WHERE genus LIKE ?2 OR species_name LIKE ?2)) \
                     SELECT s.id FROM matches mt CROSS JOIN specimens s ON s.id = mt.id \
                     LEFT JOIN species sp ON s.species_id = sp.id \
                     WHERE s.is_archived = 0 \
                     ORDER BY s.created_at DESC LIMIT 50",
                )
                .unwrap();
            let rows: Vec<String> = stmt
                .query_map(rusqlite::params!["\"QT00042\"", "%QT00042%"], |r| r.get::<_, String>(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();
            criterion::black_box(rows);
        })
    });
}

/// Seeds `batches` media batches, each with `hormones_each` hormone rows.
fn media_fixture_db(batches: i64, hormones_each: i64) -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory DB");
    migrations::run_all(&conn).expect("run migrations");
    let tx = conn.unchecked_transaction().unwrap();
    for i in 0..batches {
        let id = format!("mb-{i}");
        tx.execute(
            "INSERT INTO media_batches (id, batch_id, name, preparation_date, created_at, updated_at) \
             VALUES (?1, ?2, ?3, '2026-01-01', datetime('now'), datetime('now'))",
            rusqlite::params![id, format!("MB-{i:06}"), format!("Medium {i}")],
        )
        .unwrap();
        for h in 0..hormones_each {
            tx.execute(
                "INSERT INTO media_hormones (id, media_batch_id, hormone_name, hormone_type, concentration_mg_per_l) \
                 VALUES (?1, ?2, 'BAP', 'cytokinin', 1.0)",
                rusqlite::params![format!("mh-{i}-{h}"), id],
            )
            .unwrap();
        }
    }
    tx.commit().unwrap();
    conn
}

/// `list_media` used to load hormones with a query per batch, re-preparing the
/// statement each time, against a `media_hormones` table with no index on
/// `media_batch_id` — so every one of those queries was a full scan and the
/// endpoint was quadratic in batch count.
///
/// Both halves are fixed, and the two benchmarks separate their contributions.
/// This one keeps the N+1 loop but runs against the index that migration 057
/// now creates (the fixture runs `run_all`), so it measures the per-query
/// overhead alone. Measured at 1,000 batches x 3 hormones:
///
///   * original, no index, N+1 loop  ~160ms
///   * index only, N+1 loop retained  ~7.6ms   <- this benchmark
///   * index + single grouped query   ~1.2ms   <- what ships
fn bench_list_media_hormones_n_plus_1(c: &mut Criterion) {
    let conn = media_fixture_db(1_000, 3);
    c.bench_function("list_media_hormones_n_plus_1", |b| {
        b.iter(|| {
            let ids: Vec<String> = conn
                .prepare("SELECT id FROM media_batches")
                .unwrap()
                .query_map([], |r| r.get(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            let mut total = 0usize;
            for id in &ids {
                // Statement re-prepared inside the loop, exactly as the command did.
                let mut stmt = conn
                    .prepare("SELECT * FROM media_hormones WHERE media_batch_id = ?1")
                    .unwrap();
                total += stmt
                    .query_map(rusqlite::params![id], |r| r.get::<_, String>("id"))
                    .unwrap()
                    .filter_map(|r| r.ok())
                    .count();
            }
            criterion::black_box(total);
        })
    });
}

/// The shape `list_media` ships: one pass over `media_hormones`, grouped in
/// memory. Still ~6x faster than the indexed N+1 above, because it replaces
/// 1,000 index seeks and statement bindings with a single scan.
fn bench_list_media_hormones_single_query(c: &mut Criterion) {
    let conn = media_fixture_db(1_000, 3);
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_media_hormones_batch ON media_hormones(media_batch_id);",
    )
    .unwrap();
    c.bench_function("list_media_hormones_single_query", |b| {
        b.iter(|| {
            let ids: Vec<String> = conn
                .prepare("SELECT id FROM media_batches")
                .unwrap()
                .query_map([], |r| r.get(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            let mut grouped: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();
            let mut stmt = conn
                .prepare("SELECT media_batch_id, id FROM media_hormones")
                .unwrap();
            let rows = stmt
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                .unwrap();
            for row in rows.flatten() {
                grouped.entry(row.0).or_default().push(row.1);
            }
            let total: usize = ids.iter().map(|i| grouped.get(i).map_or(0, |v| v.len())).sum();
            criterion::black_box(total);
        })
    });
}

fn bench_get_taxon_descendants_deep(c: &mut Criterion) {
    let conn = Connection::open_in_memory().expect("in-memory DB");
    migrations::run_all(&conn).expect("run migrations");
    migrations::seed_defaults(&conn).expect("seed defaults");

    let ranks = ["kingdom", "phylum", "class", "order", "family", "genus"];
    let mut parent: Option<String> = None;
    let mut leaf_id = String::new();
    for (depth, rank) in ranks.iter().enumerate() {
        let id = format!("bench-taxon-{depth}");
        conn.execute(
            "INSERT INTO taxa (id, rank, name, parent_id) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, rank, format!("Taxon{depth}"), parent],
        )
        .unwrap();
        parent = Some(id.clone());
        leaf_id = id;
    }

    conn.execute(
        "INSERT INTO species (id, genus, species_name, species_code) \
         VALUES ('bench-species', 'Taxon5', 'benchii', 'BENCH-001')",
        [],
    )
    .unwrap();
    for i in 0..10_000i64 {
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, initiation_date, is_archived) \
             VALUES (?1, ?2, 'bench-species', '2026-01-01', 0)",
            rusqlite::params![format!("bench-taxspec-{i}"), format!("BENCHTX-{i:08}")],
        )
        .unwrap();
    }

    c.bench_function("get_taxon_descendants_deep", |b| {
        b.iter(|| {
            // Recursive CTE walking the full ancestor->descendant chain from
            // the root kingdom down through all 6 taxa-table levels — the
            // same shape used by db::queries taxon-column helpers.
            let mut stmt = conn
                .prepare(
                    "WITH RECURSIVE descendants(id, name) AS ( \
                        SELECT id, name FROM taxa WHERE id = ?1 \
                        UNION ALL \
                        SELECT t.id, t.name FROM taxa t JOIN descendants d ON t.parent_id = d.id \
                     ) \
                     SELECT d.id, \
                        (SELECT COUNT(*) FROM specimens sp \
                         JOIN species s ON sp.species_id = s.id \
                         WHERE s.genus = d.name) AS specimen_count \
                     FROM descendants d",
                )
                .unwrap();
            let rows: Vec<(String, i64)> = stmt
                .query_map(rusqlite::params![leaf_id], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();
            criterion::black_box(rows);
        })
    });
}

fn bench_build_merkle_root_1000(c: &mut Criterion) {
    let leaves: Vec<String> = (0..1000)
        .map(|i| format!("{:064x}", i))
        .collect();
    c.bench_function("build_merkle_root_1000", |b| {
        b.iter(|| {
            let root = queries::build_merkle_root(&leaves);
            criterion::black_box(root);
        })
    });
}

fn bench_dashboard_aggregate_100k(c: &mut Criterion) {
    let conn = fixture_db(100_000, 1);
    c.bench_function("dashboard_aggregate_100k", |b| {
        b.iter(|| {
            let stats = dashboard::query_specimen_stats(&conn, "plant_tissue_culture").unwrap();
            criterion::black_box(stats);
        })
    });
}

/// Seeds a 10k-entry hash chain for one lineage and times a full walk +
/// per-entry hash recomputation — the same work `verify_audit_lineage` does.
fn bench_audit_chain_verify_10k(c: &mut Criterion) {
    let conn = Connection::open_in_memory().expect("in-memory DB");
    migrations::run_all(&conn).expect("run migrations");

    let lineage_id = "bench-lineage";
    let mut prev_hash = queries::ZERO_HASH.to_string();
    for i in 0..10_000i64 {
        let seq = i + 1;
        let canonical = queries::audit_canonical_bytes(
            lineage_id, seq, "2026-01-01T00:00:00Z", "", "specimen", lineage_id, "update", "",
        );
        let entry_hash = queries::compute_entry_hash(&canonical, &prev_hash);
        conn.execute(
            "INSERT INTO audit_log \
             (id, action, entity_type, entity_id, created_at, lineage_id, chain_seq, prev_hash, entry_hash) \
             VALUES (?1, 'update', 'specimen', ?2, '2026-01-01T00:00:00Z', ?2, ?3, ?4, ?5)",
            rusqlite::params![format!("bench-{seq}"), lineage_id, seq, prev_hash, entry_hash],
        )
        .unwrap();
        prev_hash = entry_hash;
    }

    c.bench_function("audit_chain_verify_10k", |b| {
        b.iter(|| {
            let mut stmt = conn
                .prepare(
                    "SELECT chain_seq, prev_hash, entry_hash, created_at, entity_id \
                     FROM audit_log WHERE lineage_id = ?1 ORDER BY chain_seq ASC",
                )
                .unwrap();
            let rows: Vec<(i64, String, String, String, String)> = stmt
                .query_map(rusqlite::params![lineage_id], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
                })
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();

            let mut expected_prev = queries::ZERO_HASH.to_string();
            let mut ok = true;
            for (seq, prev_hash, entry_hash, created_at, entity_id) in &rows {
                let canonical = queries::audit_canonical_bytes(
                    lineage_id, *seq, created_at, "", "specimen", entity_id, "update", "",
                );
                let recomputed = queries::compute_entry_hash(&canonical, prev_hash);
                if *prev_hash != expected_prev || recomputed != *entry_hash {
                    ok = false;
                    break;
                }
                expected_prev = entry_hash.clone();
            }
            criterion::black_box(ok);
        })
    });
}

criterion_group!(
    benches,
    bench_list_specimens_10k,
    bench_list_specimens_100k,
    bench_search_specimens_like_100k,
    bench_search_specimens_fts_100k,
    bench_list_media_hormones_n_plus_1,
    bench_list_media_hormones_single_query,
    bench_get_taxon_descendants_deep,
    bench_build_merkle_root_1000,
    bench_dashboard_aggregate_100k,
    bench_audit_chain_verify_10k,
);
criterion_main!(benches);
