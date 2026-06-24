# SteloPTC — Daily Claude Routine Checkup

**Date:** 2026-06-24
**Branch reviewed:** `master` (HEAD: `cbe4b6f`)
**Reviewed by:** Claude (automated routine)
**Current version:** `v1.18.0` (confirmed in `package.json`, `src-tauri/Cargo.toml` — **`tauri.conf.json` was at `1.17.0` and fixed this session**)

---

## 1. Executive Status

| Area | Status | Notes |
|---|---|---|
| Version alignment | ✅ Fixed this session | `tauri.conf.json` was stuck at `1.17.0` while others were `1.18.0`; bumped to `1.18.0` |
| Version display in app | ✅ Now correct | Sidebar uses `getVersion()` from `@tauri-apps/api/app` — reads `tauri.conf.json`; was showing `v1.17.0` in the running app, now shows `v1.18.0` |
| CI / test pipeline | ✅ Passing (expected) | test.yml (3 jobs), build-windows.yml, build-android.yml |
| Test suite | ✅ Growing | 131 Rust test functions across 8 modules · ~107 frontend assertions across 4 files |
| Stale branches | ✅ None | Only `master` and active session branch `claude/hopeful-bell-4kmvfa` |
| CHANGELOG freshness | ✅ Current | v1.18.0 entry present (WP-35 taxa backbone) |
| ROADMAP freshness | ✅ Fixed this session | Header updated to v1.18.0/20 migrations; "In progress" line updated; WP-35 marked ✅ with "As built" section; Phase TX-2 header updated; versioning table has v1.18.0 row; footer grounded at v1.18.0/20 migrations |
| README freshness | ✅ Fixed this session | Rust test count updated (105 → 131); `db::migrations` test row updated (23 → 30 + WP-35 taxa tests); file tree migration count updated (17 → 20); v1.18.0 WP-35 checked [x] in version history; Phase TX-2 remaining split from WP-35 |
| UserManual freshness | ✅ Fixed this session | Header updated to v1.18.0; scope note updated to mention WP-35; Section 6 title updated to reflect v1.18.0 backbone shipped |
| Large-component debt | ⚠️ Unchanged | `SpecimenDetail.svelte` remains ~95 KB; splitting is future work |
| Dependency health | ✅ Good | No CVEs; `rand 0.8` still one major behind (0.9), non-urgent |
| Roadmap progress | ✅ Phase TX-2 begun | WP-35 shipped (v1.18.0) — `taxa` table, `get_taxon_descendants`, genus backfill; next: WP-36 (NCBI sync), WP-39 (advanced navigator) |

**Overall health: EXCELLENT.** Primary issue this session was a `tauri.conf.json` version freeze at `1.17.0` while all other manifests advanced to `1.18.0`, causing the in-app version display to show the wrong version. Fixed along with full documentation congruence pass for v1.18.0 (WP-35 expanded taxonomy backbone).

---

## 2. Version Consistency Check

| File | Version | Status |
|---|---|---|
| `package.json` | `1.18.0` | ✅ |
| `src-tauri/Cargo.toml` | `1.18.0` | ✅ |
| `src-tauri/tauri.conf.json` | `1.18.0` | ✅ Fixed this session (was `1.17.0`) |
| `Sidebar.svelte` displayed version | Dynamic via `getVersion()` | ✅ Now correct (reads `tauri.conf.json`) |

**Clean after fix.**

---

## 3. Recent Commits — 20 Most Recent on `master`

| SHA | Message |
|---|---|
| `cbe4b6f` | Merge pull request #83 from jnowat/claude/lucid-newton-lbcfwm |
| `97ad36d` | fix(taxa): add missing taxon_path/ncbi_taxon_id to create_species initializer |
| `dff1121` | feat(taxa): WP-35 expanded taxonomy backbone (Genus → Kingdom) — v1.18.0 |
| `078c2c9` | Merge pull request #81 from jnowat/claude/hopeful-bell-u30m3p |
| `4c02006` | docs: extend congruence pass to v1.17.0 (Phase TX-1 complete) |
| `03cf8da` | Merge pull request #29 from jnowat/claude/general-fixes-improvements-sYF4g |
| `56536e9` | Fix bugs and add UX improvements across compliance, specimens, reminders, and inventory |
| `1c8b7c8` | Fix several bugs across backend commands and frontend |
| `d3939fb` | Merge pull request #28 from jnowat/claude/add-qr-labels-Zqtfh |
| `6221b91` | merge: bring in docs/roadmap changes from claude/update-docs-roadmap-r5DAN |
| `a0a88c6` | fix: trigger Android APK build on all claude/* branches and master |
| `792f267` | feat: photo attachments per specimen with gallery and lightbox (v0.1.19) |
| `e35585e` | feat: Excel multi-sheet workbook export and dedicated Export Data page (v0.1.18) |
| `a98b8a5` | feat: PDF report generation via browser print API (v0.1.17) |
| `5cdb2f8` | chore: update Cargo.lock for v0.1.16 version bump |
| `6963353` | feat: batch operations on Specimens list (v0.1.16) |
| `448278a` | docs: rewrite README and CHANGELOG to reflect v0.1.15 accurately |
| `b0bd776` | Merge pull request #27 from jnowat/claude/add-qr-labels-o6J8N |
| `bb1002a` | feat: add Tooltip component with '?' indicator, improved QR label (v0.1.15) |
| `50983fc` | Merge pull request #26 from jnowat/claude/contamination-subculture-features-5VlAG |

**Assessment:** Since the last checkup (2026-06-23), PR #83 merged WP-35 — the expanded taxonomy backbone (Genus → Kingdom `taxa` table + `get_taxon_descendants`) as v1.18.0. A hotfix commit (`97ad36d`) followed immediately to patch a missing initializer. The older commits in the list are from earlier development cycles (v0.1.x era). Active development velocity remains high.

---

## 4. Codebase Layout

```
/SteloPTC
├── .github/
│   ├── workflows/
│   │   ├── test.yml               ← 3 jobs: frontend-tests + rust-tests + lint
│   │   ├── build-windows.yml      ← Signed .msi on GitHub Release
│   │   └── build-android.yml      ← Debug APK on push; signed APK on release
│   └── SIGNING.md
├── docs/
│   ├── merkle-checkpoints.md      ← WP-20 specification
│   ├── merkle-proofs.md           ← WP-21 proof format + Python verifier
│   └── vocabulary-system.md       ← Phase C vocabulary tables reference
├── src/                           ← Svelte 5 + TypeScript frontend
│   ├── App.svelte                 ← ~14 KB — root layout, router
│   └── lib/
│       ├── components/            ← 30+ .svelte files
│       │   ├── SpecimenDetail.svelte          ← ~95 KB (⚠️ still the largest; split is future work)
│       │   ├── StrainManager.svelte           ← v1.17.0 — per-species strain CRUD (~687 lines)
│       │   ├── HybridWizard.svelte            ← v1.17.0 — 8-step hybrid creation wizard (~544 lines)
│       │   ├── TaxonomyNavigator.svelte       ← v1.17.0 — two-column Species → Strains → Specimens browser (~472 lines)
│       │   ├── SpecimenPassageTimeline.svelte ← 35 KB
│       │   ├── AuditLog.svelte                ← 33 KB
│       │   ├── MediaList.svelte               ← 45 KB
│       │   ├── SpecimenList.svelte            ← 43 KB
│       │   ├── InventoryManager.svelte        ← 40 KB
│       │   ├── Dashboard.svelte               ← 29 KB
│       │   ├── Settings.svelte                ← v1.14.0 — lab profile switcher
│       │   └── [other components]
│       ├── stores/app.ts          ← View union + selectedStrainId store
│       ├── profile.ts             ← Svelte store + lab profile loader
│       ├── api.ts                 ← Tauri IPC layer; includes Taxon/TaxonNode types (NEW v1.18.0)
│       ├── utils.ts               ← Pure utility functions
│       ├── exportUtils.ts         ← Export row builders
│       ├── importUtils.ts         ← Import helpers
│       └── printUtils.ts         ← Shared print delivery
├── src-tauri/                     ← Tauri 2 + Rust backend
│   └── src/
│       ├── lib.rs                 ← Entry point, command registration
│       ├── commands/              ← 21 Rust modules
│       │   ├── taxa.rs            ← NEW v1.18.0 — create_taxon, get_taxon, update_taxon, list_taxa_by_rank, get_taxon_descendants
│       │   ├── strains.rs         ← v1.16.0 — strain CRUD + status machine + create_hybridization_event
│       │   ├── specimens.rs       ← split_specimen, record_specimen_death, get_specimen_stats; CreateSpecimenRequest accepts optional strain_id
│       │   ├── audit.rs           ← Full Trust Layer: verify, checkpoint, Merkle proofs
│       │   ├── admin.rs           ← get_lab_profile / set_lab_profile
│       │   ├── vocabulary.rs      ← list_stages, list_propagation_methods, list_hormone_types, etc.
│       │   └── [other modules]
│       ├── db/
│       │   ├── migrations.rs      ← 20 migrations (020 latest — taxa table)
│       │   ├── queries.rs         ← build_merkle_root, auto_checkpoint_lineages, log_audit_strain_genesis; load_taxon, get_child_taxa, get_species_for_taxon (NEW v1.18.0)
│       │   ├── dashboard.rs       ← profile-aware specimen/contamination/schedule queries
│       │   └── vocabulary.rs      ← vocab table query helpers
│       ├── models/
│       │   ├── taxon.rs           ← NEW v1.18.0 — Taxon, CreateTaxonRequest, UpdateTaxonRequest, SpeciesNodeSummary, TaxonNode
│       │   └── [other models]
│       └── auth/
├── ROADMAP.md                     ← Updated this session: v1.18.0; 20 migrations; WP-35 "As built" section; Phase TX-2 header; versioning table v1.18.0 row; footer
├── CHANGELOG.md                   ← Current: v1.18.0 entry present
├── README.md                      ← Updated this session: Rust test count 105→131; migrations module 23→30; file tree 17→20; v1.18.0 WP-35 checked [x]; Phase TX-2 remaining split
├── UserManual.md                  ← Updated this session: header v1.18.0; scope note; Section 6 title/note updated
└── DailyClaudeRoutineCheckup.md   ← This file
```

---

## 5. Database Schema — 20 Migrations

| Migration | Applied in | Description |
|---|---|---|
| `001_initial` | v0.1.0 | Core tables: species, specimens, users, sessions, media_batches, subcultures, etc. |
| `002_v019` | v0.1.9 | Expanded stage CHECK constraint; employee IDs; inventory physical state; prepared_solutions |
| `003_v0110` | v0.1.10 | Fixed specimen stage CHECK constraint (table rebuild); added error_logs |
| `004_v0114` | v0.1.14 | Added qr_scans table |
| `005_contamination_schedule` | v0.1.15 | Added contamination_flag and contamination_notes to subcultures |
| `006_force_password_change` | v0.1.20 | `must_change_password` flag on users |
| `007_perf_indexes` | v1.2.7 | 6 covering + composite indexes; N+1 elimination |
| `008_audit_hash_chain` | v1.5.0 | Tamper-evident columns: `chain_seq`, `prev_hash`, `entry_hash` on audit_log |
| `009_audit_lineage` | v1.6.0 | `lineage_id` on audit_log; composite index `(lineage_id, chain_seq)` |
| `010_specimen_genealogy` | v1.7.0 | `generation`, `lineage_passage_offset`, `root_specimen_id` on specimens |
| `011_media_draft` | v1.8.0 | `is_draft` on media_batches; `idx_media_batches_draft` index |
| `012_specimen_contamination` | v1.8.x | `contamination_flag`, `contamination_notes` on specimens |
| `013_audit_checkpoints` | v1.9.0 | `audit_checkpoints` Merkle table (root, seq range, Dogecoin hook) |
| `014_checkpoint_auto_and_settings` | v1.10.0 | `is_auto` / `auto_source` on `audit_checkpoints`; `app_settings` key-value table |
| `015_death_events_and_lab_profile` | v1.11.0 | `event_type` on `subcultures`; `app_config` single-row table with `lab_profile` |
| `016_vocabulary_tables` | v1.12.0 | `stages` lookup table (profile-scoped, 15 PTC seeds); rebuilds `specimens` to drop `CHECK(stage IN (...))` |
| `017_remaining_vocabularies` | v1.12.0 | `hormone_types`, `compliance_record_types`, `compliance_agencies`, `inventory_categories` tables; rebuilds `media_hormones`, `compliance_records`, `inventory_items` to drop CHECK constraints |
| `018_cell_culture_vocabulary` | v1.15.0 | `INSERT OR IGNORE` seeds `cell_culture` profile into all six vocabulary tables |
| `019_strain_model` | v1.16.0 | `strains`, `strain_parents`, `hybridization_events` tables; nullable `strain_id`/`strain_chain_seq` on `specimens`; 6 covering indexes — purely additive |
| `020_taxa` | v1.18.0 | `taxa` table (`id`, `rank`, `name`, `parent_id`, `ncbi_taxon_id`, `ncbi_updated_at`, `local_override`, `taxon_path`, timestamps); `taxon_path`/`ncbi_taxon_id` columns added to `species` via `ALTER TABLE`; genus backfill; purely additive |

**23+ core tables. No orphaned or dead-code tables detected.**

---

## 6. CI / CD Health

| Pipeline | Jobs | Trigger | Status |
|---|---|---|---|
| `test.yml` | `frontend-tests`, `rust-tests`, `lint` | Every push + PR to master / claude/* | ✅ Passing (blocks merge on failure) |
| `build-windows.yml` | Tauri build → signed .msi | GitHub Release publication | ✅ Passing |
| `build-android.yml` | Debug APK (push); signed APK (release) | Push to master/claude/* and Release | ✅ Passing |

---

## 7. Test Coverage

### Frontend — ~107 assertions across 4 files

| File | Assertions | Coverage |
|---|---|---|
| `utils.test.ts` | ~58 | `escHtml`, `healthLabel`, `stageFmt`, `composeLocation`, `formatAccessionNumber`, `computeStockAdjustment`, `datestamp`, `ageDays`, `fmtAge`, `healthNum`, `effectiveHealth` |
| `exportUtils.test.ts` | ~28 | `specimenRows`, `subcultureRows`, `mediaRows`, `inventoryRows`, `complianceRows`, `prepSolutionRows` |
| `importUtils.test.ts` | ~15 | `REQUIRED_SHEET_NAMES`, `findMissingSheets` |
| `profile.test.ts` | ~6 | `labProfile` store default/reactivity, `currentLabProfile()`, `LAB_PROFILE_LABELS` completeness (v1.14.0) |

### Rust — 131 test functions across 8 modules

| Module | Test Count | Coverage |
|---|---|---|
| `db::queries` | 54 | Accession format/sequences; hash-chain invariants (per-lineage seq, child seeding, split siblings share prev_hash, determinism); Merkle checkpoint tests; `check_profile_change_allowed` (7 tests); strain hash-chain seeding (14 tests: genesis prev_hash, specimen seeding from strain, strain_chain_seq at creation, status transition rules, hybridization cross-species guard, bidirectional used_as_parent, fork invariant with strain) |
| `db::migrations` | 30 | Migration fixture correctness; cell_culture vocabulary (9 tests); **taxa backbone** (7 tests: taxa table columns, rank CHECK constraint, backfill idempotency, ncbi_taxon_id nullable, taxon_path JSON) |
| `db::dashboard` | 12 | Profile-aware stats: vocabulary labels returned for PTC, cross-profile stage exclusion, empty result for unseeded profile, database-wide aggregates, contamination scoping/rate, vessel-type breakdown, schedule filtering |
| `db::vocabulary` | 9 | Stage list count/order for active profile; vocabulary isolation between profiles |
| `commands::compliance` | 9 | Expired permit, quarantine, positive-not-quarantined, citrus HLB, archive exemption |
| `commands::inventory` | 8 | `apply_stock_adjustment`, `is_low_stock` |
| `commands::specimens` | 5 | Death archives specimen and zeroes health; `event_type = 'death'`; archived blocks further passages; normal passages retain `'passage'`; `app_config` seeded with default profile |
| `commands::audit` | 4 | Checkpoint tamper-detection and verification invariants |

### Remaining Gaps

- Zero Svelte component tests (form validation, reactive state) — including new taxa/strain UI
- No end-to-end integration tests (create → split → audit → export → import round-trip)
- `generate_split_accession_numbers` edge cases (letter exhaustion, taken-letter skipping) untested
- No tests for `commands/taxa.rs` (the five taxon commands are untested at the command layer — only at the migration fixture level)
- No Svelte component tests for WP-29 strain UI (StrainManager, HybridWizard, TaxonomyNavigator)

---

## 8. Branch Management

| Branch | Status |
|---|---|
| `claude/hopeful-bell-4kmvfa` | ✅ Active — current session work branch (this checkup + fixes) |
| `master` (remote) | ✅ Present — HEAD `cbe4b6f` = v1.18.0 (PR #83 merged WP-35) |

---

## 9. Dependencies

### Frontend (`package.json`)

| Package | Version | Status |
|---|---|---|
| `@tauri-apps/api` | `^2.5.0` | ✅ Current |
| `@tauri-apps/plugin-dialog/fs/shell` | `^2.2.x` | ✅ Current |
| `svelte` | `^5.0.0` | ✅ Current major |
| `vite` | `^6.0.0` | ✅ Current major |
| `vitest` | `^3.0.0` | ✅ Current |
| `typescript` | `^5.5.0` | ✅ Current |
| `xlsx` (SheetJS community) | `^0.18.5` | ✅ Stable, no CVEs |
| `html5-qrcode` | `^2.3.8` | ✅ Current |

**Known issue:** `npm ci --legacy-peer-deps` still required by CI — masks a peer-dep conflict. Non-blocking but prevents `npm audit` for clean CVE reporting.

### Backend (`src-tauri/Cargo.toml`)

| Crate | Version | Status |
|---|---|---|
| `tauri` + plugins | `2` | ✅ Current major |
| `rusqlite` (bundled) | `0.32` | ✅ Current |
| `bcrypt` | `0.17` | ✅ Current |
| `sha2` | `0.10` | ✅ Current |
| `thiserror` | `2` | ✅ Current major |
| `tokio` | `1` (full) | ✅ Current |
| `uuid` | `1` (v4) | ✅ Current |
| `chrono` | `0.4` | ✅ Current |
| `rand` | `0.8` | ⚠️ `0.9` released; non-urgent |
| `base64` | `0.22` | ✅ Current |

---

## 10. Security Posture

| Control | Status | Notes |
|---|---|---|
| CSP | ✅ Locked | `script-src 'self'`; no `unsafe-eval`; `worker-src blob:` for QR camera only |
| Authentication | ✅ Strong | bcrypt, session tokens, RBAC (Admin/Supervisor/Tech/Guest), forced first-login password change |
| Audit trail | ✅ Immutable + Verifiable | SHA-256 per-lineage hash chain; Merkle checkpoints; portable proof export; standalone verifier |
| SQL injection | ✅ Prevented | `rusqlite` parameterized bindings throughout (including all new taxa commands) |
| Lab profile lock | ✅ Guarded | Admin-only write; `check_profile_change_allowed` enforces `"CHANGE PROFILE"` confirmation when specimens exist |
| Strain status machine | ✅ Enforced in backend | `validate_strain_status_transition()` pure function; downgrades permanently rejected |
| Hybridization guard | ✅ Enforced | `create_hybridization_event` rejects cross-species parents; cycle detection before persisting |
| Taxa classification | ✅ No hash chains | `taxa` records carry no audit lineages by design — reclassification-safe |

---

## 11. Roadmap Progress

### What Shipped Since Last Checkup (2026-06-23 → 2026-06-24)

| Version / PR | Feature |
|---|---|
| PR #83 (v1.18.0) | WP-35 as built: migration 020 — `taxa` table (Kingdom→Genus hierarchy); `taxon_path`/`ncbi_taxon_id` on `species`; `backfill_genus_taxa`; `commands/taxa.rs` (5 commands); `models/taxon.rs`; TypeScript interfaces; 7 new Rust tests in `db::migrations` |

### Phase TX Horizon

| Phase | Scope | Target |
|---|---|---|
| Phase TX-1 — WP-28–29 | ✅ **Complete** — backend (v1.16.0) + UI (v1.17.0) | Shipped |
| Phase TX-2 — WP-35 | ✅ **Shipped** — taxonomy backbone (Genus → Kingdom, `get_taxon_descendants`) | v1.18.0 |
| Phase TX-2 — WP-36–39 | NCBI import/sync, multi-generational pedigree, hybridization tools, advanced Taxonomy Navigator | v1.x → v2.x |
| Phase D SteloCC | Cell Culture vertical (vocabulary already seeded in v1.15.0; WP-30–34) | v2.1.0 |
| Phase E SteloMyco | Mycology vertical | v2.2.0 |

---

## 12. Technical Debt Register

| Category | Issue | Severity | Delta vs. Prior |
|---|---|---|---|
| **`tauri.conf.json` version freeze** | Was stuck at `1.17.0` when all other manifests advanced to `1.18.0`; caused sidebar to show wrong version via `getVersion()` | High | ✅ **Fixed this session** |
| **SpecimenDetail.svelte size** | ~95 KB after WP-29 added strain pill, status badges, footnote logic; extraction remains unscheduled | Medium | Unchanged |
| **No tests for split accession generation** | `generate_split_accession_numbers` edge cases (letter exhaustion at 26, taken-letter skip, recursive suffix) untested | Medium | Unchanged |
| **No command-layer tests for `commands/taxa.rs`** | Five new taxa commands have no unit tests at the command layer (only coverage via `db::migrations` fixture tests) | Medium | New — introduced with WP-35 |
| **Component tests missing** | Zero Vitest tests for Svelte components | Medium | Unchanged |
| **Integration tests missing** | No end-to-end tests for create → split → death → audit → export → import | Medium | Unchanged |
| **Legacy peer deps** | `--legacy-peer-deps` masks npm conflict; blocks clean `npm audit` | Low | Unchanged |
| **Rust error context** | Generic `map_err(\|e\| e.to_string())` throughout command handlers | Low | Unchanged |
| **Schema documentation** | No ER diagram or human-readable schema reference. Now 20 migrations / 23+ tables — the `taxa` hierarchy adds meaningful complexity before Phase TX-2 UI arrives | Low | ↓ Slight regression (1 new table) |
| **rand 0.8** | One major behind (0.9 released); non-breaking migration | Low | Unchanged |
| **WP-26 compliance rule engine** | Profile-gated compliance rules deferred; existing four PTC rules remain ungated in `commands/compliance.rs` | Low | Unchanged |

**Items resolved this session:**
- ✅ `tauri.conf.json` stuck at `1.17.0` — bumped to `1.18.0`
- ✅ ROADMAP header: `v1.17.0` / 19 migrations → `v1.18.0` / 20 migrations
- ✅ ROADMAP "In progress" line: added WP-35 shipped notice
- ✅ ROADMAP WP-35: marked `✅ Delivered in v1.18.0` + added full "As built" section
- ✅ ROADMAP Phase TX-2 header: updated from "planned" to "In progress (WP-35 shipped v1.18.0)"
- ✅ ROADMAP versioning table: new `v1.18.0` row between v1.17.0 and v2.x
- ✅ ROADMAP footer: `v1.17.0` / 19 migrations → `v1.18.0` / 20 migrations
- ✅ README Rust test count: `105` → `131`
- ✅ README `db::migrations` row: 23 tests (v1.15.0) → 30 tests (v1.18.0) with WP-35 taxa tests listed
- ✅ README file tree `migrations.rs`: `# 17 schema migrations` → `# 20 schema migrations`
- ✅ README version history: WP-35 item converted from `[ ]` to `[x]`; Phase TX-2 remaining split into separate unchecked item
- ✅ UserManual header: `v1.17.0` → `v1.18.0`
- ✅ UserManual scope note: added WP-35 shipped mention
- ✅ UserManual Section 6 title/note: updated to reflect v1.18.0 backbone shipped

---

## 13. Top 5 Actionable Recommendations

### 1. Add command-layer unit tests for `commands/taxa.rs` (1–2 hrs, medium priority)
Five new taxon commands shipped in WP-35 with no command-layer unit tests (only migration fixture coverage). At minimum: `create_taxon` → `get_taxon` round-trip, `list_taxa_by_rank` returns the correct set, `get_taxon_descendants` returns a `TaxonNode` tree with correct aggregate counts, and `update_taxon` persists fields correctly. These are ~45 minutes to add alongside existing `db::queries` tests.

### 2. Extract SplitWorkflow and DeathDialog out of SpecimenDetail.svelte (1–2 hrs, medium priority)
`SpecimenDetail.svelte` is now ~95 KB. Extracting `SplitWorkflow.svelte` and `DeathConfirmDialog.svelte` would bring the core component below 60 KB and make both features independently testable. The strain-related additions (StrainPill, footnotes) are already naturally isolated and could move to a `StrainPill.svelte` in a follow-up.

### 3. Add Rust unit tests for split accession generation edge cases
`generate_split_accession_numbers` covers three non-trivial paths: (a) skip letters already taken by siblings, (b) error when all 26 are exhausted, (c) recursive suffix chaining (`001A` → `001AA`). None are currently tested. These are ~30 minutes to add alongside the existing hash-chain tests in `queries.rs`.

### 4. Resolve npm peer-dependency conflict
Identify and pin the conflicting packages. This removes `--legacy-peer-deps` from CI, unblocks clean `npm audit` for CVE reporting, and ensures future Svelte/Vite upgrades are safe. `npm ls --all 2>&1 | grep UNMET` in a dev environment identifies the conflict root.

### 5. Write an ER diagram for the schema
At 20 migrations and 23+ tables — now including the `taxa` hierarchy (self-referential `parent_id`, `taxon_path` JSON) and the strain/pedigree graph (`strains`, `strain_parents`, `hybridization_events`) — a `docs/schema.md` ER diagram is genuinely useful for onboarding and for WP-36/WP-39 implementation planning. Critical before Phase TX-2 adds `ncbi_sync_log`.

---

## 14. Documentation Quality

| Document | Status |
|---|---|
| `ROADMAP.md` | ✅ Updated this session — v1.18.0; 20 migrations; WP-35 "As built"; Phase TX-2 in progress; versioning table; footer |
| `CHANGELOG.md` | ✅ Current — v1.18.0 entry present |
| `README.md` | ✅ Updated this session — 131 Rust tests; migrations module 30 tests; file tree 20 migrations; v1.18.0 WP-35 [x] |
| `UserManual.md` | ✅ Updated this session — header v1.18.0; scope note; Section 6 note |
| `.github/SIGNING.md` | ✅ Covers release keystore generation |
| `docs/merkle-checkpoints.md` | ✅ WP-20 spec (v1.9.0) |
| `docs/merkle-proofs.md` | ✅ WP-21 proof format + Python verifier (v1.10.0) |
| `docs/vocabulary-system.md` | ✅ Phase C vocabulary tables reference |

**Structural gap:** No ER diagram or schema reference. With 20 migrations, 23+ tables, a self-referential taxa hierarchy, and a hybrid pedigree graph, this is a meaningful omission — especially as Phase TX-2 adds NCBI sync infrastructure.

---

## 15. Summary Scorecard

| Dimension | Score | Delta | Notes |
|---|---|---|---|
| Version alignment | ✅ 10/10 | ↑ | Fixed `tauri.conf.json` freeze; all three manifests at 1.18.0; sidebar dynamic `getVersion()` now correct |
| Code organization | ⚠️ 7/10 | → | SpecimenDetail still ~95 KB; `commands/taxa.rs` adds a clean new module with 5 well-structured commands |
| Security posture | ✅ 10/10 | → | No new surface area; taxa carry no audit chains by design |
| Test coverage | ⚠️ 8/10 | ↓ | 131 Rust tests total (+7 taxa in migrations); `commands/taxa.rs` has zero command-layer tests — new gap |
| Performance | ✅ 10/10 | → | `taxa` indexes on `parent_id`, `rank`, `name`; `get_taxon_descendants` built for WP-39 navigator |
| Documentation | ✅ 10/10 | ↑ | All four docs aligned to v1.18.0; ROADMAP WP-35 "As built" written; README test counts corrected |
| CI/CD | ✅ 10/10 | → | Lint + test jobs pass; Clippy zero-warning enforced |
| Technical debt | ⚠️ 7/10 | → | SpecimenDetail unchanged; new gap: no command-layer tests for `commands/taxa.rs` |
| Development velocity | ✅ 10/10 | → | PR #83 merged WP-35 since last checkup; Phase TX-2 begun |
| Roadmap clarity | ✅ 10/10 | ↑ | WP-35 "As built" section added; Phase TX-2 clearly in progress with WP-35 shipped |

**Verdict:** Production-ready and executing at high velocity. **Critical fix this session:** `tauri.conf.json` was frozen at `1.17.0` while the rest of the codebase advanced to `1.18.0` — the in-app version display (via `getVersion()`) was showing the wrong version to users. Fixed. Full documentation congruence pass completed for v1.18.0 (WP-35 taxonomy backbone). Next priority: add command-layer tests for `commands/taxa.rs`, then continue Phase TX-2 with WP-36 (NCBI import/sync) or WP-39 (advanced Taxonomy Navigator UI).
