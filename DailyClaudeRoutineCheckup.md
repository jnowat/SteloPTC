# Daily Claude Routine Checkup — SteloPTC

**Date:** 2026-06-14
**Branch reviewed:** `master` (HEAD `03cf8da`)
**Reviewer:** Claude Sonnet 4.6 (automated)
**Current version:** 1.4.1

---

## 1. Project Status — Overview

SteloPTC is in **good health overall**. Phase A (security hardening, signed releases, crash-proofing, onboarding) shipped as v1.1.0. Phase B (polish, stability, performance, print reliability, accessibility) is nearly complete at v1.4.1. The codebase is clean, actively maintained, and has meaningful CI coverage. No blocking issues were found. Several moderate risks are catalogued below.

---

## 2. Version Consistency Check

| File | Version | Status |
|---|---|---|
| `package.json` | 1.4.1 | ✅ |
| `src-tauri/Cargo.toml` | 1.4.1 | ✅ |
| `src-tauri/tauri.conf.json` | 1.4.1 | ✅ |

All three version files are aligned. **No mismatch.** (Previous drift at v1.2.5 — `tauri.conf.json` was at 1.2.3 while others were at 1.2.4 — was corrected.)

### Android versionCode — ⚠️ Needs attention before next release

`tauri.conf.json` shows `"versionCode": 24` with `"autoIncrementVersionCode": false`. The last documented versionCode bump was in CHANGELOG v1.1.0 (versionCode 24). The app is now at v1.4.1, and no subsequent CHANGELOG entry has bumped this integer. Android requires a strictly increasing `versionCode` for in-place APK upgrades. Any user who installed an APK from a GitHub Release at versionCode 24 **cannot upgrade** to a new versionCode-24 APK — Android will reject the install. Before cutting the next Android release, the versionCode must be incremented above 24.

---

## 3. Recent Development Activity (20 most recent commits on master)

| Commit | Date | Summary |
|---|---|---|
| `03cf8da` | 2026-06-13 | Merge PR #29 — general fixes and UX improvements |
| `56536e9` | — | Fix bugs across compliance, specimens, reminders, inventory |
| `1c8b7c8` | — | Fix several bugs across backend commands and frontend |
| `d3939fb` | — | Merge PR #28 — QR labels work |
| `6221b91` | — | Docs/roadmap changes from update-docs branch |
| `a0a88c6` | — | Fix Android APK build trigger on claude/* branches |
| `792f267` | — | Photo attachments per specimen, gallery, lightbox (v0.1.19) |
| `e35585e` | — | Excel multi-sheet workbook export, Export Data page (v0.1.18) |
| `a98b8a5` | — | PDF report generation via browser print API (v0.1.17) |
| `5cdb2f8` | — | Cargo.lock for v0.1.16 version bump |
| `6963353` | — | Batch operations on Specimens list (v0.1.16) |
| `448278a` | — | Rewrite README and CHANGELOG to reflect v0.1.15 |
| `b0bd776` | — | Merge PR #27 — QR labels |
| `bb1002a` | — | Add Tooltip component with '?' indicator, improved QR label (v0.1.15) |
| `50983fc` | — | Merge PR #26 — contamination/subculture features |
| `40e3292` | — | Refine tooltip wording for InventoryManager and MediaList |
| `6d184a9` | — | Enhance InventoryManager tooltips with dynamic content |
| `e4fbb8a` | — | Add title tooltips to InventoryManager |
| `460a045` | — | Add title tooltips to all Svelte components |
| `2336c6d` | — | Add tooltips to AuditLog, SpecimenDetail, SpecimenList |

**Summary:** Development is rapid and disciplined. Phase B items (v1.2.0 → v1.4.1) landed entirely on 2026-06-13/14 — twelve work packets shipped in two days. The merge-PR / topic-branch pattern (every feature via a `claude/…` branch) is well-established. The most recent merge fixed bugs across multiple domains without introducing structural changes.

---

## 4. Branch Hygiene

| Branch | Status |
|---|---|
| `master` | Active, clean |
| `claude/relaxed-galileo-nouw4a` | Current work branch (this session) |

**No stale branches.** The repository is extremely clean — only the active work branch and master exist remotely. Every feature branch from prior sessions has been merged and deleted.

---

## 5. Codebase Layout

```
SteloPTC/
├── src/                      # Svelte 5 frontend
│   ├── App.svelte            # Root component, view routing
│   ├── main.ts               # Entry point
│   ├── vite-env.d.ts
│   └── lib/
│       ├── api.ts            # All Tauri invoke() wrappers
│       ├── printUtils.ts     # Shared print delivery helper (v1.4.1)
│       ├── utils.ts          # Pure utility functions
│       ├── utils.test.ts     # Vitest tests (50 assertions)
│       ├── components/       # 28 Svelte components
│       └── stores/           # app.ts, auth.ts
│           └── styles/
│               └── tokens.css  # Central design tokens (WP-10)
├── src-tauri/
│   └── src/
│       ├── commands/         # 19 Tauri command modules
│       ├── db/
│       │   ├── migrations.rs # 7 migrations, 773 lines
│       │   ├── queries.rs    # DB helpers + accession gen
│       │   └── mod.rs
│       ├── models/           # 11 Rust model structs
│       └── auth/mod.rs
└── .github/workflows/
    ├── build-windows.yml     # MSI build + release upload
    ├── build-android.yml     # APK build + signed release
    └── test.yml              # Vitest + cargo test CI
```

**Notable additions since last checkpoint:** `printUtils.ts` (v1.4.1 refactor), `WorkQueue.svelte` (WP-08), `ImportManager.svelte` (WP-17), `DataState.svelte` (WP-11), `SkeletonLoader.svelte` / `EmptyState.svelte` (WP-11 prep), `tokens.css` (WP-10), `FirstRun.svelte`, `ForceChangePassword.svelte`.

---

## 6. Schema / Migration Status

| Migration | Version introduced | Description |
|---|---|---|
| 001 | v0.x initial | Full schema: users, sessions, species, specimens, media, subcultures, inventory, audit_log, compliance, reminders, tags, attachments |
| 002 | v0.1.9 | Expanded `stage` CHECK constraint, added `employee_id`, `prepared_solutions`, inventory physical state |
| 003 | v0.1.10 | Defensive stage constraint rebuild (idempotent fix for partial-migration users), added `error_logs` table |
| 004 | v0.1.14 | Added `qr_scans` table |
| 005 | v0.1.x | Added `contamination_flag`, `contamination_notes` to subcultures |
| 006 | v0.1.20 | Added `must_change_password` to users; seeds admin with flag=1 |
| 007 | v1.2.7 | Added 6 performance indexes (specimens + subcultures) |

**Total: 7 migrations.** Note: `ROADMAP.md` header incorrectly states "6 migrations total" — it was written before WP-15 shipped migration_007. This is a documentation-only discrepancy; the code is correct.

---

## 7. Test Coverage Assessment

### Frontend (Vitest)

| File | Tests | Coverage |
|---|---|---|
| `src/lib/utils.test.ts` | 50 assertions | `utils.ts` (all 7 exports) + `printUtils.ts` (ageDays, fmtAge, healthNum) |

**Gaps:** No component-level tests despite `@testing-library/svelte` being installed. `WorkQueue.svelte`, `ImportManager.svelte`, `SpecimenForm.svelte`, `SpecimenList.svelte` validation logic — all untested.

### Rust (`cargo test --lib`)

| File | Tests | What's covered |
|---|---|---|
| `db/queries.rs` | 8 tests | `generate_accession_number` (5 cases), `PaginationParams` offset math (3 cases) |
| `commands/inventory.rs` | 8 tests | `apply_stock_adjustment`, `is_low_stock` |
| `commands/compliance.rs` | 10 tests | All 4 auto-flag rules via in-memory SQLite |

**Gaps (critical for Phase B gate on Trust Layer):**
- `commands/specimens.rs` — 0 tests (596 lines, most complex command)
- `commands/subcultures.rs` — 0 tests (transaction atomicity unverified by test)
- `commands/import.rs` — 0 tests (upsert logic, dry-run/commit path)
- `commands/backup.rs` — 0 tests (WAL checkpoint validation logic)
- `commands/work_queue.rs` — 0 tests (urgency sort, 5 detection conditions)
- `commands/auth.rs` — 0 tests (bcrypt verify, session expiry)
- `db/migrations.rs` — 0 tests (no fixture-DB migration test as WP-14 intended)

The ROADMAP states WP-14 (tests) is the **hard gate** on WP-18 (hash-chain audit log). The current test harness covers ~15% of command logic — not yet sufficient to safely land cryptographic invariants.

---

## 8. CI Health

| Workflow | Trigger | Status |
|---|---|---|
| `test.yml` | Push to master or claude/*, PR to master | Two jobs: `frontend-tests` (Vitest) + `rust-tests` (cargo test --lib) |
| `build-windows.yml` | Push to master or claude/*, PR to master, release:published | Builds MSI with 3 retries; attaches to GitHub Release |
| `build-android.yml` | Push to master or claude/*, release:published | Builds signed APK using `ANDROID_KEYSTORE_BASE64` secret |

**Positive:** Test CI blocks merges on failure. Both Windows and Android workflows fire on every push to master. Retry logic on Windows build handles transient WiX download failures.

**Risk:** The Android build has a `build.gradle.kts` signing-patch step (documented in SIGNING.md) that must be re-applied after every `cargo tauri android init` regeneration. If this step drifts or breaks, the release APK will fail to build. No CI check validates the signing config before the build starts.

---

## 9. Code Quality Findings

### Production `.unwrap()` — Minor risk

`src-tauri/src/db/migrations.rs:682`:
```rust
let password_hash = bcrypt::hash("admin", bcrypt::DEFAULT_COST).unwrap();
```
This is in `seed_defaults()` which runs only on first DB initialization. `bcrypt::hash` can theoretically fail if the system entropy source is unavailable (extremely rare). Using `.expect("bcrypt seeding failed")` or propagating via `?` would be marginally safer and more descriptive.

All other `.unwrap()` occurrences in production paths were confirmed to be inside `#[cfg(test)]` blocks only.

### Console output in App.svelte

`src/App.svelte` contains `console.warn` (line 69) and `console.error` (line 76) in error-handling paths. These are appropriate for development and diagnostic purposes but will appear in end-user browser consoles. Consider routing to the existing error-log system (`logError`) instead of raw console output in production builds.

### WP-18/WP-19 numbering conflict in CHANGELOG

The CHANGELOG uses **WP-18** for "Print reliability audit" (v1.3.1) and **WP-19** for "Professional Specimen Inventory Report" (v1.4.0). The ROADMAP reserves WP-18 for "Hash-chain the immutable audit log" and WP-19 for "Chain verification command + integrity panel" (both Trust Layer Phase 1). These are completely different features — the CHANGELOG borrowed the WP numbers for unplanned items without reserving them. This will create confusion when the Trust Layer work begins. The CHANGELOG entries should be relabeled (e.g., "unplanned polish" or a new WP-09b/WP-09c sequence) before WP-18/19 Trust Layer work starts.

### `xlsx` dependency — Supply chain awareness

`package.json` depends on `xlsx@^0.18.5` (SheetJS Community Edition). This package had a documented supply chain incident in early 2023. Version 0.18.5 is the last published community version on npm; the project is no longer actively maintained there. The current install is pinned at 0.18.5 and the package-lock.json should lock the exact hash. **Recommended action:** verify the installed package SHA against the published npm manifest, and consider evaluating `exceljs` as a maintained alternative if SheetJS maintenance becomes a concern for the import/export feature.

---

## 10. Technical Debt Register

| ID | Item | Severity | Effort |
|---|---|---|---|
| TD-01 | ROADMAP.md header says "6 migrations" — should be 7 | Low | 1 line |
| TD-02 | Android `versionCode` stuck at 24 since v1.1.0 | **High** | 1 line + process |
| TD-03 | WP-18/WP-19 number collision between CHANGELOG and ROADMAP | Medium | Rename CHANGELOG entries |
| TD-04 | Test gap: specimens, subcultures, import, backup, work_queue have 0 Rust tests | Medium | 2–4 hrs per module |
| TD-05 | No component-level Svelte tests despite `@testing-library/svelte` installed | Low | Ongoing |
| TD-06 | `seed_defaults` uses `.unwrap()` on bcrypt (migrations.rs:682) | Low | 1 line |
| TD-07 | `console.warn/error` in App.svelte reach end-user consoles | Low | Reroute to error log |
| TD-08 | `xlsx@^0.18.5` — last community SheetJS version, no active maintenance | Medium | Evaluate + pin |

---

## 11. Top 5 Actionable Recommendations

### R-1 — Fix Android versionCode before next release (CRITICAL before any release)
**Why:** Android refuses to install an APK whose versionCode is ≤ the installed version. All releases since v1.1.0 still carry versionCode 24. Any user who installed the v1.1.0 release APK cannot upgrade in-place.
**What:** Increment `versionCode` in `tauri.conf.json` (e.g., to 30 to give headroom), then set up a convention — either `autoIncrementVersionCode: true` or a pre-release CI step that bumps the integer — so this can't drift again.
**Files:** `src-tauri/tauri.conf.json` line 37.

### R-2 — Reconcile WP-18/WP-19 numbering before starting the Trust Layer
**Why:** The CHANGELOG has already used WP-18 and WP-19 for the print-reliability audit and the professional inventory report. When work begins on hash-chaining (planned as WP-18 in the ROADMAP), the mismatch will create confusion in commit messages, PR titles, and history.
**What:** Edit the two CHANGELOG entries — rename `WP-18` to `WP-09a` (or "Print Audit") and `WP-19` to `WP-09b` (or "Inventory Report"), add a note explaining the renaming. Then update the ROADMAP header to reflect 7 migrations.
**Files:** `CHANGELOG.md` (v1.3.1 and v1.4.0 headers), `ROADMAP.md` (header schema count).

### R-3 — Expand Rust test coverage to unblock Trust Layer (WP-14 gate)
**Why:** The ROADMAP explicitly states WP-14 (first test harness) is the hard gate on WP-18 (hash-chain). The current 26 Rust tests cover compliance flags, accession numbering, and inventory math — but leave the most data-critical commands (specimens, subcultures, import, backup) with zero coverage. Cryptographic invariants should not be shipped without tests asserting chain continuity.
**What:** Add `#[cfg(test)]` modules in: `commands/backup.rs` (WAL checkpoint behaviour), `commands/specimens.rs` (accession uniqueness, create + archive round-trip), `commands/subcultures.rs` (transaction rollback on partial failure), `commands/import.rs` (upsert vs insert, dry-run/commit parity). 2–4 hrs per module. Aim for 60+ Rust tests before starting WP-18.
**Files:** The four command files above.

### R-4 — Fix `seed_defaults` unwrap in migrations.rs
**Why:** A `.unwrap()` in DB initialization can panic if `bcrypt::hash` fails. It's a low-probability event but a crash during first-run initialization would be unrecoverable without deleting the DB.
**What:** Change line 682 from `.unwrap()` to `.map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?` so the error propagates cleanly through `DbResult`.
**File:** `src-tauri/src/db/migrations.rs:682`.

### R-5 — Add component-level Svelte tests for SpecimenForm validation
**Why:** Form validation logic in `SpecimenForm.svelte` (required fields, date checks, health range) is the most user-facing logic currently untested. The `@testing-library/svelte` package is already installed but unused for components.
**What:** Add `src/lib/components/SpecimenForm.test.ts` covering: submitting with empty required fields shows errors, valid form calls the API, health slider bounds are enforced. This bootstraps the component test pattern for future components.
**File:** New `src/lib/components/SpecimenForm.test.ts`.

---

## 12. Next Steps on the Roadmap

Current version is **v1.4.1** — Phase B is effectively complete:

| Work Packet | Status | Notes |
|---|---|---|
| WP-06 Bug/polish clearance | ✅ v1.1.1 | |
| WP-07 QR scanner rejects non-SteloPTC | ✅ v1.1.1 | |
| WP-08 Work Queue / Daily Task View | ✅ v1.2.0 | |
| WP-09 Tauri-reliable print | ✅ v1.2.5 | v1.4.1 further extracted to `printUtils.ts` |
| WP-10 Design token system | ✅ v1.2.2 | `tokens.css`, Dashboard, Sidebar |
| WP-11 Loading/empty/error states | ✅ v1.2.1/1.2.3 | All list views via `DataState.svelte` |
| WP-12 Accessibility pass | ✅ v1.2.6 | WCAG 2.1 AA target |
| WP-13 Print/PDF polish | ✅ v1.2.4 | Professional header/footer, A4+Letter |
| WP-14 First test harness | ✅ v1.2.4 | 50 TS + 26 Rust tests |
| WP-15 Query performance & indexing | ✅ v1.2.7 | 6 new indexes, N+1 eliminated |
| WP-16 Backup → Restore | ✅ v1.3.0 | Two-confirm UX, WAL checkpoint |
| WP-17 Excel import | ✅ v1.3.0 | Dry-run + upsert round-trip |
| WP-18* Hash-chain audit log | 🔴 Not started | Trust Layer Phase 1 |
| WP-19* Chain verification panel | 🔴 Not started | Trust Layer Phase 1 |
| WP-20 Merkle checkpoints | 🔴 Not started | Trust Layer Phase 1 |
| WP-21 Merkle proof export | 🔴 Not started | Trust Layer Phase 1 |

*WP-18/19 numbers conflict with CHANGELOG entries — see R-2 above.

**Immediate next up (planned v1.3.0 Trust Layer, now Phase B overflow):** Resolve numbering conflict, expand tests, then begin hash-chain implementation. The Trust Layer is the last planned item before Phase C (de-harden domain, v1.4.0→).

---

## 13. Health Summary

| Dimension | Score | Notes |
|---|---|---|
| Version alignment | ✅ Green | All three files at 1.4.1 |
| Branch hygiene | ✅ Green | Only 2 branches, no stale PRs |
| CI coverage | 🟡 Yellow | Tests exist and run; coverage gap in critical commands |
| Test suite | 🟡 Yellow | 76 total tests; specimens/import/backup uncovered |
| Code quality | ✅ Green | No panics in hot paths; clean architecture |
| Documentation | 🟡 Yellow | ROADMAP header stale (migration count, WP numbering) |
| Security posture | ✅ Green | CSP locked, forced password change, bcrypt, signed releases |
| Android release readiness | 🔴 Red | versionCode frozen at 24 — must be bumped before next release |
| Roadmap progress | ✅ Green | Phase A + most of Phase B complete; on track for Trust Layer |

**Overall: Healthy with two moderate items to address before the next release (versionCode and CHANGELOG numbering).**

---

*Checkup generated by Claude Sonnet 4.6 on 2026-06-14. Next suggested review: after Trust Layer (WP-18–21) implementation, or before any GitHub Release.*
