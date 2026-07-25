# SteloPTC — Claude Routine Checkup

| | |
|---|---|
| **Date** | 2026-07-25 |
| **Branch reviewed** | `master` @ `cae38c1` (PR #128) · session work on `claude/docs-cleanup-beautify-es26z3` |
| **Version reviewed** | **v1.53.1** — aligned across `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `build.gradle.kts` ✅ |
| **Released as** | **v1.53.2** — this session's fixes, `versionCode` 27 → 28 |
| **Previous checkup** | 2026-07-02 (v1.40.2) — 23 days and 13 releases ago |
| **Reviewed by** | Claude (automated routine) |

> **Headline: `master` has been red since 2026-07-19 and nobody noticed.** Every merge-gating
> workflow (`Tests`, `Build Windows`, `Build Android`) has failed on the current `master` HEAD for
> six days. The cause is a **one-line type error** in code that the fast local test command cannot
> see. It is found, fixed, and verified in this session — see §2.

---

## 1. Executive status

| Area | Status | Notes |
|---|---|---|
| **CI on `master`** | 🔴 **Broken — found and fixed this session** | `Tests`, `Build Windows`, `Build Android` all failing on `cae38c1` since 2026-07-19. Root cause: `commands/subcultures.rs:311` passed an `i32` where `signed_ledger::lifecycle::passage` takes an `i64`. Fixed and verified with a real full-feature build (§2). |
| **Why it went undetected** | 🔴 **Process gap — now documented** | The broken code lives behind the `tauri-commands` feature. `cargo test --lib --no-default-features` (the command every prior checkup used) never compiles it, so all four "local gates" were green on a tree CI could not build. Written up in `SKILLS.md` §3 and §7 so it can't recur silently. |
| Version alignment | ✅ All four manifests aligned | Reviewed at 1.53.1 (`versionCode = 27`); released here as 1.53.2 (`versionCode = 28`), all four bumped together |
| `package-lock.json` version drift | ⚠️ **Found and fixed** | Its `version` field still read `1.48.0` — five releases stale. It only refreshes on `npm install`, so version bumps since v1.48.0 never touched it. Corrected, and now regenerated as part of the 1.53.2 bump. |
| Test suite (fast) | ✅ 642/642 | `cargo test --lib --no-default-features`, fresh run, 44s |
| Test suite (**full CI gate**) | ✅ **679/679 — verified locally for the first time** | `cargo test --lib` with the `tauri-commands` feature, 51s. Previous checkups all recorded this as unverifiable; the GTK/WebKit libraries install fine on this image (§6). |
| Clippy | ✅ Clean, both feature sets | `--no-default-features` and full, `-D warnings` |
| Frontend tests | ✅ 113/113 | Vitest, 5 files |
| `svelte-check` | ✅ 0 errors / 0 warnings | 418 files |
| Production build | ✅ Succeeds | 9.0s; bundle size noted below |
| npm vulnerabilities | ⚠️ **2 of 4 fixed this session** | `fast-uri` and `postcss` (both high) resolved by `npm audit fix` — lockfile only, `package.json` untouched. `xlsx` and a dev-only `brace-expansion` chain remain (§7). |
| npm in-range drift | ⚠️ **Found and fixed** | 4 packages had fallen behind their own declared semver range. `npm update` applied; all gates re-run green after. |
| Open PRs / stale branches | ✅ None | `master` is the only remote branch |
| README freshness | ✅ Current, improved | Counts and version correct; the doc index was missing three specs — added |
| ROADMAP freshness | ⚠️ **Accurate but unreadable — rewritten** | Content was correct; the header was a ~7,000-word unbroken paragraph. Restructured (§3). |
| UserManual freshness | 🔴 **Stale by 8 releases — fixed** | Header claimed **v1.45.0**; nothing from Phase G (WP-71/72) or Phase H (WP-74–78) was documented for end users at all. Six new sections written (§3). |
| SteloPTC.md freshness | ⚠️ **Stale by 5 releases — fixed** | Frontmatter read `version: 1.48.0`, `tests_rust: 608`, `migrations: 51` |
| `SKILLS.md` accuracy | ⚠️ **One real error — fixed** | §9 told contributors to write "migration `052`" — 052 is already shipped; next is 053 |
| Large-component debt | ⚠️ Unchanged, marginally worse | `SpecimenDetail.svelte` now 2,703 lines / 128.5 KB (was 2,667 / 126.7 KB at the last checkup) |
| Bundle size | ⚠️ Unchanged in kind, larger | One 1.55 MB (476 KB gzip) chunk, up from 1.48 MB / 454 KB. Still no code-splitting. |
| Dead code | ✅ None found | All 55 Svelte components are referenced; zero `TODO`/`FIXME`/`HACK` markers in `src` or `src-tauri/src` |
| Doc link integrity | ✅ Verified | Every relative link and every in-document anchor across all 20 Markdown files resolves |

**Overall health: GOOD, with one serious process finding.** The engineering fundamentals are strong
— 679 tests, clean clippy, no dead code, no `TODO` debt, honest disclosure of every foundation-only
capability. But the project shipped a release that does not compile, merged it, and left `master`
red for six days, because the routinely-run local verification could not see the broken code. The
fix is one line; the *lesson* is the valuable part, and it is now written into the contributor
playbook rather than only into this report.

---

## 2. The CI break — root cause and fix

**Symptom.** On `master` HEAD `cae38c1` (PR #128, "Phase H (WP-74–78) … v1.53.1", merged
2026-07-19): `Tests` ❌, `Build Windows` ❌, `Build Android` ❌, `Build iOS` ❌ (scheduled run,
2026-07-20). Only `Benchmarks` ✅. The preceding commit `8c60544` was fully green, so the break was
introduced by PR #128 itself.

**Root cause.** From the `Lint` job log:

```
error[E0308]: mismatched types
   --> src/commands/subcultures.rs:311:72
    |
311 |         crate::signed_ledger::lifecycle::passage(&request.specimen_id, passage_number, "passage");
    |                                                                        ^^^^^^^^^^^^^^ expected `i64`, found `i32`
```

`passage_number` derives from `subculture_count`, read from SQLite as `i32`.
`signed_ledger::lifecycle::passage` — added by WP-75 in the same release — declares `i64`.

**Why every local gate passed.** `commands/` is compiled only under the default `tauri-commands`
feature. The four gates in `SKILLS.md` §3 that a sandboxed session can normally run
(`--no-default-features` test + clippy, `npm test`, `npm run check`) all skip it entirely. The
release was verified honestly and thoroughly against gates that structurally could not catch this.

**Fix.**

```rust
crate::signed_ledger::lifecycle::passage(&request.specimen_id, i64::from(passage_number), "passage");
```

**Verification — the real CI gate, run locally.** The GTK/WebKit dependency that blocked every
previous checkup turned out to be installable here (`apt-get install libgtk-3-dev
libwebkit2gtk-4.1-dev librsvg2-dev libssl-dev`). With the fix applied:

| Gate | Result |
|---|---|
| `cargo clippy --lib -- -D warnings` (full features) | ✅ clean |
| `cargo test --lib` (full features) | ✅ **679 passed, 0 failed** (677 before this session's two new tests) |
| `cargo test --lib --no-default-features` | ✅ 642 passed, 0 failed |
| `npm test` · `npm run check` · `npm run build` | ✅ 113 passed · 0/0 (418 files) · builds |

**Prevention.** `SKILLS.md` §3 now carries the exact `apt-get` line and the standing rule: *run the
full-feature build before pushing anything that touches `src-tauri/src/commands/`*. §7 records the
integer-width trap that produced this specific error.

**A second, quieter defect found alongside it — signed lifecycle coverage.** Reviewing the same
module surfaced two gaps the WP-75 disclosure had papered over. Both were flagged for the user's
call and then fixed on their instruction:

- **`record_specimen_death` never signed anything.** `lifecycle::passage` already accepted a
  `"death"` event type and mapped it to `SPECIMEN_DIED`, but no call site ever passed it. A
  specimen's terminal event — the one an auditor is most likely to want attributed to a person —
  produced an audit-chain entry and no signed record. It now appends **two** events: the death and
  the archival it causes, since a verifier may need to check those independently.
- **`SPECIMEN_ARCHIVED` was declared but unreachable.** The constant sat in `lifecycle::ALL` with
  no call site anywhere. `delete_specimen` (which archives rather than hard-deletes) and
  `bulk_archive_specimens` now emit it. `split_specimen` deliberately does not — `SPECIMEN_SPLIT`
  already records the parent forking, and a second event would be noise.

This is the more interesting of the two classes of bug in this session. A missing event type is
invisible to every test and every gate: the ledger *looks* complete, verifies cleanly, and is
simply missing facts. A new tripwire test, `every_declared_event_type_has_a_payload_builder`, now
fails if a type is added to `ALL` without a way to emit it.

---

## 3. Documentation pass

The user-visible docs were accurate in substance but had drifted badly in currency and readability.

| Doc | What was wrong | What was done |
|---|---|---|
| `ROADMAP.md` | The header was seven unbroken paragraphs — a single one of them ~7,000 words — duplicating the *Status at a glance* table directly beneath it. Genuinely unreadable. | Replaced with a scannable fact table, a goal statement, and a security baseline. The per-migration history became a **52-row table** in a collapsible section (nothing dropped); the release narrative became a second collapsible bullet list. Added the missing v1.48 (WP-73) and v1.53.1 rows to *Status at a glance*. |
| `UserManual.md` | Header said **v1.45.0**. Phase G (taxonomy registry, breeding coordination) and *all* of Phase H were undocumented for end users. §18 still listed Phase G as "reserved, not started". | New header + linked TOC. Six new sections (**27–32**): on-chain anchoring & the signed ledger, working with partner labs, compliance flags/rules/waivers, the submission pipeline, the data-integrity self-check, and the mycology Fruiting overview. §18 corrected — shipped items moved out of "planned", real remaining gaps listed honestly. |
| `SteloPTC.md` | Frontmatter `version: 1.48.0`, `tests_rust: 608`, `migrations: 51`, `updated: 2026-07-11`. | Refreshed to 1.53.2 / 642 (+679 full) / 52 / 2026-07-25. Phase H added to the feature catalog, release timeline, and foundation-only checklist. Fixed a wrong cross-reference (cryo pointed at *UserManual §22*, which is cloud backup). |
| `README.md` | Doc index omitted `specimen-passport`, `taxonomy-registry`, `breeding-coordination` and `SKILLS.md`. | Added, plus a link to the new spec index and the full-feature test figure. |
| `docs/*.md` (11 files) | Three different header conventions; three files had no metadata at all. | Uniform header on every spec: subtitle, a *Work packet · Shipped in · Status · Depends on* table, and a nav line. Body content untouched (verified by diff: 121 insertions, 7 deletions, all in headers). |
| `docs/README.md` | Didn't exist — `docs/` had no index. | New specification index grouping all 11 specs by purpose. |
| `CHANGELOG.md` | No orientation for a 2,400-line append-only file. | Short reading guide at the top. History itself untouched. |
| `SKILLS.md` | §9 said to write migration `052` — already shipped. Test baseline lacked the full-feature figure. | Corrected. Added the full-feature verification procedure (§3), the integer-width trap (§7), and a new **§10 "Docs that drift"** — a table of every file carrying a number that goes stale, with a copy-pasteable check. |

**On the `SKILLS.md` request:** the file already existed as `skills.md` and already served exactly
that purpose, so it was strengthened rather than duplicated — then **renamed to `SKILLS.md`** on the
user's instruction. Every live reference was updated (`README.md`, `ROADMAP.md`, `SteloPTC.md`
wikilinks, `docs/README.md`, and three source comments). CHANGELOG entries at v1.53.1 and earlier
still say `skills.md`: that file is append-only and shipped history is not rewritten, so the rename
is recorded in a note at the top of `SKILLS.md` instead.

---

## 4. Version consistency

| File | Value | Status |
|---|---|---|
| `package.json` | `1.53.1` → `1.53.2` | ✅ |
| `src-tauri/Cargo.toml` | `1.53.1` → `1.53.2` | ✅ |
| `src-tauri/tauri.conf.json` | `1.53.1` → `1.53.2` · `versionCode` 27 → 28 | ✅ |
| `src-tauri/gen/android/app/build.gradle.kts` | `versionName` → `"1.53.2"` · `versionCode` 27 → 28 | ✅ Matches |
| `package-lock.json` | `1.48.0` → **`1.53.2`** | ⚠️→✅ Fixed this session |

The `package-lock.json` drift is worth remembering: its `version` field is rewritten only by
`npm install`, so a release that bumps the three manifests and doesn't reinstall leaves it behind
silently. It has now been added to the `SKILLS.md` §10 drift checklist.

---

## 5. CI / CD health

Verified live against the GitHub Actions API — not carried over from the previous report.

| Pipeline | On `master` `cae38c1` | Previous green | Notes |
|---|---|---|---|
| `test.yml` | 🔴 failure | `8c60544` | Both the `Rust (cargo test)` and `Lint` jobs — same compile error |
| `build-windows.yml` | 🔴 failure | `8c60544` | Consistent with the same root cause (builds the same lib) |
| `build-android.yml` | 🔴 failure | `8c60544` | Same |
| `build-ios.yml` | 🔴 failure (scheduled, 2026-07-20) | `8c60544` (2026-07-13) | Notable: iOS *had* gone green on the weekly schedule, its first-ever success. It regressed with the same commit. |
| `benchmarks.yml` | ✅ success | — | Non-blocking Criterion canary; compiles benches, not the command layer |

**No open PRs. No stale branches** — `master` is the only remote branch. The fix in this session is
expected to return all five to green; that will be confirmed by the run triggered on push.

---

## 6. Test coverage

Every figure below is from a fresh run in this session.

| Command | Result |
|---|---|
| `cargo test --lib --no-default-features` | **642 passed**, 0 failed |
| `cargo test --lib` *(full `tauri-commands` — the CI gate)* | **679 passed**, 0 failed |
| `cargo clippy --lib --no-default-features -- -D warnings` | clean |
| `cargo clippy --lib -- -D warnings` | clean |
| `npm test` (Vitest) | **113 passed**, 0 failed, 5 files |
| `npm run check` (svelte-check) | **0 errors, 0 warnings**, 418 files |
| `npm run build` | succeeds, 9.0s |

**The "can't verify locally" caveat is retired.** Every previous checkup recorded the full-feature
build as impossible in a sandbox. It is not — the GTK/WebKit packages install cleanly on this image.
Given that this is precisely the gap that let a broken `master` ship, the install step is now part
of the documented workflow rather than a footnote here.

### Known gaps (carried over)

- **Zero Svelte component tests.** `@testing-library/svelte` is a devDependency and remains unused.
  The 113 TS tests cover pure utilities only. This is now the largest single coverage gap.
- No end-to-end integration test (create → split → death → audit → export → import).
- No ER diagram or schema reference doc, despite 52 migrations and 30+ tables.
- Command-layer coverage for `cloud/`, `compliance_export/`, `plugins/`, `ai/ollama.rs` is
  unit-level only.

---

## 7. Dependencies

### Frontend

`npm audit` at session start: **4 high**, three distinct causes. After `npm audit fix`
(lockfile-only — `package.json` untouched):

| Advisory | Before | After |
|---|---|---|
| `fast-uri` — host confusion ([GHSA-v2hh-gcrm-f6hx](https://github.com/advisories/GHSA-v2hh-gcrm-f6hx)) | high | ✅ fixed (3.1.3 → 3.1.4) |
| `postcss` — path traversal ([GHSA-r28c-9q8g-f849](https://github.com/advisories/GHSA-r28c-9q8g-f849)) | high | ✅ fixed |
| `brace-expansion` — DoS ([GHSA-mh99-v99m-4gvg](https://github.com/advisories/GHSA-mh99-v99m-4gvg)) | high | ⚠️ top-level fixed (5.0.7 → 5.0.8); a **nested** copy under `filelist` remains |
| `xlsx` — prototype pollution + ReDoS | high | ⚠️ unchanged, still no upstream fix |

**Read the post-fix count carefully.** `npm audit` now reports **9 high**, up from 4. This is a
reporting artifact, not a regression: npm collapses a whole dependency chain into one finding while
a non-breaking fix exists, and enumerates every link once only a `--force` path remains. The nine
are the same two real problems.

1. **`xlsx` (SheetJS) — genuinely unfixable, and reachable from user input.** Parses user-supplied
   `.xlsx` files via **Import Data**. No upstream patch exists. Options unchanged: migrate to
   SheetJS's own CDN-hosted patched build, switch parsers, or add explicit file-size/complexity
   limits on import as a mitigating control. **Carried over from three consecutive checkups.**
2. **Nested `brace-expansion` — dev-only, and the offered "fix" is a downgrade.** The chain is
   `vite-plugin-pwa → workbox-build → @trickfilm400/rollup-plugin-off-main-thread → ejs → jake →
   filelist → minimatch → brace-expansion`. Every link is a build-time devDependency; none reaches
   shipped runtime code. `npm audit fix --force` would install `vite-plugin-pwa@1.2.0` — a
   *downgrade* from the pinned 1.3.0 — so it was **not** applied.

**In-range drift fixed.** Four packages lagged behind what their own `package.json` ranges already
allow — ordinary lockfile staleness. `npm update` brought them current
(`@tauri-apps/plugin-dialog` 2.7.1→2.7.2, `svelte` 5.56.4→5.56.8, `svelte-check` 4.7.1→4.7.3,
`vitest` 3.2.6→3.2.7); all gates re-run green afterwards.

Still one or more majors behind their absolute latest, intentionally pinned to the Svelte 5 stack:
`vite` 6→8, `vitest` 3→4, `typescript` 5→7, `jsdom` 26→29, `@sveltejs/vite-plugin-svelte` 4→7,
`@testing-library/jest-dom` 6→7. Normal maintenance debt, no action needed.

### Backend

No `Cargo.toml` / `Cargo.lock` changes needed. `tauri` stays pinned at `2.11.3` to match the CI's
pinned CLI. `rusqlite` 0.32.1, `bcrypt` 0.17.1, `argon2` 0.5.3, `aes-gcm` 0.10.3, `ed25519-dalek`
2.2.0, `zip` 2.4.2, `lettre` 0.11.22 all current.

---

## 8. Codebase health

```
/SteloPTC
├── .github/workflows/   test.yml · build-windows.yml · build-android.yml · build-ios.yml · benchmarks.yml
├── docs/                11 specs + README.md (new spec index)
├── src/                 Svelte 5 + TypeScript
│   └── lib/components/  55 .svelte files — all referenced from App.svelte's view switch
├── src-tauri/src/       Rust — commands/ (36) · db/ (14) · models/ (23) · ai/ · auth/ · cloud/
│                        anchoring/ · signed_ledger/ · passport/ · registry/ · coordination/
│                        compliance_rules/ · integrity/ · monitoring/ · compliance_export/ · plugins/
└── *.md                 README · ROADMAP · UserManual · CHANGELOG · skills · SteloPTC · this file
```

- **Dead code: none.** All 55 components are referenced; no orphaned files.
- **Marker debt: none.** Zero `TODO` / `FIXME` / `XXX` / `HACK` across `src` and `src-tauri/src`.
- **Schema:** 52 migrations, 95 migration tests.
- **Largest files** (unchanged concerns): `db/queries.rs` 6,895 lines · `db/migrations.rs` 4,835 ·
  `SpecimenDetail.svelte` 2,703 · `SpecimenList.svelte` 1,348.

---

## 9. Security posture

| Control | Status | Notes |
|---|---|---|
| CSP | ✅ Locked | `script-src 'self'`, no `unsafe-eval` |
| Authentication | ✅ Strong | bcrypt, session tokens, RBAC; forced password change enforced **server-side** in `validate_session` since v1.48.0 |
| Audit trail | ✅ Immutable + verifiable | SHA-256 hash chain, Merkle checkpoints, on-chain anchoring |
| Signed ledger | ✅ Hardened | A missing registered key is a verification *failure* (key-substitution forgery closed, v1.48.0) |
| Federated imports | ✅ Atomic | Single transaction with rollback since v1.53.1 — a failed import can no longer leave an unrecoverable partial state |
| Data-integrity self-check | ✅ Shipped | WP-76 — admin-only, read-only scan for orphans, broken links, chain gaps |
| Field-level permission masking | ✅ Tripwire-tested | `MASKABLE_FIELDS` registry + guard on `set_field_permission` |
| SMTP credentials | ⚠️ Known, disclosed | Plaintext in the live DB; redacted from backups; caveat shown in the Settings UI |
| npm supply chain | ⚠️ 2 open | `xlsx` (unfixable, reachable from import) + a dev-only chain (§7) |
| Cloud backup encryption | ✅ Strong | Argon2id + AES-256-GCM, authenticate-before-decrypt |

---

## 10. Roadmap progress

| Phase | Scope | Status |
|---|---|---|
| A · B · C · TX-1 · TX-2 · TX-3 · D · E | Core product, trust layer, de-hardening, taxonomy, verticals | ✅ Complete (v0.1.20–v1.37.0) |
| F (WP-50–65, WP-56b) | Multi-user, notifications, sensors, permissions, AI, lab map, analytics, cloud backup, exports, plugins, PWA, perf, a11y | ✅ Complete (v1.38.0–v1.41.0) |
| Trust Layer 2/3 + submissions (WP-66–68) | On-chain anchoring, signed-event ledger, submission pipeline | ✅ Complete (v1.42.0–v1.44.0) |
| G (WP-70–72) | Federated passports, taxonomy registry, breeding coordination | ✅ Complete (v1.45.0–v1.47.0) |
| WP-73 | Domain-congruence & security hardening | ✅ Complete (v1.48.0) |
| H (WP-74–78) | Compliance rule engine, signed lifecycle events, integrity self-check, flag waivers, environmental monitoring | ✅ Complete (v1.49.0–v1.53.0) |
| Beyond v2.x | Automatic on-chain broadcast, remote API for the PWA, live S3/SFTP, plugin WASM sandbox, networked federated transport | ⏳ Not started |

---

## 11. Top actionable recommendations

### 1. Make the full-feature build a hard part of the workflow *(highest priority — new)*
The one-line break in §2 is not really a code problem; it is a verification-coverage problem, and it
cost six days of red `master`. `SKILLS.md` §3 now documents the `apt-get` line and the rule. The
durable fix is to stop treating `--no-default-features` as "the" local gate. If any single follow-up
lands from this checkup, make it this one.

### 2. Add a component-test harness *(carried over, now the largest coverage gap)*
`@testing-library/svelte` has been an unused devDependency across four consecutive checkups. Zero of
the 55 components have a test. Start with something small and branch-y — `Sidebar.svelte`'s
Tauri-vs-PWA version display, or `FormField.svelte`.

### 3. Resolve or mitigate `xlsx` *(carried over, unchanged, security)*
Reachable from user-supplied file input, no upstream fix, open across four checkups. If migration is
too large, ship the mitigating control (explicit size/complexity limits on import) rather than
carrying the item forward untouched again.

### 4. Extract `SpecimenDetail.svelte` *(carried over, now growing)*
2,703 lines / 128.5 KB — up from 2,667 / 126.7 KB at the last checkup, so it is no longer merely
stable-and-large. The colonization chart, fruiting records, genetic lineage card, environmental
readings, AI Assist panel, and BSL badge remain clean extraction candidates — and extraction would
produce exactly the testable sub-components recommendation #2 needs.

### 5. Code-split the frontend bundle *(carried over, growing)*
One 1.55 MB (476 KB gzip) chunk, up from 1.48 MB / 454 KB. `AnalyticsDashboard` and `LabMap` are
top-level routes in `App.svelte` and remain mechanical dynamic-`import()` swaps; the nested panels
(`ComplianceExportWizard`, `PluginManagerPanel`, `CloudBackupPanel`) still require touching their
parents' tab logic. Start with the two route-level cases.

### 6. Document the cryo / media / inventory workflows in the User Manual *(new, small)*
Media batches, supply inventory, and LN₂ cryostorage ship and are prominent in the README, but have
no `UserManual.md` section — `SteloPTC.md` was cross-referencing a section number that pointed at
unrelated content (fixed to point at the roadmap instead). A short section would close the loop.

**Resolved this session**

- ✅ Fixed the type error that had `master`'s CI red for six days — verified with the real
  full-feature build and clippy, both of which no prior checkup had ever been able to run
- ✅ Closed the signed-lifecycle-event gaps: a recorded death now signs `SPECIMEN_DIED` +
  `SPECIMEN_ARCHIVED`; explicit and bulk archives now sign `SPECIMEN_ARCHIVED` (previously a
  declared-but-unreachable event type). Two new tests, including a tripwire against the same class
  of gap recurring
- ✅ Renamed `skills.md` → `SKILLS.md` and updated every live reference
- ✅ Fixed `package-lock.json`'s five-release-stale version field
- ✅ Fixed 2 of 4 high-severity npm advisories; documented the other 2 honestly, including why the
  post-fix count *rose* to 9 and why `--force` was refused
- ✅ Brought 4 in-range-drifted packages current
- ✅ Restructured the unreadable ROADMAP header without losing a single fact
- ✅ Brought `UserManual.md` from v1.45.0 to v1.53.1 with six new end-user sections
- ✅ Refreshed `SteloPTC.md`, `README.md`, `CHANGELOG.md` intro, and all 11 `docs/*.md` headers
- ✅ Added `docs/README.md` (spec index) and `SKILLS.md` §10 (docs-drift checklist)
- ✅ Verified every relative link and in-document anchor across all 20 Markdown files

**Investigated, no action taken**

- `npm audit fix --force` — would downgrade `vite-plugin-pwa` 1.3.0 → 1.2.0 to fix a dev-only
  build-time advisory (§7)
- Signing `SPECIMEN_ARCHIVED` on the split path — `SPECIMEN_SPLIT` already records that the parent
  forked; a second event would be redundant, not more complete (§2)
- Rewriting historical `skills.md` references in `CHANGELOG.md` — that file is append-only (§3)

---

## 12. Scorecard

| Dimension | Score | Δ vs 2026-07-02 | Notes |
|---|---|---|---|
| Version alignment | ✅ 9/10 | ↓ | All four manifests correct, but `package-lock.json` had silently drifted five releases |
| CI/CD | 🔴 4/10 | ↓↓ | Every merge gate red for six days on `master`, undetected. Fixed here; the score reflects that it happened and wasn't caught. |
| Code organization | ⚠️ 6/10 | → | `SpecimenDetail.svelte` growing again; no dead code, no marker debt |
| Security posture | ✅ 9/10 | → | Strong; `xlsx` remains the one reachable open item |
| Test coverage | ✅ 9/10 | → | 679 + 113, all green and now verified against the *real* gate; zero component tests remains the gap |
| Performance | ✅ 8/10 | ↓ | Indexes and caching intact; bundle grew ~5% with still no code-splitting |
| Documentation | ✅ 9/10 | ↑↑ | Was accurate-but-stale (UserManual 8 releases behind) and partly unreadable; now current, consistent, and link-verified |
| Dependency health | ✅ 8/10 | → | 2 advisories closed, 2 disclosed with reasoning; 4 drift packages current |
| Development velocity | ✅ 9/10 | → | 13 releases in 23 days, Phases G and H both completed |
| Roadmap clarity | ✅ 10/10 | ↑ | Restructured; every phase and every foundation-only caveat is now findable in seconds |

**Verdict.** The product is in strong shape and the disclosure culture around it is genuinely good —
foundation-only features are labelled honestly everywhere, and the audit chain's guarantees are
specified well enough for an outside party to verify. The failure this session found is not a
quality failure so much as a **blind spot**: the team verified diligently against gates that
structurally could not see the command layer, and trusted green. That is fixed, and — more usefully
— written down where the next contributor will read it.

**Next priorities, in order:** (1) make the full-feature build routine, (2) add the first component
tests, (3) resolve or mitigate `xlsx`, (4) start extracting `SpecimenDetail.svelte`, (5) code-split
the two route-level components, (6) document the cryo/media/inventory workflows.
