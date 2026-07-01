/// WP-63: shared large-fixture generator for benchmarks and at-scale tests.
///
/// The actual row-generation logic lives in `stelo_ptc_lib::db::fixtures`
/// (so `benches/performance.rs` can reuse it without depending on the
/// integration-test crate); this module re-exports it under the path named
/// in ROADMAP.md so both consumers have one obvious place to look.
pub use stelo_ptc_lib::db::fixtures::seed_large_fixture;
