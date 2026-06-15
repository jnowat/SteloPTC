# SteloPTC — Daily Claude Routine Checkup

**Date:** 2026-06-15  
**Branch reviewed:** `master` (HEAD: `38aa40a`)  
**Reviewed by:** Claude (automated routine)  
**Current version:** `v1.4.1`

---

## 1. Executive Status

| Area | Status | Notes |
|---|---|---|
| Version alignment | ✅ Clean | All three manifest files at `1.4.1` |
| CI / test pipeline | ✅ Passing | All recent runs completed with success |
| Test suite | ⚠️ Thin | 50 frontend + 26 Rust tests; no component or integration tests |
| Stale branches | ⚠️ Action needed | 23 merged branches never pruned |
| Large-component debt | ⚠️ Growing | Four Svelte files are 40–74 KB each |
| Dependency health | ⚠️ Minor | `--legacy-peer-deps` flag masks a real conflict |
| Roadmap progress | ✅ On track | Phase B WP-06→WP-19 all shipped; Trust Layer (WP-20+) not yet started |

---

## 2. Version Consistency Check

All three canonical version sources are **in sync**:

| File | Version |
|---|---|
| `package.json` | `1.4.1` |
| `src-tauri/Cargo.toml` | `1.4.1` |
| `src-tauri/tauri.conf.json` | `1.4.1` |

**No mismatches detected.**

---

## 3. Recent Commit History (last 20 on `master`)

| # | SHA | Summary | Date |
|---|---|---|---|
| 1 | `38aa40a` | Merge PR #52 — refactor: print system + shared utilities (v1.4.1) | 2026-06-14 |
| 2 | `687d90b` | refactor(print): extract `deliverPrint`, fix CSP bug in all three print paths | 2026-06-14 |
| 3 | `2fc28a9` | fix(print): call `win.print()` from parent WebView context (Tauri CSP bypass) | 2026-06-14 |
| 4 | `68c41ab` | chore: update `package-lock.json` after dependency install | 2026-06-14 |
| 5 | `e8ef850` | Merge PR #49 — feat: restore-from-backup (WP-16) | 2026-06-14 |
| 6 | `f649c23` | feat(WP-16): `restore_backup` command + admin UI + double-confirmation gate | 2026-06-13 |
| 7 | `f912d80` | Merge PR #51 — feat: professional Specimens Summary report (v1.3.1) | 2026-06-13 |
| 8 | `5cb17b8` | WP-19: executive summary, per-group headers, Age column, print options popover | 2026-06-13 |
| 9 | `4d6ee59` | WP-18: audit and confirm print reliability across all three print functions | 2026-06-13 |
| 10 | `729be96` | Merge PR #50 — feat: Excel import with dry-run preview (WP-17) | 2026-06-13 |
| 11 | `3b94b6e` | feat: WP-17 — `ImportManager.svelte` + `import_xlsx` Tauri command (v1.3.0) | 2026-06-13 |
| 12 | `471cf33` | Merge PR #48 — perf: DB query performance and indexing (WP-15) | 2026-06-13 |
| 13 | `921802c` | chore: update `Cargo.lock` for v1.2.7 | 2026-06-13 |
| 14 | `a6b556c` | WP-15: 6 new indexes, eliminate N+1 subquery, paginate `list_subcultures` | 2026-06-13 |
| 15 | `83972fb` | Merge PR #47 — WP-12: Accessibility & keyboard pass (WCAG 2.1 AA) | 2026-06-13 |
| 16 | `fef9b60` | v1.2.6: skip-to-content, focus indicators, ARIA, focus traps | 2026-06-13 |
| 17 | `bab30d1` | Merge PR #46 — fix: print dialogs on Windows/Tauri/WebView2 (v1.2.5) | 2026-06-13 |
| 18 | `6cc9b3a` | WP-09: in-page print fallback for `printCultureReport` | 2026-06-13 |
| 19 | `6d21f3d` | WP-09: in-page print fallback for all three print functions | 2026-06-13 |
| 20 | `a1c6e09` | WP-09: version bump 1.2.4 → 1.2.5, CHANGELOG, ROADMAP | 2026-06-13 |

**Summary of velocity:** The project is in an extremely active Phase B sprint. In the last ~48 hours, six work packets landed (WP-12, WP-15, WP-16, WP-17, WP-18/19, and refactor WP-20-prep), taking the app from v1.2.5 → v1.4.1. Commit quality is high — descriptive messages, atomic PRs, no force-pushes, CI gating on every merge.

---

## 4. ROADMAP & CHANGELOG Summary

### Phase A — Complete ✅
Security hardening (CSP, forced password change), signed GitHub Releases (Windows MSI + Android APK), crash-proofing (transactions, WAL checkpoint validation), and onboarding first-run experience.

### Phase B — In Progress
| Work Packet | Status | Version |
|---|---|---|
| WP-01 to WP-05 (security, UX, first-run) | ✅ Done | v0.1.20 → v1.1.0 |
| WP-06 — Print error handling | ✅ Done | v1.1.1 |
| WP-07 — QR scanner non-SteloPTC rejection | ✅ Done | v1.1.1 |
| WP-08 — Work Queue / daily task view | ✅ Done | v1.2.0 |
| WP-09 — Windows/Tauri print fallback | ✅ Done | v1.2.5 |
| WP-10 — Design token system | ✅ Done (partial migration) | v1.2.2 |
| WP-11 — Loading/empty/error states | ✅ Done | v1.2.3 |
| WP-12 — Accessibility (WCAG 2.1 AA) | ✅ Done | v1.2.6 |
| WP-13 — Print/PDF polish | ✅ Done | v1.2.4 |
| WP-14 — First test harness (Vitest + cargo test) | ✅ Done | v1.2.4 |
| WP-15 — DB query performance + indexing | ✅ Done | v1.2.7 |
| WP-16 — Backup → Restore | ✅ Done | v1.3.0 |
| WP-17 — Excel import (dry-run, transaction) | ✅ Done | v1.3.0 |
| WP-18 — Print reliability audit | ✅ Done | v1.3.1 |
| WP-19 — Professional specimen inventory report | ✅ Done | v1.4.0 |
| **WP-20+ — Trust/Audit Layer (hash-chain + Merkle)** | 🔲 Not started | — |
| **Cell Culture / Mycology vertical expansion** | 🔲 Not started | — |

---

## 5. Codebase Layout

```
SteloPTC/
├── .github/
│   ├── SIGNING.md
│   └── workflows/
│       ├── test.yml            ← Vitest + cargo test on push/PR
│       ├── build-windows.yml   ← MSI build on release + master/claude/**
│       └── build-android.yml   ← APK build on release + master/claude/**
├── src/
│   ├── App.svelte              ← Root shell + view router
│   ├── main.ts
│   └── lib/
│       ├── api.ts              ← Tauri invoke() wrappers (9.5 KB)
│       ├── utils.ts            ← Pure TS utilities (2.2 KB)
│       ├── printUtils.ts       ← Shared print delivery (5.5 KB)
│       ├── utils.test.ts       ← 50 Vitest assertions (8.6 KB)
│       ├── components/         ← 23 Svelte components (see below)
│       ├── stores/             ← Svelte stores (auth, app state)
│       └── styles/
│           └── tokens.css      ← Design tokens (partial adoption)
├── src-tauri/
│   ├── Cargo.toml              ← v1.4.1, Tauri 2 + rusqlite (bundled)
│   ├── tauri.conf.json         ← v1.4.1, hardened CSP
│   └── src/
│       ├── lib.rs              ← Tauri app setup + command registration
│       ├── db/
│       │   ├── migrations.rs   ← 7 migrations (007 = index additions)
│       │   ├── mod.rs          ← DB init + WAL setup
│       │   └── queries.rs      ← Accession generation + pagination helpers
│       ├── commands/           ← 16 Tauri command modules
│       │   ├── specimens.rs    (23.9 KB)
│       │   ├── import.rs       (24.7 KB)
│       │   ├── compliance.rs   (22.5 KB)
│       │   ├── inventory.rs    (19.6 KB)
│       │   ├── subcultures.rs  (16.6 KB)
│       │   ├── work_queue.rs   (13.1 KB)
│       │   └── … 10 others
│       ├── auth/               ← Session token + role checks
│       └── models/             ← Serde-serializable data structs
└── scripts/
```

### Component size overview (bytes)
| Component | Size | Concern |
|---|---|---|
| `SpecimenDetail.svelte` | 74 KB | 🔴 God component |
| `MediaList.svelte` | 45 KB | 🟠 Large |
| `SpecimenList.svelte` | 43 KB | 🟠 Large |
| `InventoryManager.svelte` | 40 KB | 🟠 Large |
| `Dashboard.svelte` | 29 KB | 🟡 Watchable |
| `ErrorLog.svelte` | 24 KB | 🟡 Watchable |
| All others | < 16 KB | ✅ Acceptable |

---

## 6. CI / Pipeline Status

| Workflow | Last run | Conclusion |
|---|---|---|
| Tests (Vitest + cargo) | 2026-06-14 | ✅ success |
| Build Windows MSI | 2026-06-14 | ✅ success |
| (Android APK — release gated) | n/a recent | N/A |

**Workflow configuration notes:**
- `test.yml` triggers on `push` to `master` and `claude/**`, and on PRs to `master`. Coverage is good.
- `build-windows.yml` triggers on `push` to `master`/`claude/**` and on GitHub `release` events.
- `build-android.yml` is release-gated; debug builds avoid signing failures (WP-CI fix from PR #34).
- No failing CI detected. All recent runs conclude `success`.

---

## 7. Test Coverage Assessment

### Frontend (Vitest)
| File | Tests | What's covered |
|---|---|---|
| `src/lib/utils.test.ts` | 50 | `utils.ts` (33) + `printUtils.ts` helpers (17) |
| Everything else | 0 | No Svelte component tests exist |

### Rust (`cargo test --lib`)
| Module | Tests | What's covered |
|---|---|---|
| `db::queries` | 8 | `generate_accession_number`, `PaginationParams` |
| `commands::inventory` | 8 | `apply_stock_adjustment`, `is_low_stock` |
| `commands::compliance` | 10 | Auto-flag rules (expired permit, quarantine, citrus HLB) |
| Everything else | 0 | — |

### Coverage gaps (notable absences)
- `specimens.rs` (23.9 KB) — no unit tests for create/update/search logic
- `import.rs` (24.7 KB) — complex upsert logic, zero test coverage
- `work_queue.rs` (13.1 KB) — urgency prioritization logic untested
- `subcultures.rs` (16.6 KB) — passage creation/history untested
- `backup.rs` (6.9 KB) — WAL checkpoint + file-copy flow untested
- All 23 Svelte components — no component or integration tests
- No E2E tests (no Playwright/WebdriverIO setup)

---

## 8. Stale Branches

**23 merged branches** exist on the remote that are safe to prune:

```
claude/affectionate-clarke-h0udhz    (PR #41 merged)
claude/affectionate-faraday-duljr6   (merged)
claude/amazing-lovelace-6e092j       (PR #43 merged)
claude/awesome-cannon-xkksun         (PR #51 merged)
claude/confident-galileo-4lggbt      (PR #33 merged)
claude/determined-brown-hjfgv8       (PR #42 merged)
claude/dreamy-rubin-5zcxoa           (merged)
claude/festive-cori-uuc8pc           (PR #48 merged)
claude/gifted-curie-109029           (PR #39 merged)
claude/hopeful-lovelace-5d7asr       (PR #49 merged)
claude/inspiring-albattani-bguarv    (merged)
claude/modest-curie-0dcqxw          (PR #40 merged)
claude/nice-bell-89n85g             (PR #52 merged)
claude/nice-dirac-jw8xah            (PR #46 merged)
claude/optimistic-dijkstra-ivdgpt   (PR #38 merged)
claude/relaxed-galileo-nouw4a       (merged)
claude/serene-fermi-3rx2i2          (PR #34/#35 merged)
claude/stoic-tesla-p04y8m           (PR #50 merged)
claude/tender-mccarthy-dtylge       (PR #44 merged)
claude/upbeat-mendel-j660j8         (PR #37 merged)
claude/wizardly-brown-l97mk5        (PR #47 merged)
claude/zealous-wozniak-v63nid       (PR #45 merged)
direct-roadmap-patch-1              (PR #36 merged)
```

**Recommended clean-up command (run locally or via CI):**
```bash
git fetch --prune origin
# Manually confirm before batch deleting via gh CLI or GitHub UI
```

---

## 9. Dependency Health

### Frontend (`package.json`)
| Package | Version | Status |
|---|---|---|
| `svelte` | `^5.0.0` | ✅ Current major |
| `@tauri-apps/api` | `^2.5.0` | ✅ Current |
| `vite` | `^6.0.0` | ✅ Current major |
| `vitest` | `^3.0.0` | ✅ Current major |
| `typescript` | `^5.5.0` | ✅ Current |
| `xlsx` | `^0.18.5` | ⚠️ Last open-source SheetJS release; later versions require commercial license — effectively pinned |
| `html5-qrcode` | `^2.3.8` | ⚠️ Last release was 2023; package is effectively unmaintained |
| `@testing-library/svelte` | `^5.0.0` | ✅ Svelte 5 compatible |

**Peer dependency conflict:** CI uses `npm ci --legacy-peer-deps`. This suppresses a peer resolution error rather than fixing it — the root conflict should be diagnosed (`npm install --legacy-peer-deps` masks it). Likely a transitive conflict between `@testing-library/jest-dom` and `jsdom` version ranges.

### Rust (`Cargo.toml`)
| Crate | Version | Status |
|---|---|---|
| `tauri` | `2` | ✅ Current |
| `rusqlite` | `0.32` (bundled) | ✅ Recent |
| `bcrypt` | `0.17` | ✅ |
| `tokio` | `1` (full) | ✅ Current |
| `chrono` | `0.4` | ✅ |
| All others | — | ✅ No known issues |

---

## 10. Code Quality Observations

### Strengths
- Consistent error handling: all Tauri commands propagate errors through the audit/error-log system; no silent failures remaining after WP-06/WP-09.
- Transaction safety: `create_subculture`, `create_media_batch`, and `import_xlsx` are all wrapped in atomic SQLite transactions with rollback.
- Security posture: hardened CSP, bcrypt password hashing, forced first-login password change, role-based command gating, WAL-backed database.
- Print system fully resolved: CSP inline-script bug fixed across all three print functions; `deliverPrint()` shared utility eliminates duplication.
- Design tokens introduced (`tokens.css`), though adoption is **partial** — only `Dashboard.svelte` and `Sidebar.svelte` fully consume them.

### Concerns
- **God component pattern** — `SpecimenDetail.svelte` at 74 KB is a significant maintenance risk. It handles specimen info display, passage recording, photo lightbox, compliance, print, and more in a single file.
- **Incomplete design token migration** — WP-10 introduced `tokens.css` but only migrated two components. The remaining 21 components still use hardcoded values, creating inconsistency and making global theming changes expensive.
- **`any[]` type contracts in `api.ts`** — The `list_subcultures` wrapper preserves an `any[]` return type for backward compatibility. This is documented but represents type safety debt.
- **`tokio::full` feature** — `Cargo.toml` pulls in all Tokio features (`features = ["full"]`). Only `tokio::sync` (Mutex) is actually needed in this app; using `full` adds unnecessary compile time and binary size.

---

## 11. Technical Debt Register

| Item | Severity | Effort | Notes |
|---|---|---|---|
| 23 stale remote branches | Low | Low | Safe to batch-delete; all merged |
| `--legacy-peer-deps` CI workaround | Low | Low | Diagnose and fix root conflict |
| `tokio = { features = ["full"] }` | Low | Low | Replace with `features = ["sync"]` |
| Incomplete design token migration | Medium | Medium | 21 components still using hardcoded values |
| Zero Svelte component tests | Medium | High | 23 components, 0 coverage |
| Zero Rust tests for specimens/import/work_queue | Medium | Medium | Highest-risk business logic untested |
| `SpecimenDetail.svelte` god component (74 KB) | Medium | High | Decompose into sub-components |
| `html5-qrcode` unmaintained | Low | Medium | Evaluate `@zxing/browser` or similar replacement |
| Trust Layer (WP-20+) not started | High | High | Next roadmap milestone; compliance differentiator |

---

## 12. Top 5 Actionable Recommendations

### 1. 🧹 Prune Stale Branches (Quick Win — 30 min)
Delete the 23 merged `claude/` branches and `direct-roadmap-patch-1`. This cleans up the remote, makes `git branch -r` readable, and removes noise from the GitHub branch list. All SHAs are confirmed merged.

**Action:** Batch-delete via GitHub UI or `gh api` calls. Consider adding branch auto-delete on merge to the repo settings to prevent recurrence.

---

### 2. 🧪 Add Rust Tests for High-Risk Business Logic (High Impact — 1–2 days)
The `import.rs` (Excel upsert logic, 24 KB), `work_queue.rs` (urgency prioritization), and `specimens.rs` (accession generation, archive logic) contain the most complex and data-sensitive paths in the app — with zero test coverage. A regression here would be silent until a user reports corrupted data.

**Action:** Add `#[cfg(test)]` modules with in-memory SQLite (same pattern as `compliance.rs` tests) covering at minimum: import dry-run vs. commit semantics, work queue urgency ranking, and specimen search/filter.

---

### 3. 🏗️ Start Trust Layer — WP-20 (Strategic — Ongoing)
The cryptographic audit hash-chain (hash-chain + Merkle checkpoints) is the highest-priority incomplete roadmap item. It is also the feature most likely to differentiate SteloPTC in compliance-driven lab environments. Phase B cannot be considered complete without it.

**Action:** Begin WP-20 per the ROADMAP specification. The existing audit log infrastructure (`audit.rs`, `AuditLog.svelte`) and the immutable `compliance.rs` flag system are the correct foundation to extend.

---

### 4. 🔧 Resolve `--legacy-peer-deps` Properly (Low-Effort Hygiene — 2h)
The CI workaround masks a real peer dependency conflict. Running `npm install` without the flag will reveal the exact conflict. Most likely culprit: transitive version mismatch between `@testing-library/jest-dom ^6.0.0`, `jsdom ^26.0.0`, and their shared peer expectations.

**Action:** Run `npm install` locally without the flag, identify the conflict, and pin or upgrade the offending package. Remove the `--legacy-peer-deps` flag from `test.yml` once resolved.

---

### 5. ✂️ Decompose `SpecimenDetail.svelte` (Medium Term — 3–5 days)
At 74 KB, `SpecimenDetail.svelte` is the single highest-risk file for regressions during future feature work. It blends at least six distinct responsibilities. The pattern of `DataState.svelte`, `EmptyState.svelte`, `SkeletonLoader.svelte`, and other small focused components is already established — it just hasn't been applied here.

**Suggested split:**
- `SpecimenHeader.svelte` — accession, species, status badges
- `SpecimenPassageHistory.svelte` — subculture table + record passage form
- `SpecimenPhotoGallery.svelte` — lightbox, upload, delete
- `SpecimenCompliance.svelte` — compliance tab content
- `SpecimenPrintButton.svelte` — print culture report trigger

---

## 13. Proactive Improvement: Complete Design Token Migration (WP-10 Follow-Up)

WP-10 created a clean `tokens.css` design system with color, spacing, typography, shadow, and z-index tokens. Only `Dashboard.svelte` and `Sidebar.svelte` were migrated. The remaining 21 components still use hardcoded values like `#2563eb`, `8px`, `border-radius: 6px`, etc.

Completing the migration would: (a) make global theming trivially easy, (b) ensure dark mode is consistent without component-by-component `:global(.dark)` overrides, and (c) reduce CSS size as duplicate values consolidate.

This is low-risk, incremental, and has no behavioral impact — a good background task to run in parallel with larger feature work.

---

## 14. Summary

SteloPTC `master` is in **good operational health**. CI is green, all versions are aligned, the recent print system refactor (v1.4.1) landed cleanly, and Phase B has made exceptional progress over the past 48 hours — shipping accessibility, performance, import/export, backup/restore, and professional reporting in rapid succession.

The three areas needing near-term attention are:

1. **Test coverage** — the current 76 tests cover utilities only; business logic in specimens, import, and work queue is unguarded.  
2. **Stale branch cleanup** — 23 merged branches are safe to delete immediately.  
3. **Trust Layer (WP-20+)** — the next major roadmap milestone is not yet started.

No urgent regressions, security issues, or CI failures were detected.

---

*Report generated: 2026-06-15 by Claude (automated daily routine)*  
*Commit reviewed: `38aa40a286f6a24be4bc07043f3be28a91a7796a`*  
*Repository: `jnowat/SteloPTC` @ `master`*
