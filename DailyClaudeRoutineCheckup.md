# SteloPTC — Daily Claude Routine Checkup

**Date:** 2026-07-01
**Branch reviewed:** `master` (HEAD: `c580276`, PR #115) · session work on `claude/hopeful-bell-y8lhpc` (started even with `master`)
**Reviewed by:** Claude (automated routine)
**Current version:** `v1.40.2` — confirmed aligned across `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` ✅

---

## 1. Executive Status

| Area | Status | Notes |
|---|---|---|
| Version alignment | ✅ All three manifests at 1.40.2 | No drift — the CI version-sync guard added in a prior session is holding |
| Version display in app | ✅ Correct | `Sidebar.svelte` calls `getVersion()` from `@tauri-apps/api/app` at runtime — reads `tauri.conf.json`, will show `v1.40.2` |
| Android committed baseline | ⚠️ Found stale, fixed this session | `build.gradle.kts` had `versionCode = 27` (didn't match `tauri.conf.json`'s committed `24`, or README's documented baseline). Corrected to `24`; `versionName` left at the documented placeholder `"1.1.0"` since CI fully regenerates `gen/android` from `tauri.conf.json` on every build |
| CI / test pipeline | ✅ Green | `test.yml`, `build-windows.yml`, `build-android.yml` all passing on current HEAD. One historical `build-android.yml` failure (on the v1.40.1 commit) was the exact Tauri-CLI-drift bug fixed in v1.40.2 — already resolved, not recurring |
| `build-ios.yml` | ⚠️ Failing, but expected/disclosed | 3/3 historical runs failed — no Apple Developer credentials exist in this repo; workflow is deliberately scoped to `workflow_dispatch`/weekly/`release` only (not a merge gate) since v1.39.1. Documented everywhere as unverified end-to-end |
| Test suite | ✅ Verified locally | 467/467 Rust tests (`--no-default-features`), 104/104 Vitest, `svelte-check` 0 errors/0 warnings, `cargo clippy -D warnings` clean — all match CHANGELOG's claims exactly |
| Stale branches | ✅ None | Only `master` exists on the remote |
| npm vulnerabilities | ⚠️ Found 9, fixed 8 this session | `npm audit fix --legacy-peer-deps` resolved devalue/picomatch/postcss/rollup/svelte/vite chain issues with no code changes needed; verified `npm run check`, `npm test`, and `npm run build` all still pass. One high-severity issue remains unfixable (`xlsx`, no upstream fix) — see §9 |
| CHANGELOG freshness | ✅ Current | Top entry is `[1.40.2] - 2026-07-01`, matches shipped version |
| ROADMAP freshness | ⚠️ Found stale, fixed this session | Status banner and closing summary still said "v1.40.1" while the body prose already described a "v1.40.2 stabilization sprint" — self-contradictory. Also found and fixed an unrelated **unclosed parenthesis** in the closing paragraph introduced mid-session by my own first-pass edit (caught immediately, verified before commit) |
| README freshness | ⚠️ Found stale, fixed this session | One full patch behind (no v1.40.1/v1.40.2 mention), stale test counts (463/499 vs. verified 467), stale migration count comment ("29" vs. actual 45), and a self-contradictory legacy "planned work" section listing already-shipped features (Cell Culture, Mycology, notifications, field permissions) as `[ ]` incomplete |
| UserManual freshness | 🔴 Severely stale, partially fixed | Header pinned to **v1.32.0 / June 2026** — 8 minor versions and the entire Phase F body of work behind. §18 described shipped Phase TX-1/TX-2/Mycology/Cell-Culture work as future targets, contradicting the manual's own body text. Header + §18 fixed this session; **sections 1–17 still only document the app through Phase E** — see §8 recommendation #1 |
| Large-component debt | ⚠️ Unchanged | `SpecimenDetail.svelte` is 126.7 KB — flagged for extraction in the last two checkups, still not scheduled |
| Bundle size | ⚠️ New finding | Production build emits a single 1.48 MB (454 KB gzip) JS chunk; Vite warns on chunks > 500 KB. Not a regression — first time this was measured directly |
| Dependency health (Rust) | ✅ Good | `tauri` pinned at `2.11.3` matching the CLI (the actual v1.40.2 CI fix); rusqlite/bcrypt/argon2/aes-gcm/ed25519-dalek/zip/lettre all current |

**Overall health: GOOD, with real but fixable documentation drift.** The codebase itself — tests, CI, security posture, dependency pinning — is in excellent shape and the team's own hardening discipline (three consecutive patch releases fixing real bugs found via self-review) is a strong signal. The gap this session closed was almost entirely **documentation congruence**: ROADMAP/README/UserManual had all drifted out of sync with the shipped code by 1–2 patch releases (ROADMAP, README) to as much as 8 minor releases (UserManual), and one doc (README's legacy planning section) had become internally self-contradictory. No functional regressions were found or introduced.

---

## 2. Version Consistency Check

| File | Version | Status |
|---|---|---|
| `package.json` | `1.40.2` | ✅ |
| `src-tauri/Cargo.toml` | `1.40.2` | ✅ |
| `src-tauri/tauri.conf.json` | `1.40.2` (top-level) / `bundle.android.versionCode = 24` | ✅ |
| `src-tauri/gen/android/app/build.gradle.kts` | `versionCode = 27` ❌ → fixed to `24` ✅ | Was drifted from both `tauri.conf.json`'s committed value and README's documented baseline convention |
| `Sidebar.svelte` displayed version | Dynamic via `getVersion()` | ✅ Reads the running binary's `tauri.conf.json` at runtime — will correctly show `v1.40.2` |

**Why the Android drift is low-risk but still worth fixing:** `build-android.yml` does `rm -rf src-tauri/gen/android && cargo tauri android init --ci` on every run, fully regenerating the Android project from `tauri.conf.json` before building — so the committed `build.gradle.kts` value is never actually used by CI. It matters only for a developer reading the committed file directly (e.g. building locally without regenerating), where a mismatched `versionCode` is misleading. Fixed to match `tauri.conf.json`'s committed baseline of `24`.

---

## 3. Recent Commits — 20 Most Recent on `master`

| SHA | Message | Notes |
|---|---|---|
| `c580276` | Merge PR #115 — Android CI fix + masking-model hardening | **v1.40.2**, current HEAD |
| `1e60f02` | stabilize: Android CI fix + WP-55 masking-model hardening | v1.40.1 → v1.40.2 |
| `da57737` | Merge PR #114 — Phase F security & quality review pass | v1.40.1 |
| `e6f036f` | hardening: Phase F security & quality review pass | v1.40.0 → v1.40.1 |
| `65fe7da` | Merge PR #113 — Phase F combined release | v1.40.0 |
| `e6a46ab` | feat: WP-56–65 — Phase F combined release: AI, lab map, analytics, cloud backup, compliance exports, plugins, PWA, performance, taxon re-anchoring, a11y | v1.39.2 → v1.40.0, **10 work packets in one release** |
| `da42e68` | Merge PR #112 — critical fixes following WP-50–55 self-review | v1.39.2 |
| `a9634a4` | fix: critical bug fixes following WP-50–55 self-review | v1.39.1 → v1.39.2, fixed a real genomic-fingerprint corruption bug |
| `966fe69` / `a7781f8` | Merge PR #111 / #110 — iOS hardening, notifications/sensors/permissions | v1.39.0 → v1.39.1 |
| `b5ee29f` | fix: WP-53 iOS CI hardening — env var bug, credential-gated fallback | v1.39.0 → v1.39.1 |
| `7481e5b` | feat: WP-52/53/54/55 — notifications, iOS scaffold, sensors, field permissions | v1.38.0 → v1.39.0 |
| `e0bc49c` / `ae175b9` | Merge PR #109 / WP-50 & WP-51 — multi-user backend + LAN sync foundation | v1.37.1 → v1.38.0 |
| `30be126` / `bf9750f` | Merge PR #108 — ROADMAP v1.37.1 review, Phase F expansion | Docs-only |
| `cf703fb` / `a479232` | Merge PR #107 — ROADMAP comprehensive review & Phase F expansion | v1.37.0 → v1.37.1 |
| `b445a22` / `c7a44ed` | Merge PR #106 — WP-49 custom taxa & Darwin Core export | v1.36.0 → v1.37.0, **Phase TX-3 complete** |

**Assessment:** Development velocity is very high and disciplined — every feature release is followed within the same day by a dedicated self-review/hardening patch (v1.39.0→v1.39.1→v1.39.2, v1.40.0→v1.40.1→v1.40.2), and each of those hardening passes found and fixed *real* bugs (a data-corruption bug in WP-55, a CI-breaking version-pin issue, cache-invalidation gaps). All work lands via PR with green CI required. Ten work packets (WP-56–65) shipped together in v1.40.0 as "Phase F," and both Phase TX-3 and Phase F are now marked complete in ROADMAP.md. This is a mature, self-critical release cadence — the main gap is that the three most recent releases (v1.40.0/1/2) outran the top-level docs' version banners even though the *content* was mostly kept current.

---

## 4. Codebase Layout

```
/SteloPTC
├── .github/workflows/       test.yml · build-windows.yml · build-android.yml · build-ios.yml · benchmarks.yml
├── docs/                    merkle-checkpoints.md · merkle-proofs.md · plugin-authoring.md · regulatory-exports.md · vocabulary-system.md
├── src/                     Svelte 5 + TypeScript frontend
│   └── lib/
│       ├── components/      47 .svelte files — SpecimenDetail.svelte largest (126.7 KB)
│       ├── offlineQueue.ts  WP-62 PWA mutation queue
│       ├── api.ts, profile.ts, utils.ts, exportUtils.ts, importUtils.ts, printUtils.ts
├── src-tauri/                Tauri 2 + Rust backend
│   └── src/
│       ├── ai/               WP-56 — Ollama HTTP client
│       ├── auth/
│       ├── cloud/             WP-59 — Argon2id + AES-256-GCM cloud backup/sync
│       ├── commands/          36 modules
│       ├── compliance_export/ WP-60 — FDA/USDA/CITES export bundles
│       ├── plugins/           WP-61 — manifest/loader
│       ├── db/                14 modules: queries, migrations (45), dashboard, analytics, sensors,
│       │                      notifications, permissions, sync, postgres, work_queue, vocabulary…
│       └── models/            23 modules
├── ROADMAP.md                ~1,300 lines — fixed this session (was 1 patch stale, self-contradictory)
├── CHANGELOG.md               1,958 lines — current, no changes needed
├── README.md                 785 lines — fixed this session (test counts, migration count, stale planning section)
├── UserManual.md              385 lines — header/§18 fixed; body still Phase-E-era (see §8)
└── DailyClaudeRoutineCheckup.md   This file
```

**Schema:** 45 migrations (verified via `migrations.rs`), unchanged since v1.40.0 (v1.40.1/v1.40.2 were both no-schema-change hardening passes).

---

## 5. CI / CD Health

| Pipeline | Latest run (HEAD `c580276`) | Recent history | Notes |
|---|---|---|---|
| `test.yml` | ✅ success | 3/3 green on last 3 master pushes | version-sync + svelte-check + rust-clippy (lint), frontend-tests, rust-tests |
| `build-windows.yml` | ✅ success | 3/3 green | Signed `.msi` |
| `build-android.yml` | ✅ success | 2/3 green — 1 failure on the v1.40.1 commit | The failure **is** the bug v1.40.2 fixed (unpinned Tauri CLI drifting ahead of the linked runtime crate); already resolved on HEAD |
| `build-ios.yml` | ❌ failure | 0/3 — all historical runs failed | Expected: no Apple Developer credentials in this repo; scoped to `workflow_dispatch`/weekly/`release` only since v1.39.1, not a push/PR gate |
| `benchmarks.yml` | ✅ success | 3/3 green | Non-blocking Criterion canary |

**No active CI problem.** The only red workflow (`build-ios.yml`) is a known, extensively disclosed limitation (no macOS/Xcode/Apple Developer access available anywhere this repo builds), not a regression to chase.

---

## 6. Test Coverage

### Verified directly this session

| Command | Result | Matches CHANGELOG claim? |
|---|---|---|
| `cargo test --lib --no-default-features` | **467 passed, 0 failed** | ✅ Exact match |
| `cargo clippy --no-default-features --lib -- -D warnings` | Clean | ✅ |
| `npm test` (Vitest) | **104 passed, 0 failed**, 5 files | ✅ Exact match |
| `npm run check` (svelte-check) | **0 errors, 0 warnings**, 406 files | ✅ Exact match |
| `npm run build` | Succeeds | New data point — 1 chunk-size warning (see §1) |

**Could not verify locally:** the default `tauri-commands` feature Rust build (which CHANGELOG puts at ~499–503 tests) requires GTK/WebKit system libraries not installed in this sandboxed container (`cargo check --features postgres`/default build fails on a missing `gdk-3.0.pc`). This is an environment limitation, not a code problem — all three CI workflows that *do* have those libraries are green on HEAD, so the default-feature build is verified there.

### Known gaps (carried over, unchanged this session)
- Zero Svelte component tests (only utility/store/queue-level Vitest coverage)
- No end-to-end integration tests (create → split → death → audit → export → import)
- No ER diagram / schema reference doc despite 45 migrations and 30+ tables
- Several Phase F modules (`cloud/`, `compliance_export/`, `plugins/`, `ai/ollama.rs`) have unit-level coverage per CHANGELOG's stated counts but no integration-level test against a real Ollama/PostgreSQL/cloud-storage endpoint — all explicitly disclosed as untestable in this environment, not silently skipped

---

## 7. Dependencies

### Frontend — found and fixed 8 of 9 vulnerabilities this session

Ran `npm audit`: **9 vulnerabilities (5 moderate, 4 high)** in `devalue`, `picomatch`, `postcss`, `rollup`, `svelte`, and the `vite`/`@sveltejs/vite-plugin-svelte` chain. Ran `npm audit fix --legacy-peer-deps` (non-breaking): resolved **8 of 9**, touching only `package-lock.json` (154 insertions / 136 deletions, no `package.json` version-range changes). Re-verified `npm run check`, `npm test`, and `npm run build` all still pass after the fix.

**Remaining: `xlsx` (SheetJS community package on npm) — HIGH severity, no fix available.**
- Prototype Pollution ([GHSA-4r6h-8v6p-xvw6](https://github.com/advisories/GHSA-4r6h-8v6p-xvw6)) and ReDoS ([GHSA-5pgg-2g8v-p4x9](https://github.com/advisories/GHSA-5pgg-2g8v-p4x9))
- This is a real, live known-issue in the npm-published `xlsx` package (SheetJS stopped patching the npm registry copy some time ago; their patched builds are only distributed via their own CDN). **This matters here** because the app's **Import Data** feature parses user-supplied `.xlsx` files with this exact library — it's not a dev-only or unreachable dependency. See §8 recommendation #2.

### Frontend — other observations
`npm outdated` shows several devDependencies one or more majors behind (`vite` 6→8, `vitest` 3→4, `typescript` 5→6, `jsdom` 26→29, `@sveltejs/vite-plugin-svelte` 4→7). All are intentionally pinned to the current major for the Svelte 5 stack; no urgent action needed, just tracked as normal maintenance debt.

### Backend (`src-tauri/Cargo.toml`)
`tauri` is pinned at `2.11.3`, matching the CI's now-pinned CLI version (`--version 2.11.3`) — this exact-match pin is the substance of the v1.40.2 Android CI fix. `rusqlite` 0.32.1, `bcrypt` 0.17.1, `argon2` 0.5.3, `aes-gcm` 0.10.3, `ed25519-dalek` 2.2.0, `zip` 2.4.2, `lettre` 0.11.22 all current and reasonable. `Cargo.lock` carries duplicate major versions for `rand` (0.8/0.9), `thiserror` (1/2), and `base64` (0.21/0.22) via transitive deps — cosmetic, not a functional issue.

---

## 8. Top Actionable Recommendations

### 1. Expand `UserManual.md` to cover Phase F (highest priority — user-facing doc gap)
Sections 1–17 stop at Phase E (v1.32.0). None of the following have any how-to documentation for an end user: lab profile-specific workflows beyond the core PTC flow, notifications/SMTP setup, environmental sensor logging, the AI Assist note/photo features, the interactive lab map, the analytics dashboard, cloud backup configuration, regulatory compliance export wizard, the plugin manager, or the installable PWA. This session fixed the header and the "Future Features" section (§18) so the manual is no longer self-contradictory, but writing 8+ new how-to sections accurately requires hands-on verification against the running app rather than being generated from source alone — scope it as a dedicated session.

### 2. Resolve the `xlsx` package's unpatched prototype-pollution/ReDoS vulnerabilities (security, medium-high priority)
The **Import Data** feature accepts user-supplied `.xlsx` files parsed by the vulnerable, unfixable `xlsx` npm package. Options: (a) migrate to SheetJS's own CDN-hosted patched build (breaking distribution change, evaluate license/CI implications of a non-npm-registry dependency), (b) switch to an actively maintained alternative parser, or (c) if staying on `xlsx`, add explicit file-size/complexity limits on import as a mitigating control and document the accepted risk. This has been "stable, no CVEs" in every prior checkup — that's no longer true and should be corrected going forward.

### 3. Extract `SpecimenDetail.svelte` (126.7 KB) into smaller components (unchanged from last 2 checkups)
Colonization chart, fruiting records, genetic lineage card, environmental readings, AI Assist panel, and the BSL badge are all self-contained candidates. This file has grown, not shrunk, across the last several checkups as more Phase E/F panels were added to it.

### 4. Code-split the production frontend bundle (new finding)
`npm run build` emits a single 1.48 MB (454 KB gzip) JS chunk — Vite's own build output flags this. Given the app now has 15+ major feature areas (analytics, lab map, cloud backup, compliance export wizard, plugin manager, AI assist, etc.), route/component-level dynamic `import()` for the less-frequently-used panels (Analytics, Lab Map, Compliance Export Wizard, Plugin Manager) would meaningfully cut initial load size.

### 5. Add command-layer integration tests for the newest Phase F modules (medium priority, carried over)
`ai/ollama.rs`, `cloud/`, `compliance_export/`, and `plugins/` all have real unit-test coverage per CHANGELOG's counts, but none are exercised at the command layer end-to-end. Combine with recommendation #1 — writing the manual sections and adding these tests both require the same kind of hands-on verification pass.

**Items resolved this session:**
- ✅ Fixed stale Android `versionCode` in the committed `build.gradle.kts` (27 → 24)
- ✅ Fixed 8 of 9 npm vulnerabilities via `npm audit fix --legacy-peer-deps`, verified no regressions
- ✅ Fixed ROADMAP.md's stale version banner + closing summary (was `v1.40.1`, self-contradicted its own body text) and an unrelated unclosed-parenthesis typo caught mid-edit
- ✅ Fixed README.md's stale test/migration counts and rewrote the self-contradictory legacy "planned work" section
- ✅ Fixed UserManual.md's header (was `v1.32.0`, 8 minor versions stale) and rewrote §18 to stop describing shipped features as future work

---

## 9. Security Posture

| Control | Status | Notes |
|---|---|---|
| CSP | ✅ Locked | `script-src 'self'`, no `unsafe-eval`; verified directly in `tauri.conf.json` |
| Authentication | ✅ Strong | bcrypt, session tokens, RBAC, forced first-login password change |
| Audit trail | ✅ Immutable + verifiable | SHA-256 hash chain, Merkle checkpoints |
| Field-level permission masking | ✅ Hardened in v1.40.2 | New `MASKABLE_FIELDS` registry + tripwire test + `set_field_permission` guard closes the one path a maskable field could gain a read path without a matching mask call |
| SMTP credentials | ⚠️ Known, disclosed | Still plaintext in the live DB (no OS-keychain integration); redacted from both local and cloud backups; explicitly documented in Settings UI |
| npm supply chain | ⚠️ 1 unfixable issue | `xlsx` — see §7/§8 recommendation #2 |
| Cloud backup encryption | ✅ Strong | Argon2id (128 MiB/3/4-way) + AES-256-GCM, authenticate-before-decrypt on restore |

---

## 10. Roadmap Progress

| Phase | Scope | Status |
|---|---|---|
| Trust Layer Phase 1, Phase C, Phase TX-1, Phase TX-2 | — | ✅ Complete |
| Phase D — Cell Culture (WP-30–34) | — | ✅ Complete (v1.23.0–v1.27.0) |
| Phase E — Mycology (WP-40–44) | — | ✅ Complete (v1.28.0–v1.32.0) |
| Phase TX-3 (WP-45–49) | Full taxonomic hash chain, cross-domain, breeding programs | ✅ Complete (v1.33.0–v1.37.0) |
| Phase F (WP-50–65) | Multi-user/LAN sync, notifications, iOS, sensors, permissions, AI, lab map, analytics, cloud backup, compliance exports, plugins, PWA, performance, taxon re-anchoring, a11y | ✅ Complete (v1.38.0–v1.40.0), hardened through v1.40.2 |
| Phase G, Trust Layer Phase 2/3, regulatory submission pipeline | Federated networks, on-chain anchoring, signed transactions | Reserved, not started (v2.x+) |

---

## 11. Summary Scorecard

| Dimension | Score | Delta | Notes |
|---|---|---|---|
| Version alignment | ✅ 10/10 | → | All three manifests at 1.40.2; Android baseline drift found & fixed |
| Code organization | ⚠️ 6/10 | → | `SpecimenDetail.svelte` still 126.7 KB, unchanged; new bundle-size finding |
| Security posture | ✅ 9/10 | → | Strong overall; one unfixable npm supply-chain issue newly surfaced |
| Test coverage | ✅ 9/10 | → | 467 Rust + 104 Vitest, all verified locally and matching claims exactly |
| Performance | ✅ 9/10 | ↓ | Indexes/caching all intact; new bundle-size finding not previously measured |
| Documentation | ⚠️ 7/10 | ↑ | Was internally self-contradictory in 2 of 3 docs before this session; now congruent, though UserManual body still needs a Phase F content pass |
| CI/CD | ✅ 10/10 | → | All merge-gating workflows green; the one historical failure was already fixed |
| Dependency health | ⚠️ 7/10 | ↑ | 8/9 npm vulnerabilities fixed this session; 1 real, disclosed, unfixable issue remains |
| Development velocity | ✅ 10/10 | → | Three patch releases in one day, each a genuine self-review hardening pass |
| Roadmap clarity | ✅ 9/10 | ↑ | Version banner now matches shipped code; Phase F fully marked complete |

**Verdict:** The engineering itself is in excellent shape — disciplined releases, real self-review hardening passes, green CI, and test claims that check out exactly against a from-scratch local run. This session's contribution was closing a documentation-congruence gap that had been quietly widening: ROADMAP.md and README.md were about one patch release stale and (in README's case) internally contradicted themselves; UserManual.md was badly stale and also self-contradictory. All three are now consistent with the shipped v1.40.2 code and with each other. Also fixed a real stale Android version-baseline value and closed 8 of 9 known npm vulnerabilities with a verified-safe dependency update. **Next priorities:** (1) a dedicated UserManual.md content pass for Phase F features, (2) resolve or mitigate the `xlsx` supply-chain issue given it's reachable via user-supplied file import, (3) `SpecimenDetail.svelte` extraction, (4) frontend code-splitting.
