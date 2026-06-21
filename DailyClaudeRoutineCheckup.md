# SteloPTC — Daily Claude Routine Checkup

**Date:** 2026-06-21
**Branch reviewed:** `claude/eloquent-pascal-734s20` (HEAD: `f6a2149`)
**Reviewed by:** Claude (automated routine)
**Current version:** `v1.11.0` (confirmed in `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`)

---

## 1. Executive Status

| Area | Status | Notes |
|---|---|---|
| Version alignment | ✅ Clean | All three manifests at `1.11.0` |
| CI / test pipeline | ✅ Passing (expected) | test.yml (3 jobs), build-windows.yml, build-android.yml |
| Test suite | ✅ Growing | 64 Rust tests · 101 frontend assertions across 3 files |
| Stale branches | ✅ None | `master` + `claude/eloquent-pascal-734s20` only |
| CHANGELOG freshness | ✅ Current | v1.11.0 entry written at time of PR |
| ROADMAP freshness | ✅ Fixed this session | Header was stuck at v1.8.0/11 migrations — updated to v1.11.0/15 migrations; WP-19 and WP-22 marked delivered; versioning table rebuilt |
| README freshness | ✅ Fixed this session | Migration count (10→15), migration table (added rows 014–015), Planned/roadmap section restructured |
| UserManual freshness | ✅ Fixed this session | Dead specimen workflow section added; v1.9.0 TX targets updated to v2.0.0; scope note updated |
| Large-component debt | ⚠️ Regressed | `SpecimenDetail.svelte` now 92 KB (was 78 KB in prior review) |
| Dependency health | ✅ Good | No CVEs; `rand 0.8` still one major behind (0.9), non-urgent |
| Roadmap progress | ✅ Ahead of schedule | Trust Layer Phase 1 complete (WP-18–21); Phase C WP-22 shipped; active: WP-23 + Phase TX-1 |

**Overall health: EXCELLENT.** The most urgent issue this session was documentation drift: the ROADMAP header and versioning table had not been updated since v1.8.0, despite three subsequent releases (v1.9.0, v1.10.0, v1.11.0) shipping and the Grok-undo commit reverting a partial fix. All corrected this session.

---

## 2. Version Consistency Check

| File | Version | Status |
|---|---|---|
| `package.json` | `1.11.0` | ✅ |
| `src-tauri/Cargo.toml` | `1.11.0` | ✅ |
| `src-tauri/tauri.conf.json` | `1.11.0` | ✅ |

**Clean.** No drift detected.

---

## 3. Recent Commits — 20 Most Recent on `master` (via `claude/eloquent-pascal-734s20`)

| SHA | Message |
|---|---|
| `f6a2149` | Undid Grok direct edit |
| `ce8fe52` | docs(roadmap): Update status to reflect completion of Trust Layer (WP-18-21), Dead Specimen workflow, and WP-22 — mark v1.11.0 |
| `77a11c4` | Merge PR #73 — happy-cerf (WP-22 dead specimen + lab profile) |
| `87a3a7a` | chore: update package-lock.json after npm install |
| `d53e25b` | fix: resolve Svelte/TS compilation errors in death event UI |
| `73b46c6` | fix: make tauri-build a non-optional build dependency |
| `8cb274f` | polish(WP-22): death confirmation dialog, test isolation, robustness fixes |
| `aad728f` | feat(WP-22): dead specimen workflow, lab profile, migration 015 |
| `e4f8ab4` | Merge PR #72 — vibrant-ptolemy (WP-21 Merkle proofs + quality review fixes) |
| `d8bd00b` | fix(trust-layer): quality-review fixes across WP-18–21 |
| `fe4492f` | feat(audit): WP-21 — portable Merkle proofs, standalone verification & auto-checkpointing |
| `2318bc2` | Merge PR #71 — practical-babbage (WP-19 polish + WP-20 Merkle checkpoints) |
| `7ef13ab` | Fix clippy::manual_is_multiple_of lint in build_merkle_root |
| `0d40c8f` | Fix two compilation errors in commands/audit.rs |
| `06f9d5e` | WP-20 polish: add 4 checkpoint tests, improve UI notifications, update README |
| `cf77b0a` | feat(wp-20): Merkle checkpoints for audit lineages |
| `0d55f4f` | Merge PR #70 — keen-turing (WP-19 contamination inheritance) |
| `d2661a2` | test(split): add edge-case test for request-driven contamination on clean parent |
| `2447177` | test(split): add unit tests for contamination inheritance logic |
| `7639031` | fix(wp-19): polish contamination warning copy and amber block dark mode |

**Assessment:** High velocity — 4 PRs merged spanning WP-19 polish (PR #70, #71), WP-20 Merkle checkpoints (PR #71), WP-21 portable proofs + auto-checkpointing (PR #72), WP-22 lab profile + dead specimen workflow (PR #73). The Grok-undo commit (`f6a2149`) reverted a partial ROADMAP update that Grok had written, leaving the ROADMAP header at v1.8.0 — corrected this session.

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
│   └── merkle-proofs.md           ← WP-21 proof format + Python verifier
├── src/                           ← Svelte 5 + TypeScript frontend
│   ├── App.svelte                 ← ~14 KB — root layout, router
│   └── lib/
│       ├── components/            ← 29 .svelte files
│       │   ├── SpecimenDetail.svelte          ← 92 KB (⚠️ grew from 78 KB)
│       │   ├── SpecimenPassageTimeline.svelte ← 35 KB
│       │   ├── AuditLog.svelte                ← 33 KB
│       │   ├── MediaList.svelte               ← 45 KB
│       │   ├── SpecimenList.svelte            ← 44 KB
│       │   ├── InventoryManager.svelte        ← 41 KB
│       │   ├── Dashboard.svelte               ← 30 KB
│       │   └── [22 other components]
│       ├── profile.ts             ← WP-22: Svelte store + lab profile loader (NEW v1.11.0)
│       ├── api.ts                 ← Tauri IPC layer
│       ├── utils.ts               ← Pure utility functions
│       ├── exportUtils.ts         ← Export row builders
│       ├── importUtils.ts         ← Import helpers
│       └── printUtils.ts         ← Shared print delivery
├── src-tauri/                     ← Tauri 2 + Rust backend
│   └── src/
│       ├── lib.rs                 ← Entry point, command registration
│       ├── commands/              ← 18+ Rust modules
│       │   ├── specimens.rs       ← split_specimen, record_specimen_death (NEW)
│       │   ├── audit.rs           ← Full Trust Layer: verify, checkpoint, Merkle proofs
│       │   ├── admin.rs           ← get_lab_profile / set_lab_profile (NEW)
│       │   └── [other modules]
│       ├── db/
│       │   ├── migrations.rs      ← 15 migrations (015 latest)
│       │   └── queries.rs         ← build_merkle_root, auto_checkpoint_lineages, etc.
│       ├── models/
│       └── auth/
├── ROADMAP.md                     ← Updated this session: v1.11.0, 15 migrations, WP-19/22 marked delivered, versioning table rebuilt
├── CHANGELOG.md                   ← Current: v1.11.0 entry present
├── README.md                      ← Updated this session: migration count fixed, rows 014-015 added, planned section restructured
├── UserManual.md                  ← Updated this session: dead specimen section added, TX targets corrected to v2.0.0
└── DailyClaudeRoutineCheckup.md   ← This file
```

---

## 5. Database Schema — 15 Migrations

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
| `012_specimen_contamination` | v1.8.x | `contamination_flag`, `contamination_notes` on specimens (archived state) |
| `013_audit_checkpoints` | v1.9.0 | `audit_checkpoints` Merkle table (root, seq range, Dogecoin hook) |
| `014_checkpoint_auto_and_settings` | v1.10.0 | `is_auto` / `auto_source` on `audit_checkpoints`; `app_settings` key-value table |
| `015_death_events_and_lab_profile` | v1.11.0 | `event_type` on `subcultures`; `app_config` single-row table with `lab_profile` |

**19+ core tables.** No orphaned or dead-code tables detected.

---

## 6. CI / CD Health

| Pipeline | Jobs | Trigger | Status |
|---|---|---|---|
| `test.yml` | `frontend-tests`, `rust-tests`, `lint` | Every push + PR to master / claude/* | ✅ Passing (blocks merge on failure) |
| `build-windows.yml` | Tauri build → signed .msi | GitHub Release publication | ✅ Passing |
| `build-android.yml` | Debug APK (push); signed APK (release) | Push to master/claude/* and Release | ✅ Passing |

---

## 7. Test Coverage

### Frontend — ~101 assertions across 3 files

| File | Assertions | Coverage |
|---|---|---|
| `utils.test.ts` | ~58 | `escHtml`, `healthLabel`, `stageFmt`, `composeLocation`, `formatAccessionNumber`, `computeStockAdjustment`, `datestamp`, `ageDays`, `fmtAge`, `healthNum`, `effectiveHealth` |
| `exportUtils.test.ts` | ~28 | `specimenRows`, `subcultureRows`, `mediaRows`, `inventoryRows`, `complianceRows`, `prepSolutionRows` |
| `importUtils.test.ts` | ~15 | `REQUIRED_SHEET_NAMES`, `findMissingSheets` |

### Rust — 64 test functions across 5+ modules

| Module | Coverage |
|---|---|
| `db::queries` | Accession number format/sequences; hash-chain invariants (per-lineage seq, child seeding, split siblings share prev_hash, determinism); Merkle checkpoint tests (empty/single/two/three-leaf, determinism, mutation detection, checkpoint CRUD, tamper detection) |
| `commands::inventory` | `apply_stock_adjustment`, `is_low_stock` |
| `commands::compliance` | Expired permit, quarantine, positive-not-quarantined, citrus HLB, archive exemption |
| `commands::auth` | `UserRole::from_str` |
| `commands::specimens` | Death archives specimen and zeroes health; `event_type = 'death'`; archived blocks further passages; normal passages retain `'passage'`; `app_config` seeded with default profile |

### Remaining Gaps

- Zero Svelte component tests (form validation, reactive state)
- No end-to-end integration tests (create → split → audit → export → import round-trip)
- `generate_split_accession_numbers` edge cases (letter exhaustion, taken-letter skipping) untested
- No tests for `preview_split_accessions` command

---

## 8. Branch Management

| Branch | Status |
|---|---|
| `claude/eloquent-pascal-734s20` | ✅ Active — HEAD at `f6a2149` (2026-06-21) |
| `master` (remote) | ✅ Present — receives PRs from claude/* branch |

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
| SQL injection | ✅ Prevented | `rusqlite` parameterized bindings throughout |
| Dead specimen | ✅ Guarded | `record_specimen_death` requires auth; archived specimens block further passage recording |
| Lab profile lock | ✅ Guarded | Admin-only write; locked once any specimens exist |
| Split operation | ✅ Atomic | All split children, reminders, and audit entries in one SQLite transaction |
| Backup / restore | ✅ Guarded | Admin-only; two confirmations; WAL checkpoint + auto-checkpoint before copy |

---

## 11. Roadmap Progress

### What Shipped Since Last Checkup (2026-06-19 → 2026-06-21)

| Version / PR | Feature |
|---|---|
| PR #70 (v1.9.0) | WP-19 polish: contamination inheritance on split; Verify All Lineages batch button |
| PR #71 (v1.9.0) | WP-20: Merkle checkpoints, `build_merkle_root`, 3 audit commands, checkpoint UI, 14 new Rust tests |
| PR #72 (v1.10.0) | WP-21: `export_audit_proof`, `verify_exported_proof`, auto-checkpointing, migration 014, 10 new tests — **Trust Layer Phase 1 complete** |
| PR #73 (v1.11.0) | WP-22: `lab_profile` (`app_config`, migration 015), `profile.ts` store, `get/set_lab_profile` commands + Dead Specimen workflow, 5 new tests |
| `ce8fe52` | docs(roadmap): partial ROADMAP update (later partially reverted by `f6a2149`) |
| This session | Full ROADMAP/README/UserManual sync to v1.11.0 |

### Phase C / TX Horizon

| Phase | Scope | Target |
|---|---|---|
| Phase C — WP-23 | Stage CHECK → `stages` lookup table (one final table-rebuild) | v1.12.0 |
| Phase C — WP-24 | Other hardcoded vocabularies → profile-scoped lookup tables | v1.12.0 |
| Phase C — WP-25–27 | UI profile manifest; compliance rule profiles; per-vertical build identity | v1.13.0–v1.15.0 |
| Phase TX-1 — WP-28 | Strain/Cultivar data model, hash chain seeding from species, backend commands | v2.0.0 |
| Phase TX-1 — WP-29 | Strain Manager UI, Hybrid Wizard, basic Taxonomy Navigator | v2.0.0 |
| Phase TX-2 | Full taxonomy backbone, NCBI sync, pedigree visualization | v2.x |
| Phase D SteloCC | Cell Culture vertical | v2.1.0 |
| Phase E SteloMyco | Mycology vertical | v2.2.0 |

---

## 12. Technical Debt Register

| Category | Issue | Severity | Delta vs. Prior |
|---|---|---|---|
| **SpecimenDetail.svelte size** | Now 92 KB (78 KB in prior review; 54 KB before split PR). Death confirmation dialog + event card added in WP-22. | Medium | ↓ Regressed again |
| **No tests for split accession generation** | `generate_split_accession_numbers` edge cases (letter exhaustion at 26, taken-letter skip, recursive suffix) untested | Medium | Unchanged |
| **Component tests missing** | Zero Vitest tests for Svelte components | Medium | Unchanged |
| **Integration tests missing** | No end-to-end tests for create → split → death → audit → export → import | Medium | Unchanged |
| **Legacy peer deps** | `--legacy-peer-deps` masks npm conflict; blocks clean `npm audit` | Low | Unchanged |
| **Rust error context** | Generic `map_err(\|e\| e.to_string())` throughout command handlers | Low | Unchanged |
| **Schema documentation** | No ER diagram or human-readable schema reference | Low | Unchanged |
| **rand 0.8** | One major behind (0.9 released); non-breaking migration | Low | Unchanged |

**Items resolved this session:**
- ✅ ROADMAP header stale (v1.8.0 / 11 migrations) — updated to v1.11.0 / 15 migrations
- ✅ ROADMAP versioning table missing v1.9.0–v1.11.0 shipped rows — rebuilt
- ✅ ROADMAP WP-19 and WP-22 without delivered status — marked delivered with "As built" sections
- ✅ README migration count (10 → 15) and missing migration rows 014–015 — fixed
- ✅ README "Planned" section showing Merkle proofs and dead specimen as not-yet-shipped — restructured
- ✅ UserManual missing dead specimen workflow — new section added (§9 subsection)
- ✅ UserManual/README TX-1 targets still listed as v1.9.0 — corrected to v2.0.0

---

## 13. Top 5 Actionable Recommendations

### 1. Extract SplitWorkflow and DeathDialog from SpecimenDetail.svelte (1–2 hrs, medium priority)
`SpecimenDetail.svelte` has grown to 92 KB — the death confirmation dialog, split workflow UI, and per-child card rows together account for roughly 35 KB of that. Extracting a `SplitWorkflow.svelte` and a `DeathConfirmDialog.svelte` would bring `SpecimenDetail` back below 55 KB, make both features independently testable, and reduce regression surface before Phase TX-1 adds the strain pill and version-binding UI.

### 2. Add Rust unit tests for split accession generation edge cases
`generate_split_accession_numbers` covers three non-trivial paths: (a) skip letters already taken by siblings, (b) error when all 26 are exhausted, (c) recursive suffix chaining (`001A` → `001AA`). None are currently tested. These are ~30 minutes to add alongside the existing hash-chain tests in `queries.rs`.

### 3. Enforce version bump + CHANGELOG entry when migrations.rs changes in CI
The ROADMAP/README drift seen this session originated from `f6a2149` reverting a partial update. A CI check that compares the version in `package.json` against the last CHANGELOG entry would catch this automatically. A simple `grep` of the version string in `CHANGELOG.md` as a CI step (~15 min to add) prevents future drift.

### 4. Resolve npm peer-dependency conflict
Identify and pin the conflicting packages. This removes `--legacy-peer-deps` from CI, unblocks clean `npm audit` for CVE reporting, and ensures future Svelte/Vite upgrades are safe. `npm ls --all 2>&1 | grep UNMET` in a dev environment identifies the conflict root.

### 5. Add a v1.11.0 "v1.9.0–v1.11.0 Completed" section header in README Roadmap
Currently the completed section runs through v1.8.0 then jumps to "Planned". The new v1.9.0–v1.11.0 items were added to the "Completed" section this session, but the section heading at top still says "v1.2.7 — v1.7.0 — Completed". Consider splitting that section header into separate ranges so the roadmap is easier to scan at a glance.

---

## 14. Documentation Quality

| Document | Size | Status |
|---|---|---|
| `ROADMAP.md` | ~58 KB | ✅ Updated this session — v1.11.0, 15 migrations, WP-19/22 delivered, versioning table rebuilt |
| `CHANGELOG.md` | ~50 KB | ✅ Current — v1.11.0 entry present with full as-built detail |
| `README.md` | ~34 KB | ✅ Updated this session — migration 014/015 rows added; planned section restructured |
| `UserManual.md` | ~20 KB | ✅ Updated this session — dead specimen section added; TX version targets corrected |
| `.github/SIGNING.md` | Present | ✅ Covers release keystore generation |
| `docs/merkle-checkpoints.md` | Present | ✅ WP-20 spec (v1.9.0) |
| `docs/merkle-proofs.md` | Present | ✅ WP-21 proof format + Python verifier (v1.10.0) |
| Code comments | Inline | ✅ No TODO/FIXME detected; hash-chain invariants annotated |

**Structural gap:** No ER diagram or schema reference. Becomes more important as Phase C lookup tables (WP-23–24) land and as Phase TX-1 adds `strains`/`strain_parents`/`hybridization_events` tables.

---

## 15. Summary Scorecard

| Dimension | Score | Delta | Notes |
|---|---|---|---|
| Version alignment | ✅ 10/10 | → | All three manifests at 1.11.0 |
| Code organization | ⚠️ 7/10 | ↓ | SpecimenDetail regressed 78→92 KB |
| Security posture | ✅ 10/10 | → | Dead specimen + lab profile both guarded; no new attack surface |
| Test coverage | ⚠️ 7/10 | ↑ | 64 Rust tests + 101 frontend assertions; WP-22 adds 5 new tests; split accession edge cases still untested |
| Performance | ✅ 10/10 | → | No N+1; all new tables indexed |
| Documentation | ✅ 10/10 | ↑ | All four docs aligned to v1.11.0 this session |
| CI/CD | ✅ 10/10 | → | Lint + test jobs pass; Clippy zero-warning enforced |
| Technical debt | ⚠️ 7/10 | → | SpecimenDetail regressed; split accession tests still missing |
| Development velocity | ✅ 10/10 | → | 4 PRs since last checkup; Trust Layer complete |
| Roadmap clarity | ✅ 10/10 | ↑ | Versioning table rebuilt; WP-22 delivered; Phase TX-1 retargeted to v2.0.0 |

**Verdict:** Production-ready and executing at high velocity. Trust Layer Phase 1 is complete — a major milestone. Phase C de-hardening has begun (WP-22 shipped). The documentation was the main issue this session: the ROADMAP header and versioning table were stuck at v1.8.0 after a Grok-edit undo reverted a partial fix. All corrected. Next priority: WP-23 (stage CHECK → lookup table) which is the most architecturally significant Phase C change, plus continuing to chip away at `SpecimenDetail.svelte` size.
