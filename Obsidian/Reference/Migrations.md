---
title: Migrations
aliases: [Schema Migrations, run_all, schema_version]
tags: [reference, database, sqlite, migrations, rust]
type: reference
status: binding
version: 0.54.0
created: 2026-07-29
updated: 2026-07-29
cssclasses: [wide-tables]
---

> [!abstract] In one sentence
> `src-tauri/src/db/migrations.rs` is a single 5,775-line file holding **59 numbered, append-only
> migrations** run by one flat `run_all` on every app start, stamped into a `schema_version` table,
> and applied through one of two harnesses — `apply`, which is atomic, and `apply_untransacted`,
> which is not and is used by exactly seven legacy migrations that cannot tolerate a transaction.

## `run_all`

```rust
pub fn run_all(conn: &Connection) -> DbResult<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
    ")?;

    let current: i64 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))
        .unwrap_or(0);

    if current < 1 { apply(conn, 1, migration_001_initial)?; }
    // … 57 more gates …
    if current < 59 { apply(conn, 59, migration_059_location_layout)?; }

    Ok(())
}
```

| Fact | Detail |
|---|---|
| Called from | `Database::run_migrations()`, invoked in `lib.rs`'s Tauri `.setup()` before `seed_defaults()` |
| Failure surface | `"Migration error: {e}"` — the app does not start |
| `schema_version` | One row per applied migration. A fresh database ends with 59 rows and `MAX(version) = 59` |
| Idempotence | Guaranteed by the `if current < N` gates plus `IF NOT EXISTS` throughout. Two tests pin it: `migrations_are_idempotent` and `a_retried_run_all_is_a_no_op` |
| Ordering in the file | `run_all` first, then the migration bodies in **reverse-chronological** order (059, 058, 057 …) with a few historical exceptions where 016–018 and 019–023 sit out of sequence. The **numbers** are what matter, not the file position |

> [!danger] Append-only. Never edit a shipped migration.
> A migration that has already run on someone's database will never run again — `schema_version`
> holds its number. Editing its body changes nothing for existing installs and silently produces two
> different schemas for the same version number. Fix forward with a new migration.

## `apply` vs `apply_untransacted`

```rust
fn apply<F>(conn: &Connection, version: i64, migrate: F) -> DbResult<()>
where F: FnOnce(&Connection) -> DbResult<()> {
    let tx = conn.unchecked_transaction()?;
    migrate(&tx)?;
    tx.execute("INSERT INTO schema_version (version) VALUES (?1)", [version])?;
    tx.commit()?;
    Ok(())
}
```

`apply` wraps the migration body **and** the version stamp in one transaction. The doc comment
records the failure it exists to close, and it is not hypothetical:

> SQLite DDL is transactional, but `execute_batch` is not — it applies statements one at a time — so
> a failure partway through left earlier statements committed with the version *not* advanced. On
> the next start the migration re-ran, and `ALTER TABLE … ADD COLUMN` is not idempotent: it fails
> with "duplicate column name", and the application can no longer start at all. On a desktop app
> that is the user's only copy of their lab records.

`apply_untransacted` runs the body, then stamps the version as a **separate** statement.

```rust
fn apply_untransacted<F>(conn: &Connection, version: i64, migrate: F) -> DbResult<()>
where F: FnOnce(&Connection) -> DbResult<()> {
    migrate(conn)?;
    conn.execute("INSERT INTO schema_version (version) VALUES (?1)", [version])?;
    Ok(())
}
```

### Why exactly seven migrations are transaction-hostile

Two constructs make a migration impossible to wrap, and both appear in this file.

1. **`PRAGMA foreign_keys = OFF/ON` is a documented no-op inside a transaction.** The
   table-rebuild migrations (create `_vN`, copy rows, drop the original, rename) depend on it
   actually taking effect. Run under `apply`, FK enforcement would silently stay on and the rebuild
   would either fail or leave dangling references.
2. **An `execute_batch` string that contains its own `BEGIN; … COMMIT;`** errors with
   "cannot start a transaction within a transaction".

| # | Function | Which construct |
|---|---|---|
| 2 | `migration_002_v019` | `PRAGMA foreign_keys` OFF/ON around a `specimens_v2` rebuild |
| 3 | `migration_003_v0110` | `PRAGMA foreign_keys` OFF/ON around a conditional `specimens_v3` rebuild |
| 16 | `migration_016_vocabulary_tables` | `PRAGMA` **and** an inner `BEGIN;`/`COMMIT;` around the `specimens_v16` rebuild |
| 17 | `migration_017_remaining_vocabularies` | Inner `BEGIN;`/`COMMIT;` around three table rebuilds |
| 18 | `migration_018_cell_culture_vocabulary` | Inner `BEGIN;`/`COMMIT;` |
| 23 | `migration_023_cell_culture_vocabulary` | Inner `BEGIN;`/`COMMIT;` |
| 27 | `migration_027_mycology_vocabulary` | Inner `BEGIN;`/`COMMIT;` |

> [!warning] The retry hazard is accepted here, not fixed
> These seven are all additive-or-rebuild migrations that predate `apply`, and they were left alone
> rather than rewritten. If one of them fails midway on an old database, the app can end up unable to
> boot — the exact failure mode `apply` was introduced to close. That trade was made deliberately:
> rewriting a shipped rebuild migration risks a correctness bug on real data, which is worse.
> **A new migration must use `apply`**, and therefore must not use `PRAGMA foreign_keys` or an inner
> `BEGIN;`/`COMMIT;`.

---

## The 59 migrations

`†` marks `apply_untransacted`. **Bold** names are new tables.

| # | Function | What it does |
|---|---|---|
| 1 | `migration_001_initial` | The founding schema: **users**, **sessions**, **species**, **projects**, **specimens**, **tags**, **specimen_tags**, **media_batches**, **media_hormones**, **subcultures**, **attachments**, **reminders**, **compliance_records**, **inventory_items**, **audit_log** |
| 2 † | `migration_002_v019` | Rebuilds `specimens` as `specimens_v2` (adds `employee_id`, widens the stage CHECK); inventory gains `physical_state` / `concentration` / `concentration_unit`; `media_batches.employee_id`; `subcultures.employee_id` + `health_status`; `media_hormones.amount_used` + `amount_unit`; **prepared_solutions** |
| 3 † | `migration_003_v0110` | Conditional second `specimens` rebuild, guarded on the stored SQL containing `shoot_meristem`; **error_logs** |
| 4 | `migration_004_v0114` | **qr_scans** |
| 5 | `migration_005_contamination_schedule` | `subcultures.contamination_flag` + `contamination_notes` |
| 6 | `migration_006_force_password_change` | `users.must_change_password`; sets it to 1 for the seeded `admin` |
| 7 | `migration_007_perf_indexes` | Six indexes across `specimens` and `subcultures` |
| 8 | `migration_008_audit_hash_chain` | `audit_log.chain_seq` / `prev_hash` / `entry_hash` — the hash chain begins here |
| 9 | `migration_009_audit_lineage` | `audit_log.lineage_id` + a back-fill from `entity_id`, and `idx_audit_lineage` |
| 10 | `migration_010_specimen_genealogy` | `specimens.generation`, `lineage_passage_offset`, `root_specimen_id` |
| 11 | `migration_011_media_draft` | `media_batches.is_draft` |
| 12 | `migration_012_specimen_contamination` | `specimens.contamination_flag` + `contamination_notes` |
| 13 | `migration_013_audit_checkpoints` | **audit_checkpoints**, including the `anchored_txid` hook used much later by WP-66 |
| 14 | `migration_014_checkpoint_auto_and_settings` | `audit_checkpoints.is_auto` + `auto_source`; **app_settings**; seeds `auto_checkpoint_enabled=1`, `auto_checkpoint_interval=100`, `auto_checkpoint_on_backup=1` |
| 15 | `migration_015_death_events_and_lab_profile` | `subcultures.event_type` (default `'passage'`); **app_config** with its single `CHECK (id = 1)` row carrying `lab_profile` |
| 16 † | `migration_016_vocabulary_tables` | **stages**, **propagation_methods**; rebuilds `specimens` as `specimens_v16` to **drop the stage and propagation CHECK constraints** — vocabulary becomes data |
| 17 † | `migration_017_remaining_vocabularies` | **hormone_types**, **compliance_record_types**, **compliance_agencies**, **inventory_categories**; rebuilds `media_hormones`, `compliance_records` and `inventory_items` to drop their CHECKs |
| 18 † | `migration_018_cell_culture_vocabulary` | Seeds the `cell_culture` vocabulary across all six tables |
| 19 | `migration_019_strain_model` | **strains**, **strain_parents**, **hybridization_events**; `specimens.strain_id` + `strain_chain_seq` |
| 20 | `migration_020_expanded_taxonomy` | **taxa**; `species.taxon_path` + `ncbi_taxon_id`; runs `backfill_genus_taxa` **once** |
| 21 | `migration_021_ncbi_sync_log` | **ncbi_sync_log** |
| 22 | `migration_022_hybrid_generation_labels` | `hybridization_events.generation_label` + `backcross_depth`; `strains.is_cross_species` |
| 23 † | `migration_023_cell_culture_vocabulary` | More `cell_culture` vocabulary rows |
| 24 | `migration_024_pdl_fields` | `specimens.cumulative_pdl`; `subcultures.seed_cell_count`, `harvest_cell_count`, `split_ratio`, `pdl_gained`, `doubling_time_hours` |
| 25 | `migration_025_frozen_vials` | **frozen_vials** |
| 26 | `migration_026_biosafety_level` | `specimens.biosafety_level`, CHECK BSL-1 / 2 / 2+ / 3 |
| 27 † | `migration_027_mycology_vocabulary` | Seeds the `mycology` vocabulary across all six tables |
| 28 | `migration_028_colonization_contaminant` | `subcultures.colonization_pct` (CHECK 0–100) and `contaminant_type` |
| 29 | `migration_029_genetic_lineage_markers` | `specimens.origin_type` (CHECK `multi_spore` / `isolated_dikaryon` / `tissue_clone`) and `is_best_performer` |
| 30 | `migration_030_fruiting_records` | **fruiting_records** |
| 31 | `migration_031_taxon_hash_chain` | **No DDL.** Calls `backfill_taxa_genesis` to write genesis audit entries for existing taxa, kingdom → genus. Labelled EXPERIMENTAL (WP-45) |
| 32 | `migration_032_domain_column` | `app_config.domain`, back-filled from `lab_profile` → Plantae / Animalia / Fungi |
| 33 | `migration_033_breeding_programs` | **breeding_programs**, **breeding_records** |
| 34 | `migration_034_provisional_taxa` | `taxa.status`, `taxa.provisional_notes`; **taxon_mappings** |
| 35 | `migration_035_multiuser_foundation` | Seeds `app_settings.backend_type = 'sqlite'`; **sync_peers**, **sync_conflicts** |
| 36 | `migration_036_field_permissions` | **field_permissions** plus 12 permissive seed rows (4 roles × 3 maskable fields) |
| 37 | `migration_037_environmental_readings` | **environmental_readings** |
| 38 | `migration_038_notifications` | **notification_preferences**, **smtp_config**; seeds `notification_check_interval_minutes = 15` |
| 39 | `migration_039_perf_indexes_v2` | Five composite indexes; seeds `pedigree_max_depth = 10` |
| 40 | `migration_040_locations` | **locations**; `specimens.location_id` |
| 41 | `migration_041_ai_suggestions` | **ai_suggestions** |
| 42 | `migration_042_backup_targets` | **backup_targets**, **cloud_sync_segments** |
| 43 | `migration_043_reanchor_events` | **reanchor_events** |
| 44 | `migration_044_signing_keys` | **signing_keys** — the lab's single Ed25519 export identity |
| 45 | `migration_045_installed_plugins` | **installed_plugins** |
| 46 | `migration_046_checkpoint_anchors` | **checkpoint_anchors** |
| 47 | `migration_047_signed_events` | **user_signing_keys**, **signed_events** |
| 48 | `migration_048_regulatory_submissions` | **regulatory_submissions** |
| 49 | `migration_049_specimen_passports` | **specimen_passports** |
| 50 | `migration_050_taxonomy_registries` | **taxonomy_registries**, **registry_record_dispositions** |
| 51 | `migration_051_breeding_bundles` | `breeding_records.origin_lab`; **breeding_bundles**, **breeding_bundle_dispositions** |
| 52 | `migration_052_compliance_flag_waivers` | **compliance_flag_waivers** |
| 53 | `migration_053_specimen_lab_profile` | **`specimens.lab_profile`** + a two-pass back-fill + `idx_specimens_lab_profile`. The pass-1/pass-2 split is the interesting part — see below |
| 54 | `migration_054_specimen_search_index` | **specimens_fts** (FTS5, trigram, external content) + three triggers + a back-fill |
| 55 | `migration_055_purge_plaintext_sessions` | `DELETE FROM sessions` — token storage moved to a SHA-256 digest, so every existing row was unusable |
| 56 | `migration_056_dashboard_aggregate_indexes` | Three lab-scoped dashboard indexes, one of them **partial** (quarantine) |
| 57 | `migration_057_media_hormones_batch_index` | `idx_media_hormones_batch` — the missing FK index that made the media list quadratic |
| 58 | `migration_058_relink_orphan_species` | **No DDL.** Calls `queries::rebuild_species_taxonomy` to repair species left with `taxon_path = NULL` |
| 59 | `migration_059_location_layout` | `ALTER TABLE locations ADD COLUMN layout_json TEXT;` — one nullable column, nothing else |

### Three worth reading in full

> [!example] 053 — the two-pass back-fill
> Before it, lab membership was inferred by joining `specimens.stage` against `stages.profile`.
> That is wrong in both directions: `archived`, `custom`, `suspension`, `contaminated`, `discarded`
> and `other` are each defined for two or three profiles, and `list_specimens` / `search_specimens`
> never applied the join at all. Pass 1 assigns a profile only where the stage code is defined for
> **exactly one** profile — a reliable answer. Pass 2 falls back to the currently-active profile for
> the ambiguous remainder, because no information is left to recover.
> Its comment carries a standing instruction: **any future migration that rebuilds `specimens` must
> carry `lab_profile` across**, or every culture silently collapses into the default profile.

> [!example] 054 — why trigram, and why not `content=''`
> The default `unicode61` tokenizer matches token *prefixes*, so `0042` would stop finding
> `FIX-00000042` — a silent behaviour regression for anyone typing a partial accession. Trigram
> indexes every 3-character sequence and gives results identical to `LIKE '%q%'` for needles of 3+
> characters; shorter ones are left on the old LIKE path by `search_specimens`. `content='specimens'`
> makes it an *external content* index, so the text is not duplicated.

> [!example] 056 — a measurement that argued against `ANALYZE`
> Three covering `(lab_profile, is_archived, <grouped column>)` indexes, with the quarantine one
> partial (10.8 ms → 7 µs, measured). The comment records that adding `ANALYZE` was tried and makes
> the total *worse*: with `sqlite_stat1` present the planner re-plans the per-species aggregate onto
> a different index, 6.8 ms → 21.3 ms. Nothing in the app runs `ANALYZE` today.

> [!note] A stray doc comment
> The doc comment for `migration_057_media_hormones_batch_index` sits above
> `migration_058_relink_orphan_species` (`migrations.rs:314-323`) with no blank line between them, so
> rustdoc attaches both paragraphs to 058 and 057 documents itself as nothing. Cosmetic, but it
> misleads anyone reading top-down.

---

## Adding migration 60 — the exact recipe

### 1. Add the gate at the end of `run_all`

Immediately before the closing `Ok(())` at `migrations.rs:307-311`, copying this shape exactly:

```rust
    if current < 60 {
        apply(conn, 60, migration_060_short_slug)?;
    }

    Ok(())
}
```

### 2. Define the body immediately after `run_all`

New migrations go at the top of the body section (the file descends 059, 058, 057 …), with a doc
comment that explains **why** rather than what — every recent migration's comment states the bug it
closes, the measurement that justified it, or what it is safe to re-run against.

```rust
/// One paragraph on WHY: what broke, what this fixes, and what it is safe to
/// re-run against.
fn migration_060_short_slug(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS my_table (
             id         TEXT PRIMARY KEY,
             thing_id   TEXT NOT NULL REFERENCES specimens(id) ON DELETE CASCADE,
             created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );

         CREATE INDEX IF NOT EXISTS idx_my_table_thing ON my_table(thing_id);",
    )?;
    Ok(())
}
```

### 3. Rules the file enforces on itself

| Rule | Why |
|---|---|
| Use `apply`, never `apply_untransacted` | The seven exceptions are legacy; a new one re-opens the unbootable-app hazard |
| No `PRAGMA foreign_keys`, no inner `BEGIN;`/`COMMIT;` | Either one forces `apply_untransacted` |
| `CREATE TABLE IF NOT EXISTS` · `CREATE INDEX IF NOT EXISTS` | Cheap insurance if a gate is ever mis-numbered |
| `ALTER TABLE … ADD COLUMN` is **not** idempotent | Rely on `apply`'s transaction (preferred), or use the defensive `let _ = conn.execute(...)` pattern already used by `migration_034_provisional_taxa` and `migration_040_locations` |
| Rebuilding `specimens`? Carry `lab_profile` across | Migration 053's standing instruction |
| Seeding a setting | `INSERT OR IGNORE INTO app_settings (key, value) VALUES ('k','v');` inside the body — see 038 and 039 |
| Adding a vocabulary term | `INSERT OR IGNORE INTO stages (profile, code, label, sort_order, is_terminal) VALUES …` — a data insert, never a schema change ([[Lab Profiles]]) |
| Never edit a shipped migration | Fix forward |

### 4. Tests — one per behaviour

Tests live in the `#[cfg(test)] mod tests` block at `migrations.rs:3222+` (**131 `#[test]`
functions**). The fixture is universal across the whole crate:

```rust
fn migrated_db() -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory DB");
    run_all(&conn).expect("all migrations must succeed on a fresh in-memory DB");
    conn
}
```

There is also a `column_exists(conn, table, column)` helper (`migrations.rs:5190`) built on
`PRAGMA table_info`.

The convention is **one test per behaviour, named as a sentence**, not one test per migration. The
six tests for migration 058 are the model to copy:

```
migration_058_links_species_that_have_no_taxon_path
migration_058_puts_two_species_of_a_genus_under_one_taxon
migration_058_is_case_insensitive_on_genus
migration_058_leaves_an_already_classified_species_alone
migration_058_is_idempotent
migration_058_skips_a_species_with_a_blank_genus
```

Harness tests already covering the machinery — do not duplicate them:

| Test | What it pins |
|---|---|
| `all_migrations_run_on_empty_db` | `run_all` succeeds on a fresh DB |
| `migrations_are_idempotent` | A second `run_all` is not an error |
| `a_retried_run_all_is_a_no_op` | `MAX(version)` is unchanged by a re-run |
| `apply_rolls_back_the_whole_migration_when_the_body_fails` | A failed body leaves neither partial DDL nor a version stamp |
| `apply_commits_body_and_version_together_on_success` | Both land, or neither |

> [!caution] Migration 059 shipped with no tests
> `migration_059_location_layout` has no test of its own — it is covered only by
> `all_migrations_run_on_empty_db`. It is a single nullable `ALTER TABLE … ADD COLUMN`, which is
> about as low-risk as a migration gets, but the file's own convention was not followed.

### 5. Verify

```bash
cd src-tauri
cargo test --lib --no-default-features db::migrations   # the migration suite
cargo test --lib --no-default-features                  # everything that compiles headless
```

`migrations.rs` is *not* behind the `tauri-commands` feature, so the headless build exercises it in
full. See [[Build and Test Commands]] for the rest of the gate, and for the version-bump lockstep
list — `SKILLS.md` §2 carries a "N migrations today; next is NNN" line that drifts.

## `seed_defaults`

Runs immediately after `run_all`, from the same `.setup()` block. **Guard: it returns early if
`SELECT COUNT(*) FROM users > 0`**, so it only ever populates a genuinely empty database.

| Seeds | Detail |
|---|---|
| One user | `admin` / `admin`, bcrypt `DEFAULT_COST`, display name `Administrator`, email `admin@stelolab.local`, role `admin`, `must_change_password = 1` |
| Six species | `ASP-OFF`, `NAN-DOM`, `CIT-SIN`, `CIT-LIM`, `CIT-PAR`, `CIT-RET` |
| A tag tree | Two levels over six categories — Health, Disease, Growth, Issue, Contamination Type, Action Needed — with hex colours |

Two public back-fill helpers also live in this file and are safe to re-run:
`backfill_taxa_genesis(conn)` and `backfill_genus_taxa(conn)`.

**Back to [[Home]]**

#steloptc #reference #database #migrations
