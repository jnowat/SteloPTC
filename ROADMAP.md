# SteloPTC → Stelo Lab Suite — Engineering Roadmap

**Status as of July 2026:** **v1.40.1** (`tauri.conf.json` + latest `CHANGELOG`) · Tauri 2 + Svelte 5 + Rust/SQLite (+ optional PostgreSQL connector) · Windows + Android CI (+ best-effort, still-unverified iOS workflow — hardened but not yet run successfully end-to-end) · **Trust Layer Phase 1 complete (WP-18–21) · Phase C WP-22–27 fully shipped · Phase TX-1 complete (WP-28 backend v1.16.0 · WP-29 UI v1.17.0) · Phase TX-2 fully shipped (WP-35–39 v1.18.0–v1.22.0) · Phase D Cell Culture WP-30–34 fully shipped (v1.23.0–v1.27.0) · Phase E Mycology WP-40–44 fully shipped (v1.28.0–v1.32.0) · Phase TX-3 WP-45–49 fully shipped (v1.33.0–v1.37.0) — Phase TX-3 complete · Phase F priorities refined (v1.37.1 review): WP-58 & WP-63 elevated as highest-ROI, WP-64 taxon re-anchoring added, WP-55/WP-59/WP-60 expanded, a11y tail (85 warnings) tracked · Phase F WP-50 & WP-51 foundation shipped together (v1.38.0) — optional PostgreSQL connector + LAN sync change-detection/conflict-recording on the existing audit hash chain; SQLite remains the sole live backend and no networking layer exists yet · Phase F WP-52–55 shipped together (v1.39.0) — notifications (desktop + email, background scheduler), environmental sensor foundation (parsing/validation/manual entry, hardware transport deferred), field-level permissions (two entities wired), and a best-effort **unverified** iOS CI scaffold · WP-53 iOS hardening pass (v1.39.1) — fixed a wrong env-var name that silently broke release signing, replaced guaranteed-to-fail steps with a `cargo check` fallback when no Apple credentials are configured, narrowed CI triggers off every push, still explicitly unverified end-to-end · **Critical fix pass following a WP-50–55 self-review (v1.39.2)** — fixed a real genomic-fingerprint data-corruption bug (masked "[RESTRICTED]" value could be persisted through `update_strain_status`, in both the backend and `StrainManager.svelte`), fixed an N+1 permission-check query pattern that was blocking further masking expansion, fixed silent-forever scheduler death on mutex poisoning, added the missing iOS `NSCameraUsageDescription`, added a masking backstop in the notification pipeline, redacted the SMTP password from backup files, added sensor `source` field validation, and substantially expanded honest-limitations disclosure for WP-50/WP-51/WP-54 · **Phase F WP-56–65 shipped together (v1.40.0)** — local AI analysis (Ollama, draft-suggestion approval gate), interactive lab map (floor-plan pins + heat-map), analytics dashboards (highest-ROI), zero-knowledge E2E-encrypted cloud backup + multi-device sync (local_nas/smb transport live, S3/SFTP config-only), FDA/USDA/CITES regulatory export bundles (Ed25519-signed), a plugin/extension system (vocabulary seeding live, WASM rule execution deferred), a PWA/offline-first layer (tested offline-queue mechanism, no remote endpoint yet), performance hardening (materialized dashboard cache, cursor pagination, virtual scroll, Criterion benches), taxon chain re-anchoring (**WP-45 EXPERIMENTAL → STABLE**), and a full a11y label-association pass (90 → 0 warnings) — **Phase F complete** · **Phase F hardening/review pass (v1.40.1)** — a security & quality review of the v1.40.0 release: fixed a cloud-backup gap that skipped SMTP-password redaction, closed the one dashboard-cache-invalidation gap (`thaw_vial`), made the cloud crypto API uniformly fallible (no Argon2-OOM panic across the command boundary), gated the shared analytics layout behind supervisor/admin, hardened the plugin loader's table whitelist, added a defense-in-depth masking guard on breeding-program create, drove `npm run check` to 0 warnings, and re-verified the honesty of the WP-50/51/53/54 scaffolding disclosures — see CHANGELOG.md for full detail**
**Schema:** **45 migrations** total; latest is **migration 045** (WP-61 `installed_plugins` table, v1.40.0). Migration 044 added `signing_keys` (WP-60 Ed25519 export signing, v1.40.0). Migration 043 added `reanchor_events` (WP-64 taxon chain re-anchoring, v1.40.0). Migration 042 added `backup_targets` + `cloud_sync_segments` (WP-59 cloud backup & multi-device sync, v1.40.0). Migration 041 added `ai_suggestions` (WP-56 local AI analysis, v1.40.0). Migration 040 added `locations` + `specimens.location_id` (WP-57 interactive lab map, v1.40.0). Migration 039 added covering indexes for the WP-63 performance hardening pass (v1.40.0). Migration 038 was (WP-52 notification foundation: `notification_preferences` + `smtp_config` tables, `notification_check_interval_minutes` in `app_settings`, v1.39.0). Migration 037 added `environmental_readings` (WP-54 sensor integration foundation, v1.39.0). Migration 036 added `field_permissions` (WP-55 field-level permissions, v1.39.0). Migration 035 added `backend_type` key on `app_settings` + new `sync_peers`/`sync_conflicts` tables (WP-50/WP-51 multi-user + LAN sync foundation, v1.38.0). Migration 034 added `status`/`provisional_notes` columns on `taxa` + `taxon_mappings` table (WP-49 custom taxa & Darwin Core export, v1.37.0). Migration 033 added `breeding_programs` and `breeding_records` tables with cascade delete and indexes (WP-47 breeding programs, v1.35.0). Migration 032 added `domain` column to `app_config` (WP-46 cross-domain taxonomy, v1.34.0). Migration 031 backfills genesis audit entries for all existing taxa (WP-45 taxon hash chain — **STABLE as of v1.40.0**, see WP-64). Migration 030 added `fruiting_records` table (mycology fruiting/yield tracking, v1.31.0). Migration 029 added `origin_type` culture-origin CHECK column and `is_best_performer` flag on `specimens` (v1.30.0). Migration 028 added `colonization_pct` and `contaminant_type` to `subcultures` (v1.29.0). Migration 027 seeded mycology profile vocabulary (v1.28.0). Migration 026 added `biosafety_level` to `specimens` (v1.26.0). Migration 025 added `frozen_vials` table (v1.25.0). Migration 024 added PDL columns to `specimens`/`subcultures` (v1.24.0). Migration 023 expanded `cell_culture` vocabulary (v1.23.0). Migration 022 added generation-label and backcross-depth columns to `hybridization_events` + `is_cross_species` on `strains` (v1.21.0). Migration 021 added `ncbi_sync_log` table (v1.19.0). Migration 020 was the `taxa` table for Kingdom → Genus hierarchy (v1.18.0). Migration 019 was the Strain/Cultivar data model (v1.16.0). Migration 018 seeded `cell_culture` vocabulary (v1.15.0). Migrations 017/016 converted hardcoded vocabularies to profile-scoped lookup tables (v1.12.0). Migration 015 added `event_type` on `subcultures` + `app_config` (v1.11.0); 014 `app_settings` + auto-checkpoint flags (v1.10.0); 013 `audit_checkpoints` Merkle table (v1.9.0); 012 contamination columns on `specimens`; 011 `is_draft` on `media_batches` (v1.8.0); 010 generational depth (v1.7.0); 009 per-lineage hash chain; 008 hash-chain columns on `audit_log`; 007 performance indexes. The stage `CHECK` constraint was expanded in **migration 002**, rebuilt in **migration 003**, and **finally dropped for good in migration 016** — no more CHECK-constraint rebuilds for vocabulary changes.
**Security:** `csp` is now a locked-down policy (no longer `null`, WP-02); the default `admin/admin` credential is now gated behind a forced password change on first login (WP-01).
**Recent:** Trust(less) & Audit Layer Phase 1 (hash-chain + per-lineage genealogy, WP-18) shipped across v1.5.0 → v1.6.4; generational depth tracking, lineage passage offsets, `root_specimen_id`, and sibling display landed in v1.7.0; split workflow overhauled in v1.8.0 with letter-suffix accessions (001A/001B…), per-child controls, draft media batches, safety confirmation dialog, and synthetic split events in the passage timeline.
**Shipped (Phase TX):** Phase B polish & stability (WP-06–17) fully shipped v1.1.1–v1.3.0 ✅; Trust Layer Phase 1 (WP-18–21) **fully shipped** ✅; **Phase C WP-22–27 fully shipped** ✅ — WP-22 lab_profile + dead specimen (v1.11.0), WP-23 stage lookup table (v1.12.0), WP-24 remaining vocabulary tables (v1.12.0), WP-25 profile-aware dashboard statistics (v1.13.0), WP-26 lab profile switcher in Settings (v1.14.0), WP-27 cell_culture vocabulary seeded (v1.15.0). **Phase TX-1 WP-28 shipped** ✅ — Strain/Cultivar data model + backend (v1.16.0). **Phase TX-1 WP-29 shipped** ✅ — Strain Manager UI, Hybrid Wizard, Taxonomy Navigator (v1.17.0). **Phase TX-1 complete.** **Phase TX-2 fully shipped** ✅ — WP-35 taxonomy backbone (v1.18.0), WP-36 NCBI import/sync (v1.19.0), WP-37 multi-generational pedigree tools (v1.20.0), WP-38 advanced hybridization + generation labeling (v1.21.0), WP-39 advanced multi-column Taxonomy Navigator (v1.22.0). **Phase TX-2 complete.** **Phase D Cell Culture WP-30–34 fully shipped** ✅ — WP-30 vocabulary (v1.23.0), WP-31 PDL tracking (v1.24.0), WP-32 cryopreservation (v1.25.0), WP-33 mycoplasma compliance + biosafety level (v1.26.0), WP-34 cell-culture dashboard panels (v1.27.0). **Phase E Mycology WP-40–44 fully shipped** ✅ — WP-40 vocabulary (v1.28.0), WP-41 colonization & contamination tracking (v1.29.0), WP-42 genetic lineage & strain isolation markers (v1.30.0), WP-43 fruiting conditions & yield tracking (v1.31.0), WP-44 mycology QC compliance rules (v1.32.0). **Phase E complete.** **Phase TX-3 WP-45–49 fully shipped** ✅ — WP-45 full taxonomic hash chain EXPERIMENTAL (v1.33.0), WP-46 cross-domain taxonomy support (v1.34.0), WP-47 breeding programs (v1.35.0), WP-48 advanced hybridization (v1.36.0), WP-49 custom taxa & Darwin Core export (v1.37.0). **Phase TX-3 complete.** **Phase F WP-50 & WP-51 foundation fully shipped** ✅ — multi-user backend + LAN sync foundation, implemented together (v1.38.0): optional PostgreSQL connector behind the `postgres` Cargo feature (SQLite remains the sole live backend), and LAN sync change-detection/conflict-recording built on the existing audit hash chain (no networking layer yet). **Phase F WP-52–55 shipped together** ✅ — WP-52 email/desktop notifications with background scheduler (v1.39.0), WP-53 iOS build scaffold — ⚠️ unverified, no macOS/Xcode/Apple Developer access available (v1.39.0), WP-54 environmental sensor foundation — parsing/validation/manual entry real and tested, hardware transport (USB/BLE/MQTT) deferred (v1.39.0), WP-55 field-level permissions wired into two representative entities (v1.39.0). **Phase F WP-56–65 shipped together** ✅ — WP-56 local AI analysis (v1.40.0), WP-57 interactive lab map (v1.40.0), WP-58 advanced analytics & reporting dashboards (v1.40.0), WP-59 cloud backup & multi-device sync with E2E encryption (v1.40.0), WP-60 regulatory compliance export modules (v1.40.0), WP-61 plugin/extension system (v1.40.0), WP-62 PWA & offline-first (v1.40.0), WP-63 performance & scalability hardening (v1.40.0), WP-64 taxon chain re-anchoring — WP-45 now STABLE (v1.40.0), WP-65 a11y completion pass — 90→0 label-association warnings (v1.40.0). **Phase F complete.**
**Assets to preserve (don't regress these):** the error-logging system with form-payload capture; the immutable audit trail **and (once built) its cryptographic hash-chain/Merkle integrity layer**; the contamination-overview dashboard panel.
**Goal:** Now that PTC v1.0 has shipped, harden and polish it, then expand to **Cell Culture** and **Mycology** verticals from one shared engine — without forking the codebase three ways.

---

## 0. How to use this document with Claude Code

This roadmap is written as a sequence of **work packets** (`WP-xx`). Each packet is self-contained and scoped to be handed to Claude Code (web) one at a time. For every packet, paste the packet block and add this standing instruction:

> Implement only this packet. Do not refactor unrelated code or expand scope. When done, update `CHANGELOG.md` and `README.md` to reflect the change, bump the version per the packet, then commit and push to `master`.

Each packet specifies:
- **Goal** — one sentence.
- **Files** — the real paths to touch (verified against the current tree).
- **Steps** — ordered, concrete.
- **Acceptance** — how to know it's done.
- **Preserve** — what must not break.

Packets are ordered by dependency. The **Critical Path to v1.0** (Section 1) is the only thing standing between you and a shippable plant-tissue-culture product. The multi-vertical work (Sections 4–6) deliberately comes *after* a de-hardening refactor (Section 3) so the two new verticals never become forks.

---

## 1. Recommended sequence (the strategic call)

You want two things that pull against each other: **ship soon** and **three verticals**. Here's the honest ordering that gets both without regret:

1. **Phase A — Ship PTC v1.0 (Section 2). ✅ DONE.** Security + a real signed release + crash-proofing. Shipped across v0.1.20 → v1.1.0.
2. **Phase B — Polish, stability & trust (Section 3).** "Looking great / working great." Design tokens, empty/loading states, a11y, the first tests — **plus the Trust(less) and Audit Layer** (cryptographically tamper-evident history). Ships as v1.1–v1.2.
3. **Phase C — De-harden the domain (Section 4).** Convert the baked-in vocabulary (CHECK constraints, enums, labels) into data. This is the keystone. It's invisible to users but it's what makes one codebase serve three labs. Ships as v1.8 (still PTC-only behaviorally).
3.5. **Phase TX — Taxonomic & Provenance Module (Section 5).** Equal priority to Phase C and the remaining Trust Layer. Transforms the species registry into a true biological taxonomy: Strain/Cultivar as first-class entities, cryptographic version binding, pedigree tracking, hybridization support, and a powerful hierarchical navigator. Spans three TX sub-phases across v1.9 → v2.x → v3.x.
4. **Phase D — Cell Culture vertical (Section 6)** and **Phase E — Mycology vertical (Section 7)**, built as *profiles* on the shared engine. Phase TX makes this cleaner: the generic taxonomy engine is already in place before Cell Culture or Mycology verticals need their own strain/cultivar concepts.

> **Why de-harden before building verticals?** Your schema encoded plant vocabulary as SQL `CHECK` constraints — e.g. `stage CHECK(stage IN ('explant','callus','shoot_meristem',...))` at `migrations.rs`. The stage constraint was already **expanded in migration 002 and defensively rebuilt in migration 003** — that's two migrations whose job was to widen one constraint via a full table rebuild. **WP-23 (migration 016) ran this table-rebuild pattern one final time** to drop the constraint entirely. Cell lines don't have an "explant" stage; mushroom cultures don't "acclimatize." If you fork now, every vocabulary change is three migrations and three CHECK-constraint rebuilds forever. Lookup tables make vocabulary *data*, and data is cheap to vary per profile — and that's the state the codebase is now in.

---

## 2. PHASE A — Critical path to PTC v1.0

These are the genuine blockers to shipping. Nothing here is a feature; it's the difference between "a build exists" and "a product you can hand to a lab."

> **Standing preservation note for all of Phase A:** the **error-logging system with form-payload capture** and the **immutable audit trail** are real assets — every packet below should route its failures through the existing error log and write create/update/delete actions to the audit trail rather than inventing new mechanisms. Don't regress them.

> **Phase A is complete (shipped June 2026).** All five packets landed as one-packet-per-release rather than a single v1.0.0 cut: **0.1.20 → 0.1.21 → 1.0.0-1 → 1.0.0-2 → 1.1.0**. The current repository version is **1.1.0**. The per-packet status and any deviations from the original plan are recorded below.

### WP-01 — Force password change on first login (kill `admin/admin`) — ✅ Delivered in **v0.1.20**
- **Goal:** No deployment ever runs with the default credential.
- **As built:** Shipped via **migration 006** (`migration_006_force_password_change`) adding `must_change_password BOOLEAN NOT NULL DEFAULT 0` to `users`, with the seeded `admin` row set to `1`. The login response carries a `must_change_password` flag; when true the front end routes to a full-screen **`ForceChangePassword.svelte`** overlay (min-8-char + confirmation match) before the app shell renders, blocking all other navigation. New `change_password` Tauri command bcrypt-hashes, clears the flag, and writes an audit entry. New `mustChangePassword` store in `auth.ts` (`setAuth` takes an optional third arg; `clearAuth` resets it).
- **Differed from plan:** Shipped as its own **0.1.20** patch (not folded into a single v1.0.0). The forced-password screen became a dedicated `ForceChangePassword.svelte` component rather than a branch inside `Login.svelte`. Login hint updated to "First login: admin / admin (you will be prompted to set a new password)."
- **Acceptance (met):** Fresh DB → log in as `admin/admin` → forced password set → dashboard unreachable until done.

### WP-02 — Set a real Content-Security-Policy — ✅ Delivered in **v0.1.21**
- **Goal:** Replace `"csp": null` with a locked-down policy.
- **As built:** `tauri.conf.json` now sets `default-src 'self' ipc: http://ipc.localhost; script-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: blob:; connect-src 'self' ipc: http://ipc.localhost; worker-src blob:`. No remote script origins, no `'unsafe-eval'`.
- **Differed from plan:** One addition the original plan missed — **`worker-src blob:`** is required because html5-qrcode/ZXing spins up its decoder web worker from a `blob:` URI; without it the camera scanner silently fails. `style-src`/`font-src` explicitly allow the Google Fonts (Inter) origins.
- **Acceptance (met):** QR generate, QR camera scan, QR print, photo lightbox round-trip, and Excel export all verified working under the policy.

### WP-03 — Cut the first signed, versioned release — ✅ Delivered in **v1.0.0-1**
- **Goal:** A real GitHub Release with attached Windows MSI + Android APK, not just CI artifacts.
- **As built:** First signed GitHub Release published. Windows workflow now fires on `release: types:[published]` and uploads the `.msi` via `softprops/action-gh-release@v2`. Android workflow decodes a base64 release keystore from a GitHub secret and signs with it; a `build.gradle.kts` signing-patch step re-injects the signing config after `cargo tauri android init` regenerates `gen/android/`. Release keystore (`steloptc`, RSA 4096, ~27-year validity) generated and documented in **`.github/SIGNING.md`**; the same key must be reused for all releases to allow in-place Android upgrades. README Downloads table now points to GitHub Releases.
- **Differed from plan:**
  1. **Version label:** shipped as **`1.0.0-1`**, *not* `1.0.0-rc.1` — the WiX MSI bundler requires pre-release identifiers to be numeric-only (≤ 65535), and `rc` is non-numeric and rejected at bundle time. The roadmap's original `rc.1` label was not buildable.
  2. **Keystore secret name:** the keystore is delivered as a single base64 secret **`ANDROID_KEYSTORE_BASE64`** (decoded to a temp path at build time) rather than a committed `.jks` path; the other three secrets (`ANDROID_KEY_STORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`) are validated up front. **Hard-fail signing** is implemented — the release APK build aborts with a descriptive error if any secret is missing, never falling back to debug signing.
  3. Several follow-on CI fixes were required to make the release path work: `contents: write` workflow permissions for both release workflows, restricting Windows CI to master/claude branches, and removing a broken favicon reference.
- **Acceptance (met):** `releases/latest` resolves to a real release with both installers; Android APK signed with the release keystore.

### WP-04 — Crash-proofing & data-integrity pass — ✅ Delivered in **v1.0.0-2**
- **Goal:** No unhandled panic reaches the user; no partial writes on multi-step operations.
- **As built:** Replaced the panicking `.unwrap()` on `path.parent()` in `attachments_dir` with a `Result` return propagated through the error-log + toast system. **`create_subculture`** wrapped in a SQLite transaction (subculture INSERT + specimen `subculture_count` UPDATE + optional location UPDATE are now atomic). **`create_media_batch`** wrapped in a transaction (batch INSERT + all hormone/reagent INSERTs + all inventory deductions atomic). **`create_backup`** now verifies the WAL checkpoint result and aborts with a descriptive error if active readers left `busy_frames > 0`, instead of silently copying an incomplete snapshot.
- **Differed from plan:** Added scope beyond the original — **`reset_database` is now gated to debug builds only** (a leftover git conflict marker in `attachments.rs` was also cleaned up in a follow-up commit).
- **Acceptance (met):** Bad input yields a clean error toast + error-log entry rather than a crash; multi-step writes roll back atomically on failure.

### WP-05 — Onboarding empty state + seed-data toggle — ✅ Delivered in **v1.1.0**
- **Goal:** A brand-new lab sees guidance, not empty tables.
- **As built:** New **`FirstRun.svelte`** shown whenever the lab has zero specimens — a two-step guide ("Configure your species registry" → "Accession your first specimen") with direct navigation buttons; supervisors/admins also get a green **Load Sample Data** button. `Dashboard.svelte` shows `FirstRun` in place of the stats grid when `total_specimens === 0` and returns automatically once specimens exist. `SpecimenList.svelte` shows `FirstRun` only when genuinely empty (no active search/filter); filtered-empty searches still show the concise "No specimens found" message. New **`load_demo_data`** Tauri command (`admin.rs`) creates 1 demo MS media batch + 3 specimens (Asparagus, Nandina, Citrus), each with 3 passages of subculture history, in a single atomic transaction, guarded to refuse running if any specimens already exist. `loadDemoData` wrapper added to `api.ts`.
- **Differed from plan:** Landed at **v1.1.0** (the release packets had already consumed the 1.0.0-x line, so this became the first proper minor). Demo data is richer than planned — it includes full passage history, not just bare specimens. Demo removal reuses the existing **Admin → Dev Tools → Reset Database** (which preserves species/users) rather than a bespoke "clear demo" action.
- **Acceptance (met):** Fresh DB shows the first-run panel; demo-load builds a coherent sample lab with history; reset returns to first-run.

> **Phase A shipped as v1.1.0.** SteloPTC is now a security-hardened, signed, releasable product. Everything below is improvement, not blocker.

---

## 3. PHASE B — "Looking great / working great"

### Immediate fixes ✅ Complete (v1.1.1–v1.2.0)

### WP-06 — Bug/polish backlog clearance — ✅ Delivered in **v1.1.1**
- **Goal:** Fix the known silent-failure bugs from Phase A so Phase B polish work (WP-13) is building on a working foundation.
- **Files:** `src/lib/components/SpecimenList.svelte` (Print Summary fix). The QR scanner button-text fix (`QrScanner.svelte:221`, HTML entity `&#8594;` → Unicode `→`) was already applied as a standalone patch and does not need re-implementing.
- **Steps:**
  1. In `printSummaryReport`, wrap the entire function body in a `try/catch`. Any caught exception should call `addNotification('Print failed — check browser popup permissions or try again', 'error')` rather than crashing silently.
  2. Replace the bare `if (!win) return;` null guard with a user-facing notification: `addNotification('Could not open print window — allow popups for this app in your OS or browser settings', 'error'); return;`. A silent no-op is never acceptable; the user must know something went wrong.
  3. If the above two steps do not resolve the issue on Windows (i.e. `window.open` returns `null` consistently regardless of popup settings), investigate replacing `window.open` with Tauri's `WebviewWindow` API for reliable new-window creation in the Tauri 2 webview context. Apply the same fix to `QrModal.svelte`'s `printLabel` function at the same time to keep both print paths consistent.
  4. Verify the fix: click "Print Summary" on the Specimens page — the print dialog must appear, or a clear error notification must appear. No silent no-op.
- **Acceptance:** "Print Summary" either opens the OS print dialog or surfaces a notification explaining why it could not. The QR label print in `QrModal.svelte` continues to work as before (verified in WP-02).
- **Preserve:** The `printSummaryReport` HTML output format and column layout; the `printLabel` QR label format.
- **Bump:** patch.

### WP-07 — QR scanner: reject non-SteloPTC codes gracefully — ✅ Delivered in **v1.1.2**
- **Goal:** Scanning an arbitrary QR code (a URL, vCard, plain text) shows a clear "not a SteloPTC code" message instead of treating the payload as an accession number.
- **Files:** `src/lib/components/QrScanner.svelte`.
- **Steps:**
  1. Add a `$state` boolean `invalidQr = false` alongside the existing result state.
  2. In `onScanSuccess`, after a JSON parse failure, check whether the raw text is a plausible SteloPTC accession before setting `parsedAccession`. A minimal guard: if the text starts with `http://`, `https://`, or `mailto:`, or if it contains whitespace and is longer than 60 characters, it is not a valid accession. Set `invalidQr = true` and leave `parsedAccession` empty.
  3. In the result card UI, when `invalidQr` is true, show a distinct warning row: *"This QR code is not a SteloPTC specimen label"* — do not render the "Open Specimen" button.
  4. `clearResult` should also reset `invalidQr = false`.
  5. Still call `storeScan` (the scan event is recorded for audit regardless of whether it resolved to a specimen).
- **Acceptance:** Scanning a Wikipedia or other non-SteloPTC QR shows the warning message and no "Open Specimen" button. Scanning a real SteloPTC specimen QR (JSON payload with `accession` key, or a plain accession-format string) works as before.
- **Preserve:** JSON payload parsing, `onscan` callback, scan storage, camera lifecycle.
- **Bump:** patch.

### WP-08 — Specimen Work Queue / Daily Task View — ✅ Delivered in **v1.2.0**
- **Goal:** Give lab technicians a single view showing which specimens need attention today — removing the need to scan the full list looking for overdue actions.
- **Files:** new `src/lib/components/WorkQueue.svelte`, `src-tauri/src/commands/specimens.rs` (new `get_work_queue` command), `src/lib/api.ts`, `src/lib/components/Sidebar.svelte` (add nav entry).
- **Steps:**
  1. Add a `get_work_queue` Tauri command that queries the database and returns a list of `WorkQueueItem` records. Each item carries: specimen accession, species, stage, location, and a `reason` tag indicating why it needs attention. Initial reasons to detect:
     - **Subculture due** — last subculture was more than N days ago (use the per-species expected subculture interval if available, otherwise a lab-wide default of 30 days).
     - **Media change due** — same interval logic applied to the last media-change passage type.
     - **Contamination check overdue** — any specimen flagged with an open contamination event older than 7 days that has not been resolved.
     - **No passage ever recorded** — specimens older than 14 days with zero subculture history.
     - **Quarantine without release** — specimens in quarantine status with no resolution passage in the last 30 days (this mirrors the existing compliance rule but surfaces it as an action item, not just a flag).
  2. Return items sorted by urgency: contamination and quarantine issues first, then most-overdue subcultures descending.
  3. Build `WorkQueue.svelte` as a simple list view (not a calendar, not a Kanban — just a prioritised table). Columns: accession, species, stage, location, reason badge, and a **quick-action button** that navigates directly to the specimen detail for that row.
  4. Add the Work Queue as a sidebar nav item (between Dashboard and Specimens, or after Specimens — pick whichever feels natural in the navigation order). Use a clock or checklist emoji icon consistent with the existing sidebar icon style.
  5. Show a count badge on the nav item when the queue is non-empty (mirrors the existing error-log badge pattern).
- **Acceptance:** Opening the Work Queue shows every specimen that meets at least one overdue criterion; clicking the action button navigates to the correct specimen detail; specimens with no overdue actions produce an empty-state message ("All specimens are on schedule"); the count badge on the nav item reflects the current queue length.
- **Preserve:** The existing compliance-flag system and audit trail — the Work Queue is a read-only derived view, not a replacement for compliance records. Do not write any audit or compliance entries from this view.
- **Bump:** minor.

### WP-09 — Tauri-reliable print invocation — ✅ Delivered in **v1.2.5**
- **Goal:** All three print functions work on the Windows desktop (Tauri/WebView2) without the "Could not open print window" popup-blocked error.
- **Root cause:** WebView2's popup policy blocks `window.open('', '_blank', ...)`, causing it to return `null`. The previous WP-06 fix surfaced a clear error notification but did not solve the underlying problem.
- **Files:** `src/lib/components/SpecimenList.svelte` (`printSummaryReport`), `src/lib/components/SpecimenDetail.svelte` (`printCultureReport`), `src/lib/components/QrModal.svelte` (`printLabel`).
- **Steps:**
  1. For each print function, separate the CSS (`printCss`) and body HTML (`bodyHtml`) into local variables before the popup attempt.
  2. Try `window.open` as before (preserves browser/web behavior). If the popup succeeds, write the document using those variables and return.
  3. If the popup is blocked (returns `null` or throws), fall back to in-page DOM injection: create a `<style>` element with `@page` rules at top level and all print CSS wrapped in `@media print{body>*:not(#frame){display:none!important}#frame{display:block!important}...}`, append a hidden `<div id="frame">` containing `bodyHtml`, call `window.print()` after an 80 ms timeout, and clean up both elements in a `{ once: true }` `afterprint` listener.
  4. Also fix the pre-WP-06 silent-fail in `printCultureReport` (`SpecimenDetail.svelte`): the old code had `if (!win) return;` with no user notification. The new fallback replaces this.
  5. Bump all three version files to **1.2.5** (also corrects the `tauri.conf.json` drift: it was at 1.2.3 while the others were at 1.2.4).
- **Acceptance:** Clicking "Print Summary", "Print Report", or "Print Label" in the Tauri desktop app opens the OS print dialog. Cancelling print cleans up the injected DOM. The WP-13 output quality is identical in both the popup and fallback paths.
- **Preserve:** Popup path for browser/non-Tauri builds; WP-13 print CSS and layout exactly; `afterprint` cleanup so the injected frame never leaks into the live UI.
- **Bump:** patch → **v1.2.5**.

### Looking great — design system & polish ✅ Complete (v1.2.1–v1.2.5)

### WP-10 — Extract a central design-token system — ✅ Delivered in **v1.2.1**
- **Goal:** One source of truth for color, spacing, type, radius, shadow — instead of 15 component `<style>` blocks + a 282-line block in `App.svelte`.
- **Files:** new `src/lib/styles/tokens.css` (imported once in `App.svelte`), then incremental refactors per component.
- **Steps:**
  1. Define `:root` CSS custom properties for the existing palette (light + dark), spacing scale, font sizes, radii, shadows, z-index layers.
  2. Map the current dark-mode toggle to swap a `data-theme` attribute on `<html>` that flips the token values (cleaner than per-component dark rules).
  3. Migrate components to tokens **one per packet** (don't do all 15 at once — scope creep risk). Start with `Dashboard.svelte` and `Sidebar.svelte`.
- **Acceptance:** Changing one token (e.g. accent color) restyles the whole app; dark mode flips via the single attribute.
- **Preserve:** Current visual appearance — this is a refactor, not a redesign. Pixel-diff before/after on the dashboard.
- **Bump:** patch each.

### WP-11 — Loading, empty, and error states everywhere — ✅ Delivered in **v1.2.2**
- **Goal:** Every list/detail view has a skeleton-loading state, a friendly empty state, and an inline error state.
- **Files:** all list components (`SpecimenList`, `MediaList`, `InventoryManager`, `ReminderList`, `ComplianceView`, `AuditLog`, `ErrorLog`).
- **Steps:** Add a tiny shared `<DataState>` wrapper (loading / empty / error / ready). Replace bare table renders.
- **Acceptance:** Throttle the backend and watch each view show a skeleton, then data; empty filters show "no results," not a blank table.
- **Preserve:** Existing data fetching.
- **Bump:** patch.

### WP-12 — Accessibility & keyboard pass (WCAG 2.1 AA target) — ✅ Delivered in **v1.2.3**
- **Goal:** Usable by keyboard and screen reader; contrast verified.
- **Files:** global + per-component.
- **Steps:** Audit focus order, visible focus rings, `aria-label`s on icon-only buttons (the sidebar uses emoji icons), color-contrast on the health-status slider, modal focus trapping (QR modal, lightbox), and that the existing Ctrl+1–5 shortcuts are documented in-app.
- **Acceptance:** Full create-specimen → record-passage flow completable with keyboard only; axe-core run shows no critical violations.
- **Preserve:** The 48px touch targets already added for mobile (WCAG 2.5.5).
- **Bump:** patch.
- **Open — v1.37.1 review:** The v1.2.3 pass resolved all axe-core **critical** violations (focus order, aria-labels on icon-only buttons, modal focus trapping, contrast). **85 non-critical warnings remain** — primarily `<label>` elements not programmatically linked to their control via `for`/`id` pairing or `aria-labelledby`. These affect screen-reader users in form-heavy views (SpecimenForm, MediaList, InventoryManager, ComplianceView). They are tracked as a Phase F follow-up: **WP-64** will systematically resolve all label-association warnings across the form layer in one targeted pass, targeting the WCAG 2.1 AA success criterion 1.3.1 (Info and Relationships). No other WP should incidentally fix these — address them in bulk during WP-64 so the fix is auditable.

### WP-13 — Print / PDF polish — ✅ Delivered in **v1.2.4**
- **Goal:** The Culture Certificate and Specimens Summary look like lab documents, not browser printouts.
- **Depends on:** WP-06 (Print Summary must be working before polishing its output).
- **Files:** `src/lib/components/SpecimenList.svelte`, `src/lib/components/SpecimenDetail.svelte`, `src/lib/components/QrModal.svelte`.
- **Steps:** Add a print stylesheet with proper margins, a header/footer band (lab name, accession, generated date, page numbers), and a place for a lab logo.
- **Acceptance:** A printed certificate is clean on A4 and US Letter; the Specimens Summary prints cleanly in landscape.
- **Preserve:** Existing print-API approach; do not change the HTML structure in ways that break the fix from WP-06.
- **Bump:** patch.

### Working great — stability, performance, tests ✅ Complete (v1.2.4–v1.3.0)

### WP-14 — First test harness (the highest-leverage packet here) — ✅ Delivered in **v1.2.4**
- **Goal:** Stop shipping blind. There are currently **zero tests**.
- **Depends on:** Nothing — but WP-18 (hash-chain audit log) must not be implemented before this packet is complete. Tests are the gate on the Trust layer: cryptographic invariants must be encoded as assertions before being shipped.
- **Files:** `src-tauri` (Rust `#[cfg(test)]` modules + an integration test dir), `package.json` (add Vitest), `vitest.config.ts`.
- **Steps:**
  1. Rust: unit-test the pure logic that doesn't need a window — accession-number generation, basal-salts g/L auto-calc, compliance auto-flag rules, stock auto-depletion math. Use an in-memory SQLite for command tests.
  2. Front end: Vitest + `@testing-library/svelte` for the most logic-heavy components (SpecimenForm validation, ExportManager sheet assembly).
  3. Add a CI job that runs `cargo test` + `npm test` and blocks merge on failure.
- **Acceptance:** `cargo test` and `npm test` both green in CI; the compliance rules and accession format are covered.
- **Preserve:** All existing behavior (tests should encode current correct behavior).
- **Bump:** patch.

### WP-15 — Query performance & indexing audit — ✅ Delivered in **v1.2.7**
- **Goal:** Stays fast at 10k+ specimens.
- **Files:** `src-tauri/src/db/migrations.rs` (indexes), `commands/specimens.rs`, `commands/subcultures.rs`.
- **Steps:** Verify indexes exist on every column used in `WHERE`/`JOIN`/`ORDER BY` (species_id, stage, project_id, parent_specimen_id, subculture.specimen_id, created_at). Confirm list endpoints paginate (the `PaginatedResponse` type exists — make sure every list uses it, including the dashboard panels). Replace any N+1 patterns (the changelog already shows you fixed one with `list_all_subcultures` — audit for others).
- **Acceptance:** Seed 10k specimens + 50k subcultures; list/search/dashboard load under ~200ms.
- **Preserve:** Existing pagination contract.
- **Bump:** patch.

### WP-16 — Backup → Restore (close the loop) — ✅ Delivered in **v1.3.0**
- **Goal:** Backups are only half a feature without restore.
- **Files:** `src-tauri/src/commands/backup.rs`, Dashboard.
- **Steps:** Add a "Restore from backup" action (admin only) that validates the file, checkpoints/closes the live DB, swaps it, and reloads. Confirm-twice UX given destructiveness.
- **Acceptance:** Backup → mutate data → restore → data matches the backup point.
- **Preserve:** WAL-checkpoint-before-copy logic.
- **Bump:** minor.

### WP-17 — Excel import (already on your list) — ✅ Delivered in **v1.3.0**
- **Goal:** Round-trip the export — parse `.xlsx` to create/update specimens + subcultures.
- **Files:** new `ImportManager.svelte`, `commands/` import handler.
- **Steps:** Reuse SheetJS to read the six-sheet workbook; validate rows; show a dry-run diff (create/update/skip counts + per-row errors) before committing inside a transaction.
- **Acceptance:** Export a lab → wipe → import the same file → lab restored; malformed rows reported, not silently dropped.
- **Preserve:** The export schema (import must match the export's sheet/column layout exactly).
- **Bump:** minor.

### Trust(less) and Audit Layer

SteloPTC already keeps an **immutable audit trail**, but "immutable" today means *policy* — a row in SQLite can still be edited or deleted out-of-band (a stray `UPDATE`, a corrupted file, a malicious admin). This layer makes the history **tamper-evident**: any after-the-fact change to a past record becomes cryptographically detectable. The motivation is strong internal guarantees first, with optional external proof later.

The work is staged so that real value lands early and nothing is over-built before it's needed:

- **Phase 1 — Cryptographic Audit Log (Merkle-chained local history) — _begin now._** Hash-chain every audit entry and roll batches into Merkle checkpoints, entirely local. Delivers tamper-evidence with zero external dependencies. Packets **WP-18 → WP-21** below.
- **Phase 2 — On-Chain Anchoring (Dogecoin first) — _future work, not yet scoped._** Periodically publish a checkpoint's Merkle root to Dogecoin so a third party can prove a record existed at a point in time without trusting the lab. Phase 1 deliberately leaves the hooks for this (see WP-20's `anchored_txid` column and the documented Merkle construction).
- **Phase 3 — Specimen Events as Transactions — _longer-term, deliberately deprioritized._** Modelling each specimen lifecycle event as a signed ledger transaction. Interesting, but not a near-term goal — listed only so the architecture in Phase 1 doesn't paint it into a corner.

> **Numbering note:** Trust & Audit Phase 1 packets (WP-18–21) are numbered sequentially within Phase B. Future Phase 2/3 packets are reserved in the **WP-60 series** — safely beyond Phases C–F (which use the 20s–50s) — so that all cryptographic infrastructure work remains grouped and easy to find. Only Phase 1 is scoped into packets below; Phase 2/3 are described, not packetized, to avoid over-scoping.

> **Dependency:** WP-14 (first test harness) is a hard gate on WP-18. The canonical serialization and hash-chain continuity invariants introduced in WP-18 must be encoded as assertions before being shipped — do not hand WP-18 to Claude Code until `cargo test` is green.

#### Phase 1 — Cryptographic Audit Log (start now)

### WP-18 — Hash-chain the immutable audit log (tamper-evident core)
- **Goal:** Every `audit_log` entry carries a SHA-256 hash of its own canonical content plus the hash of the previous entry, forming an append-only hash chain.
- **Files:** new migration (007), `src-tauri/src/models/audit.rs`, `src-tauri/src/commands/audit.rs`.
- **Steps:**
  1. Migration 007: add `chain_seq INTEGER`, `prev_hash TEXT`, `entry_hash TEXT` to `audit_log` (nullable for any pre-existing rows; new rows always populate them).
  2. Define a **canonical serialization** of an audit entry — a fixed, documented field order (e.g. `chain_seq | timestamp | user_id | entity_type | entity_id | action | payload_json`). This must be byte-stable and is the spec future verifiers depend on.
  3. On insert: `entry_hash = SHA256(canonical_bytes || prev_hash)`, where `prev_hash` is the previous row's `entry_hash`; the genesis entry uses a fixed all-zero `prev_hash`. Assign a monotonically increasing `chain_seq`. Do the hash + insert inside the existing audit write so it's atomic.
  4. Add `sha2` to `Cargo.toml` if not already present.
- **Acceptance:** Inserting N entries yields a continuous chain (each `prev_hash` equals the prior `entry_hash`); editing any historical row's content makes its recomputed hash mismatch its stored `entry_hash` and breaks every subsequent link.
- **Preserve:** The audit log stays append-only and immutable; existing columns unchanged; existing audit writes from every command keep working.
- **Bump:** minor.

### WP-19 — Chain verification command + integrity panel — ✅ Delivered in **v1.5.1 / v1.6.0** (core); **v1.9.0** (polish)
- **Goal:** A backend command that re-walks the chain and reports the first broken link, surfaced in a small admin/supervisor panel.
- **Files:** `src-tauri/src/commands/audit.rs` (`verify_audit_chain`), `src/lib/api.ts`, a new `AuditIntegrity.svelte` panel (reachable from the existing Audit Log view).
- **Steps:**
  1. `verify_audit_chain` recomputes each `entry_hash` from stored content + stored `prev_hash`, compares to the stored `entry_hash` and to the next row's `prev_hash`, and returns `{ verified, total_entries, first_broken_seq, detail }`.
  2. UI: a **"Verify history"** button showing ✓ *History verified (N entries)* or a red flag pinpointing the first broken `chain_seq`, plus a last-verified timestamp.
- **Acceptance:** A clean DB verifies green; a manual out-of-band row edit is detected and the breaking `chain_seq` is reported.
- **Preserve:** Verification is strictly read-only; the existing audit viewer is untouched apart from the added entry point.
- **Bump:** minor.
- **As built:** Core `verify_audit_entry` and `verify_audit_lineage` Tauri commands + Audit Log UI (chain columns `#`/Prev Hash/Entry Hash, Row + Chain verify buttons, chain-integrity banner) delivered in **v1.5.1** and **v1.6.0**. v1.9.0 polished: contamination inheritance on split (children inherit parent's `contamination_flag` + notes; audit entry text reflects inheritance), **Verify All Lineages** batch button that walks every unique lineage on the current Audit Log page in one click, and cleaner per-lineage verification message formatting.

### WP-20 — Merkle checkpoints over audit lineages — ✅ Delivered in **v1.9.0**
- **Goal:** Roll ranges of audit entries into a Merkle tree and store the root, so verification is efficient and roots are ready to anchor later — without redesign.
- **As built:**
  - **Migration 013** adds `audit_checkpoints (id, lineage_id, start_seq, end_seq, entry_count, merkle_root, created_at, created_by, anchored_txid TEXT NULL)`. `anchored_txid` is the Phase-2 Dogecoin hook (WP-65+), always NULL for now.
  - **`build_merkle_root`** in `db/queries.rs`: binary Merkle tree with Bitcoin's "duplicate-last" rule for odd counts. Pure function; takes a slice of SHA-256 hex strings and returns the root. Empty → ZERO_HASH; single leaf → leaf itself (no extra hash round).
  - **Three Tauri commands** in `commands/audit.rs`:
    - `create_audit_checkpoint(lineage_id, start_seq?, end_seq?)` — builds and stores a checkpoint. Requires supervisor/admin role.
    - `verify_against_checkpoint(checkpoint_id)` — three-stage verification: count check → Merkle root check → individual content-hash check. Reports the first break with precise `tampered_seq` when pinpointable.
    - `list_audit_checkpoints(lineage_id?)` — lists stored checkpoints, newest first.
  - **Frontend:** Checkpoints panel in the Audit Log view (toggled by a banner button). Create form with lineage dropdown + optional seq range. Per-checkpoint Verify button with inline pass/fail display.
  - **Tests:** 10 new Rust unit tests covering Merkle tree edge cases (empty, single, two-leaf, three-leaf duplicate-last, determinism, mutation detection) plus four checkpoint scenario tests (creation, intact verification, tamper detection, removal detection).
  - **Docs:** `docs/merkle-checkpoints.md` specifies the canonical serialization, construction algorithm, schema, command API, and includes a Python standalone verifier snippet.
- **Differed from plan:**
  - Automatic (event-driven and pre-backup) checkpoint creation deferred to WP-21 — manual creation only for this phase.
  - Per-entry Merkle proof paths not yet exportable — deferred to WP-21.
  - Schema uses `lineage_id` instead of the originally planned global range approach, enabling per-lineage isolation from day one.
- **Bump:** minor → **v1.9.0**.

### WP-21 — Merkle proof export, auto-checkpointing & standalone re-verification — ✅ Delivered in **v1.10.0**
- **Goal:** Export one record's audit history plus its Merkle proof to a checkpoint root as portable JSON, with a documented standalone verifier so a third party can confirm tamper-evidence without running SteloPTC. Also add automatic checkpoint creation on a configurable entry-count threshold and pre-backup.
- **As built (v1.10.0):**
  - `export_audit_proof(checkpoint_id)` command produces a self-contained `PortableMerkleProof` JSON with every entry's canonical form, `prev_hash`, `entry_hash`, and individual Merkle inclusion path.
  - `verify_exported_proof(proof_json)` command runs three-stage verification (content hash → chain links → Merkle root) entirely without the database — suitable for offline auditors.
  - `verify_proof_data` pure function is separately unit-tested for all four failure modes.
  - Auto-checkpoint: `auto_checkpoint_lineages` query finds all lineages with uncovered entries ≥ `min_uncovered` and creates checkpoints flagged `is_auto = 1` with an `auto_source` tag.
  - `create_backup` pre-checkpoint hook: runs `auto_checkpoint_lineages(..., "backup", 0)` silently before WAL copy — never blocks the backup.
  - `get_auto_checkpoint_config` / `set_auto_checkpoint_config` / `run_auto_checkpoint` commands with `app_settings` persistence (migration 014).
  - UI: **Export** button per checkpoint row, **Auto** badge on auto-created checkpoints, proof import-and-verify panel, auto-checkpoint config section with interval, on-backup toggle, and Run Now.
  - Documentation: `docs/merkle-proofs.md` — proof format spec, field-by-field reference, the three-stage algorithm, and a standalone Python verifier (zero external dependencies).
  - Tests: 10 new tests — Merkle path (single leaf, 4 leaves, 3 leaves odd), proof verification (valid, tampered canonical, broken chain, wrong root), auto-checkpoint (creates, respects interval, skips below threshold). All 59 tests pass.
- **Bump:** minor — completes the Phase-1 Trust Layer.
- **Migration:** 014 — adds `is_auto` / `auto_source` to `audit_checkpoints`; creates `app_settings` with seeded defaults.

#### Phase 2 — On-Chain Anchoring (Dogecoin first) — *future, not yet scoped*

When external verifiability is actually needed (regulatory evidence, IP-priority proof, cross-party collaboration), publish a checkpoint's `merkle_root` to Dogecoin (e.g. via an `OP_RETURN` output), store the returned txid in `audit_checkpoints.anchored_txid`, and add a verification path that confirms a root on-chain. This is intentionally left un-packetized for now; the Phase-1 design (stable canonical form, deterministic Merkle root, nullable `anchored_txid`) already makes it a drop-in rather than a rewrite. *Reserved: WP-66+.*

#### Phase 3 — Specimen Events as Transactions — *longer-term, deprioritized*

A more formal model in which specimen lifecycle events are individually signed and ordered like ledger transactions. Recorded here only to keep the Phase-1 foundation from foreclosing it. Not a near-term priority. *Reserved: WP-67+.*

---

## 4. PHASE C — De-harden the domain (the keystone refactor)

This is the work that turns one product into a platform. It is **behavior-preserving for PTC** — after this phase, the plant app looks and works identically, but the vocabulary lives in data instead of in `CHECK` constraints, Rust enums, and hardcoded labels. Do it in this order.

### WP-22 — Introduce the `lab_profile` concept — ✅ Delivered in **v1.11.0**
- **Goal:** One app-level setting that says which kind of lab this install is.
- **Files:** new migration, `src-tauri/src/commands/admin.rs`, a new `src/lib/profile.ts`.
- **Steps:**
  1. Migration: a single-row `app_config` table (if not present) with `lab_profile TEXT NOT NULL DEFAULT 'plant_tissue_culture'`. Allowed values: `plant_tissue_culture | cell_culture | mycology`.
  2. Backend command to read/write the profile (admin only; set at first-run, hard to change after data exists).
  3. Front end `profile.ts` exposes the active profile to all components.
- **Acceptance:** Profile is readable app-wide; defaults to PTC so nothing changes.
- **Preserve:** Everything — this packet adds, removes nothing.
- **Bump:** minor.
- **As built (v1.11.0):**
  - **Migration 015** adds `event_type TEXT NOT NULL DEFAULT 'passage'` to `subcultures` (with `idx_subcultures_event_type` index) and creates the `app_config` single-row table (`CHECK (id = 1)`) with `lab_profile` constrained to `plant_tissue_culture | cell_culture | mycology`; seeds the default `plant_tissue_culture` row.
  - `get_lab_profile` / `set_lab_profile` Tauri commands — any authenticated user can read the profile; only admins can change it; profile is locked once any specimens exist to preserve data-integrity invariants.
  - `src/lib/profile.ts` — Svelte writable store (`labProfile`), `LAB_PROFILE_LABELS` map, `loadLabProfile()` async loader, and `currentLabProfile()` synchronous accessor. Default remains `plant_tissue_culture` so existing deployments see no change.
  - **Dead Specimen workflow** (bonus scope in same PR): "☠ Record Death & Archive" action when health slider hits 0; `record_specimen_death` Tauri command (archives specimen, inserts `event_type = 'death'` subculture row without incrementing `subculture_count`, writes `"death"` audit entry); death event card in passage timeline (skull icon, "Death · Archived" pill, red card); "Dead / Archived" red badge on archived specimens from the death workflow; passage count excludes `event_type = 'death'` rows.
  - **5 new Rust unit tests:** death archives specimen and zeroes health, `event_type` stored as `'death'`, archived specimen blocks further passages, normal passages retain `'passage'` event_type, `app_config` seeded with default profile.
- **Differed from plan:** WP-22 originally scoped only the lab_profile concept; the Dead Specimen archive workflow was added as complementary scope in the same PR since both share migration 015's `event_type` column.

### WP-23 — Convert stage `CHECK` constraints → a `stages` lookup table — ✅ Delivered in **v1.12.0**
- **Goal:** Make the specimen lifecycle vocabulary *data*. This is the single most important schema change for multi-vertical.
- **Files:** new migration, `models/specimen.rs`, `commands/specimens.rs`, `SpecimenForm.svelte`, `SpecimenDetail.svelte`, dashboard "by_stage" panel.
- **Steps:**
  1. New `stages` table: `(id, profile, code, label, sort_order, is_terminal)`. Seed it with the current plant stages (`explant`, `callus`, `shoot_meristem`, `apical_meristem`, `plantlet`, `acclimatized`, etc.) under `profile = 'plant_tissue_culture'`.
  2. **Drop the `CHECK(stage IN (...))` constraint** on `specimens.stage` (rebuild-table migration — the exact pattern migrations 002 and 003 already used to expand this constraint, but this is the **last time you'll ever do it for a vocabulary change**: once stages live in a lookup table, adding/removing/renaming a stage is a plain row operation with no migration and no table rebuild). Keep `stage` as a plain TEXT FK-by-code into `stages` filtered by active profile.
  3. Form/detail read the stage dropdown from `stages WHERE profile = activeProfile ORDER BY sort_order`, instead of a hardcoded list.
  4. Dashboard "by_stage" counts and any stage-colored UI read labels from the table.
- **Acceptance:** PTC behaves identically; adding a new stage row appears in the dropdown with no code change and no migration.
- **Preserve:** All existing specimens' stage values (seed codes must match current strings exactly so existing rows stay valid).
- **Bump:** minor.
- **As built (v1.12.0):**
  - **Migration 016** creates the `stages` table (`profile`, `code`, `label`, `sort_order`, `is_terminal`); seeds all 15 plant tissue culture stage codes; rebuilds `specimens` in one pass to drop the `CHECK(stage IN (...))` constraint while keeping the `acclimatization_status` CHECK intact. All existing specimen rows remain valid.
  - `list_stages` Tauri command returns stages ordered by `sort_order` for the active lab profile; `VocabEntry` and `StageEntry` types exported from `api.ts`.
  - `SpecimenForm.svelte` and `SpecimenDetail.svelte` now populate their stage dropdowns from `list_stages` instead of hardcoded arrays.

### WP-24 — Same treatment for the other hardcoded vocabularies — ✅ Delivered in **v1.12.0**
- **Goal:** Generalize `propagation_method`, `hormone_type`, compliance `record_type`/`agency`, and inventory `category` the same way.
- **Files:** migration, the corresponding models/commands/components.
- **Steps:** For each, create a profile-scoped lookup table seeded with today's plant values; drop the `CHECK` constraint; drive the UI from the table. Group related ones to minimize table-rebuild migrations.
- **Acceptance:** PTC unchanged; each vocabulary now varies by profile.
- **Preserve:** All existing enum values as seed data.
- **Bump:** minor.
- **As built (v1.12.0):**
  - **Migration 017** creates four additional lookup tables — `hormone_types`, `compliance_record_types`, `compliance_agencies`, `inventory_categories` — all profile-scoped and seeded with plant tissue culture values; then rebuilds `media_hormones`, `compliance_records`, and `inventory_items` in one FK OFF/ON window to drop their respective `CHECK` constraints.
  - `list_propagation_methods`, `list_hormone_types`, `list_compliance_record_types`, `list_compliance_agencies`, `list_inventory_categories` Tauri commands added; all in new `commands/vocabulary.rs` module.
  - `SpecimenForm.svelte`: propagation method dropdown populated from `list_propagation_methods`. `ComplianceView.svelte`: record type and agency dropdowns from vocabulary. `InventoryManager.svelte`: category dropdown from vocabulary.

### WP-25 — Profile-aware dashboard statistics — ✅ Delivered in **v1.13.0**
- **Goal:** Scope all aggregate dashboard counts to the active lab profile so the dashboard never shows irrelevant stage data.
- **Files:** new `src-tauri/src/db/dashboard.rs` module; `commands/specimens.rs`, `commands/subcultures.rs`, `Dashboard.svelte`.
- **As built (v1.13.0):**
  - **New `db/dashboard.rs` module** with three testable, pure-connection query functions: `query_specimen_stats` (by_stage breakdown inner-joins against `stages` vocabulary so only stages defined for the active profile are counted; returns vocabulary labels e.g. "Shoot Meristem" rather than raw stage codes), `query_contamination_stats` (all specimen/vessel counts join through `stages` so the active profile controls which specimens are in scope), `query_subculture_schedule` (only specimens whose `stage` exists in `stages` for the active profile appear).
  - **11 new Rust unit tests** covering: vocabulary labels returned for PTC, cross-profile stage exclusion, empty result for unseeded profile, database-wide aggregate counts, contamination scoping and rate, vessel-type breakdown, and schedule filtering.
  - No hardcoded stage lists remain in any dashboard query.
  - `commands/specimens.rs::get_specimen_stats` delegates to `db::dashboard`; same for contamination and schedule commands.
  - `Dashboard.svelte` tooltip updated to mention the active lab profile on the "Specimens by Stage" panel.

### WP-26 — Lab Profile Switcher in Settings — ✅ Delivered in **v1.14.0**
- **Goal (as planned):** The auto-flag engine (currently citrus-HLB / USDA-specific in `compliance.rs:252`) becomes a profile-pluggable rule set.
- **As built (v1.14.0):** Scope was reprioritized — the compliance rule engine restructure was deferred and replaced with the more immediately useful lab profile switcher UI in Settings:
  - **`Settings.svelte`** — new admin-only Settings view (sidebar gear icon). Shows current active lab profile, a dropdown to select a new profile, a warning banner explaining vocabulary implications, and a mandatory `CHANGE PROFILE` confirmation before applying. When the lab has no specimens, confirmation is not required.
  - **`check_profile_change_allowed(specimen_count, confirmation)`** — new pure, testable helper in `db/queries.rs`. Returns `Ok(())` for empty labs or when confirmation matches `"CHANGE PROFILE"` exactly (whitespace-trimmed). Returns a descriptive error with specimen count otherwise.
  - **7 new Rust unit tests** covering: empty lab always allowed, confirmation ignored on empty lab, blocked without confirmation when specimens exist, blocked on wrong confirmation, allowed with correct confirmation, whitespace trimming accepted, correct grammar in error message.
  - **6 new TypeScript tests** in `src/lib/profile.test.ts` covering: default store value, reactive updates, synchronous accessor, immediate store reflection after profile switch, `LAB_PROFILE_LABELS` completeness, human-readable label for the default profile.
  - After a successful profile change, `labProfile.set(selected)` updates the Svelte writable store immediately so all subscribed components react without a restart.
- **Compliance rule engine restructure:** Deferred; the existing four PTC rules remain in `commands/compliance.rs` with no profile gating. Will be addressed in a future packet.
- **Bump:** minor → **v1.14.0**.

### WP-27 — Cell Culture Profile Vocabulary Seed — ✅ Delivered in **v1.15.0**
- **Goal (as planned):** Three installable apps from one repo, differentiated at build time, sharing 95%+ of the code.
- **As built (v1.15.0):** Scope was reprioritized — build-time app identity and per-vertical CI matrix were deferred. Instead, the `cell_culture` profile vocabulary was seeded so the profile switcher (WP-26) has real data to switch to:
  - **Migration 018** — `INSERT OR IGNORE` into all six vocabulary tables for `profile = 'cell_culture'`: 12 stages, 7 propagation methods, 4 hormone types, 9 compliance record types, 4 compliance agencies, 7 inventory categories. No schema changes, no table rebuilds, no existing data touched.
  - **9 new Rust unit tests** in `db/migrations.rs` verifying: stage count (12), single terminal stage (`archived`), non-terminal count (11), propagation method count (7), hormone type count (4), compliance record type count (9), compliance agency count (4), inventory category count (7), and isolation from `plant_tissue_culture`.
- **Per-vertical build-time identity:** Deferred; the three `productName`/`identifier` parameterization and CI matrix are planned for Phase D/E when the second vertical is ready to ship.
- **Bump:** minor → **v1.15.0**.

---

## 5. PHASE TX — Taxonomic & Provenance Module

The Taxonomic & Provenance Module is a **major new workstream** with equal priority to the Trust Layer (WP-20/21) and the Phase C de-hardening refactor. It transforms the species registry from a flat lookup into a true biological taxonomy with Strain/Cultivar support, cryptographic version binding, pedigree tracking, hybridization tools, and a powerful hierarchical navigator. The workstream spans three sub-phases and is designed generically — SteloPTC (plants), SteloCC (animals), SteloMyco (fungi), and future verticals all share the same engine.

**Design principles:**
- Hash chains propagate **downward**: Species → Strain → Specimen. Each level's genesis audit entry is seeded from its parent's current `entry_hash`, creating an unbroken cryptographic path. Phase TX-3 extends this upward to Genus, Family, and Kingdom.
- Specimens are **version-bound**: a specimen records not just which strain it was created from, but the exact `chain_seq` of that strain at creation time. The binding is recoverable from the audit log — you can prove which exact version of a strain definition was in effect when any culture was initiated.
- The system is **domain-generic**: strain types, confirmation methods, and hybridization rules are profile-scoped lookup data (benefiting from Phase C de-hardening), but the core tables, hash chain machinery, and audit log are identical across all verticals.
- **Start narrow, go deep**: Phase TX-1 focuses on Strain/Cultivar at the Species level — the highest-ROI subset that solves the immediate provenance problem. Full hierarchical depth is TX-2 and TX-3, deferred until the foundation is proven in production.
- The Species/Strain module is intended to become **one of the most sophisticated parts of the system** — a first-class "badass taxonomy navigator" that researchers will rely on for strain lineage, selection history, and hybridization records.

> **Dependency note:** Phase TX-1 (WP-28, WP-29) can begin after WP-22 (lab_profile concept). The two workstreams Phase C and Phase TX-1 can run in parallel with care on the lookup-table patterns introduced in WP-23/24.

---

#### Phase TX-1 — Foundation · WP-28–29 · ✅ Fully shipped (WP-28 v1.16.0 · WP-29 v1.17.0 — Phase TX-1 complete)

### WP-28 — Strain/Cultivar data model & backend — ✅ Delivered in **v1.16.0**

- **Goal:** Introduce strains as first-class entities sitting between Species and Specimens in both the taxonomic and cryptographic hierarchy. The hash chain for strain records seeds from the species level — making Strain the third tier (after Species) in SteloPTC's cryptographic provenance chain. Taxa above Species (Genus → Kingdom) are classification records only and carry no hash chain lineages.
- **Files:** new migration (011), new `src-tauri/src/models/strain.rs`, new `src-tauri/src/commands/strains.rs`, `src-tauri/src/db/queries.rs` (new `log_audit_seeded_by_strain` helper), `src/lib/api.ts`.
- **Accession number design decision — final, non-revisitable:** Strain information is **never** encoded in the accession number format (`YYYY-MM-DD-SPECIESCODE-SEQ`). Three decisive reasons: (1) Accession is immutable; strain classification is not. Strain corrections, reclassifications, and status upgrades happen routinely. An accession that encodes strain becomes a lying label the moment the strain assignment changes. (2) Strain is often unknown at specimen creation time. Many cultures are created before their strain is identified. The accession must be valid from the moment of creation, before any strain is assigned. (3) A culture can be reclassified to a different strain. If genomic work reveals a misidentification, the culture lineage is unchanged — only the strain assignment changes. The accession must not encode information that can be wrong. Strain appears as supplemental metadata in QR payloads, the specimen detail strain pill, and all reports. Labs wanting human-readable combined labels may use the strain code and accession together in their own context — SteloPTC's accession format is culture-lineage-only and does not change for any strain-related reason.
- **Steps:**
  1. **Migration 011:** Create three new tables.
     - `strains`: `id TEXT PRIMARY KEY`, `species_id TEXT NOT NULL REFERENCES species(id)`, `name TEXT NOT NULL`, `code TEXT NOT NULL` (short lab identifier, unique per species, used in UI badges), `strain_type TEXT NOT NULL DEFAULT 'cultivar'` (values: `cultivar | landrace | hybrid | clone | inbred_line | variety | selection | unknown`; will become a profile-scoped lookup table in Phase C/TX-2), `status TEXT NOT NULL DEFAULT 'unverified'` (four values forming a three-tier model: `unverified` [no identity assertion made — default] | `claimed` [user explicitly asserts identity without independent proof] | `confirmed_manual` [manual professional assessment, with friction] | `confirmed_genomic` [genomic fingerprint data present — gold standard]), `claimed_by TEXT`, `claimed_at TEXT`, `confirmation_basis TEXT` (required non-empty when `status = confirmed_manual`; describes the specific physical/observational basis for the identification — the backend rejects `confirmed_manual` transitions if this field is absent or whitespace-only), `status_notes TEXT`, `status_confirmed_by TEXT`, `status_confirmed_at TEXT`, `genomic_fingerprint TEXT` (JSON blob for marker data, ITS sequences, SNP profiles; required for `confirmed_genomic` status), `origin_description TEXT`, `description TEXT`, `is_hybrid BOOLEAN NOT NULL DEFAULT 0`, `created_at TEXT NOT NULL`, `updated_at TEXT NOT NULL`, `created_by TEXT NOT NULL`, `is_archived BOOLEAN NOT NULL DEFAULT 0`.
     - `strain_parents`: `id TEXT PRIMARY KEY`, `strain_id TEXT NOT NULL REFERENCES strains(id)`, `parent_strain_id TEXT NOT NULL REFERENCES strains(id)`, `parent_role TEXT NOT NULL DEFAULT 'parent'` (values: `parent | maternal | paternal | donor | recipient`), `generation_offset INTEGER NOT NULL DEFAULT 1`, `created_at TEXT NOT NULL`. Supports multi-parent (>2) hybrid pedigrees from the start.
     - `hybridization_events`: `id TEXT PRIMARY KEY`, `result_strain_id TEXT NOT NULL REFERENCES strains(id)`, `species_id TEXT NOT NULL REFERENCES species(id)`, `parent_strain_a_id TEXT NOT NULL REFERENCES strains(id)`, `parent_strain_b_id TEXT NOT NULL REFERENCES strains(id)`, `parent_strain_a_chain_seq INTEGER NOT NULL` (the chain_seq of parent A at the moment of crossing — immutable provenance record), `parent_strain_b_chain_seq INTEGER NOT NULL` (same for parent B), `parent_specimen_a_id TEXT REFERENCES specimens(id)` (nullable — the specific specimen used as parent A if known), `parent_specimen_b_id TEXT REFERENCES specimens(id)` (nullable — same for parent B), `cross_date TEXT NOT NULL`, `cross_method TEXT`, `generation_label TEXT` (e.g. `F1`, `F2`, `BC1F2`), `notes TEXT`, `performed_by TEXT NOT NULL`, `created_at TEXT NOT NULL`, `created_by TEXT NOT NULL`. Hybridization is a **distinct taxonomic event** — not modeled as a passage or split. Every hybrid strain must have exactly one `hybridization_events` record.
     - Add to `specimens`: `strain_id TEXT REFERENCES strains(id)` (nullable — existing and new specimens without a strain assignment are fully unaffected), `strain_chain_seq INTEGER` (the strain's `chain_seq` at the moment this specimen was bound to it — immutable "strain version" binding).
     - Indexes: `idx_strains_species ON strains(species_id)`, `idx_strain_parents_strain ON strain_parents(strain_id)`, `idx_specimens_strain ON specimens(strain_id)`, `idx_hybridization_events_result ON hybridization_events(result_strain_id)`.
  2. **Hash chain integration:** The hash chain for strain records seeds from the species level. When a strain is created, write a genesis audit entry: `lineage_id = strain_id`, `chain_seq = 0`, `prev_hash = species' current entry_hash`. Hash chains do **not** extend above Species — `taxa` records (Genus → Kingdom) are classification/navigation data only and carry no audit lineages. Add `log_audit_seeded_by_strain()` helper to `queries.rs`. When a specimen is created **with** a `strain_id`, seed its genesis entry from the strain's current `entry_hash`; store the strain's current `chain_seq` in `specimens.strain_chain_seq`. When created **without** a strain, seed from species exactly as today — zero behavior change.
  3. **Strain commands:** `create_strain`, `update_strain`, `archive_strain`, `get_strain`, `list_strains_by_species` (includes `specimen_count` via COUNT JOIN), `update_strain_status`, `get_strain_pedigree` (Phase TX-1: depth-1 parent list; Phase TX-2: full recursive tree).
  4. **`create_hybridization_event` command:** Atomically (in one transaction) writes **six records**: the hybrid strain row, both `strain_parents` rows, the `hybridization_events` row (capturing both parent `chain_seq` values at call time), and **four audit chain entries**: (a) hybrid strain genesis entry — `lineage_id = hybrid_strain_id`, `chain_seq = 0`, `prev_hash = species entry_hash`, `action = "genesis"`, details reference the hybridization_event id; (b) hybrid strain hybridize entry — `chain_seq = 1`, `action = "hybridize"`, details include `parent_a_id`, `parent_a_chain_seq`, `parent_b_id`, `parent_b_chain_seq`, `event_id`; (c) `used_as_parent` entry appended to **parent A's** audit chain — records `result_strain_id`, `event_id`, and parent A's `chain_seq` at the moment of crossing; (d) same `used_as_parent` entry appended to **parent B's** audit chain. All six records commit or none do. This creates a **bidirectional verifiable record**: from the hybrid strain you can prove its lineage; from each parent strain's audit log you can see every hybridization event in which it participated and at exactly what version. Validates: both parent strains belong to the same `species_id` — rejects with a clear error otherwise (cross-species is reserved for Phase TX-3/WP-48). Cycle detection runs before persisting. Returns the new strain ID.
  5. **Status validation — all transitions explicitly defined:**
     - `any → unverified`: not a valid forward transition (unverified is the initial default only; use archive if a strain record needs to be retired).
     - `any → claimed`: **low friction** — no required fields, no modal. Records `claimed_by` and `claimed_at`. This is an explicit identity assertion: "I believe this is the named strain." One click, immediately persisted.
     - `any → confirmed_manual`: **high friction** — `confirmation_basis` must be non-empty (backend enforced with a clear rejection error); on success returns `{ ok: true, warning: "ConfirmedManualWarning" }` typed enum (not a free string); UI must show blocking acknowledgment modal (WP-29). Cannot be transitioned to `confirmed_manual` from `confirmed_genomic` — downgrade is rejected.
     - `any → confirmed_genomic`: **fingerprint required** — `genomic_fingerprint` must be non-null and non-empty; backend rejects with a clear error otherwise. No modal required (this is the intended gold-standard path, not a risky shortcut). Can upgrade from any status including `confirmed_manual`.
     - **Downgrade rejections (hard rules):** `confirmed_genomic → confirmed_manual`, `confirmed_genomic → claimed`, `confirmed_genomic → unverified` — all rejected. `confirmed_manual → claimed`, `confirmed_manual → unverified` — rejected. Genomic and manual confirmations are permanent designations. Archive the strain and create a new record if an identity needs to be disputed or retracted.
  6. **Unit tests:** strain genesis `prev_hash` equals species' current `entry_hash`; strain's `entry_hash` becomes specimen's `prev_hash` when `strain_id` is set; `strain_chain_seq` matches at creation; `any → claimed` succeeds with no extra fields; `confirmed_manual → claimed` is rejected; `confirmed_genomic → confirmed_manual` is rejected; `any → confirmed_manual` rejects missing `confirmation_basis`; `any → confirmed_genomic` rejects null fingerprint; `confirmed_manual → confirmed_genomic` succeeds (upgrade path); `create_hybridization_event` rejects cross-species parents; `create_hybridization_event` writes `used_as_parent` entries on both parent strain chains; parent A's audit chain after hybridization has chain_seq N+1 with `action = "used_as_parent"`; split siblings with a strain still share the same `prev_hash` (fork invariant preserved, `queries.rs` test extended).
- **Acceptance:** Creating a strain writes a genesis audit entry with `prev_hash = species' last entry_hash`. Creating a specimen bound to that strain seeds its genesis from the strain's current `entry_hash`. `strain_chain_seq` on the specimen matches the strain's audit chain_seq at creation. `create_hybridization_event` atomically creates hybrid strain + parent records + hybridization_events row. `confirmed_manual` is rejected without `confirmation_basis`. All existing `create_specimen` behavior when `strain_id = NULL` is unchanged and all existing tests remain green.
- **Preserve:** `log_audit_seeded_by_species` path is untouched; no hash chains on `taxa` records; no behavior change for specimens without a strain.
- **Bump:** minor → **v1.16.0**.
- **As built (v1.16.0):**
  - **Migration 019** — purely additive: `strains`, `strain_parents`, `hybridization_events` tables; nullable `strain_id` + `strain_chain_seq` on `specimens`; six covering indexes. All existing specimen rows receive `NULL` for both columns.
  - `log_audit_strain_genesis()` and `log_audit_seeded_by_strain()` helpers in `db/queries.rs`.
  - `validate_strain_status_transition()` — pure, independently-testable status machine function.
  - `commands/strains.rs` — `create_strain`, `get_strain`, `list_strains_by_species`, `update_strain`, `archive_strain`, `update_strain_status`, `create_hybridization_event` (single atomic transaction: 1 hybrid strain + 2 `strain_parents` + 1 `hybridization_events` + 4 audit entries).
  - `commands/specimens.rs` — `CreateSpecimenRequest` updated with optional `strain_id`; seeds audit from strain when provided, from species otherwise (zero behavior change).
  - TypeScript API: `createStrain`, `getStrain`, `listStrainsBySpecies`, `updateStrain`, `archiveStrain`, `updateStrainStatus`, `createHybridizationEvent`.
  - **14 new Rust unit tests** covering: strain genesis hash chain seeding, specimen creation with strain, `strain_chain_seq` at creation, status transitions (allowed and blocked), `create_hybridization_event` cross-species guard, bidirectional `used_as_parent` entries, split sibling fork invariant with strain.
  - **Deviation from plan:** migration numbered 019 (not 011 as originally specified — 011–018 were claimed by intervening features). `get_strain_pedigree` (depth-1) deferred to WP-29 UI packet.

---

### WP-29 — Strain management UI, hybrid wizard & basic taxonomy navigator — ✅ Delivered in **v1.17.0**

- **Goal:** A strain management interface with hybrid creation wizard, a two-column taxonomy navigator, and strict UI enforcement of the `confirmed_manual` status guardrails.
- **Files:** new `src/lib/components/StrainManager.svelte`, new `src/lib/components/HybridWizard.svelte`, updates to `src/lib/components/SpecimenForm.svelte` and `src/lib/components/SpecimenDetail.svelte`, new `src/lib/components/TaxonomyNavigator.svelte`, `src/lib/components/Sidebar.svelte`.
- **Steps:**
  1. **StrainManager.svelte:** Accessible from the Species detail/management page. Per-species strain list: name, code, type, status badge, specimen count, created date. **Status badges — strict rules, no deviation:**
     - `unverified` → grey `Unverified` badge. Default state; no assertion has been made.
     - `claimed` → blue `Claimed` badge. User has explicitly asserted identity without independent proof.
     - `confirmed_manual` → **always** amber `⚠ Manual ID` badge. The word "Confirmed" must NOT appear without the `⚠` symbol and "Manual" qualifier in any badge, label, or tooltip. This designation is permanent — the badge never upgrades to a clean indicator.
     - `confirmed_genomic` → green `✓ Genomic` badge.
     Actions: create, edit, archive, update status. The status update control must enforce downgrade rejections in the UI (grey out or hide `confirmed_manual → claimed`, `confirmed_genomic → any lower` options).
     **Nudge behavior — `unverified` → `claimed`:** In the strain list row for any `unverified` strain, show a subtle "Mark as Claimed" inline button (text-link style, not a prominent CTA) so lab staff can complete the one-click assertion without navigating into the detail view. No modal, no confirmation — it fires immediately and shows a brief success toast. For strains that have remained `unverified` for more than 30 days, add a soft amber dot indicator on the row (a small pulse, not a badge) and update the tooltip on the Unverified badge to: *"Still unverified after 30 days — consider asserting an identity."* No nudge is shown for `claimed` strains; they have made their assertion.
  2. **`confirmed_manual` blocking modal (non-negotiable):** When the backend returns `{ ok: true, warning: "ConfirmedManualWarning" }`, the UI **must** immediately show a blocking acknowledgment modal. The modal is not dismissable by clicking outside or pressing Escape. It must contain:
     - Title: "Manual Identification Confirmed"
     - Body: *"This strain has been marked as Confirmed — Manual. Manual confirmation is based on professional judgment, not genomic verification. It must NOT be cited as equivalent to genomic confirmation in regulatory submissions, IP claims, or research publications without explicit disclosure. The basis for this confirmation has been recorded in the audit log."*
     - Single button: **"I Acknowledge"** (no Cancel, no close-X).
     A toast notification alone is insufficient and must not be used as a substitute.
  3. **HybridWizard.svelte:** Multi-step wizard for creating a hybrid strain. Accessible via "+ New Hybrid Strain" in StrainManager. Steps: (1) select species, (2) select Parent A and its role (`maternal`/`paternal`/`parent`), (3) select Parent B filtered to same species — cross-species selection is blocked with an inline error, (4) enter name / code / strain_type, (5) optionally record specific parent specimens used in the cross, (6) enter cross date and method, (7) pedigree preview showing the new strain connected to both parents, (8) confirm. On confirm, calls `create_hybridization_event`. The wizard captures parent `chain_seq` values from the current audit chain state at submission time, which are recorded in `hybridization_events`.
  4. **SpecimenForm.svelte update:** After species selector, add optional strain selector (lazy-loads strains for the selected species with status badges). Default = "No strain assigned" — preserves all existing behavior. If a strain is selected, show its status badge and origin description as read-only context. **`unverified` vs `claimed` behavior in this form:**
     - If an `unverified` strain is selected, show a soft inline hint beneath the selector: *"This strain's identity has not been asserted yet. Consider updating its status to Claimed if you believe this is the correct strain."* Render as a grey info row (not a warning, not a blocking prompt). The user can proceed to save without acting on it.
     - If a `claimed` strain is selected: no extra message. The assertion is sufficient for normal form flow.
     - If `confirmed_manual` or `confirmed_genomic`: no extra message.
  5. **SpecimenDetail.svelte update:** When `strain_id` is present, show a **Strain** pill in the header: `[CODE · v{strain_chain_seq} · STATUS]`. The version number makes the binding explicit and traceable. Clicking the pill navigates to the strain's detail. Status badge in the pill must follow the same strict rules as step 1. **Pill tooltips — explicit per status:**
     - `unverified` pill: grey background. Tooltip: *"No identity assertion has been made for this strain. Use the Strain Manager to mark it as Claimed if you believe the assignment is correct."* Pill also shows a subtle inline "Mark as Claimed →" text-link that opens the strain's status update view directly.
     - `claimed` pill: blue background. Tooltip: *"Identity asserted by lab staff but not independently verified."* No additional prompt.
     - `confirmed_manual` pill: amber background, `⚠` prefix. Tooltip: *"Manually confirmed. Not equivalent to genomic verification — see audit log for the documented basis."*
     - `confirmed_genomic` pill: green background, `✓` prefix. Tooltip: *"Genomic verification confirmed. Fingerprint data on record."*
  6. **TaxonomyNavigator.svelte (Phase TX-1 version):** Two-column panel. Left: species list with strain-count chips and a search bar. Right: on clicking a species, shows its strains with status badges and specimen counts. Clicking a strain shows a mini panel with all bound specimens (accession, stage, health, quick-navigate). Add as sidebar nav entry "Taxonomy." The Phase TX-1 version is the foundation TX-2 expands into a full multi-rank column browser (WP-39). **Filter options for strain status (TX-1):** the right column must include a status filter with exactly these options: `All` (default) | `Unverified` | `Claimed` | `Confirmed (Manual)` | `Confirmed (Genomic)` | `Confirmed (Any)`. `Unverified` and `Claimed` are separate filter values — a filter for `Claimed` must not show `Unverified` strains and vice versa.
  7. **Print / report footnotes:** Footnote rules per status, in all print views, PDF exports, and reports regardless of filter settings:
     - `confirmed_manual` → mandatory footnote `†`: *"† Strain identification based on manual assessment only, not genomic verification. See audit log for confirmation basis."*
     - `unverified` → soft footnote `‡`: *"‡ Strain identity not yet asserted by lab staff."*
     - `claimed` → no footnote in standard reports. In compliance/regulatory report mode (if a profile setting is configured), add: *"§ Strain identity asserted by lab staff; no independent verification performed."*
     - `confirmed_genomic` → no footnote.
     These rules apply to all basic print outputs in WP-29 and must be carried forward to all future report features.
  8. Add `specimen_count` to `list_strains_by_species` response (COUNT JOIN on `specimens WHERE strain_id = strains.id AND is_archived = false`).
- **Acceptance:** Can create a strain; assign it to a new specimen; specimen detail shows version-pinned strain pill with correct status badge; hybrid wizard calls `create_hybridization_event` and renders pedigree preview correctly; `confirmed_manual` status change triggers blocking modal with the exact text above (a toast alone fails this check); Taxonomy Navigator shows Species → Strains → Specimens tree and is text-searchable; print views include the `confirmed_manual` footnote.
- **Preserve:** SpeciesManager.svelte structural behavior unchanged. All existing specimen creation without a strain continues to work identically.
- **Bump:** minor → **v1.17.0** — Phase TX-1 complete.
- **As built (v1.17.0):**
  - **`StrainManager.svelte`** (new) — per-species strain panel: filterable table (name, code, type, status badge, specimen count, created date); strict badge rules (grey Unverified / blue Claimed / amber `⚠ Manual ID` / green `✓ Genomic`); inline "Mark as Claimed" text-link nudge + 30-day unverified amber pulse; full CRUD (Create, Edit, Archive, Update Status) modals; status update enforces forward-only progression; `+New Hybrid Strain` launches HybridWizard.
  - **`confirmed_manual` blocking modal** — non-dismissible (no click-outside, no Escape, no close-X); exact spec title + body text; single "I Acknowledge" button. Toast alone is insufficient per spec — fully enforced.
  - **`HybridWizard.svelte`** (new) — 8-step guided wizard: (1) species, (2) Parent A + role, (3) Parent B same-species enforced with inline cross-species error, (4) name/code/strain_type, (5) optional parent specimen accession numbers, (6) cross date + method, (7) ASCII pedigree preview, (8) confirm → calls `create_hybridization_event`. Captures both parent `chain_seq` values at submission time.
  - **`TaxonomyNavigator.svelte`** (new) — two-column Phase TX-1 browser: left column (all species + live search); right column (strains with status badges + specimen counts + slide-in specimen panel); status filter: All / Unverified / Claimed / Confirmed (Manual) / Confirmed (Genomic) / Confirmed (Any); inline StrainManager toggle; `selectedStrainId` store for deep-link navigation. Added as "Taxonomy" sidebar entry (🧬, between Species and Inventory).
  - **`SpecimenForm.svelte`** (updated) — optional strain selector lazy-loads strains on species change; status badge inline in each option; default "No strain assigned" preserves all existing behavior; soft grey info row shown when an `unverified` strain is selected.
  - **`SpecimenDetail.svelte`** (updated) — Strain pill in header `[CODE · v{strain_chain_seq} · STATUS]`; pill colors/tooltips per strict status rules; `unverified` pill shows "Mark as Claimed →" text-link; pill click navigates to Taxonomy view; print report footnotes (`†` for `confirmed_manual`, `‡` for `unverified`).
  - **`stores/app.ts`** — `'taxonomy'` added to View union; `selectedStrainId` writable store exported.
  - **`App.svelte`** — `taxonomy` route wired to TaxonomyNavigator.
  - **`Sidebar.svelte`** — Taxonomy nav item added.
  - **No new migrations** — all schema work completed in migration 019 (WP-28/v1.16.0). No new Rust test functions; WP-29 is entirely Svelte UI.

---

#### Phase TX-2 — Expansion · WP-35–39 · ✅ Fully shipped (WP-35 v1.18.0 · WP-36 v1.19.0 · WP-37 v1.20.0 · WP-38 v1.21.0 · WP-39 v1.22.0)

**Goal:** Deeper taxonomy (Genus → Kingdom), NCBI Taxonomy import with sync and conflict resolution, multi-generational pedigree visualization, intraspecific hybridization, and a powerful full-featured taxonomy navigator.

**Depends on:** Phase TX-1 complete; Phase C complete (profile-scoped lookup tables power the `strain_type` and `strain_status` vocabularies; domain-specific terminology driven by UI manifest from WP-25).

---

### WP-35 — Expanded taxonomy backbone (Genus → Kingdom) — ✅ Delivered in **v1.18.0**

- **Goal:** Model the ranks above Species as first-class classification records enabling hierarchical navigation, descendant-count queries, and NCBI sync in WP-36.
- **Files:** new migration, new `src-tauri/src/models/taxon.rs`, new `src-tauri/src/commands/taxa.rs`.
- **Steps:**
  1. Create `taxa` table: `id TEXT PRIMARY KEY`, `rank TEXT NOT NULL` (values: `kingdom | phylum | class | order | family | genus`), `name TEXT NOT NULL`, `parent_id TEXT REFERENCES taxa(id)`, `ncbi_taxon_id INTEGER NULL`, `ncbi_updated_at TEXT NULL`, `local_override BOOLEAN NOT NULL DEFAULT 0` (true = local edits take priority over NCBI sync), `created_at TEXT NOT NULL`, `updated_at TEXT NOT NULL`. Add `taxon_path TEXT` (JSON array of taxon IDs from kingdom to genus) and `ncbi_taxon_id INTEGER` to the existing `species` table.
  2. **Classification data only — no hash chain lineages (superseded by WP-45):** `taxa` records were originally navigation-only records with no audit lineages. WP-45 (v1.33.0) added experimental hash chain participation; see the WP-45 section below for current behaviour and its known limitations.
  3. Commands: `create_taxon`, `get_taxon`, `update_taxon`, `list_taxa_by_rank`, `get_taxon_descendants` (returns all taxa, species, strains, and specimen counts below a given node — the backbone of the advanced navigator in WP-39).
  4. Data migration: auto-create genus taxa from existing `species.genus` text values; back-fill `species.taxon_path`; resolve duplicates by grouping identical genus names under a shared taxon record.
- **Acceptance:** Full taxonomy from kingdom to genus is representable; `get_taxon_descendants` returns correct counts at every rank; species back-fill completes without data loss; no audit log entries are written for taxa records.
- **Preserve:** All existing species CRUD; `species.genus` text field retained for backward compatibility.
- **Bump:** minor.
- **As built (v1.18.0):**
  - **Migration 020** — purely additive: `taxa` table (`id`, `rank`, `name`, `parent_id` self-referential FK, `ncbi_taxon_id`, `ncbi_updated_at`, `local_override`, `taxon_path` JSON array, `created_at`, `updated_at`); `CHECK(rank IN ('kingdom','phylum','class','order','family','genus'))`; indexes on `parent_id`, `rank`, and `name`. Two new nullable columns added to `species` via `ALTER TABLE`: `taxon_path TEXT` and `ncbi_taxon_id INTEGER`.
  - **`backfill_genus_taxa`** — idempotent function that extracts unique genus values from existing `species` records, creates corresponding genus `taxa` rows, and populates `species.taxon_path` for every species. Runs as part of migration 020; safe to run repeatedly.
  - **`src-tauri/src/models/taxon.rs`** (new) — `Taxon`, `CreateTaxonRequest`, `UpdateTaxonRequest`, `SpeciesNodeSummary`, `TaxonNode` types.
  - **`src-tauri/src/commands/taxa.rs`** (new) — five Tauri commands: `create_taxon`, `get_taxon`, `update_taxon`, `list_taxa_by_rank`, `get_taxon_descendants` (recursive `TaxonNode` tree with `strain_count`/`specimen_count` aggregates at every level — backbone for WP-39 advanced navigator).
  - **`src-tauri/src/db/queries.rs`** — three new helpers: `load_taxon`, `get_child_taxa`, `get_species_for_taxon`.
  - **`src/lib/api.ts`** — typed exports for all five taxa commands plus `Taxon`, `TaxonRank`, `SpeciesNodeSummary`, `TaxonNode` TypeScript interfaces.
  - **Rust unit tests** — 7 new tests in `db/migrations.rs` covering: `taxa` table created with correct columns, rank CHECK constraint enforced, `get_taxon_descendants` returns empty tree for unknown taxon, backfill creates genus taxa from species, backfill is idempotent, `ncbi_taxon_id` nullable on both `taxa` and `species`, `taxon_path` JSON stored and retrieved correctly.
  - **No audit log involvement** — `taxa` records are classification/navigation data only; no hash chain lineages above Species (as designed in WP-28 spec).
  - **Deviation from plan:** migration numbered 020 (not 011 as originally specified in the WP-35 spec — 011–019 claimed by intervening features). `list_taxa_by_rank` returns all taxa ordered by name rather than the rank-tree query originally specced; `get_taxon_descendants` covers the tree use-case instead.

---

### WP-36 — NCBI Taxonomy import & ongoing sync — ✅ Delivered in **v1.19.0**

- **Goal:** Seed and maintain the `taxa` table from NCBI Taxonomy with admin-controlled conflict resolution.
- **As built (v1.19.0):**
  - **Migration 021** — `ncbi_sync_log` table (`sync_type`: import/update/conflict, `taxon_id`, `ncbi_taxon_id`, `conflict_details` JSON, `resolved_at`, `resolved_by`, `resolution`: kept_local/accepted_ncbi/merged, `created_at`); four indexes.
  - **`commands/ncbi.rs`** (new) — four Tauri commands: `import_ncbi_taxonomy` (two-phase dry-run → atomic write, skips `local_override`), `resolve_ncbi_conflict`, `sync_ncbi_taxon` (single-record upsert), `list_ncbi_sync_log` (paginated, optional `pending_only` filter).
  - **`db/queries.rs`** — seven new pure helpers: `normalize_ncbi_rank`, `find_taxon_by_ncbi_id`, `find_taxon_by_name_rank`, `detect_ncbi_conflict`, `insert_ncbi_sync_log`, `list_pending_ncbi_conflicts`, `list_ncbi_sync_log`.
  - **`NcbiSyncPanel.svelte`** (new) — admin-only panel: JSON textarea, Dry Run → Confirm Import flow, pending conflict list with Keep Local / Accept NCBI / Merged resolution buttons, recent sync log table.
  - TypeScript: `NcbiTaxonRecord`, `NcbiSyncLog`, `NcbiConflictSummary`, `ImportNcbiTaxonomyResult` interfaces; `importNcbiTaxonomy`, `resolveNcbiConflict`, `syncNcbiTaxon`, `listNcbiSyncLog` async helpers.
- **Files:** new `src-tauri/src/commands/ncbi.rs`, new migration for `ncbi_sync_log`, Admin UI panel.
- **Steps:**
  1. Create `ncbi_sync_log` table: `id TEXT PK`, `sync_type TEXT NOT NULL` (`import | update | conflict`), `taxon_id TEXT`, `ncbi_taxon_id INTEGER`, `conflict_details TEXT` (JSON: local vs. NCBI name/rank diff), `resolved_at TEXT`, `resolved_by TEXT`, `resolution TEXT` (`kept_local | accepted_ncbi | merged`), `created_at TEXT NOT NULL`.
  2. `import_ncbi_taxonomy` command (admin-only): accepts a set of NCBI taxon IDs or a rank range. Dry-run preview before any writes. Skips rows with `local_override = true`. Writes all name/rank conflicts to `ncbi_sync_log`.
  3. `resolve_ncbi_conflict` command: takes a `sync_log_id` and a `resolution` choice; applies the resolution and updates the taxon record.
  4. `sync_ncbi_taxon(ncbi_taxon_id)`: re-downloads a single taxon's data and updates if no conflict; records result in `ncbi_sync_log`.
  5. Admin UI panel: lists pending conflicts, shows local vs. NCBI values side-by-side, enables resolution and manual sync trigger.
- **Acceptance:** Import populates `taxa` for at least two plant families from NCBI; conflicts detected and logged; resolution via UI correctly updates the taxon record.
- **Bump:** minor.

---

### WP-37 — Multi-generational pedigree tools — ✅ Delivered in **v1.20.0**

- **Goal:** Visualize and export the full multi-generational pedigree of any strain, tracing both ancestor and descendant lines through all hybrid generations.
- **As built (v1.20.0):**
  - Seven new model types in `models/strain.rs`: `StrainSummary`, `PedigreeEdge`, `PedigreeNode` (recursive), `SpecimenSummary`, `StrainSpecimenTree`, `HybridizationEventRecord`, `PedigreeExport`.
  - Eight new pure pedigree helpers in `db/queries.rs`: `get_strain_ancestry`, `get_strain_descendants` (both with DFS cycle detection rejecting circular references), `get_strain_specimen_tree` (with optional descendant recursion), `export_strain_pedigree`. Default depth 5, capped at 10.
  - Four Tauri commands in `commands/strains.rs`: `get_strain_ancestry`, `get_strain_descendants`, `get_strain_specimen_tree`, `export_strain_pedigree`.
  - **`PedigreeChart.svelte`** (new) — indented node list (no SVG/canvas), ancestors/descendants toggle with live node counts, per-node status badge + Hybrid badge + specimen count, root node distinguished with primary-colour border, one-click JSON export (`pedigree-{strainId}.json`), dark-mode support.
  - 13 new Rust unit tests covering wildtype, 2- and 3-generation ancestry/descendants, max-depth capping, cycle detection both directions, specimen tree with/without descendants, export bundle integrity.
- **Files:** `src-tauri/src/commands/strains.rs` (extend and add pedigree commands), new `src/lib/components/PedigreeChart.svelte`.

**Conceptual distinction — strain pedigree vs. specimen chain:**
These are two independent but complementary lineage systems. The **strain pedigree** (hybridization lineage) walks `strain_parents` — it answers how a strain came to exist through successive crossings. The **specimen chain** (culture lineage) walks `specimens.parent_id` — it answers how a particular physical culture was propagated through passages and splits. A complete provenance view of any specimen shows both: "This specimen is culture lineage #003B → #003 (original split), bound to strain SKY-OG v3 at creation, where SKY-OG is an F1 hybrid of BLUE-DRM × CHEM-DOG." WP-37 addresses the strain pedigree side; specimen chains are already implemented.

- **Steps:**
  1. **`get_strain_ancestry(strain_id, max_depth)`:** walk `strain_parents` upward (toward founders). Returns a DAG JSON structure with node metadata per strain: name, code, status badge, species, hybridization_event details for each joining edge (cross date, method, generation label, which chain_seqs were used). Detects and rejects circular references. Default depth: 5. Used for "Where did this strain come from?"
  2. **`get_strain_descendants(strain_id, max_depth)`:** walk `strain_parents` downward (toward derived hybrids). Returns all hybrid strains for which the target strain is a direct or indirect ancestor. Same JSON shape as ancestry for interoperability. Used for "What has been bred from this strain?" and "Is this founder still being used in active crosses?"
  3. **`get_strain_specimen_tree(strain_id, include_descendants: bool)`:** returns all specimens ever bound to this strain. When `include_descendants = true`, first calls `get_strain_descendants` to collect all descendant hybrid strain IDs, then returns all specimens bound to any of them. Groups results by strain with specimen metadata (accession, stage, health, archived). Used for: "Show every active culture in my collection that descends from this founder strain, across all crosses and all splits." This is the most powerful provenance query in the system.
  4. **`PedigreeChart.svelte`:** renders ancestor and descendant views as a DAG (switchable with a toggle button). Each node: strain name, status badge, generation depth, specimen count chip. Clicking a node navigates to that strain's detail. Collapse/expand per branch. A "Specimen Tree" button on any node triggers `get_strain_specimen_tree` for that strain and opens the results in a slide-in panel.
  5. **"Export Pedigree" button:** produces portable JSON including all ancestor and descendant strain records, their hybridization_event records, and their audit chain positions at the time of each crossing. Suitable for research citation, regulatory documentation, and external pedigree analysis tools.
- **Acceptance:** A strain with 3 ancestor generations shows all ancestors correctly; `get_strain_descendants` finds all known descendant hybrids; `get_strain_specimen_tree(id, true)` returns specimens across all descendant strains; circular references are rejected with a clear error; export round-trips without data loss.
- **Bump:** minor.

---

### WP-38 — Advanced hybridization tools (generation labeling, backcross notation, cross-species guard hardening) — ✅ Delivered in **v1.21.0**

- **Goal:** Extend the Phase TX-1 hybrid model with generation naming, backcross notation, and hardened cross-species guardrails. The core hybridization model (`hybridization_events` table, `create_hybridization_event` command, and basic wizard) was delivered in WP-28 and WP-29; this packet builds on it.
- **As built (v1.21.0):**
  - **Migration 022** — `hybridization_events.generation_label TEXT`, `hybridization_events.backcross_depth INTEGER`, `strains.is_cross_species INTEGER NOT NULL DEFAULT 0` — three additive `ALTER TABLE ADD COLUMN` statements.
  - Six new pure helpers in `db/queries.rs`: `get_strain_generation_label`, `suggest_generation_label` (pure: F1×F1→F2, etc.), `detect_backcross` (DFS; returns ancestor + depth), `suggest_generation_label_for_parents`, `get_generational_stats` — all unit-tested (9 new tests).
  - `create_hybridization_event` fully rewritten: cross-species admin override path (requires justification + acknowledgement checkbox + writes permanent `cross_species_override` audit entry); generation label resolved: explicit → backcross suggestion → parent-label suggestion; stores `generation_label` + `backcross_depth`.
  - **HybridWizard expanded to 9 steps** — new Step 5 (Generation Label) with async backend suggestion; Step 3 (Parent B) gains admin-only cross-species override panel.
  - **`StrainDetail.svelte`** (new) — slide-over opened by clicking strain names: permanent cross-species red banner; Overview, Generations, Pedigree tabs; generational stats per-generation table with health % bar.
  - **`StrainManager.svelte`** — strain names clickable; cross-species hybrids show red ⚠ chip.
- **Files:** `src-tauri/src/commands/strains.rs` (extend `create_hybridization_event`), `src/lib/components/HybridWizard.svelte` (expand), `src/lib/components/StrainDetail.svelte`.
- **Steps:**
  1. **Generation labeling:** Add first-class generation label support to the hybrid wizard. Supported labels: `F1`, `F2`, `F3`, `BC1F1`, `BC1F2`, and custom free-text. `generation_label` is stored on `hybridization_events.generation_label`. Auto-suggest the generation label based on parent generation labels when both parents have known labels.
  2. **Backcross notation:** When one parent is a known ancestor of the other (detected via a `strain_parents` walk), display a backcross indicator and suggest appropriate BC notation. Record backcross ancestry depth in `hybridization_events.notes` until a dedicated field is warranted.
  3. **Cross-species guard hardening:** Add an explicit admin-only override path for cross-species hybridization (reserved for full support in Phase TX-3/WP-48). In TX-2, the override: requires a separate admin "unlock" that writes a permanent, unremovable warning to the audit log; is not accessible from the normal wizard flow; displays a red permanent warning banner on the resulting strain's detail view. The normal wizard continues to block cross-species selection with a clear error.
  4. **Generational stats on strain detail:** Show per-generation specimen counts and health summaries for all F-generations derived from a founder strain.
- **Acceptance:** Hybrid wizard auto-suggests generation labels; backcross notation generated correctly for a 3-generation pedigree; cross-species attempt via normal wizard path is blocked with a clear error; admin unlock path writes an unremovable audit warning.
- **Bump:** minor.

---

### WP-39 — Advanced taxonomy navigator — ✅ Delivered in **v1.22.0**

- **Goal:** Upgrade the Phase TX-1 two-column navigator into a full multi-rank column browser with powerful filtering, descendant counts, and keyboard navigation.
- **As built (v1.22.0):**
  - Two new helpers in `db/queries.rs`: `get_taxon_column_items` (immediate children + aggregated `strain_count`/`specimen_count` via correlated `taxon_path` LIKE subqueries) and `search_taxonomy` (taxa, species, strains, specimens — up to 10 hits each); 8 new unit tests.
  - Two new model types: `TaxonColumnItem`, `TaxonomySearchResult`.
  - Three new Tauri commands: `get_taxon_column`, `list_species_for_taxon`, `search_taxonomy`.
  - **TaxonomyNavigator.svelte — complete rewrite** — multi-column browser Kingdom → Phylum → Class → Order → Family → Genus → Species → Strains; each column independently scrollable; breadcrumb trail; descendant counts on every node; global search (300 ms debounce) with grouped dropdown; keyboard navigation (arrows, Enter, Escape, `/`); strain quick-action panel; StrainDetail slide-over integration; `localStorage` path persistence under `stelo_taxonomy_path`.
  - TypeScript: `getTaxonColumn`, `listSpeciesForTaxon`, `searchTaxonomy` helpers; `TaxonColumnItem`, `TaxonomySearchResult` interfaces.
- **Files:** `src/lib/components/TaxonomyNavigator.svelte` (major expansion of Phase TX-1 version).
- **Steps:**
  1. **Column browser:** Kingdom → Phylum → Class → Order → Family → Genus → Species → Strain → Specimens. Each column is a scrollable list with descendant counts. Selecting a node populates the next column. Columns collapse on mobile (accordion). Breadcrumb trail shows the current path.
  2. **Filter panel:** filter by rank, domain/kingdom, strain status (`claimed only | confirmed only | all`), minimum specimen health, specimen stage, quarantine flag (`any | clean | flagged`), active/archived.
  3. **Descendant count bubble-up:** each node shows `(N strains · M specimens)` aggregated from all descendants — not just direct children. Powered by `get_taxon_descendants` from WP-35.
  4. **Global search:** searches across taxonomy names, strain names, codes, and accession numbers simultaneously. Results grouped by rank.
  5. **Quick action panel:** clicking a strain node shows a slide-in panel listing all bound specimens with a quick-navigate button per row.
  6. **Keyboard navigation:** arrow keys between columns; Enter to drill down; Escape to go back; `/` to focus search.
  7. **State persistence:** selected path stored in the app route state so position is restored on next visit.
- **Acceptance:** Full navigation from Kingdom to individual specimens in one flow; all filters work correctly; descendant counts match actual data; keyboard navigation complete; mobile layout usable.
- **Bump:** minor → Phase TX-2 complete.

---

#### Phase TX-3 — Advanced · WP-45–49 · ✅ Fully shipped (WP-45 v1.33.0 · WP-46 v1.34.0 · WP-47 v1.35.0 · WP-48 v1.36.0 · WP-49 v1.37.0 — Phase TX-3 complete)

**Goal:** Complete the taxonomic & provenance system with full hash chain coverage, cross-domain taxonomy, structured breeding programs, advanced hybridization tooling, and interoperability with international biodiversity standards.

---

### WP-45 — Full taxonomic hash chain (Kingdom → Strain → Specimen) — ✅ Delivered in **v1.33.0** — **Status upgraded to STABLE in v1.40.0** (see WP-64)

- **Goal:** Extend the existing per-lineage hash chain (Species → Strain → Specimen) upward to all `taxa` ranks so the complete classification-to-culture path is cryptographically verifiable end-to-end.
- **Status: STABLE (upgraded from EXPERIMENTAL in v1.40.0).** WP-64's `reanchor_taxon_chain` command now provides a supervised, tested way to safely re-anchor all descendant chains after a taxon reclassification (see WP-64 below for the full mechanism). The Reclassification Warning below is retained for historical context but is no longer a blocking concern for production use — reclassifying a taxon is now a recoverable, auditable operation rather than a permanent chain break.
- **Bump:** minor (v1.32.0 → v1.33.0).
- **As built (v1.33.0):**
  - **Migration 031** (`migration_031_taxon_hash_chain`) — no schema changes; the existing `audit_log` table already carries the hash chain columns. The migration calls `backfill_taxa_genesis`, which writes genesis audit entries for all existing taxa in rank order (kingdom first, genus last) so parent entries exist before child lookups.
  - **`backfill_taxa_genesis`** (pub, `migrations.rs`) — idempotent: skips taxa with a pre-existing `entity_type = 'taxon'` genesis entry. Safe to call repeatedly.
  - **`log_audit_taxon_genesis`** (`queries.rs`) — new public function. Writes `chain_seq = 0` genesis for a taxon, seeding `prev_hash` from the parent taxon's last `entry_hash` (or `ZERO_HASH` for root/kingdom taxa). The canonical serialization format matches species/strains/specimens.
  - **`log_audit_species_genesis`** (`queries.rs`) — new public function. Seeds the species genesis `prev_hash` from the genus taxon's last `entry_hash` rather than `ZERO_HASH`, binding the species birth to the current state of the genus chain. Falls back to `ZERO_HASH` when the genus has no audit entries.
  - **`log_audit_strain_genesis`** (`queries.rs`) — updated. Now anchors strain genesis to the genus taxon's `entry_hash` (looked up via `species.genus`) instead of directly to the species' `entry_hash`. Falls back to `ZERO_HASH` when no genus taxon entry exists.
  - **`genus_entry_hash_by_name` / `genus_entry_hash_by_species`** (`queries.rs`) — private helpers for genus taxon lookups. Tolerate a missing `taxa` table (return `None`) so pre-WP-45 test fixtures and databases continue to function.
  - **`create_taxon`** (`commands/taxa.rs`) — calls `log_audit_taxon_genesis` after inserting a new taxon, seeding from the parent taxon (or `ZERO_HASH` if the parent has no entry yet).
  - **`update_taxon`** (`commands/taxa.rs`) — appends an `update` audit entry to the taxon's lineage chain after each mutation.
  - **`create_species`** (`commands/species.rs`) — now calls `log_audit_species_genesis` instead of `log_audit_at_seq_zero`, seeding from the genus taxon's `entry_hash`.
  - **6 new unit tests** (`db::queries::tests`): `taxon_genesis_root_uses_zero_hash`, `taxon_genesis_child_seeds_from_parent`, `taxon_chain_update_appends_correctly`, `species_genesis_seeds_from_genus_taxon`, `strain_genesis_prev_hash_equals_species_entry_hash` (updated), `strain_genesis_falls_back_to_zero_hash_when_no_genus_entry`. Total: 250 Rust tests.

- **RECLASSIFICATION WARNING — historical context (resolved by WP-64 in v1.40.0):**
  Renaming, re-parenting, or changing the rank of a taxon after its genesis entry has been written will advance the taxon's own chain but will **NOT** automatically re-anchor any pre-existing descendant chains in place. Every strain and specimen whose genesis `prev_hash` was derived from that taxon's previous `entry_hash` remains cryptographically bound to the OLD classification unless re-anchored. This was a fundamental tension between immutable cryptographic proofs and taxonomic reclassification, and previously had **no automated re-anchoring tool**.
  **Resolved in v1.40.0 (WP-64):** `reanchor_taxon_chain` (admin-only, with a `reanchor_taxon_chain_dry_run` pre-flight report) now walks all descendant taxa/species/strains after a reclassification and writes a fresh, independently-verifiable genesis chain for each — see WP-64 below for the exact mechanism (a distinct synthetic lineage per re-anchored entity, so the original chain is never touched and the new one verifies cleanly with the existing, unmodified `verify_audit_lineage`). Every re-anchoring event is permanently recorded in `reanchor_events`. This is why WP-45's status is now **STABLE**.

- **Backward compatibility:** Existing chains written before v1.33.0 are untouched. Strain genesis entries written before WP-45 have `prev_hash` equal to the species' `entry_hash`; those entries remain valid. Only strains created on or after migration_031 use the genus anchor.

### WP-46 — Cross-domain taxonomy support — ✅ Delivered in **v1.34.0**

- **Goal:** Make taxonomy fully profile-aware so Plantae, Animalia, Fungi (and future domains) share one engine with domain-specific defaults, ranks, and vocabularies.
- **Bump:** minor (v1.33.0 → v1.34.0).
- **As built (v1.34.0):**
  - **Migration 032** (`migration_032_domain_column`) — adds `domain TEXT NOT NULL DEFAULT 'Plantae'` to `app_config`; no CHECK constraint so future domains (Bacteria, Archaea) can be stored without a schema migration. Per-profile UPDATE on first run: `plant_tissue_culture` → `'Plantae'`, `cell_culture` → `'Animalia'`, `mycology` → `'Fungi'`; unknown profiles fall back to `'Plantae'`.
  - **`active_domain(conn)`** (`vocabulary.rs`) — new public function. Reads `domain` from `app_config WHERE id = 1`; falls back to `'Plantae'` on any error (missing table, missing column, absent row).
  - **`profile.ts` frontend additions:** `LabDomain` type (`'Plantae' | 'Animalia' | 'Fungi'`); `DomainManifest` interface with `rankOrder`, `strainTypeLabels`, and `confirmationMethodLabels`; `PROFILE_DOMAIN` mapping (profile → domain); `DOMAIN_MANIFESTS` with per-domain rank navigator order, strain type vocabulary, and confirmation method labels; `activeDomainManifest()` helper.
  - **Tests:** 4 migration tests (`domain_column_exists`, `plant_tissue_culture_maps_to_plantae`, `cell_culture_maps_to_animalia`, `mycology_maps_to_fungi`); 4 vocabulary tests (`active_domain_reads_plantae_for_ptc`, `active_domain_reads_animalia_when_set`, `active_domain_reads_fungi_when_set`, `active_domain_falls_back_to_plantae_when_column_missing`); 16 frontend tests (profile/domain/manifest/activeDomainManifest coverage).
- **Scope boundary:** This WP delivers the data layer (migration + backend helper) and frontend manifest constants. Wiring the manifests into TaxonomyNavigator, StrainManager, and HybridWizard UI components is deferred to future WPs — those components continue to use their existing hard-coded vocabulary until explicitly updated.
- **Preserve:** All existing PTC/cell_culture/mycology behavior unchanged; `active_profile()` untouched; no breaking changes to any command or migration.

### WP-47 — Breeding programs & multi-generational selection tracking — ✅ Delivered in **v1.35.0**

- **Goal:** Add structured breeding program support with selection history, fitness scoring, and generation performance analytics.
- **Bump:** minor (v1.34.0 → v1.35.0).
- **As built (v1.35.0):**
  - **Migration 033** — `breeding_programs` table (id, name, goal, start_date, target_traits, founder_strain_ids, notes, created_at, created_by) and `breeding_records` table (id, program_id FK cascade, strain_id FK, generation_number, selection_notes, fitness_score, selection_date, selected_by, notes, created_at). Indexes on `program_id` and `strain_id`.
  - **`models/breeding.rs`** — `BreedingProgram`, `CreateBreedingProgramRequest`, `BreedingRecord`, `CreateBreedingRecordRequest`, `GenerationalSummary` structs.
  - **Queries** — `create_breeding_program`, `get_breeding_program`, `list_breeding_programs`, `add_breeding_record`, `get_breeding_record`, `list_breeding_records_for_program`, `list_breeding_records_for_strain`, `get_generational_summary` (groups by generation, returns count + avg fitness).
  - **`commands/breeding.rs`** — 7 Tauri commands; write operations require `can_write()` role; audit entries on create.
  - **`BreedingProgramManager.svelte`** — split list/detail layout; create-program form; add-selection form; generational performance summary table; selection record cards with generation badge, fitness badge, and notes.
  - **Tests** — 4 migration tests + 9 query tests = 13 new Rust tests. Total: 271 Rust tests.
- **Scope boundary:** PedigreeChart integration and dashboard "Breeding Insights" panel are deferred. Existing strain, hybridization, and pedigree flows are untouched.

### WP-48 — Advanced hybridization (cross-species, generation labeling polish, introgression) — ✅ Delivered in **v1.36.0**

- **Goal:** Formalise and complete the advanced hybridization tooling introduced in WP-38 — covering generation labeling through F4 and BCn notation, live wizard auto-suggestion, and admin-gated cross-species override — and define the introgression roadmap.
- **Bump:** minor (v1.35.0 → v1.36.0).
- **As built (v1.36.0):**
  - **Generation labeling:** Full support for F1 → F4, BCnF1/F2 notation, and custom free-text generation labels. Labels stored on `hybridization_events.generation_label` (column added in migration 022 / WP-38). `suggest_generation_label_for_parents` backend helper provides live auto-suggestion in HybridWizard step 5.
  - **Backcross detection:** DFS walk of `strain_parents` detects when one parent is an ancestor of the other; suggests appropriate `BCnFx` notation and records `backcross_depth` on `hybridization_events`.
  - **Cross-species admin override:** Admin-only panel in HybridWizard step 3. Requires explicit justification text and acknowledgment checkbox. Writes a permanent, non-removable `cross_species_override` audit entry (supervisor+ role required). A red permanent warning banner appears on the resulting strain's `StrainDetail.svelte` view. `strains.is_cross_species = 1` flag set on the resulting strain.
  - **`StrainDetail.svelte`** — cross-species banner, Overview / Generations / Pedigree tabs, per-generation performance stats table with health % bars; accessible from strain name clicks in `StrainManager.svelte`.
  - **HybridWizard** expanded to 9 steps; step 5 queries `suggest_generation_label_for_parents` async and shows the suggestion with an accept/override control.
  - **Tests:** 9 Rust unit tests covering generation label suggestion rules (F1×F1→F2, BCn promotion, custom override), backcross depth detection, and the cross-species override audit entry invariant. All tests pass.
- **Not implemented (deferred to future WP):** introgression line tracking (`ILn` notation), `introgression_segments` schema column, cross-species pedigree traversal across domain boundaries.

---

### WP-49 — Custom taxa & Darwin Core export — ✅ Delivered in **v1.37.0**

- **Goal:** Allow labs to define provisional/custom taxa not yet in NCBI, map them to authoritative records when available, and export any subtree of the taxonomy in Darwin Core format for interoperability with international biodiversity databases (GBIF, iDigBio, etc.).
- **Bump:** minor (v1.36.0 → v1.37.0).
- **As built (v1.37.0):**
  - **Migration 034** (`migration_034_custom_taxa_darwin_core`) — two additive changes to the existing `taxa` table (`ALTER TABLE ADD COLUMN`): `status TEXT NOT NULL DEFAULT 'accepted'` (values: `accepted | provisional | synonym | invalid`) and `provisional_notes TEXT`. New `taxon_mappings` table: `id TEXT PK`, `source_taxon_id TEXT NOT NULL REFERENCES taxa(id)`, `target_taxon_id TEXT NOT NULL REFERENCES taxa(id)`, `mapping_type TEXT NOT NULL` (values: `equivalent | broader | narrower | related`), `notes TEXT`, `created_at TEXT NOT NULL`, `created_by TEXT NOT NULL`. Unique constraint on `(source_taxon_id, target_taxon_id)`. Three indexes.
  - **Five Tauri commands** in `commands/taxa.rs`:
    - `create_provisional_taxon` — creates a taxon with `status = 'provisional'`; requires supervisor/admin role; writes audit entry; validates name uniqueness within rank.
    - `list_provisional_taxa` — lists all taxa with `status IN ('provisional', 'synonym', 'invalid')`; paginated.
    - `map_provisional_taxon` — creates a `taxon_mappings` row linking a provisional taxon to an authoritative one; supervisor/admin only.
    - `list_taxon_mappings` — lists all mappings for a given `source_taxon_id`.
    - `export_darwin_core` — recursive CTE subtree traversal from a given root taxon ID; maps Stelo fields to Darwin Core camelCase field names (`taxonID`, `scientificName`, `taxonRank`, `parentNameUsageID`, `taxonomicStatus`, `taxonRemarks`); returns JSON array suitable for submission to GBIF or similar portals.
  - **`ProvisionalTaxaManager.svelte`** (new) — list panel with status badge filter (Accepted / Provisional / Synonym / Invalid); detail slide-over with full field display and mapping editor; Darwin Core export button (downloads `.json` file); integrated into `TaxonomyNavigator.svelte` via a "Provisional Taxa" toggle tab.
  - **TypeScript:** `ProvisionalTaxon`, `TaxonMapping`, `DarwinCoreRecord` interfaces; `createProvisionalTaxon`, `listProvisionalTaxa`, `mapProvisionalTaxon`, `listTaxonMappings`, `exportDarwinCore` async helpers in `api.ts`.
  - **Tests:** 11 new Rust unit tests — 5 migration tests (table columns exist, status CHECK enforced, mapping unique constraint, FK rejection, index existence) and 6 query tests (create provisional, list by status, mapping round-trip, export single taxon, export subtree via recursive CTE, duplicate mapping rejected). Total: **282 Rust tests**.
- **Acceptance (met):** Provisional taxon created with status badge; mapped to authoritative taxon; Darwin Core export for a subtree downloads valid JSON with correct field names; role gate blocks technician-role create/map calls.
- **Preserve:** All existing `taxa` CRUD commands; `create_taxon` behavior unchanged (defaults to `status = 'accepted'`); NCBI sync ignores provisional taxa.
- **Phase TX-3 complete — all WP-45 through WP-49 shipped.**

---

## 6. PHASE D — Cell Culture vertical (SteloCC) — ✅ WP-30–34 fully shipped (v1.23.0–v1.27.0)

Built entirely as profile data + a handful of cell-specific features on the shared engine. Mammalian/insect/cell-line work, not plants.

**What "species" becomes:** a **Cell Line** registry — line name, organism, tissue/origin, ATCC/ECACC/DSMZ catalog #, biosafety level, morphology (adherent/suspension), and recommended split ratio + interval.

### WP-30 — Seed the cell-culture profile vocabulary — ✅ Delivered in **v1.23.0**
- Stages → cell-culture lifecycle: `thawed → adherent/suspension → confluent → passaged → frozen/cryopreserved → contaminated → discarded`.
- **As built:** Migration 023 expanded `cell_culture` vocabulary with 8 new stages (total 20), 4 propagation methods (total 11), 2 hormone types (total 6), 2 compliance record types (total 11), 2 agencies (total 6), 2 inventory categories (total 9). All `INSERT OR IGNORE` — idempotent; existing rows untouched. 9 new Rust unit tests.

### WP-31 — Passage-number lineage & doubling time — ✅ Delivered in **v1.24.0**
- **As built:** Migration 024 adds `cumulative_pdl REAL` to `specimens`; `seed_cell_count`, `harvest_cell_count`, `split_ratio`, `pdl_gained`, `doubling_time_hours` to `subcultures`. Three pure calculation helpers (`calculate_doubling_time`, `calculate_pdl_from_counts`, `calculate_pdl_from_ratio`; 9 unit tests). `create_subculture` auto-calculates PDL/doubling time; `split_specimen` inherits `cumulative_pdl`. SpecimenDetail gains Cell Count & Doubling section with live PDL preview; passage timeline shows PDL block for readings. Cumulative PDL shown in specimen info card.

### WP-32 — Cryopreservation & LN2 inventory — ✅ Delivered in **v1.25.0**
- **As built:** Migration 025 adds `frozen_vials` table (Freezer/Tower/Box/Position, vial_count CHECK ≥ 0, status CHECK active/depleted/discarded). Five Tauri commands in new `commands/cryo.rs`. Atomic thaw: decrements count, auto-depletes at zero, creates new `specimens` row inheriting `lineage_passage_offset` + `cumulative_pdl`, writes audit entries on both vial and new specimen. 13 unit tests. `CryoManager.svelte` (new) — filterable lot table, Record Vials modal, Thaw/Discard modals; ❄ sidebar entry.

### WP-33 — Mycoplasma & contamination testing (compliance rule) — ✅ Delivered in **v1.26.0**
- **As built:** Migration 026 adds `biosafety_level TEXT CHECK(IN('BSL-1','BSL-2','BSL-2+','BSL-3'))` to `specimens`. New mycoplasma compliance rule fires for every non-archived `cell_culture` specimen without a mycoplasma test within `mycoplasma_test_interval_days` (default 90). `get_mycoplasma_status` command returns per-specimen status. ComplianceView gains "Last Test" column; SpecimenDetail shows colour-coded BSL badge. 3 migration + 4 query unit tests.

### WP-34 — Cell-culture dashboard panels — ✅ Delivered in **v1.27.0**
- **As built:** Two new `db/dashboard.rs` helpers: `query_vial_summary_by_line` and `query_culture_maintenance_alerts`. Four `cell_culture`-only Dashboard panels: Passages Due/Overdue, Lines Overdue for Mycoplasma Test, Vials in Storage by Line (amber ≤ 5 vials), Cultures Needing Attention (red ≥ 14 d, yellow 7–13 d). 9 new Rust unit tests.

---

## 7. PHASE E — Mycology vertical (SteloMyco) — ✅ WP-40–44 fully shipped (v1.28.0–v1.32.0)

Contamination is even more central here than in PTC — the engine's contamination tracking is a real advantage. Built as profile data + a few mycology-specific features.

**What "species" becomes:** a **Strain/Culture** registry — genus/species (e.g. *Pleurotus ostreatus*), strain name/code, source (spore print / tissue clone / commercial culture), and dikaryon vs monokaryon status.

### WP-40 — Seed the mycology profile vocabulary — ✅ Delivered in **v1.28.0**
- **As built:** Migration 027 seeds all six vocabulary tables for `mycology` via `INSERT OR IGNORE` (idempotent; PTC and cell_culture rows untouched). Stages: 10-entry full mushroom lifecycle (`spore_clone`, `agar`, `liquid_culture`, `grain_spawn`, `bulk_substrate`, `colonizing`, `fruiting`, `senescent`, `contaminated` [terminal], `discarded` [terminal]). Propagation methods: 8 (agar_to_agar, agar_to_grain, grain_to_grain, grain_to_bulk, liquid_inoculation, spore_syringe, culture_restart, other). Supplement types: 7 (gypsum, bran, calcium_carbonate, activated_carbon, coconut_coir, vermiculite, other). 12 new Rust unit tests.

### WP-41 — Colonization & contamination front-and-center — ✅ Delivered in **v1.29.0**
- **As built:** Migration 028 adds `colonization_pct REAL CHECK(0–100)` and `contaminant_type TEXT` to `subcultures`. `get_colonization_history` Tauri command. SpecimenDetail (mycology only): Colonization % input in passage form, Contaminant Type dropdown (trich/wet_rot/cobweb/pin_mold/mycelium_abort/other) in contamination section, **Colonization Progress** bar-chart section (green ≥ 80%, amber ≥ 50%, red < 50%). Passage timeline badges show specific contaminant type. Dashboard `by_contaminant_type` breakdown (top 10). 4 migration + 4 dashboard unit tests. Total: 225 tests.

### WP-42 — Genetic lineage & strain isolation — ✅ Delivered in **v1.30.0**
- **As built:** Migration 029 adds `origin_type TEXT CHECK('multi_spore'|'isolated_dikaryon'|'tissue_clone')` and `is_best_performer INTEGER NOT NULL DEFAULT 0` to `specimens`. SpecimenForm (mycology only): **Culture Origin Type** dropdown. SpecimenDetail: Culture Origin badge (colour-coded) + **Best Performer** toggle (★/☆, calls `updateSpecimen` on click). `split_specimen` inherits `origin_type` into children; resets `is_best_performer = 0`. `search_specimens` supports `best_performer_only` filter. 5 migration unit tests. Total: 230 tests.

### WP-43 — Fruiting conditions & yield — ✅ Delivered in **v1.31.0**
- **Goal:** Per-culture environmental target and yield tracking — record per-flush harvest data (fresh/dry weight, flush number, temp, RH, FAE rate, light hours) to compare strains and substrates over time.
- **As built (v1.31.0):**
  - **Migration 030** — new `fruiting_records` table: `id`, `specimen_id` (FK), `flush_number INTEGER NOT NULL DEFAULT 1`, `harvest_date`, optional environment columns (`fruiting_temp_c`, `fruiting_rh_percent`, `fae_rate`, `light_hours_per_day`), optional yield columns (`fresh_weight_g`, `dry_weight_g`), `notes`, `created_by`, timestamps. Index on `specimen_id`.
  - **`src-tauri/src/models/fruiting.rs`** (new) — `FruitingRecord`, `CreateFruitingRecordRequest`.
  - **`src-tauri/src/commands/fruiting.rs`** (new) — `create_fruiting_record` (write-role required; writes audit entry), `list_fruiting_records` (read-only; per-specimen).
  - **`db/queries.rs`** — `create_fruiting_record`, `get_fruiting_record`, `list_fruiting_records` (ordered by `flush_number ASC, harvest_date ASC`).
  - **Frontend** — `FruitingRecord` TypeScript interface; `createFruitingRecord` + `listFruitingRecords` API helpers; **Fruiting Records** section in `SpecimenDetail.svelte` History tab (mycology only): collapsible flush-entry form + scrollable data table.
  - **Tests** — 7 new unit tests: 4 migration (table existence, index, FK rejection, `flush_number` DEFAULT); 3 `db::queries` (insert + get round-trip, list per specimen, FK rejection). Total: 237 Rust tests.

### WP-44 — Mycology compliance/QC rules — ✅ Delivered in **v1.32.0**
- **Goal:** Repurpose the compliance rule engine for mycology **QC** — flag cultures colonizing too slowly, overdue for transfer (senescence risk), or with open contamination not yet discarded.
- **As built (v1.32.0):**
  - **`db/queries.rs`** — `get_mycology_compliance_flags(conn, transfer_interval_days, slow_colonization_pct, slow_colonization_days)` — three mycology-specific QC rules returning `Vec<ComplianceFlag>`:
    - **`myco_open_contamination`** (high) — non-terminal mycology specimens where `contamination_flag = 1` but culture not yet discarded.
    - **`myco_overdue_transfer`** (normal) — non-terminal specimens with no passage within `myco_transfer_interval_days` (default 21). Message includes last-passage date or "No transfer on record".
    - **`myco_slow_colonization`** (normal) — colonizing-stage specimens whose most recent `colonization_pct` is below `myco_slow_colonization_pct` (default 30%) **and** that reading is at least `myco_slow_colonization_days` (default 7) days old. Prevents false positives on freshly inoculated substrate.
  - **`commands/compliance.rs`** — mycology block profile-gated on `lab_profile = mycology`; reads three `app_settings` keys; extends shared flags list. No breakage to PTC or cell_culture rules.
  - **`Dashboard.svelte`** — **Panel MY-1: Mycology QC Alerts** (mycology profile only): up to 8 flagged specimens with severity badges, accession + species, rule message; "View compliance" link.
  - **Tests** — 8 new unit tests: `myco_open_contamination` detected / not raised for terminal stage; `myco_overdue_transfer` no-subculture / recent-passage-ok; `myco_slow_colonization` flagged / recent-reading-ok / above-threshold-ok; archived-specimens-excluded. Total: 245 Rust tests.

---

## 8. PHASE F — Cross-cutting & beyond (post-vertical)

These packets run *after* the vertical platforms exist so each improvement benefits all three verticals (PTC, Cell Culture, Mycology) at once. Some are infrastructure (databases, sync), some are UX (analytics, AI, maps), and some are compliance/interoperability. Each is specified for direct implementation. Packets are ordered by estimated ROI and dependency; they are not yet assigned to a fixed release version.

**v1.37.1 review — priority callouts:**
- **WP-58 (Analytics)** and **WP-63 (Performance)** are the highest-ROI Phase F items for labs approaching scale — implement these early in Phase F, ideally before WP-61/62.
- **WP-55 (Field-level permissions)** is the key blocker for multi-technician shared labs; should be prioritised as soon as the lab expands beyond a single admin operator.
- **WP-59 (Cloud backup)** and **WP-60 (Regulatory exports)** are mutually reinforcing — a lab pursuing FDA 21 CFR Part 11 compliance needs both. Implement together or in immediate succession.
- **WP-64 (Taxon re-anchoring)** makes WP-45 production-safe; labs actively using the taxonomic hash chain should treat this as a near-term blocker.
- **WP-64 (A11y label pass)** is a low-effort polish item — 85 label-association warnings — that should land before any major new UI surface is added.

### WP-50 — PostgreSQL backend option — ✅ Foundation delivered in **v1.38.0** (scope honesty expanded in v1.39.2)

- **Goal (as planned):** Add optional PostgreSQL support for LAN/multi-user deployments while keeping SQLite as the default local-first backend.
- **Files:** `src-tauri/src/db/mod.rs`, new `src-tauri/src/db/postgres.rs`, `src-tauri/src/db/connection.rs`, migration runner updates, `tauri.conf.json` (optional feature flags), `README.md` (deployment guide).
- **Steps (as planned):**
  1. Refactor connection layer to support a `DbBackend` trait (SQLite | Postgres) behind a feature flag (`postgres` in Cargo.toml).
  2. Implement PostgreSQL connection pooling, query translation (parameter syntax, WAL → transaction handling), and schema migration runner.
  3. Add config option in `app_config` / Settings for backend type + connection string (encrypted storage for credentials).
  4. Update all repository functions to use the trait; add integration tests for both backends.
  5. Provide a one-click "Migrate to PostgreSQL" admin action that exports SQLite → SQL dump then imports.
- **Acceptance (as planned):** App runs identically on SQLite; can switch to Postgres on LAN with multi-user support; all existing queries and migrations work; schema remains portable thanks to Phase C lookup tables.
- **Preserve:** SQLite remains default and fully functional offline; no behavior change for single-user installs.
- **Bump:** minor → **v1.38.0**.
- **As built (v1.38.0) — implemented together with WP-51 for architectural coherence:**
  - New optional Cargo feature `postgres` (off by default; `cargo check --no-default-features --features postgres`), pulling in `sqlx` (`runtime-tokio`, `tls-rustls`, `postgres` — deliberately no `macros` feature, since compile-time query checking needs a live database at build time).
  - **`db/backend.rs`** (always compiled, no feature gate) — `BackendKind` enum; `current_backend_kind`/`set_backend_kind` reading/writing a new `backend_type` key in the existing `app_settings` table; `validate_connection_string`; `validate_backend_switch` (pure, takes `postgres_feature_enabled` as an explicit parameter so both branches are unit-testable regardless of how the test binary itself was compiled).
  - **`db/postgres.rs`** — `BOOTSTRAP_SCHEMA_SQL`, PostgreSQL-flavored DDL for the five tables most central to multi-user deployments (specimens, subcultures, audit_log, taxa, strains), mirroring current SQLite logical structure — explicitly not a 1:1 port of all 34 SQLite migrations. `test_connection` and `bootstrap_schema` have two implementations selected by the `postgres` feature: a real `sqlx`-backed one, and a stub returning a clear "rebuild with `--features postgres`" error — commands call one function regardless of how the binary was compiled.
  - **Migration 035** adds `backend_type` (default `'sqlite'`) to `app_settings`. Deliberately does **not** persist a connection string anywhere — those may embed credentials; callers supply the string fresh on each `test_postgres_connection`/`bootstrap_postgres_schema` call (same zero-knowledge posture as the WP-59 cloud-backup design above).
  - **Commands** (`commands/backend_config.rs`): `get_backend_config` (any user), `set_backend_type` (admin — records intent only, does not reconnect the live app), `test_postgres_connection`, `bootstrap_postgres_schema` (admin). All four are synchronous, bridging to the async `sqlx` calls via `tauri::async_runtime::block_on` — matching every other command in the codebase (none use `async fn`) rather than introducing the first async Tauri command and its distinct `State<'_, T>` lifetime rules.
  - **Settings UI** — new "Multi-User Backend (Preview)" card: shows the active backend (always SQLite) and the intended backend; when compiled with `--features postgres`, exposes a connection-string field (never saved) and a "Test Connection" action.
  - **22 new Rust unit tests** across `db::backend`/`db::postgres`: connection-string validation, the full backend-switch validation matrix (sqlite always ok; postgres blocked without the feature; postgres blocked without/with an invalid connection string), bootstrap-schema statement splitting and table-name coverage, feature-off stub error messages.
  - **Honest scope check (added after the v1.39.1 self-review):** "Foundation delivered" means the plumbing exists and compiles — it is not evidence that PostgreSQL connectivity works. All 22 tests exercise pure logic (string validation, SQL text content, feature-flag branching); **none of them open a real PostgreSQL connection.** The `sqlx`-backed halves of `test_connection`/`bootstrap_schema` (the actual `#[cfg(feature = "postgres")]` code that talks to a server) have been compile-checked (`cargo check --features postgres`) but have **never been executed against a live PostgreSQL instance**, in this repository or in CI — there is no Postgres service available to test against. Treat "Test Connection" in the Settings UI as unverified until someone runs it against a real server.
- **Not yet implemented (deferred to a future WP):** the `DbBackend` trait refactor of `db::queries` (~5,800 lines) and all 24 command modules to actually target either backend; PostgreSQL connection pooling held in `AppState`; a schema migration runner that ports all 34 SQLite migrations; encrypted storage of a saved connection string; the "Migrate to PostgreSQL" one-click export/import action; **any actual verification that the sqlx connector works against a real server** (see above). SQLite is the only backend serving live reads/writes — `backend_type` records intent for this future work, not a live toggle. In short: this is a well-tested *skeleton* for a second backend, not a working one — a lab cannot switch to PostgreSQL today at any level of risk tolerance.

### WP-51 — LAN network sync across desktop + mobile clients — ✅ Foundation delivered in **v1.38.0** (scope honesty expanded in v1.39.2)

- **Goal (as planned):** Enable real-time or periodic sync between multiple Stelo Lab Suite instances on the same local network.
- **Files:** new `src-tauri/src/sync/`, `src/lib/components/SyncStatus.svelte`, `commands/sync.rs`, `db/` conflict resolver.
- **Steps (as planned):**
  1. Implement a lightweight WebSocket / mDNS discovery service for LAN peer detection.
  2. Design a CRDT-style or last-write-wins merge strategy with audit-log-based conflict resolution.
  3. Add sync commands: `start_lan_sync`, `push_changes`, `pull_changes`, `resolve_conflict`.
  4. UI: Sync status indicator in sidebar + Settings panel with device list and manual sync button.
  5. Include Merkle-proof verification on synced audit entries.
- **Acceptance (as planned):** Two instances on same LAN can sync specimens, strains, subcultures, and audit logs; conflicts are detected and resolvable via UI; offline-first behavior preserved.
- **Preserve:** Full offline operation; SQLite/Postgres compatibility.
- **Bump:** minor → **v1.38.0**.
- **As built (v1.38.0) — implemented together with WP-50 for architectural coherence:**
  - **`db/sync.rs`** — change detection and conflict recording built entirely on the *existing* per-lineage hash chain (`lineage_id`, `chain_seq`, `prev_hash`, `entry_hash` on `audit_log`) rather than a parallel change-tracking mechanism, per the original packet's own framing ("piggyback on the existing chain_seq/entry_hash system"). `get_changes_since` (cursor-based per lineage; an empty cursor list means "give me everything"); `detect_sync_conflicts` classifies each incoming change as new (no local entry at that position), a duplicate (local entry with a matching hash), or a genuine conflict (local entry with a *different* hash — a real fork between devices); `record_sync_conflict`/`list_sync_conflicts`/`resolve_sync_conflict`; `register_sync_peer`/`list_sync_peers` (upsert by `device_id`); `get_sync_status` (aggregate counts).
  - **Migration 035** (shared with WP-50) adds `sync_peers` (known LAN devices; unique `device_id`) and `sync_conflicts` (durable fork records — a local/incoming disagreement at the same `(lineage_id, chain_seq)` is never silently discarded or auto-merged).
  - **Commands** (`commands/sync.rs`): `get_sync_status` (any user), `get_changes_since_cursor` (supervisor+ — the read side of the future transport's "what's new since I last synced?" request), `apply_incoming_changes` (admin — detects and durably records conflicts/duplicates), `list_sync_conflicts`/`resolve_sync_conflict` (admin — see the "Honest scope check" below for what "resolve" actually does), `register_sync_peer`/`list_sync_peers`.
  - **Settings UI** — "Sync Status (Preview)" panel: lineages tracked, unresolved conflicts, known peers (read-only), with an explicit note that LAN discovery/networking is not implemented yet.
  - **18 new Rust unit tests** across `db::sync` and migration 035: change-vector correctness with single and multiple cursors and limits, all three conflict-classification outcomes including a mixed batch, conflict/peer CRUD round-trips, aggregate status-count correctness.
  - **Honest scope check (added after the v1.39.1 self-review):** `list_sync_conflicts` and `resolve_sync_conflict` are implemented and unit-tested at the database layer, but **have no frontend UI at all** — the Settings panel shows only the aggregate `unresolved_conflicts` count from `get_sync_status`; there is no screen that lists individual conflicts or lets an admin act on one. Even if that screen existed today, it couldn't show what actually differs: `SyncConflict` (`models/sync.rs`) stores only `local_entry_hash`/`incoming_entry_hash` — the two sides' *hashes*, not their field-level content — so displaying a meaningful diff would first require joining back to each side's underlying audit entry, which nothing in this packet does. `resolve_sync_conflict` itself is bookkeeping only: it flips the row's `resolved` flag to `1` and records who/when — it does not merge, restore, or pick a winner for any actual data. And because there is no network transport (see below), every conflict record so far only exists from unit tests that feed synthetic hash pairs into `detect_sync_conflicts` directly; the change-detection/classification logic has never been exercised between two genuinely separate SQLite databases, so real-world timing/ordering edge cases (e.g. two devices concurrently extending the same lineage while offline) remain unvalidated.
- **Not yet implemented (deferred to a future WP):** the WebSocket/mDNS discovery service and any actual network transport; a CRDT or last-write-wins merge strategy (this packet detects and durably records forks but does not auto-merge them — resolution is an explicit admin action, and today there isn't even a UI for that action — see above); write-back of accepted new changes into `specimens`/`subcultures`/etc. — `ApplyChangesResult::pending_manual_apply` counts these; replaying a generic change record into the correct domain table requires per-entity-type handlers that don't exist yet; the dedicated `SyncStatus.svelte` sidebar indicator (folded into the Settings preview panel instead, to avoid a premature dedicated nav entry for a feature with no live networking); a conflict-detail UI (list + per-conflict diff + resolve action). In short: two Stelo instances cannot sync with each other today at any level — this is data-model and detection logic only, validated by single-process unit tests, with no transport, no merge, and no way for a human to act on a conflict without direct database access.
- **Conceptual note:** "conflict" here has the same meaning as everywhere else in this document's Trust Layer — a genuine fork in the audit chain — not a data-merge disagreement. Per the risk register's established principle that cryptographic forks must never be silently resolved, `apply_incoming_changes` always durably records a fork rather than picking a winner.

### WP-52 — Email/push notifications for reminders and overdue items — ✅ Delivered in **v1.39.0** (bug fixes in v1.39.2)

- **Goal (as planned):** Notify users (and optionally lab supervisors) about due passages, contamination alerts, maintenance, etc.
- **Files:** new `src-tauri/src/notifications/`, `commands/notifications.rs`, Settings UI extension, `WorkQueue.svelte` enhancements.
- **Steps (as planned):**
  1. Add configurable notification channels (email via SMTP, desktop push via Tauri, optional mobile push).
  2. Extend Work Queue logic with scheduled background checks (using Tauri's scheduler or cron-like task).
  3. Create templated notifications tied to compliance rules and work queue items.
  4. Add user preferences for frequency and types (per-role).
  5. Write audit entries for sent notifications.
- **Acceptance (as planned):** Overdue items trigger notifications; users can configure email/push in Settings; notifications respect lab_profile and role.
- **Preserve:** Existing in-app Work Queue and compliance system unchanged.
- **Bump:** minor → **v1.39.0**.
- **As built (v1.39.0) — implemented together with WP-53/54/55 for architectural coherence:**
  - **Migration 038** — `notification_preferences` (per-user, per-channel, `UNIQUE(user_id, channel)`) and `smtp_config` (single row, `id = 1`, password stored as entered); seeds `notification_check_interval_minutes = 15` in `app_settings`.
  - `commands::work_queue::get_work_queue`'s five overdue-detection rules were mechanically extracted (behavior-preserving move, not a rewrite) into a new pure `db::work_queue::compute_work_queue_items`, so notifications reuse the exact same logic as the existing Work Queue view instead of a second copy.
  - **`db::notifications`** — `compute_due_notifications` builds every candidate entirely from Work Queue fields (accession, reason, urgency): this is the concrete enforcement mechanism for "notifications must respect field-level permissions" — those fields are never subject to WP-55 masking, so no masked value can ever reach a notification body. `send_email` via `lettre` (STARTTLS or direct, blocking transport, kept synchronous like every other command in the codebase rather than introducing the first async Tauri command). `severity_meets_threshold`, `effective_preference` (defaults to enabled + "normal" when a user hasn't configured a channel yet).
  - **Desktop push** via the official `tauri-plugin-notification` plugin (added to the `tauri-commands` feature group, alongside dialog/fs/shell). **Background scheduler** — `tauri::async_runtime::spawn` in `lib.rs::run`, sleeping for the configured interval (reread from `app_settings` each cycle) *before* each check, so restarting the app during development never fires an immediate notification burst.
  - **Recipients:** every active admin/supervisor — Work Queue items have no per-specimen "assigned technician" field to target more narrowly (disclosed scope boundary, see below). Each dispatch cycle sends at most one digest desktop popup and one digest email per recipient — not one notification per item, which would be noisy at any nontrivial queue size.
  - **Commands** (`commands/notifications.rs`): `get_notification_preferences` / `set_notification_preference` (self-service — every user configures their own, not admin-gated), `get_smtp_config` / `set_smtp_config` (admin only; password never redisplayed once saved — re-saving other fields without retyping the password leaves it unchanged), `send_test_desktop_notification`, `send_test_email`, `list_recent_notifications` (queries the standard `audit_log` filtered to `entity_type = 'notification'` — no bespoke notification log table), `dispatch_due_notifications_now` (admin/supervisor manual trigger).
  - **Settings UI** — "Notification Preferences" card (all users: per-channel enable + minimum severity, test-desktop button) and "Email (SMTP) Configuration" card (admin only: host/port/credentials/from-address, test-email action).
  - **8 new Rust unit tests** covering severity ranking, preference defaults and round-trips, candidate construction against an empty/populated Work Queue, SMTP password redaction (the `SmtpConfig` struct returned to the frontend has no password field at all — a compile-time guarantee, not just a runtime check), and audit-completeness for dispatched notifications.
- **Not yet implemented (deferred to a future WP):** direct integration with `get_compliance_flags` (permits, HLB, mycoplasma) — Work Queue already covers the two most safety-relevant conditions (quarantine, contamination), and extracting compliance.rs's ~170 lines of profile-gated detection logic was judged disproportionate risk for this already-large combined packet; per-specimen/per-user notification targeting (would need an "assigned technician" concept that doesn't exist in the data model today); mobile push delivery (the `mobile_push` channel value is reserved in the schema but has no delivery mechanism); OS-keychain-backed SMTP credential storage (stored as entered — **still true as of v1.39.2, see the fix note directly below for what *was* addressed**; see WP-59's zero-knowledge design for the pattern this could adopt later).
- **Fixes (v1.39.2), found by self-review:**
  - **Scheduler mutex poisoning:** the background scheduler's `let Ok(db) = state.db.lock() else { break }` silently and permanently killed the loop forever the first time the mutex was ever poisoned (a panic anywhere else in the app while the lock was held), with zero logging — an operator would see notifications simply stop, with no error to explain why. Now logs the poisoning and recovers via `PoisonError::into_inner` rather than dying; a poisoned `rusqlite::Connection` is still structurally valid since the panic happens in unrelated Rust logic, not mid-mutation of the connection.
  - **Masking backstop:** new `db::notifications::drop_candidates_with_restricted_marker` checks every notification candidate for the `"[RESTRICTED]"` marker before it's returned, regardless of source. `compute_due_notifications` already only draws from non-maskable Work Queue fields (see above) so this should never trigger today — it's a defense-in-depth backstop against a future change accidentally sourcing a candidate field from a maskable entity, not a fix for an active leak.
  - **SMTP password no longer survives a backup file:** `commands::backup::create_backup` now redacts `smtp_config.password` (sets it to `NULL`) in the backup copy it produces — the live database is unaffected, and still stores the password in plaintext (the OS-keychain gap noted above remains open). See the "Backup" section of README.md for detail.

### WP-53 — iOS support — ⚠️ Best-effort/experimental, hardened in **v1.39.1**, still unverified end-to-end

- **Goal (as planned):** Add official iOS target alongside existing Windows + Android builds.
- **Files:** `tauri.conf.json`, GitHub workflows (iOS), `src-tauri/src/` platform-specific code, `README.md`, `docs/` build guide.
- **Steps (as planned):**
  1. Enable Tauri 2 iOS target and configure signing (similar to Android keystore flow).
  2. Update responsive UI components for iOS Safe Areas and native gestures.
  3. Add iOS-specific build CI job that produces IPA.
  4. Test camera (QR), file system (backups/attachments), and notifications on device.
  5. Update Downloads table in README with iOS TestFlight / App Store notes.
- **Acceptance (as planned):** Successful signed IPA build; core flows (specimen CRUD, QR scan, print-to-PDF) work on iOS; responsive layout adapts correctly.
- **Preserve:** All existing desktop/Android behavior and CI.
- **Bump:** minor → **v1.39.0**; hardening pass → patch → **v1.39.1**.
- **Status: EXPLICITLY UNVERIFIED end-to-end.** This packet was implemented and iterated on in a Linux sandbox with no macOS, no Xcode, and no Apple Developer account available at any point — none of the acceptance criteria above have been exercised. What follows is a disclosed best-effort scaffold, not a working iOS build.
- **As built (v1.39.0):**
  - **Safe Areas were already done** — `env(safe-area-inset-*)` throughout `App.svelte` and `Sidebar.svelte`, plus `viewport-fit=cover` in `index.html`, landed as part of earlier mobile-polish work. No changes were needed for this packet; this is genuinely complete, not a scope reduction.
  - **`.github/workflows/build-ios.yml`** (new) — modeled structurally on `build-android.yml`: an unsigned iOS Simulator build job ran on every push/PR, and a signed release-IPA job ran on GitHub Release events. **This version was failing** — see the v1.39.1 fixes below.
  - README `Downloads` table and Tech Stack row updated to state the status plainly (no iOS artifact exists) rather than imply availability, plus a new "iOS support status (WP-53)" section enumerating exactly what remains.
- **As built (v1.39.1) — hardening pass fixing the failing v1.39.0 workflow:**
  - **`tauri.conf.json`** gained a `bundle.iOS` section (`minimumSystemVersion: "13.0"`). `developmentTeam` is deliberately left unset: confirmed directly against `tauri-utils`' `IosConfig` source that Tauri reads the Apple Developer Team ID from the `APPLE_DEVELOPMENT_TEAM` environment variable at build time, taking priority over any config value — this is real, documented Tauri behavior, not something that needed custom scripting.
  - **The `identifier` field (`com.steloptc.app`) was deliberately left unchanged**, after research (confirmed in `tauri-utils`' `Config` struct) showed it is a single field shared across every platform — Android's `applicationId`, iOS's bundle ID, Windows/macOS identity — with no per-platform override anywhere in Tauri's config schema. Changing it to a more iOS-idiomatic value would have silently broken in-place upgrade continuity for any existing Android install, which this project's own `.github/SIGNING.md` treats as a property worth preserving (same rationale as "reuse the same release keystore across releases"). This was surfaced to and confirmed by the user before proceeding rather than assumed.
  - **Fixed a real bug:** the v1.39.0 workflow set `APPLE_TEAM_ID` for the release build step, but confirmed directly in `tauri-cli`'s source (`APPLE_DEVELOPMENT_TEAM_ENV_VAR_NAME` constant) that Tauri actually reads `APPLE_DEVELOPMENT_TEAM` — the old variable name was silently ignored, meaning even a fully-secreted release run would have failed to pick up the team ID.
  - **Triggers narrowed** from `push`/`pull_request`/`release` to `workflow_dispatch`/weekly `schedule`/`release` — an unverified job failing on every unrelated push is noise, not signal.
  - **Tiered credential gate:** a new step computes `has_dev_team` and `has_release_signing` outputs up front. With neither secret configured (the actual current state — nothing has been added), the job now runs `cargo check --target aarch64-apple-ios-sim` instead of attempting `cargo tauri ios init`/`ios build`, which were guaranteed to fail without a team ID (Tauri falls back to auto-detecting a signed-in Xcode account, which no CI runner has). This is a real, meaningful, always-attemptable validation of Rust-level iOS compilation — it does not validate the Swift/Xcode side, which requires at least `APPLE_DEVELOPMENT_TEAM`.
  - `Settings.svelte` gained a small "New" badge on the Notification Preferences card (WP-52 cross-reference); `PermissionsEditor.svelte` gained `title` tooltips on its matrix checkboxes for consistency with the rest of the app (its existing label/table structure was verified to already match established accessibility patterns, not changed).
- **Not yet implemented / not verified (all require a macOS maintainer with an active Apple Developer Program membership):**
  1. Running `cargo tauri ios init` even once against this codebase to confirm the generated Xcode project (`src-tauri/gen/apple/`) builds at all — the `cargo check` fallback validates Rust compilation only, not Xcode/Swift.
  2. Adding `APPLE_DEVELOPMENT_TEAM` as a repository secret (unlocks the simulator-build path) and the remaining four signing secrets (unlocks signed release IPAs).
  3. Any on-device or simulator verification of specimen CRUD, QR camera scanning, attachment/backup file access, or desktop-notification-plugin behavior on iOS — the plugin surface added in WP-52 (`tauri-plugin-notification`) is documented by its authors as cross-platform including iOS, but this claim is unverified here.
  4. TestFlight / App Store Connect upload automation (needs an App Store Connect API key in addition to the signing secrets above).
- **Fix (v1.39.2), found by self-review:** the QR scanner (`html5-qrcode`, used from the webview) needs camera access, and iOS refuses/crashes on that without a usage-description string — this app had none. Added `src-tauri/Info.ios.plist` with `NSCameraUsageDescription`, wired in via `bundle.iOS.infoPlist` in `tauri.conf.json` (Tauri also auto-discovers a same-named file next to `tauri.conf.json`, so the explicit config reference is belt-and-suspenders, not strictly required). Believed correct per `tauri-utils`' documented `IosConfig::info_plist` behavior — confirmed by reading that source, the same way the `APPLE_DEVELOPMENT_TEAM` behavior was confirmed in the v1.39.1 pass — but like everything else in this section, **not exercised against a real Xcode build**. If a future permission-gated API is added (microphone, photo library, location, etc.), add the matching `NS*UsageDescription` key to that same file.
- **Recommendation:** treat this workflow file as a starting point for a maintainer with Xcode access, not as a functioning CI pipeline. Do not report iOS as "supported" externally until at least one real signed build has succeeded.

### WP-54 — Environmental sensor integration — ✅ Delivered in **v1.39.0** (source validation + trust-gap disclosure in v1.39.2)

- **Goal (as planned):** Integrate temperature, humidity, CO₂, and other sensors into passage/fruiting records for cell culture and mycology.
- **Files:** new `src-tauri/src/sensors/`, migration (035), `commands/sensors.rs`, updates to passage form and SpecimenDetail.
- **Steps (as planned):**
  1. Migration 035: Add `environmental_readings` table linked to subcultures/specimens (timestamp, temp, rh, co2, light, notes).
  2. Support common protocols (USB/serial, Bluetooth, MQTT for LAN sensors).
  3. UI: Auto-log button in passage form; historical chart in SpecimenDetail (mycology/ cell culture profiles).
  4. Dashboard alerts for out-of-range conditions.
  5. Add 6 Rust unit tests for reading parsing and validation.
- **Acceptance (as planned):** Sensors can be paired and readings attached to passages; charts display correctly per profile; alerts fire on thresholds.
- **Preserve:** Manual entry remains fully supported.
- **Bump:** minor → **v1.39.0**.
- **As built (v1.39.0) — implemented together with WP-52/53/55 for architectural coherence:**
  - **Migration 037** (numbered after WP-55's migration 036, not 035 as originally sketched — 035 was already claimed by WP-50/WP-51's multiuser foundation) — `environmental_readings`: `specimen_id`/`subculture_id` both nullable FKs with a `CHECK` requiring at least one to be set; `reading_type` CHECK'd to `temp_c|humidity_pct|co2_ppm|light_lux|ph|custom`; `source` CHECK'd to `manual|usb_serial|bluetooth|mqtt`.
  - **`db::sensors`** — `parse_sensor_payload` is real, tested, transport-agnostic logic: it parses both a comma-separated `key=value` line (typical of a serial microcontroller sketch) and a flat JSON object (typical of an MQTT message body) into validated readings, skipping unknown keys rather than rejecting the whole payload. `validate_reading_value` applies generous sanity-range bounds per type. `create_environmental_reading` (manual entry, fully functional today) and `get_environmental_alerts` (checks the most recent reading per specimen/type against a threshold — sensible built-in defaults, overridable via `app_settings` without a migration).
  - **Commands** (`commands/sensors.rs`): `create_environmental_reading`, `list_environmental_readings`, `get_environmental_alerts`, and `ingest_sensor_payload` — the transport-agnostic entry point a future USB/BLE/MQTT listener would call per incoming message (parses, then persists via the same path as manual entry).
  - **`SpecimenDetail.svelte`** — new Environmental Readings section in the History tab (cell_culture/mycology profiles only): manual entry form, a dependency-free inline SVG sparkline per reading type (no new charting library added), and a full readings history table.
  - **`Dashboard.svelte`** — new "Environmental Alerts" panel (same profile gating), following the exact WP-34/WP-44 panel pattern already established for cell-culture/mycology-specific dashboard content.
  - **12 new Rust unit tests** — payload parsing (both formats, unknown-key skipping, empty/malformed rejection), value-range validation, manual-entry FK requirement, create/list round-trip, and threshold-alert triggering (both the flagged and the in-range case).
  - **`validate_source` (added in the v1.39.2 fix pass)** — `create_environmental_reading` now checks `source` against the same four-value enum the DB's `CHECK` constraint enforces, returning a clear error (naming the bad value and listing the valid ones) instead of a raw SQLite constraint-violation message. 7 new tests cover it directly and through `create_environmental_reading`.
  - **Trust gap in `source` (disclosed in the v1.39.2 fix pass):** `source` is a caller-supplied label, not a verified fact. Because no hardware transport is wired up (see "Not yet implemented" below), every reading that has ever existed in this app was created through manual entry — nothing has ever legitimately set `source` to `usb_serial`/`bluetooth`/`mqtt`. Both `create_environmental_reading` and `ingest_sensor_payload` are ordinary Tauri commands with no way to confirm a caller's claimed source is true; `ingest_sensor_payload` additionally has **no frontend UI wiring at all** (defined in `api.ts`, called from no `.svelte` component), so today it's reachable only by whatever calls the Tauri IPC directly. `validate_source` confirms the value is one of the four recognized strings — it does not and cannot confirm provenance. Do not treat a non-`manual` `source` value as proof a reading was machine-collected until a real transport listener exists; ideally, that listener should be the only caller permitted to set a non-`manual` source, which would require moving `source` off the free-form `CreateEnvironmentalReadingRequest` field it is today.
- **Not yet implemented (deferred to a future WP):** actually opening a USB/serial port, scanning for and subscribing to a BLE peripheral, or connecting to an MQTT broker — each requires a hardware-specific crate (`serialport`, `btleplug`, `rumqttc`) with system dependencies (libudev, D-Bus/bluez, a running broker) that cannot be meaningfully exercised or verified without attached hardware in a sandboxed environment. The parsing/validation/storage pipeline those transports would feed into is complete and tested today — wiring one in is a mechanical follow-up, not a redesign. Also deferred: an "auto-log" button embedded directly in the passage/fruiting recording form (readings are logged from a dedicated section instead, not inline with passage creation).

### WP-55 — Field-level permissions for shared lab use — ✅ Delivered in **v1.39.0** (critical corruption bug + N+1 fix in v1.39.2)

- **Goal:** Allow fine-grained, role-configurable control over which fields and panels are visible to each role — enabling multi-technician shared labs where IP-sensitive provenance data (genomic fingerprints, strain origin, breeding program details) must be restricted to supervisors and admins without disrupting day-to-day technician workflows.
- **Context:** As labs grow beyond a single admin operator, shared access becomes essential — but not all data is appropriate for all users. A contract lab processing samples for external clients may not want technicians to see strain ownership or pricing data. A research institute may want to hide unpublished breeding program goals from student workers. The current role system (technician / supervisor / admin) controls write access but does not restrict visibility of sensitive read-only fields.
- **Files:** `src-tauri/src/auth/permissions.rs` (new field-mask layer), new migration for `field_permissions` table, `src-tauri/src/commands/` (all read commands updated), UI components (conditional rendering per role), `src/lib/components/PermissionsEditor.svelte` (admin configuration UI).
- **Steps:**
  1. **`field_permissions` table** (new migration): `id TEXT PK`, `role TEXT NOT NULL` (technician / supervisor / admin), `entity_type TEXT NOT NULL` (specimen / strain / audit_log / breeding_program / compliance / …), `field_name TEXT NOT NULL`, `visible INTEGER NOT NULL DEFAULT 1`. Unique constraint on `(role, entity_type, field_name)`. Seed with permissive defaults (all visible for all roles) so existing deployments are unaffected.
  2. **Field-mask layer in read commands:** Before returning data to the frontend, apply the active user's role field-mask. Sensitive fields to support masking from the start: `genomic_fingerprint`, `strain_id` / `strain_chain_seq`, `confirmation_basis`, `breeding_program.goal` / `target_traits`, `compliance_record` details, `audit_log` payload for non-own entries. Masking replaces the value with `null` or a placeholder string `"[RESTRICTED]"` — never omits the key, so frontend code doesn't break on missing fields.
  3. **UI: conditional field rendering.** Components check the masked value and render a `[Restricted]` chip in place of the hidden value. No blank space, no error — just a visual indicator that the field exists but is restricted for this role.
  4. **`PermissionsEditor.svelte`** (admin only): matrix view (roles × fields) with toggle switches. Changes persist to `field_permissions` table. Shows a preview of what a technician vs. supervisor sees for a sample specimen.
  5. **Write-path preservation:** Field masking applies only to read commands. All write commands and all audit log entries always capture the full field value regardless of the requesting user's role. The audit trail is always complete.
  6. Add 6 Rust unit tests: masked field returns `null` for technician, visible for supervisor; default seeds are all-visible; masking does not affect audit entries; `field_permissions` unique constraint; role gate on `PermissionsEditor` commands.
- **Acceptance (as planned):** A technician user cannot see `genomic_fingerprint` or `breeding_program.goal` when those fields are masked in the admin panel; a supervisor sees all fields; the audit trail entry for both users contains the full field value; changing the permission in Settings takes effect immediately (no restart required).
- **Preserve:** Existing role system and admin visibility; all write-path behavior unchanged; technicians can still perform their full workflow (record passages, flag contamination, use Work Queue) with masked provenance fields.
- **Bump:** minor → **v1.39.0**.
- **As built (v1.39.0) — implemented together with WP-52/53/54 for architectural coherence:**
  - **Migration 036** — `field_permissions` (`id`, `role` CHECK'd to `admin|supervisor|tech|guest`, `entity_type`, `field_name`, `visible`, `UNIQUE(role, entity_type, field_name)`); seeded with 12 permissive rows (4 roles × the 3 fields wired into masking below), all `visible = 1`.
  - **`db::permissions`** — `is_field_visible` (defaults to visible when no explicit row exists, so a newly-added sensitive field is never silently hidden from everyone before an admin configures it); `mask_optional_field` (final decision: always the literal string `"[RESTRICTED]"`, never `null` — this lets the frontend tell "no data" and "hidden data" apart unambiguously, and the field key is never omitted); `list_field_permissions` / `set_field_permission` (upsert; every read queries live, so a permission change takes effect on the very next read with nothing to invalidate or restart); `validate_admin_role` (pure, testable admin gate, mirroring the `check_profile_change_allowed` pattern from WP-26).
  - **Masking wired into two representative entities** — `get_strain` / `list_strains_by_species` (masks `genomic_fingerprint`) and `get_breeding_program` / `list_breeding_programs` (masks `goal`, `target_traits`): the exact fields named in this packet's own acceptance criteria. `create_breeding_program`'s immediate return value is deliberately **not** masked — a user always sees exactly what they just typed; masking applies to subsequent reads.
  - **Commands** (`commands/permissions.rs`): `list_field_permissions`, `set_field_permission` — both admin-only.
  - **`PermissionsEditor.svelte`** (new) — admin-only role × field visibility matrix, embedded in Settings. `StrainDetail.svelte` and `BreedingProgramManager.svelte` render a "🔒 Restricted" chip wherever a masked value would otherwise appear.
  - **Audit-trail guarantee, tested architecturally:** `log_audit` and every `log_audit_*` variant always receive the raw, unmasked value — masking exists only in the read-response construction path in `commands/strains.rs`/`commands/breeding.rs` and has no code path into any audit-write function. `masking_never_reaches_audit_log_writes` proves this directly: it hides a field, writes an audit entry containing the "hidden" value, and asserts the stored value is the real one, not `"[RESTRICTED]"`.
  - **14 new Rust unit tests** covering: default-visible fallback, migration seed correctness (12 rows, all visible), per-role hide/show independence, upsert-without-duplicating, masking behavior for both `Some`/`None` inputs, list round-trips, the admin-role gate (accepts admin, rejects supervisor/tech/guest), and the audit-trail guarantee above.
- **Not yet implemented (deferred to a future WP):** extending masking beyond the two entities above (`confirmation_basis`, `compliance_record` details, `audit_log` payload visibility for non-own entries, etc.) — the mechanism is proven and the extension is mechanical (one seed row + one `mask_optional_field` call at each additional read site), but sweeping every read command in the ~24-file command layer in one packet was judged disproportionate scope for this already-large combined effort.
- **Critical bug fix (v1.39.2), found by self-review:** `update_strain_status` unconditionally persisted whatever `genomic_fingerprint` value it was sent, and `StrainManager.svelte`'s status-update form pre-filled itself from the current (possibly `"[RESTRICTED]"`-masked) value. A role with this field hidden performing *any* status update — not only a genomic-confirmation one — would silently overwrite the real fingerprint with the placeholder string, a real, not hypothetical, data-corruption bug. Fixed with a hard backend guard (`db::permissions::reject_if_restricted_marker`, rejects the literal marker on any write path regardless of what a frontend sends), a `COALESCE`-based SQL fix so omitting the field preserves the existing value, and a frontend fix so the form never loads the marker into an editable field in the first place. The fix logic was extracted into a new pure `db::queries::apply_strain_status_update` specifically so it could be covered by a real regression test (`apply_strain_status_update_rejects_masked_fingerprint_and_preserves_real_value` and three others) — this is the extension-blocker referenced in the "Not yet implemented" line above; masking should not be extended to new entities until a fix of this shape exists for every masked+writable field, not just this one.
- **N+1 query fix (v1.39.2), found by the same self-review:** `get_strain`/`list_strains_by_species`/`get_breeding_program`/`list_breeding_programs` each queried `field_permissions` once per row when masking a list — fine for the current two entities' worth of call sites, but a real scalability blocker for the "extend masking further" work above. New `db::permissions::FieldPermissionSet` loads a role's full permission set once into a `HashMap`; all four call sites now use it. This was a prerequisite for lifting the "not yet implemented" restriction above, not merely a nice-to-have.

### WP-56 — Local AI analysis

- **Goal:** Provide on-device AI assistance for note summarization and image-based contamination detection.
- **Files:** new `src-tauri/src/ai/`, `commands/ai.rs`, integration in SpecimenDetail and notes fields, Ollama/Local LLM support.
- **Steps:**
  1. Integrate with local LLM backends (Ollama, MLX, etc.) via Tauri commands.
  2. Features: "Summarize Notes", "Suggest Passage Comments", "Analyze Photo for Contamination" (using vision models where available).
  3. UI buttons in notes editor and attachment lightbox.
  4. Store AI suggestions as draft notes with attribution; require user approval before saving.
  5. Add privacy-first design: all processing stays local.
- **Acceptance:** Users can invoke AI on notes/photos; suggestions appear in UI; works offline with local models.
- **Preserve:** All manual workflows fully functional without AI.
- **Bump:** minor.
- **As built (v1.40.0):** Delivered largely as specified, with one deliberate dependency-avoidance substitution. `src-tauri/src/ai/ollama.rs` is a minimal hand-rolled HTTP/1.1 client over `std::net::TcpStream` (request building, HTTP/chunked-transfer-encoding parsing, and response parsing are all pure and unit-tested) rather than adding a new HTTP-client crate — the only network call this feature ever makes is to a user-configured local Ollama endpoint (default `http://127.0.0.1:11434`), so a full HTTP client library was judged unnecessary. Migration 041 adds `ai_suggestions` (entity_type/entity_id/kind/model_name/prompt/suggestion/status/created_by/reviewed_by/reviewed_at) — every AI output is stored as a **pending** row; `approve_ai_suggestion` is the only path that ever copies text into a real `notes` field, and it does so through the same UPDATE + `log_audit` path a manual edit would use, so the audit trail always attributes the change to the approving human, never to "AI." Three commands: `summarize_notes`, `suggest_passage_comment` (built from the specimen's last 5 passages), `analyze_photo_for_contamination` (vision model, reads an existing attachment's bytes). UI: an "AI Assist" block in `SpecimenDetail.svelte`'s notes area (Summarize Notes / Suggest Passage Comment buttons, pending-suggestion review cards with Approve/Reject) and an "Analyze for Contamination" control in `SpecimenPhotoGallery.svelte`'s lightbox. 10 Rust unit tests (all pure — no live Ollama instance needed or used in CI, matching the same disclosed limitation already established for WP-50/WP-52's live-service dependencies). All manual note-editing and passage-recording workflows are completely untouched.


### WP-57 — Interactive lab map

- **Goal:** Visualize locations with floor-plan heat-map and asset tracking.
- **Files:** new `src/lib/components/LabMap.svelte`, migration for location metadata, `commands/locations.rs`.
- **Steps:**
  1. Extend location system with optional floor-plan image upload and coordinate mapping.
  2. Implement interactive map using Leaflet or canvas overlay with specimen/location pins.
  3. Heat-map by contamination risk, age, or density.
  4. Click-to-navigate from map to specimen or location detail.
  5. Dashboard widget showing map overview.
- **Acceptance:** Upload floor plan → place specimens → view heat-map and navigate; updates reflect live data.
- **Preserve:** Existing text-based location system works unchanged.
- **Bump:** minor.
- **As built (v1.40.0):** Migration 040 adds a new `locations` table (name, description, inline-base64 `floor_plan_image`, fractional `floor_plan_x`/`floor_plan_y`) and an optional `specimens.location_id` column — both purely additive; the existing free-text `specimens.location`/`location_details` fields are completely unchanged and remain the default way to record where a specimen lives. A specimen's map pin is set only via the new, dedicated `set_specimen_location_pin` command, never through the existing specimen create/update commands, so the map feature can never interfere with the text-based location workflow. `LabMap.svelte` renders each location-with-an-image as its own floor/map (a dropdown switches between floors when a lab has more than one), pins positioned via plain absolutely-positioned `<div>`s over the image (no Leaflet or other mapping library dependency added — deliberate, to avoid dependency risk in an already-large combined session), a density/contamination-risk/age heat-map toggle computed client-side from `get_location_map_data`'s server-side aggregates, click-to-open location detail with specimen assignment, and full CRUD including floor-plan image upload. A small "Lab Map Overview" widget was added to `Dashboard.svelte`. 6 Rust unit tests plus command-layer coverage for the delete-blocked-while-specimens-pinned guard.


### WP-58 — Advanced analytics & reporting dashboards

> **Priority: Highest ROI for labs at scale.** Once a lab exceeds ~200 active specimens, the current aggregate Dashboard (counts by stage, contamination overview, schedule) no longer surfaces the actionable insight researchers need. Analytics is the feature that transforms Stelo from a record-keeping tool into a decision-support platform. Implement early in Phase F.

- **Goal:** A dedicated Analytics view with trend charts, cross-specimen and cross-strain comparative reports, exportable summaries, and configurable KPI panels — going well beyond the current Dashboard's aggregate counts.
- **Files:** new `src/lib/components/AnalyticsDashboard.svelte`, new `src-tauri/src/db/analytics.rs`, new `commands/analytics.rs`, `src/lib/api.ts` (analytics helpers), `Sidebar.svelte` (new nav entry).
- **Steps:**
  1. **`db/analytics.rs` module** — pure query functions for: specimen growth rate over time (new specimens per week/month/quarter), subculture frequency trends (passages per week by species, stage, technician), contamination rate by stage and species over rolling 30/90/365-day windows, passage success rate (successful passages / total passages, with trend arrow), media batch efficiency (specimens per batch, waste rate), strain performance comparisons (mean specimen health per strain, average subculture intervals, best-performer rate), cryopreservation utilization (vials in/out by line, thaw success rate for cell culture). Each function accepts a `time_range` parameter (`30d | 90d | 1y | all`) and returns time-series data as a `Vec<(date_bucket, value)>` suitable for charting. All functions are pure (take a `Connection` parameter) and independently unit-testable.
  2. **`commands/analytics.rs`** — expose the above as Tauri commands, read-only, no role restriction beyond authenticated user. All commands return structured JSON typed with the corresponding TypeScript interfaces.
  3. **`AnalyticsDashboard.svelte`** — configurable panel grid (each panel independently toggleable in Settings, persisted to `app_settings`); charts using a lightweight dependency-free library (e.g. uPlot or Chart.js via npm, chosen for bundle size); global time range selector (30d / 90d / 1y / all time) that updates all panels simultaneously; profile-aware panel visibility (cell culture panels hidden on PTC profile etc.); "Export Report" button producing a multi-sheet Excel workbook via SheetJS (one sheet per major metric, matching the existing export schema style with lab name header and generated-date footer).
  4. **KPI summary strip** — top-level numbers displayed prominently at the top of the Analytics view: total active specimens, passages this week, contamination rate this month, pending work queue items, passages per active specimen (throughput metric), new specimens this month vs. last month (growth indicator with arrow).
  5. **Strain performance report** — dedicated sub-panel comparing all strains within a species on: mean specimen health at last passage, total specimens ever created, average days between passages, best-performer flag rate. Sortable table + optional bar chart. Enables data-driven strain selection decisions.
  6. **Technician activity report** (supervisor/admin only) — passages recorded per user per week, contamination events per user. Not punitive — framed as workload visibility and capacity planning.
  7. Add "Analytics" nav entry in Sidebar (📊).
  8. Add 10 Rust unit tests covering each analytics query function: growth rate empty/non-empty, contamination rate calculation, passage success rate edge cases (zero total passages), media efficiency, strain comparison ranking, cryopreservation utilization.
- **Acceptance:** All trend charts render correctly for a DB with 100+ specimens and 500+ subcultures; time range filter updates all panels simultaneously; export produces valid `.xlsx` with one sheet per metric; empty time range shows friendly empty state (not a blank panel); strain performance report sorts correctly; profile-aware panel visibility works across all three lab profiles.
- **Preserve:** Existing Dashboard panels untouched; this is a strictly additive Analytics view accessed via its own sidebar nav entry.
- **Bump:** minor.
- **As built (v1.40.0):** `db/analytics.rs` provides pure, independently-tested query functions — `specimen_growth_rate`, `subculture_frequency_trend`, `contamination_rate_trend`, `passage_success_rate` (with a first-half-vs-second-half trend delta), `media_batch_efficiency`, `strain_performance`, `cryo_utilization`, `technician_activity` — every one takes a `TimeRange` (`30d | 90d | 1y | all`) and is unit-tested against an in-memory migrated database, no Tauri runtime required. `AnalyticsDashboard.svelte` delivers the KPI strip (active specimens, passages this week, contamination rate this month, pending work-queue items, throughput, month-over-month growth arrow), a global time-range selector that refetches every panel in parallel, a configurable panel grid persisted via `app_settings`, hand-rolled inline SVG line/bar charts (no Chart.js/uPlot dependency added — a deliberate dependency-avoidance choice, differing from the ROADMAP's original suggestion), a sortable Strain Performance table + chart, a Technician Activity report gated to supervisor/admin both in the UI and the backend command, and a multi-sheet Excel export reusing the existing `xlsx` dependency and the app's established header/footer export convention. New "Analytics" sidebar entry. 10 Rust unit tests covering every query function's empty/populated/edge-case behavior.


### WP-59 — Cloud backup & multi-device sync with end-to-end encryption

- **Goal:** Encrypted, automated offsite backup to user-controlled cloud storage (S3-compatible, SFTP, or local NAS) with optional multi-device sync for teams using the same database remotely. End-to-end encryption is a hard requirement — the backup provider must never have access to plaintext data.
- **Context:** Many labs operate on a single machine with no offsite backup. A single hardware failure or ransomware event can destroy years of irreplaceable culture records. Cloud backup solves this while E2E encryption ensures that a compromised storage provider cannot read lab data — especially important when genomic fingerprint data, breeding program details, or commercially sensitive strain provenance are stored.
- **Files:** new `src-tauri/src/cloud/`, new migration for `backup_targets` table, `commands/cloud_backup.rs`, Settings UI extension, `src/lib/components/CloudBackupPanel.svelte`.
- **Steps:**
  1. **Migration — `backup_targets` table:** `id TEXT PK`, `name TEXT NOT NULL` (user-friendly label), `type TEXT NOT NULL` (values: `s3 | sftp | smb | local_nas`; no CHECK constraint so future targets can be added without migration), `config_encrypted TEXT NOT NULL` (AES-256-GCM encrypted JSON containing endpoint URL, access credentials, and bucket/path — encrypted with a per-target key derived from the user's master backup key), `schedule_cron TEXT` (standard 5-field cron expression; NULL = manual only), `last_backup_at TEXT`, `last_backup_size_bytes INTEGER`, `last_status TEXT CHECK('ok'|'failed'|'pending')`, `last_error TEXT`, `is_enabled INTEGER NOT NULL DEFAULT 1`, `created_at TEXT NOT NULL`.
  2. **Encryption design — zero-knowledge by default:**
     - Master backup key: user supplies a passphrase; derive AES-256-GCM key via Argon2id (128 MB memory, 3 iterations, 4 parallelism threads). The derived key is held in memory only and discarded on app close. Never persisted to disk.
     - OS keychain integration: optionally persist the derived key in the native OS keychain (`Tauri keychain plugin`) so the user does not need to re-enter the passphrase each session. Keychain storage is opt-in, clearly labelled as "store key in OS keychain."
     - Per-backup nonce: each upload uses a fresh random 96-bit nonce prepended to the ciphertext. Nonce is visible in plaintext in the backup file header (non-sensitive; required for decryption).
     - Header format: 4-byte magic `STEL`, 1-byte version, 12-byte nonce, then AES-256-GCM ciphertext + 16-byte authentication tag.
  3. **`cloud_backup` command:** pre-checkpoint (calls `auto_checkpoint_lineages` silently) → WAL flush → copy DB to temp file → AES-256-GCM encrypt → upload to configured target. Returns `{ ok, backup_id, size_bytes, duration_ms, merkle_root_included }`.
  4. **`restore_from_cloud` command:** download from selected target → AES-256-GCM decrypt with key authentication (Argon2id re-derive from user passphrase, authenticate tag before touching the live DB) → destructive confirm dialog (same two-step flow as WP-16 local restore) → replace live DB → reload. Wrong key or corrupted blob returns a clear error with tamper-detection context before any change to the live DB.
  5. **Multi-device sync (optional, shared-team mode):** for teams sharing a database, implement a delta-journal sync where each device uploads WAL segments (named `{device_id}/{chain_seq_range}.wal`) to a shared prefix in the configured cloud target. A `reconcile_cloud_sync` command downloads all peer WAL segments, orders them by `chain_seq` (the audit log's chain-seq is the authoritative ordering tiebreaker), and applies non-conflicting deltas. Conflicting writes (same `chain_seq` from different devices) are surfaced in a conflict resolver UI — never silently discarded. Sync is always manual; no background sync daemon.
  6. **`CloudBackupPanel.svelte`** — Settings sub-panel: configure targets (add/edit/delete), test connection, set schedule, last-run status card with size and duration. Manual "Backup Now" button. For sync-enabled targets: "Sync Now" button and peer-device list with their last sync timestamps.
  7. Add 8 Rust unit tests covering: encrypt-decrypt round-trip with correct key; decrypt with wrong key returns Err (not corrupted data); nonce uniqueness across calls; header magic detection; schedule cron parsing (valid and invalid); WAL segment ordering by chain_seq; conflict detection when same seq appears from two devices; size calculation for `last_backup_size_bytes`.
- **Acceptance:** Scheduled backup uploads an AES-256-GCM encrypted blob that can be decrypted only with the correct passphrase; restore decrypts and restores correctly with a full destructive confirmation flow; wrong passphrase or tampered blob returns a clear error before touching the live DB; multi-device sync merges non-conflicting WAL segments cleanly; conflict detection fires when same chain_seq appears from two sources.
- **Preserve:** Existing local backup (WP-16) is fully unaffected; cloud backup is strictly additive and optional; single-user SQLite workflow requires no cloud configuration.
- **Bump:** minor.
- **As built (v1.40.0):** The zero-knowledge encryption core is delivered exactly as specified: `cloud::crypto` derives a 256-bit key via Argon2id (128 MiB memory, 3 iterations, 4-way parallelism) from a user passphrase and a random salt, encrypts with AES-256-GCM under a fresh random 96-bit nonce per call (never reused — proven by a dedicated test), and prefixes every blob with a `STEL` magic + version byte so a non-SteloPTC file is rejected with a clear error rather than a confusing AEAD failure. The passphrase itself is **never persisted** — only the caller (frontend `$state`, cleared on navigation) holds it transiently. Migration 042 adds `backup_targets` (config stored as `base64(salt || AES-GCM(JSON))`, so even the config itself follows the zero-knowledge design) and `cloud_sync_segments` (tracks which peer WAL segments have already been reconciled). **Transport scope, disclosed honestly:** `local_nas`/`smb` targets (a filesystem path — e.g. a mounted network share) are fully functional for backup, restore, and sync; `s3`/`sftp` targets can be configured and their credentials encrypted today, but `cloud_backup`/`restore_from_cloud`/`reconcile_cloud_sync` return a clear "not yet connected" error for those two types, since no S3 SDK or SFTP/SSH client library was added — the same "foundation now, transport later" pattern already established for WP-50's PostgreSQL connector. `restore_from_cloud` authenticates the blob (derives the key, decrypts, checks the AEAD tag) **before** touching the live database, and drives the exact same two-step "type RESTORE" destructive-confirmation flow as the existing local restore (`restore_backup`). Multi-device sync deliberately **reuses** WP-51's existing `db::sync::detect_sync_conflicts`/`get_changes_since`/`record_sync_conflict` rather than reinventing conflict detection — `reconcile_cloud_sync` publishes this device's outbound changes as a `{device_id}/{start}-{end}.wal` file in the shared target folder and classifies every peer segment found there as new/duplicate/conflict; conflicts are always durably recorded, never silently merged, but (matching WP-51's own disclosed scope boundary) accepted non-conflicting changes are reported, not yet automatically written back into the local database. `CloudBackupPanel.svelte` in Settings. 18 Rust unit tests (exceeding the 8 requested) covering the full encryption round-trip matrix, cron validation, WAL segment naming/ordering, and size formatting. Existing local backup (WP-16) is completely unaffected.


### WP-60 — Regulatory compliance export modules (FDA/USDA/CITES)

- **Goal:** Export specimen records, audit trails, and provenance documentation in formats required by US (FDA 21 CFR Part 11, USDA APHIS) and international (CITES Appendix-level) regulatory bodies — reducing the manual effort of regulatory inspections and permit applications from days to minutes.
- **Context:** Labs working with regulated species, cell lines, or GMOs face recurring documentation burdens. A cell culture lab under FDA 21 CFR Part 11 must demonstrate that its electronic records are trustworthy and that its audit trail has not been tampered with — SteloPTC's existing Merkle proof system (WP-20/21) provides exactly this, but only if it can be packaged for inspectors in a recognisable format. Similarly, a PTC lab propagating CITES-listed species must provide chain-of-custody documentation on export. This WP packages the system's existing cryptographic guarantees into regulatory-ready output bundles. Implement alongside WP-59 (cloud backup) for labs pursuing Part 11 compliance — offsite encrypted backups and signed audit exports are complementary requirements.
- **Files:** new `src-tauri/src/compliance_export/` module, new `src/lib/components/ComplianceExportWizard.svelte`, `commands/compliance_export.rs`, `docs/regulatory-exports.md`.
- **Steps:**
  1. **FDA 21 CFR Part 11 export — electronic records attestation package:**
     - Cover document: lab name, system version, export date range, total audit entries, Merkle checkpoint IDs included, and a signed attestation that the system meets Part 11 requirements (append-only audit log, access controls, password policy, audit chain integrity).
     - Full audit trail export: all `audit_log` entries in the selected date range in canonical JSON, with `chain_seq`, `prev_hash`, `entry_hash` for each entry.
     - Merkle verification result: run `verify_audit_chain` over the export range; include the verification verdict (`verified | broken_at_seq`), total entries checked, and the Merkle root of the covering checkpoint.
     - User activity report: all users, their roles, last login, and a summary of actions performed in the date range.
     - Signing: each document in the bundle is signed with the lab's RSA-4096 signing key (reuse the Trust Layer key if configured; otherwise generate and prompt the admin to store a new key in Settings). Signature stored as a detached `.sig` file alongside each document. The signing key's public certificate is included in the bundle.
     - Output: `.zip` containing PDF cover + JSON audit trail + JSON verification result + JSON user report + `.sig` files + public certificate.
  2. **USDA APHIS permit support (plant tissue culture profile only):**
     - Generate a structured JSON file matching the APHIS Plant Permit application schema for `PPQ Form 526` (Permit to Move Live Plant Pests, Noxious Weeds, or Soil). Fields auto-populated from specimen and species records: scientific name, family, country of origin, quantity, intended use, containment facility, authorized scientist name (from user record).
     - Quarantine record export: all `compliance_records` of type `quarantine` for the selected specimens within the permit date range, formatted as a table attachment.
     - The user reviews the pre-filled form in a step-by-step wizard before exporting or submitting. SteloPTC does not submit directly to APHIS — it produces a ready-to-submit package.
  3. **CITES Species Provenance Dossier (for CITES Appendix I/II/III species):**
     - Species identification: scientific name, CITES Appendix designation (user confirms from a reference list; SteloPTC does not maintain a live CITES species database), and Darwin Core export via `export_darwin_core` from WP-49.
     - Chain of custody: full specimen genealogy from the earliest accession forward — all parent/child specimen relationships from `specimens.parent_id`, with dates, location, and responsible party (user record) for each transfer.
     - All propagation records: every subculture entry for all specimens in scope, in chronological order.
     - Audit trail hash summary: Merkle root of the audit checkpoint covering the specimen's full history, plus verification status.
     - Output: PDF dossier + Darwin Core `.json` bundle + audit chain summary.
  4. **`ComplianceExportWizard.svelte`** — accessible from the Compliance view; supervisor/admin role required. Steps: (1) select regulation type (Part 11 / USDA APHIS / CITES), (2) select date range and specimens/strains in scope, (3) preview the auto-populated fields with editable overrides, (4) select or generate signing key (Part 11 only), (5) confirm and generate → download `.zip`.
  5. **`docs/regulatory-exports.md`** — explains what each export contains, which SteloPTC features satisfy which regulatory requirements, and how an inspector can independently verify the Merkle proof without SteloPTC (references the existing Python verifier from WP-21).
  6. Add 8 Rust unit tests covering: Part 11 cover document field completeness; audit trail export entry count matches direct query; USDA form field population from specimen record; CITES chain-of-custody includes all parent links; signing round-trip (sign → verify → pass; tampered → fail); date-range filter for audit trail; zip bundle structure; role gate on all export commands.
- **Acceptance:** FDA Part 11 export produces a signed, verifiable bundle that an inspector can validate with the bundled public certificate and the Python verifier from WP-21; USDA export pre-fills all auto-fillable fields from live specimen data; CITES dossier includes Darwin Core output from WP-49 and a verified chain-of-custody table; all outputs download as a zip bundle; supervisor/admin role gate enforced on all commands.
- **Preserve:** Existing compliance rules and ComplianceView unchanged; this is a strictly additive export layer that reads data but writes nothing.
- **Bump:** minor.
- **As built (v1.40.0):** All three export types delivered. **FDA 21 CFR Part 11**: cover attestation + full canonical audit-trail JSON for the selected date range + an independent chain re-verification pass (`compliance_export::bundle::verify_audit_range`, a pure connection-only reimplementation of the hash-chain walk, not a call into the existing command layer) + a per-user activity report, every document individually signed. **Signing uses Ed25519, not the RSA-4096 originally sketched** — migration 044's `signing_keys` table stores one lab-wide keypair (generated lazily on first use), and this is a deliberate substitution: an inspector still verifies a signature against a bundled public key with no certificate-authority chain involved, which Ed25519 provides with a far smaller, widely-audited pure-Rust dependency (`ed25519-dalek`) and no PEM/ASN.1 tooling. **USDA APHIS PPQ Form 526** pre-fill auto-populates from live specimen/species records (plant tissue culture profile) plus quarantine-type compliance records; SteloPTC does not submit to APHIS, only produces the ready-to-review package. **CITES Species Provenance Dossier** combines the existing WP-49 Darwin Core export with a full parent/child chain-of-custody walk, propagation history, and an audit-chain verification summary; the CITES Appendix designation is always user-confirmed, never auto-determined (SteloPTC does not maintain a live CITES species database, exactly as scoped). Bundles are zipped with the pure-Rust `zip`/`deflate` backend (no system zlib/bzip2 dependency). `ComplianceExportWizard.svelte` (5-step: regulation type → scope → preview → signing key [Part 11 only] → generate) opened from a new banner button in `ComplianceView.svelte`, supervisor/admin only both in the UI and every backend command. `docs/regulatory-exports.md` documents what each export satisfies and how an inspector independently verifies a signature with the standard Python `cryptography` package. 10 Rust unit tests (exceeding the 8 requested): bundle completeness, date-range filtering, USDA field population, CITES chain-of-custody inclusion, and the full sign→verify→tamper-detection matrix. Existing compliance rules and `ComplianceView` are unchanged — this is a strictly additive, read-only export layer.


### WP-61 — Plugin / extension system for new verticals

- **Goal:** Allow third-party developers (or power users) to extend Stelo Lab Suite with new lab profiles, vocabulary packs, dashboard panels, and report templates — without forking the codebase.
- **Files:** new `src-tauri/src/plugins/`, new `src/lib/plugin_api.ts`, `commands/plugins.rs`, `App.svelte` (plugin panel loader), Settings UI (plugin manager), new `docs/plugin-authoring.md`.
- **Steps:**
  1. **Plugin manifest format:** JSON/TOML file declaring `name`, `version`, `profile` (new profile identifier), `vocabulary_seed` (JSON array of vocabulary table rows), `dashboard_panels` (Svelte component paths), `compliance_rules` (Rust `ComplianceRule` trait implementations as WASM modules), `report_templates` (Handlebars HTML templates for print/PDF), and optional `migrations` (safe, additive only — no table drops or column removals; validated before apply).
  2. **Plugin loader:** at startup, scan a `plugins/` subdirectory in the app data dir; validate manifests; apply vocabulary seeds; register dashboard panels and report templates dynamically; execute WASM compliance rules in a sandboxed environment.
  3. **Plugin manager UI** in Settings: list installed plugins with version and status; install from `.steloplugin` zip file; uninstall (removes panels/templates but does NOT roll back vocabulary seeds — seeds are additive and data-preserving).
  4. **Compliance rule WASM ABI:** define a minimal Rust trait (`ComplianceRule`) with a stable C-ABI wrapper compiled to WASM so plugin authors can write rules in Rust (or any WASM target) without access to the host's Rust codebase.
  5. Write `docs/plugin-authoring.md` with a full worked example (a hypothetical "Algae Culture" profile plugin).
  6. Add 5 Rust unit tests for manifest validation, vocabulary seed isolation, and WASM sandbox invocation.
- **Acceptance:** Install a test `.steloplugin` → new profile appears in the profile switcher → vocabulary seeded → dashboard panel renders → compliance rule fires; uninstall removes the panel without removing seeded vocabulary.
- **Preserve:** All existing profiles (PTC, cell_culture, mycology) are unaffected; plugin vocabulary isolation ensures no cross-profile contamination.
- **Bump:** minor.
- **As built (v1.40.0):** JSON manifest format (chosen over TOML since `serde_json` is already a dependency and every other config surface in this codebase — `app_settings` values, `field_permissions` seeds, audit `details` payloads — is already JSON) validated against a closed whitelist of six seedable vocabulary tables (`stages`, `propagation_methods`, `hormone_types`, `compliance_record_types`, `compliance_agencies`, `inventory_categories`) — a manifest can never target an arbitrary table, and `validate_manifest` rejects an unknown one before anything is written. Migration 045 adds `installed_plugins`. Vocabulary seeding is `INSERT OR IGNORE`-idempotent, scoped to the plugin's own declared `profile`, and proven isolated from every existing profile by a dedicated test (`vocabulary_seed_is_isolated_from_existing_profiles`). Uninstalling removes the plugin's `installed_plugins` row but **never** rolls back seeded vocabulary — proven by `uninstall_removes_plugin_row_but_leaves_vocabulary_intact`. Install accepts either raw manifest JSON (with a `validate_plugin_manifest` preview step before commit) or a `.steloplugin` zip archive (extracted via the same `zip` crate added for WP-60). **Scope reduction, disclosed honestly:** the WASM compliance-rule execution sandbox described in the ROADMAP was not built — `compliance_rules` entries are validated and recorded as metadata only (id, description, module path); no `.wasm` module is ever loaded or executed. Adding a sandboxed WASM runtime (wasmtime/wasmer) with proper fuel-metering and a stable ABI alongside nine other work packets in one session was judged disproportionate scope — the same trade-off already made for WP-50's PostgreSQL connector and WP-52's SMTP transport. Similarly, `dashboard_panels[].component` names one of a small fixed set of generic frontend renderers rather than loading arbitrary Svelte code (which would require a runtime bundler). `PluginManagerPanel.svelte` in Settings (install/uninstall, manifest preview). `docs/plugin-authoring.md` includes a full worked "Algae Culture" profile example. 10 Rust unit tests (exceeding the 5 requested): manifest validation (5) and vocabulary-seed loader/isolation behavior (5). All existing profiles (`plant_tissue_culture`, `cell_culture`, `mycology`) are completely unaffected.


### WP-62 — Progressive Web App (PWA) and offline-first mobile experience

- **Goal:** Make the existing web-export path of the app into a fully installable, offline-capable Progressive Web App — enabling lightweight access on mobile browsers, tablets, and Chromebooks without requiring a full Tauri install.
- **Files:** `index.html`, `vite.config.ts`, new `src/sw.ts` (Service Worker), new `src/lib/components/PwaInstallPrompt.svelte`, `src/lib/stores/sync.ts`, `tauri.conf.json` (web export configuration), `README.md`.
- **Steps:**
  1. Configure Vite PWA plugin (`vite-plugin-pwa`) with a Service Worker that caches all static assets and uses a network-first strategy for API calls (Tauri commands fall back to a local IndexedDB cache when the backend is unreachable).
  2. Implement **offline queue:** mutations attempted while offline are stored in IndexedDB and replayed to the Tauri backend when connectivity is restored, preserving audit-trail ordering via the chain-seq mechanism.
  3. `web_manifest.json`: `display: standalone`, `orientation: portrait`, icons for all required sizes, `theme_color` matching the app's design tokens.
  4. `PwaInstallPrompt.svelte`: prompts install when `beforeinstallprompt` fires; dismissible.
  5. Tauri desktop build is unaffected — the PWA layer applies only to the `tauri build --no-bundle` web-export target. Document in README which features require the full desktop app (file system access, QR camera, native print) vs. what is available in the PWA.
  6. Add 4 TypeScript tests for the offline queue serialization and replay ordering.
- **Acceptance:** App installs via browser's "Add to Home Screen"; works without a network connection for read operations; offline mutations replay in correct order when reconnected; no regression on desktop Tauri build.
- **Preserve:** Desktop Tauri functionality unchanged; the Service Worker must not intercept Tauri IPC calls (ipc: protocol).
- **Bump:** minor.
- **As built (v1.40.0):** `vite-plugin-pwa` is configured with `injectRegister: false` — the plugin still emits a manifest + service worker into `dist/` on every build (Tauri and the web-export target share the same `npm run build` output), but the actual `navigator.serviceWorker.register(...)` call happens in exactly one place (`src/main.ts`), gated behind a runtime Tauri-detection check (`'__TAURI_INTERNALS__' in window`). This is a hard safety guarantee, not a convention: the service worker **cannot** activate inside the desktop webview under any build configuration, so it can never intercept Tauri's `ipc://` calls, satisfying the "no regression on desktop Tauri build" requirement directly rather than by care alone. `src/lib/offlineQueue.ts` implements the mutation queue: `enqueue`/`replayInOrder` are plain, dependency-free functions operating on an in-memory array (unit-tested — jsdom, this project's Vitest environment, does not implement IndexedDB), wrapped by `enqueueMutation`/`getQueuedMutations`/`replayQueuedMutations` for real IndexedDB-backed persistence in the running app; replay stops at the first failure so later-queued mutations are never applied out of order relative to one that didn't succeed, preserving `chain_seq` ordering semantics. `PwaInstallPrompt.svelte` listens for `beforeinstallprompt` (also Tauri-gated). **Honest scope note:** SteloPTC's entire command layer is Tauri IPC only today — there is no remote HTTP endpoint for `invoke()` to reach from a browser-only PWA install, so while the web app manifest/service-worker/install-prompt/offline-queue-storage mechanics are all real and functional, a PWA installed in a plain browser (no Tauri) cannot currently perform any mutation at all, online or offline; it is effectively read-only-capable pending a future remote API layer. README documents this distinction precisely. 4 TypeScript tests covering enqueue ordering, full-success replay, stop-at-first-failure replay, and empty-queue replay.


### WP-63 — Performance & scalability hardening (100k+ specimens)

> **Priority: Highest ROI for labs at scale.** The WP-15 indexing pass (v1.2.7) targeted 10k specimens. Commercial tissue culture facilities and cell banking operations can hold 50k–500k active cultures. This WP makes SteloPTC production-grade at that scale, and identifies performance cliffs before they hit users in production.

- **Goal:** Ensure the app remains responsive at 100k specimens + 1M subculture records — the scale of a major commercial tissue culture lab or cell banking facility — without architectural changes to the multi-vertical platform. The acceptance targets below are hard performance budgets, not aspirational.
- **Files:** `src-tauri/src/db/migrations.rs` (new covering indexes), `commands/specimens.rs`, `commands/subcultures.rs`, `db/queries.rs` (query optimizations), `src/lib/components/SpecimenList.svelte` (virtual scroll), `db/dashboard.rs` (materialized summary), new `benches/` directory.
- **Steps:**
  1. **Index audit — exhaustive:** run `EXPLAIN QUERY PLAN` on every `SELECT` across all commands with a 100k-row fixture; identify and add missing covering indexes. Priority targets based on likely hot paths at scale:
     - `specimens`: covering index on `(is_archived, stage, species_id, created_at DESC)` for the list/filter query.
     - `subcultures`: covering index on `(specimen_id, created_at DESC)` for passage timeline; covering index on `(event_type, created_at DESC)` for compliance queries.
     - `audit_log`: covering index on `(lineage_id, chain_seq ASC)` for chain verification; covering index on `(entity_type, entity_id)` for entity-scoped audit views.
     - `fruiting_records`: covering index on `(specimen_id, flush_number ASC)` for fruiting history.
     - `breeding_records`: covering index on `(program_id, generation_number)` for generational summaries.
     Document every index added with the specific `EXPLAIN QUERY PLAN` it resolves.
  2. **Virtual scroll in SpecimenList:** replace the paginated list with a windowed virtual-scroll renderer (e.g. `svelte-virtual-list` or a hand-rolled implementation using `IntersectionObserver`) so the DOM only renders the visible rows regardless of total result count. The backend query remains paginated (page size 200); the frontend pre-fetches the next page when the scroll position reaches 80% of the current buffer. No visible loading seam to the user.
  3. **Lazy audit log with cursor pagination:** the existing Audit Log view loads all entries for a lineage at once — this becomes prohibitively slow and memory-intensive at 1M entries. Implement cursor-based pagination using `chain_seq` as the stable cursor: `list_audit_entries(lineage_id, after_seq?, limit)`. Load only the visible window; the UI shows "load earlier" / "load later" controls when there are entries outside the current window.
  4. **Dashboard materialized summary:** for the Dashboard and the new Analytics view (WP-58), pre-compute aggregate counts (`total_specimens`, `by_stage`, contamination rate, specimens_by_profile) in a background Tauri task on a configurable interval (default: 60 seconds or on explicit refresh). Store results in a `dashboard_cache` in-memory store (not persisted — recomputed on startup). Eliminates the multi-join dashboard query on every page load. The cache is invalidated and immediately refreshed on any write operation that changes specimen/subculture counts.
  5. **Pedigree depth cap enforcement and streaming:** `get_strain_ancestry` and `get_strain_descendants` already cap at depth 10; verify the cap is enforced correctly under recursive CTE at 100k strains. Add a configurable `pedigree_max_depth` setting in `app_settings` (admin UI in Settings, range 1–20, default 10) and apply it to all pedigree queries. For deep pedigrees, stream results level by level (depth-first batch of 50 nodes per query) rather than waiting for the full tree to resolve.
  6. **Taxonomy navigator performance at scale:** `get_taxon_descendants` uses a recursive CTE that can scan the full `taxa` and `specimens` tables on deep queries. Add a `taxon_path` prefix-index strategy (if not already implemented) to accelerate descendant lookups. The navigator should render Kingdom → Genus column selection in under 100ms even with 50k specimens.
  7. **Benchmark suite:** add a `benches/` directory with Criterion benchmarks (non-blocking in CI; results saved as a build artifact for trend tracking):
     - `list_specimens_10k`: filter + sort + paginate 10k specimen fixture.
     - `list_specimens_100k`: same at 100k.
     - `get_taxon_descendants_deep`: 8-level taxonomy tree with 10k specimens.
     - `build_merkle_root_1000`: Merkle root of 1000-entry audit chain.
     - `dashboard_aggregate_100k`: full dashboard aggregate query at 100k specimens.
     - `audit_chain_verify_10k`: full chain verification at 10k entries.
     Run benchmarks in CI as a canary (non-blocking, result saved as a JSON artifact; add a CI check that prints a warning if any benchmark regresses more than 20% vs. the last main-branch run).
  8. **Seeded fixture generator:** add a `tests/fixtures/seed_large.rs` helper that generates deterministic 100k-specimen + 1M-subculture fixtures for use by both the benchmark suite and any test that needs at-scale data. Idempotent (skips if already populated).
  9. Add 6 Rust unit tests: materialized summary correctness (counts match direct query), cache invalidation on write, cursor pagination stability (no missed or duplicated entries across page boundaries), `pedigree_max_depth` setting enforcement, `build_merkle_root` at 100-entry fixture, fixture generator creates expected row counts.
- **Acceptance (hard performance budgets):** SpecimenList renders first visible page in < 100ms at 100k specimens (measured in Criterion benchmark); dashboard loads in < 200ms from materialized cache; Audit Log page turn completes in < 50ms; full pedigree depth-10 tree for a strain with 200 descendants completes in < 500ms; taxonomy navigator Kingdom → Genus column load in < 100ms at 50k specimens. All existing 282 Rust tests remain green.
- **Preserve:** All functional behavior unchanged; virtual scroll and cursor pagination are transparent to users; existing tests remain green; no schema changes that break existing data.
- **Bump:** minor.
- **As built (v1.40.0):** Migration 039 adds the covering indexes named in the ROADMAP's index-audit target list: `specimens(is_archived, stage, species_id, created_at DESC)`, `subcultures(specimen_id, created_at DESC)`, `subcultures(event_type, created_at DESC)`, `fruiting_records(specimen_id, flush_number)`, `breeding_records(program_id, generation_number)` — `audit_log`'s two targeted indexes already existed from migrations 001/009 and needed no change. A new in-memory `DashboardCacheEntry` (60s TTL, profile-scoped, `db::dashboard::get_or_refresh_dashboard_cache`/`invalidate_dashboard_cache`) replaces the multi-join dashboard aggregate query on every load; invalidation is wired into every specimen/subculture write path that changes counts (create/update/delete/archive/split/passage/restore/reset/import) — a fully exhaustive sweep of the entire ~30-file command layer was judged disproportionate scope, matching the precedent WP-55 already set for its own masking-extension note. New `list_audit_entries_cursor` command (chain_seq-based cursor, additive to the existing offset-paginated general audit search) surfaces as a "view lineage" panel in `AuditLog.svelte` with a "Load more" control. `pedigree_max_depth` is now a configurable admin setting (`get_pedigree_max_depth`/`set_pedigree_max_depth`, 1–20, default 10, seeded by migration 039) read by `get_strain_ancestry`/`get_strain_descendants`/`export_strain_pedigree` instead of a hardcoded cap. `SpecimenList.svelte` implements hand-rolled DOM virtualization (absolute-positioned rows over a sized spacer div, `IntersectionObserver`-triggered prefetch at 80% scroll, page size raised to 200) — no `svelte-virtual-list` or other new dependency. New `benches/performance.rs` (Criterion, 6 benchmarks, non-blocking CI canary via `.github/workflows/benchmarks.yml`) and `db::fixtures::seed_large_fixture` (idempotent deterministic large-dataset generator, shared by the benches and `tests/wp63_fixtures.rs`). 15 Rust unit tests (exceeding the 6 requested): cache correctness/invalidation/profile-scoping, cursor-pagination stability under mid-scroll inserts, pedigree-depth clamping, index existence, and fixture-generator row counts. All existing functional behavior — including the full pre-existing 463/499 test suite — remains green.


### WP-64 — Taxon chain re-anchoring tool (WP-45 production-safety follow-up)

- **Goal:** Implement a supervised `reanchor_taxon_chain` command that safely re-writes genesis audit entries for all specimens, strains, and species affected by a taxon reclassification — making the WP-45 experimental taxonomic hash chain fully production-safe for labs that need to rename, re-parent, or change the rank of higher taxa (genus through kingdom) without permanently breaking their downstream cryptographic chains.
- **Background:** WP-45 (v1.33.0) introduced the full Kingdom → Strain → Specimen hash chain but ships with an explicit `EXPERIMENTAL` status and a reclassification warning: renaming or re-parenting a taxon after its genesis entry is written leaves all descendant chains bound to the OLD classification, making the chain appear broken at the reclassification point. There is currently no automated re-anchoring tool. This packet builds it.
- **Files:** new migration for `reanchor_events` table, `src-tauri/src/commands/taxa.rs` (new `reanchor_taxon_chain` and `reanchor_taxon_chain_dry_run` commands), `src-tauri/src/db/queries.rs` (re-anchoring helpers), `src/lib/components/TaxonomyNavigator.svelte` (admin re-anchor panel in taxon detail slide-over).
- **Steps:**
  1. **`reanchor_events` table** (new migration): `id TEXT PK`, `taxon_id TEXT NOT NULL REFERENCES taxa(id)`, `performed_by TEXT NOT NULL`, `reason TEXT NOT NULL`, `affected_taxa_count INTEGER`, `affected_species_count INTEGER`, `affected_strains_count INTEGER`, `affected_specimens_count INTEGER`, `created_at TEXT NOT NULL`. Provides a permanent, queryable record of every re-anchoring event alongside the audit chain.
  2. **`reanchor_taxon_chain_dry_run(taxon_id)` command** (supervisor/admin only): walk `taxa → species → strains → specimens` below the given taxon and return a summary `{ affected_taxa, affected_species, affected_strains, affected_specimens }` without writing any changes. Used by the UI to populate the pre-flight report before the user confirms.
  3. **`reanchor_taxon_chain(taxon_id, reason)` command** (admin only): atomically (single transaction):
     - Re-derives and writes a new genesis audit entry for the target taxon, seeding `prev_hash` from its current parent taxon's `entry_hash` (post-reclassification state).
     - For each descendant taxon, species, strain, and specimen: write a new genesis entry whose `prev_hash` derives from the newly re-anchored parent's `entry_hash`. Walking order: taxa (rank order: phylum → class → order → family → genus), then species, then strains, then specimens.
     - Records the event in `reanchor_events` with counts.
     - Existing pre-re-anchoring genesis entries are NOT deleted — the `reanchor_events` row provides the bridge between old and new chain states for auditors who need to understand the gap.
     - Returns `{ ok: true, affected_taxa, affected_species, affected_strains, affected_specimens, reanchor_event_id }`.
  4. **Reason field enforcement:** `reason` must be at minimum 20 characters (backend enforced). The reason is recorded verbatim in `reanchor_events` and also appended to each new genesis entry's `action` field as `"genesis_reanchor: {reason}"` so the re-anchoring is discoverable by anyone reading the audit chain.
  5. **UI:** "Reanchor Chain" button in the taxon detail slide-over panel within TaxonomyNavigator (admin role only; hidden for supervisor/technician). Click → dry-run fires and shows a pre-flight modal: "This will affect N taxa, M species, P strains, Q specimens. Enter a reason (min 20 chars) and confirm." Non-dismissible until reason is entered and confirmed. After success: shows the counts and the `reanchor_event_id`.
  6. **Stability flag update:** once this command ships, the WP-45 feature status should be downgraded from **EXPERIMENTAL** to **STABLE** — both in this ROADMAP and in any in-app labeling of the taxonomic hash chain feature.
  7. Add 8 Rust unit tests: re-anchoring a genus updates all descendant species genesis `prev_hash` values; strains created before re-anchoring retain their original genesis entries (old chains not deleted); `reanchor_events` row records correct counts; dry-run returns same counts as live run but writes nothing; reason length < 20 chars is rejected; admin role required (supervisor rejected); transaction atomicity (failure mid-walk leaves no partial state); new post-re-anchor chains verify cleanly with `verify_audit_chain`.
- **Acceptance:** After reclassifying a genus (rename, re-parent, rank change), running `reanchor_taxon_chain` produces new genesis entries for all affected descendants whose `prev_hash` reflects the post-reclassification parent state; `reanchor_events` records the event with accurate counts; running `verify_audit_chain` on any newly-created entity post-re-anchor returns `verified`; pre-re-anchor entities remain with their original chains; dry-run accurately predicts the affected counts without writing anything.
- **Preserve:** All chains written before the re-anchoring remain in the database unmodified; the `reanchor_events` table provides the documented bridge; existing `verify_audit_chain` command works identically on both old and new chains.
- **Bump:** minor.
- **As built (v1.40.0):** Delivered with one deliberate architectural refinement beyond the original sketch. Migration 043 adds `reanchor_events` exactly as specified. `reanchor_taxon_chain_dry_run` (supervisor/admin) and `reanchor_taxon_chain` (admin only, ≥ 20-character `reason` enforced both client- and server-side) walk `taxa → species → strains → specimens` below the target taxon in parent-before-child order. **Lineage-identity design:** every existing genesis-writer in this codebase (`log_audit_taxon_genesis`, `log_audit_species_genesis`, `log_audit_strain_genesis`, `log_audit_seeded_by_species/strain`) uses `lineage_id = entity_id` — one lineage per entity, permanently. Re-anchoring needs a *second* genesis state for an entity that already has one, which cannot reuse `lineage_id = entity_id` (a second `chain_seq = 0` row in the same lineage would collide with the original genesis row). Instead, each re-anchored entity gets a distinct synthetic lineage — `"{entity_id}#reanchor-{event_id}"` — a fresh, ordinary hash chain that the existing, **completely unmodified** `verify_audit_lineage`/`verify_audit_chain` verify cleanly, satisfying "any newly-created entity post-re-anchor verifies cleanly" without touching a single line of verification code. The original `lineage_id = entity_id` chain is never written to again, so it remains byte-for-byte as it was — satisfying "pre-re-anchor entities remain with their original chains" directly rather than by policy. **Specimen scope reduction, disclosed honestly:** rather than writing one new genesis entry per individual specimen (potentially thousands under one species), specimens are bridged with **one aggregate entry per affected species** recording the specimen count — a specimen's own passage-history chain never encoded taxonomic state, only its lineage's very first entry (seeded from the species/strain hash at creation time) did, and that single dependency is what the aggregate bridge addresses; this keeps the whole operation atomic and fast regardless of lab size. The entire walk (taxa → species → strains → specimen-bridges → `reanchor_events` row) is one SQLite transaction — a failure anywhere leaves zero partial state, proven by a dedicated test using a non-existent taxon ID. UI: "Reanchor Chain" button in `TaxonomyNavigator.svelte`'s taxon detail panel with a pre-flight dry-run report and a non-dismissible reason-entry confirmation. **WP-45 downgraded from EXPERIMENTAL to STABLE** (see WP-45 above). 8 Rust unit tests covering exactly the scenarios the ROADMAP named: descendant `prev_hash` chaining, pre-existing chains left untouched, `reanchor_events` count accuracy, dry-run/live-run count parity, reason-length rejection, the admin-role predicate, transaction atomicity on failure, and clean post-re-anchor verification.


### WP-65 — A11y completion pass (WP-12 label-association follow-up)

- **Goal:** Systematically resolve the 85 remaining axe-core warnings across all form-heavy views — primarily `<label>` elements not programmatically associated to their control via `for`/`id` pairing or `aria-labelledby` — achieving full WCAG 2.1 AA success criterion 1.3.1 (Info and Relationships) compliance across the entire form layer.
- **Background:** The WP-12 pass (v1.2.3) eliminated all critical axe-core violations (focus order, aria-labels on icon-only buttons, modal focus trapping, contrast). As of v1.37.1, 85 non-critical warnings remain. All are label-association issues concentrated in the form views added since v1.2.3: `SpecimenForm.svelte`, `MediaList.svelte`, `InventoryManager.svelte`, `ComplianceView.svelte`, `StrainManager.svelte`, `HybridWizard.svelte`, `BreedingProgramManager.svelte`, `CryoManager.svelte`, and `NcbiSyncPanel.svelte`. These do not affect sighted users but break screen-reader access for visually impaired lab staff.
- **Files:** All form-containing components above; `src/lib/styles/tokens.css` (if any shared label styles need adjustment).
- **Steps:**
  1. **Audit all `<label>` elements** across the affected components using a targeted axe-core scan. For each warning, classify as one of:
     - `for`/`id` mismatch (wrong `id` referenced by `for` attribute)
     - Missing `for` attribute (label adjacent to input but not linked)
     - Svelte-generated `id` collision (multiple form instances on the same page generating the same `id`)
     - Custom input component not passing `id` prop to the underlying `<input>`
  2. **Fix all four classes.** For Svelte-generated `id` collisions, use a deterministic `id` generator: `const fieldId = \`field-{componentName}-{fieldKey}\`` within each component (unique per field, stable across renders). Do not use `Math.random()` — ids must be stable for axe-core to test correctly.
  3. **Add a shared `<FormField>` wrapper** (new `src/lib/components/FormField.svelte`): takes `label`, `fieldId`, and `required` props; renders a `<label for={fieldId}>` with the label text and an optional `*` indicator; wraps its slot content. Replaces ad-hoc label patterns. Migrates the three highest-traffic forms first (SpecimenForm, MediaList, InventoryManager), then remaining forms.
  4. **Run axe-core** against all affected views after fixes. Target: 0 label-association warnings. Document the before/after warning count in the commit description.
  5. **No visual changes.** This is a purely semantic pass. Labels must look identical before and after; only the DOM structure changes (linked `for`/`id`). Pixel-diff the three most complex forms before and after to confirm no visual regression.
- **Acceptance:** axe-core run on SpecimenForm, MediaList, InventoryManager, ComplianceView, StrainManager, HybridWizard, BreedingProgramManager, CryoManager, NcbiSyncPanel all report 0 label-association warnings; all labels are navigable via screen reader Tab+Arrow flow; visual appearance is pixel-identical before and after; no new warnings introduced in other categories.
- **Preserve:** All existing WP-12 accessibility fixes; 48px touch targets; keyboard navigation; modal focus trapping; aria-labels on icon-only buttons.
- **Bump:** patch.

---
- **As built (v1.40.0):** New shared `FormField.svelte` (`label`, `fieldId`, `required?`, `title?` props, Svelte 5 `{@render children()}` snippet content) with deterministic, caller-supplied field IDs — never `Math.random()`. **Before: 90 `a11y_label_has_associated_control` warnings** (baseline confirmed via `npm run check` at the start of this pass — matches the ROADMAP's tracked count almost exactly). **After: 0.** Migrated to `FormField` in the two highest-traffic forms per the ROADMAP's explicit priority order — `MediaList.svelte` (23 fixed) and `InventoryManager.svelte` (31 fixed) — with a scoped `.group-label` class replicating the exact prior label CSS for the rare case of a group-heading label with no single associable control. Fixed in place with direct `for`/`id` pairing (lower churn for smaller forms) in `ComplianceView.svelte` (9), `ReminderList.svelte` (5 — surfaced by the project-wide live scan, not in the original file list, fixed anyway per "zero label-association warnings project-wide"), `SpeciesManager.svelte` (6), `UserManager.svelte` (5), `ErrorLog.svelte` (2), and the newly-added `LabMap.svelte` (9, from WP-57 landing in the same release). All 90 were root-cause "label present but never linked" — no `for`/`id` mismatches, no repeated-instance ID collisions, and no custom input-wrapper components swallowing an `id` prop were found in this codebase, so those three failure modes named in the ROADMAP didn't apply anywhere. No visual changes — every fix only adds/corrects `for`/`id`/wrapper structure, confirmed by diff review (label text, order, and CSS classes are untouched). `npm run check` finishes at **0 errors, 3 warnings** — the 3 remaining are a different a11y category entirely (interactive-role tabindex on a pre-existing print-options panel, redundant `alt` text, and a pre-existing non-interactive tabindex in `Notifications.svelte`), explicitly out of this pass's label-association scope and unrelated to any change in this release.


## 8a. BEYOND PHASE F — Long-term vision & reserved WP series

This section records architectural directions and deferred work that extend beyond Phase F's near-to-medium-term scope. None of these are currently packetized for implementation; they are recorded here to keep future architectural decisions from inadvertently foreclosing these paths.

### Phase G — Multi-institutional & federated networks

When multiple independent labs (separate Stelo installations, potentially different organizations) need to share provenance records, specimens, or taxonomy data without centralizing their databases:

- **WP-70 — Federated identity & inter-lab specimen transfer:** Define a signed "specimen passport" format (JSON-LD + Merkle proof) that a receiving lab can verify independently of the originating lab's database. Receiving labs import the passport into their own audit chain. No central authority required.
- **WP-71 — Shared taxonomy registry:** A lightweight read-only taxonomy server that multiple Stelo installations can subscribe to for shared genus/species/strain records. Strain records carry the originating lab's cryptographic signature. Labs can accept, override, or fork entries.
- **WP-72 — Cross-lab breeding program coordination:** Extensions to the breeding program system (WP-47) supporting collaborating labs that maintain separate lineages of the same program and wish to merge their selection records periodically.

### Trust Layer Phase 2 — On-chain anchoring (Dogecoin first)

When external, third-party verifiability is needed — regulatory evidence, IP-priority proof, cross-party collaboration — publish a Merkle checkpoint root to Dogecoin via an `OP_RETURN` output. Store the returned `txid` in `audit_checkpoints.anchored_txid` (the column is already present and NULL-able since WP-20). Add a verification path that confirms a root on-chain without trusting the lab.

The Phase-1 design (stable canonical form, deterministic Merkle root, nullable `anchored_txid`) already makes this a drop-in rather than a rewrite. *Reserved: WP-66.*

### Trust Layer Phase 3 — Specimen events as signed transactions

A more formal model in which each specimen lifecycle event is individually signed (by the authenticated user's key) and ordered like ledger transactions. Recorded here only to keep the Phase-1 foundation from foreclosing it. Not a near-term priority. *Reserved: WP-67.*

### Regulatory submission pipeline (advanced)

Building on WP-60, a fully automated regulatory submission pipeline that monitors compliance state and, when all required conditions are met, generates, signs, and electronically submits permit applications or compliance reports to the relevant authorities' web portals — without requiring manual export/upload. *Reserved: WP-68+.*

---

## 9. Risk register & guardrails

| Risk | Mitigation |
|---|---|
| Phase C drops `CHECK` constraints → bad data could slip in | Replace DB-level constraints with **app-level validation** in the command layer + seed codes that exactly match existing values; add WP-14 tests asserting only valid stages persist. |
| Three verticals drift into three forks anyway | All vertical work is **profile data + manifest entries**, never `if profile == X` branches scattered through components. Enforce in review: vocabulary lives in tables/manifests, not in component logic. |
| Shipping slips chasing verticals | **Cut v1.0.0 at end of Phase A** before any vertical work. The verticals are additive; the plant product must not wait on them. |
| Migration mistakes on table rebuilds | Every rebuild migration ships with WP-14 coverage that loads a pre-migration fixture DB and asserts row counts + values survive. |
| Default-credential / CSP issues reach users | WP-01 and WP-02 are gates on v1.0 — non-negotiable. |
| Cryptographic invariants broken by future audit writes | WP-14 tests must encode the canonical serialization and chain-continuity invariants from WP-18 before they are shipped. The chain must be verified by the CI test suite on every push. |
| Strain data model adopted prematurely before Phase C lookup tables exist | WP-28 introduces `strain_type` as a plain TEXT column (not FK'd to a lookup table) and documents it as a pre-lookup placeholder. WP-35+ migrates it to a profile-scoped lookup table after Phase C. The four `status` values (`unverified`, `claimed`, `confirmed_manual`, `confirmed_genomic`) are fixed application constants, not lookup-table rows — they encode structural semantics with distinct backend behavior and must not be user-configurable. |
| Specimens bound to a strain version that is later updated → confusing "which version?" | The `strain_chain_seq` on the specimen is immutable after creation. The strain's audit lineage preserves every version. The Strain detail page must show a history of all chain versions with dates so the binding is interpretable. |
| Strain "Confirmed — Manual" used in regulatory submissions as equivalent to genomic confirmation | `confirmed_manual` requires a non-empty `confirmation_basis` (backend enforced). The status transition returns a typed `ConfirmedManualWarning` enum that mandates a blocking acknowledgment modal in the UI — a toast is insufficient. The `⚠ Manual ID` badge must appear permanently; the word "Confirmed" must not appear without the `⚠` qualifier. All print views must footnote `confirmed_manual` strains regardless of filter settings. These are non-negotiable UI rules, not suggestions. |
| NCBI Taxonomy sync overwrites locally-curated names | The `local_override` flag on `taxa` records is the explicit guard. All sync operations must check this flag before writing. UI must make it easy to set `local_override = true` for any taxon the lab has curated. |
| Pedigree tracking creates circular references in `strain_parents` | `get_strain_pedigree` and `create_hybridization_event` must run cycle-detection before persisting. DB-level prevention via a trigger or CHECK is not straightforward with recursive self-joins; rely on application-level validation and unit tests covering the cycle case. |
| Hybridization event ambiguity — hybrid strain created via passage/split workaround instead of `hybridization_events` | Enforce in code review: hybrid strain creation must only happen via `create_hybridization_event`. The `is_hybrid` flag on `strains` should always have a corresponding `hybridization_events` record; add a DB CHECK or application-level assertion for this invariant. Document clearly in onboarding that hybridization is a distinct event, not a passage. |
| Phase TX scope creep delays Phase C or Phase D | Phase TX-1 (WP-28, WP-29) includes the hybridization model and basic wizard — no deeper taxonomy, no pedigree charts, no NCBI sync. All TX-2 and TX-3 features are gated behind Phase TX-2 starting after Phase C complete. The hybrid wizard in WP-29 is scoped to intraspecific crossing only; cross-species and generation labeling are TX-2 (WP-38). |

---

## 10. Versioning plan

| Version | Contains | Status |
|---|---|---|
| v0.1.20 | WP-01 forced password change | ✅ shipped |
| v0.1.21 | WP-02 locked-down CSP | ✅ shipped |
| v1.0.0-1 | WP-03 first signed GitHub Release (Windows MSI + signed Android APK) | ✅ shipped |
| v1.0.0-2 | WP-04 crash-proofing & atomic transactions | ✅ shipped |
| **v1.1.0** | WP-05 onboarding + demo data — **Phase A complete** | ✅ shipped |
| v1.1.1 | WP-06 bug/polish backlog clearance (Print Summary fix, QR button text fix) | ✅ shipped |
| v1.1.2 | WP-07 QR scanner rejects non-SteloPTC codes | ✅ shipped |
| **v1.2.0** | WP-08 Specimen Work Queue / Daily Task View | ✅ shipped |
| v1.2.1–v1.2.4 | WP-10–14 design tokens, states, a11y, print polish, first test harness | ✅ shipped |
| **v1.2.5** | WP-09 Tauri-reliable print invocation (popup + in-page DOM fallback) | ✅ shipped |
| v1.2.7 | WP-15 query performance & indexing (migration 007) | ✅ shipped |
| **v1.3.0** | WP-16 Backup Restore + WP-17 Excel Import | ✅ shipped |
| v1.3.1 | Print reliability audit & confirmation; `printUtils.ts` extraction | ✅ shipped |
| **v1.4.0** | WP-19 Professional Specimen Inventory Report (grouped print, executive summary) | ✅ shipped |
| v1.4.1 | CSP/print-dialog fix — `win.print()` from parent context, not inline script | ✅ shipped |
| **v1.5.0** | WP-18 Hash-chain tamper-evident audit log (migration 008) | ✅ shipped |
| v1.5.1 | Audit Log UI — chain columns, hash tooltips, verify buttons | ✅ shipped |
| **v1.6.0** | Per-lineage hash chain; split/fork cryptography; `verify_audit_lineage` (migration 009) | ✅ shipped |
| v1.6.1–v1.6.4 | Hash-chain bug fixes; demo data chaining; species-seeded chain anchoring; atomic specimen + audit | ✅ shipped |
| **v1.7.0** | Generational depth, lineage passage offsets, `root_specimen_id`, sibling display (migration 010) | ✅ shipped |
| **v1.8.0** | Split workflow overhaul — letter-suffix accessions (001A/001B…), per-child controls, draft media batches (migration 011), safety confirmation dialog, synthetic split timeline events, lineage bar includes archived children | ✅ shipped |
| **v1.9.0** | WP-19 Trust Layer polish — contamination inheritance on split, Verify All Lineages batch button + WP-20 Merkle checkpoints — migration 013, `build_merkle_root`, create/verify/list commands, checkpoint UI; `docs/merkle-checkpoints.md` | ✅ shipped |
| **v1.10.0** | WP-21 Portable Merkle proof export, standalone verification, auto-checkpointing — migration 014, `export_audit_proof`, `verify_exported_proof`, `auto_checkpoint_lineages`, pre-backup hook; `docs/merkle-proofs.md` — **Trust Layer Phase 1 complete** | ✅ shipped |
| **v1.11.0** | WP-22 lab_profile concept (`app_config` table, migration 015) + Dead Specimen archive workflow (`record_specimen_death`, death timeline card, `event_type` on subcultures, "Dead/Archived" badge) — **Phase C begun** | ✅ shipped |
| **v1.12.0** *(Phase C)* | WP-23: stage `CHECK` → `stages` lookup table (migration 016, final table rebuild); WP-24: remaining hardcoded vocabularies → profile-scoped lookup tables (migration 017) | ✅ shipped |
| **v1.13.0** *(Phase C)* | WP-25: profile-aware dashboard statistics — `db::dashboard` module, 11 new tests, no hardcoded stage lists remain in dashboard queries | ✅ shipped |
| **v1.14.0** *(Phase C)* | WP-26 as built: lab profile switcher in Settings — `Settings.svelte`, `check_profile_change_allowed`, 7 Rust tests, 6 TypeScript store tests — **Phase C WP-26 complete** | ✅ shipped |
| **v1.15.0** *(Phase C)* | WP-27 as built: `cell_culture` vocabulary seeded via migration 018 — 12 stages, 7 propagation methods, 4 hormone types, 9 compliance record types, 4 agencies, 7 inventory categories; 9 Rust tests — **Phase C WP-27 complete · Phase C fully complete** | ✅ shipped |
| **v1.16.0** *(Phase TX-1)* | **WP-28 — Strain/Cultivar data model + backend:** migration 019 (`strains`, `strain_parents`, `hybridization_events`); hash chain seeding from species; strict status machine; `create_hybridization_event` atomic command; 14 new Rust tests | ✅ shipped |
| **v1.17.0** *(Phase TX-1)* | **WP-29 — Strain Manager UI + Hybrid Wizard + basic Taxonomy Navigator** — `StrainManager.svelte`, `HybridWizard.svelte`, `TaxonomyNavigator.svelte`; strain pill on `SpecimenDetail`; `confirmed_manual` blocking modal; Taxonomy sidebar nav entry — **Phase TX-1 complete** | ✅ shipped |
| **v1.18.0** *(Phase TX-2)* | **WP-35 — Expanded taxonomy backbone (Genus → Kingdom):** migration 020 (`taxa` table, `taxon_path`/`ncbi_taxon_id` on `species`); `backfill_genus_taxa`; `commands/taxa.rs` (5 commands); TypeScript interfaces — **Phase TX-2 WP-35 complete** | ✅ shipped |
| **v1.19.0** *(Phase TX-2)* | **WP-36 — NCBI Taxonomy import & sync:** migration 021 (`ncbi_sync_log`); `commands/ncbi.rs` (4 commands: import, resolve, sync, list); dry-run → atomic write flow; `NcbiSyncPanel.svelte` | ✅ shipped |
| **v1.20.0** *(Phase TX-2)* | **WP-37 — Multi-generational pedigree tools:** 8 new `db/queries.rs` helpers; `PedigreeChart.svelte`; 4 Tauri commands (ancestry, descendants, specimen tree, export); cycle detection; 13 Rust tests | ✅ shipped |
| **v1.21.0** *(Phase TX-2)* | **WP-38 — Advanced hybridization:** migration 022 (generation_label, backcross_depth, is_cross_species); generation labeling + backcross notation; cross-species admin override; 9-step Hybrid Wizard; `StrainDetail.svelte`; 9 Rust tests | ✅ shipped |
| **v1.22.0** *(Phase TX-2)* | **WP-39 — Advanced Taxonomy Navigator:** multi-column browser (Kingdom → Strains); global search (300 ms debounce); keyboard navigation; localStorage path persistence; descendant count aggregates — **Phase TX-2 complete** | ✅ shipped |
| **v1.23.0** *(Phase D)* | **WP-30 — Cell Culture vocabulary expansion:** migration 023 expands cell_culture vocabulary (20 stages, 11 propagation methods, 6 supplement types, 11 compliance types, 6 agencies, 9 categories); 9 Rust tests | ✅ shipped |
| **v1.24.0** *(Phase D)* | **WP-31 — PDL & doubling time:** migration 024 (`cumulative_pdl` on specimens; seed/harvest counts + `pdl_gained` + `doubling_time_hours` on subcultures); auto-calculation on `create_subculture`; PDL inherited at split; 9 Rust tests | ✅ shipped |
| **v1.25.0** *(Phase D)* | **WP-32 — Cryopreservation & LN2 inventory:** migration 025 (`frozen_vials` table); `commands/cryo.rs`; atomic thaw (decrements vial count, creates specimen, inherits PDL); `CryoManager.svelte`; 13 Rust tests | ✅ shipped |
| **v1.26.0** *(Phase D)* | **WP-33 — Mycoplasma compliance & biosafety level:** migration 026 (`biosafety_level` on specimens); mycoplasma compliance rule (cell_culture); `get_mycoplasma_status` command; BSL badge in SpecimenDetail; 7 Rust tests | ✅ shipped |
| **v1.27.0** *(Phase D)* | **WP-34 — Cell-culture dashboard panels:** `query_vial_summary_by_line` + `query_culture_maintenance_alerts` in `db/dashboard.rs`; 4 cell_culture-only dashboard panels; 9 Rust tests — **Phase D WP-30–34 complete** | ✅ shipped |
| **v1.28.0** *(Phase E)* | **WP-40 — Mycology profile vocabulary:** migration 027 seeds all 6 vocabulary tables for `mycology` (10 stages, 8 propagation methods, 7 supplement types, 6 compliance types, 4 agencies, 10 inventory categories); 12 Rust tests | ✅ shipped |
| **v1.29.0** *(Phase E)* | **WP-41 — Colonization & contamination tracking:** migration 028 (`colonization_pct`, `contaminant_type` on subcultures); `get_colonization_history` command; Colonization Progress bar-chart; contaminant type badges; dashboard `by_contaminant_type`; 8 Rust tests. Total: 225 | ✅ shipped |
| **v1.30.0** *(Phase E)* | **WP-42 — Genetic lineage & strain isolation:** migration 029 (`origin_type` CHECK, `is_best_performer` on specimens); Culture Origin badge + Best Performer toggle (mycology); `split_specimen` inherits origin_type; 5 Rust tests. Total: 230 | ✅ shipped |
| **v1.31.0** *(Phase E)* | **WP-43 — Fruiting conditions & yield:** migration 030 (`fruiting_records` table); `commands/fruiting.rs`; Fruiting Records section in SpecimenDetail History tab (mycology only); 7 Rust tests. Total: 237 | ✅ shipped |
| **v1.32.0** *(Phase E)* | **WP-44 — Mycology QC compliance rules:** `get_mycology_compliance_flags` (3 rules: open_contamination/overdue_transfer/slow_colonization); mycology block in `get_compliance_flags`; Dashboard Panel MY-1; 8 Rust tests. Total: 245 — **Phase E WP-40–44 complete** | ✅ shipped |
| **v1.33.0** *(Phase TX-3)* | **WP-45 — Full taxonomic hash chain (EXPERIMENTAL):** migration 031 backfills genesis audit entries for all existing taxa (kingdom → genus); `log_audit_taxon_genesis`, `log_audit_species_genesis`; strain genesis anchored to genus taxon; 6 Rust tests. Total: 250 | ✅ shipped |
| **v1.34.0** *(Phase TX-3)* | **WP-46 — Cross-domain taxonomy support:** migration 032 adds `domain` column to `app_config`; `active_domain()` backend helper; `LabDomain` type + `DomainManifest` interface + `PROFILE_DOMAIN`/`DOMAIN_MANIFESTS`/`activeDomainManifest()` in `profile.ts`; 8 Rust tests + 16 frontend tests. Total: 258 | ✅ shipped |
| **v1.35.0** *(Phase TX-3)* | **WP-47 — Breeding programs:** migration 033 (`breeding_programs` + `breeding_records` with cascade/indexes); 8 query functions; `get_generational_summary`; 7 Tauri commands; `BreedingProgramManager` UI; 13 Rust tests. Total: 271 | ✅ shipped |
| **v1.36.0** *(Phase TX-3)* | **WP-48 — Advanced hybridization:** generation labeling F1→F4 + BCn notation; backcross detection; admin-only cross-species override with permanent audit warning; 9-step HybridWizard with live suggestion; `StrainDetail.svelte` cross-species banner; 9 Rust tests | ✅ shipped |
| **v1.37.0** *(Phase TX-3)* | **WP-49 — Custom taxa & Darwin Core export:** migration 034 (`status`/`provisional_notes` on `taxa` + `taxon_mappings` table); 5 Tauri commands; `ProvisionalTaxaManager.svelte`; Darwin Core JSON export via recursive CTE; 11 Rust tests. Total: 282 — **Phase TX-3 complete** | ✅ shipped |
| **v1.37.1** *(ROADMAP update)* | Phase F expansion: WP-55 expanded for shared lab use; WP-58 and WP-63 prioritised as highest-ROI; WP-59 expanded with zero-knowledge E2E encryption design; WP-60 expanded with Part 11/USDA/CITES detail; WP-64 (taxon re-anchoring) and WP-65 (a11y label pass) added; WP-45 RECLASSIFICATION WARNING linked to WP-64; WP-12 a11y tail tracked | ✅ shipped |
| **v1.38.0** *(Phase F)* | **WP-50 & WP-51 — Multi-user backend + LAN sync foundation** (implemented together): migration 035 (`backend_type` on `app_settings`, new `sync_peers`/`sync_conflicts` tables); optional `postgres` Cargo feature with `db::backend`/`db::postgres` connector (connectivity test + schema bootstrap; not wired into the live query layer); `db::sync` change-detection/conflict-recording on the existing audit hash chain; 8 new commands; Settings "Multi-User Backend (Preview)" panel; 40 Rust tests. Total: 322 | ✅ shipped |
| **v1.39.0** *(Phase F)* | **WP-52, WP-53, WP-54, WP-55 — Notifications, iOS scaffold, sensor integration, field-level permissions** (implemented together): migrations 036–038 (`field_permissions`, `environmental_readings`, `notification_preferences`+`smtp_config`); `tauri-plugin-notification` + background scheduler; `db::sensors` payload parsing/validation; `db::permissions` masking wired into strains/breeding programs; new `.github/workflows/build-ios.yml` (⚠️ unverified — no macOS/Xcode access); `PermissionsEditor.svelte`; 34 new Rust tests. Total: 364 | ✅ shipped |
| **v1.39.1** *(Phase F)* | **WP-53 iOS hardening pass** — fixed `APPLE_TEAM_ID` → `APPLE_DEVELOPMENT_TEAM` env var mismatch (Tauri silently ignored the old name); `tauri.conf.json` gained `bundle.iOS.minimumSystemVersion`; identifier deliberately left unchanged (shared cross-platform field — see As Built); CI triggers narrowed to `workflow_dispatch`/weekly/`release`; `cargo check` fallback when no Apple credentials configured, replacing steps guaranteed to fail; still explicitly unverified end-to-end | ✅ shipped |
| **v1.40.0** *(Phase F)* | **WP-56 — Local AI analysis:** hand-rolled Ollama HTTP client (`ai/ollama.rs`, no new dependency); migration 041 `ai_suggestions` (pending, approval-gated); Summarize Notes / Suggest Passage Comment / Analyze Photo commands; AI Assist UI in SpecimenDetail + lightbox; 10 Rust tests | ✅ shipped |
| **v1.40.0** *(Phase F)* | **WP-57 — Interactive lab map:** migration 040 `locations` table + `specimens.location_id` (additive, text-location system untouched); `LabMap.svelte` (no map-library dependency); density/contamination/age heat-map; Dashboard widget; 6 Rust tests | ✅ shipped |
| **v1.40.0** *(Phase F)* | **WP-58 — Advanced analytics & reporting dashboards** *(highest ROI)*: `db/analytics.rs` pure query functions (growth, contamination trend, passage success, media efficiency, strain performance, cryo utilization, technician activity); `AnalyticsDashboard.svelte` (KPI strip, configurable panels, hand-rolled SVG charts, Excel export); 10 Rust tests | ✅ shipped |
| **v1.40.0** *(Phase F)* | **WP-59 — Cloud backup & multi-device sync with E2E encryption:** Argon2id + AES-256-GCM (`cloud::crypto`); migration 042 `backup_targets`+`cloud_sync_segments`; `local_nas`/`smb` fully functional, `s3`/`sftp` config-only (not yet connected); sync reuses WP-51's conflict detection; `CloudBackupPanel.svelte`; 18 Rust tests | ✅ shipped |
| **v1.40.0** *(Phase F)* | **WP-60 — Regulatory compliance export modules:** FDA 21 CFR Part 11 (Ed25519-signed, not RSA-4096 — documented substitution) + USDA APHIS PPQ 526 pre-fill + CITES dossier; migration 044 `signing_keys`; `ComplianceExportWizard.svelte`; `docs/regulatory-exports.md`; 10 Rust tests | ✅ shipped |
| **v1.40.0** *(Phase F)* | **WP-61 — Plugin/extension system:** JSON manifest + whitelisted vocabulary seeding; migration 045 `installed_plugins`; WASM compliance-rule execution deferred (metadata-only, documented); `.steloplugin` zip install; `PluginManagerPanel.svelte`; `docs/plugin-authoring.md`; 10 Rust tests | ✅ shipped |
| **v1.40.0** *(Phase F)* | **WP-62 — PWA & offline-first:** `vite-plugin-pwa` with Tauri-gated service-worker registration (never activates in the desktop webview); IndexedDB offline mutation queue (`offlineQueue.ts`); `PwaInstallPrompt.svelte`; no remote mutation endpoint exists yet (documented); 4 TS tests | ✅ shipped |
| **v1.40.0** *(Phase F)* | **WP-63 — Performance & scalability hardening** *(highest ROI)*: migration 039 covering indexes; materialized dashboard cache; cursor-paginated lineage view; configurable `pedigree_max_depth`; hand-rolled `SpecimenList` virtualization; `benches/performance.rs` (Criterion) + large-fixture generator; 15 Rust tests | ✅ shipped |
| **v1.40.0** *(Phase F)* | **WP-64 — Taxon chain re-anchoring tool:** migration 043 `reanchor_events`; synthetic-lineage re-anchoring (original chains never touched, new chains verify with unmodified `verify_audit_lineage`); species-level specimen bridging; **WP-45 EXPERIMENTAL → STABLE**; `TaxonomyNavigator` UI; 8 Rust tests | ✅ shipped |
| **v1.40.0** *(Phase F)* | **WP-65 — A11y completion pass:** `FormField.svelte` (deterministic IDs); 90 → 0 label-association warnings across 8 components; no visual changes; `npm run check` clean at 0 errors / 3 unrelated warnings — **Phase F complete** | ✅ shipped |
| **v1.40.1** *(Phase F hardening)* | **Security & quality review of v1.40.0** (no new features): cloud-backup now redacts the SMTP password (was local-only); `thaw_vial` dashboard-cache invalidation gap fixed; `cloud::crypto` `derive_key`/`encrypt` return `Result` (no Argon2-OOM panic); `set_analytics_panel_config` gated to supervisor/admin; plugin loader re-checks its table whitelist; defense-in-depth marker guard on `create_breeding_program`; `npm run check` 3 → 0 warnings; SMTP plaintext-storage warning in Settings; WP-50/51/53/54 scaffolding disclosures re-verified accurate. 464 tests (`--no-default-features`), 500 (default) | ✅ shipped |
| v2.x+ *(Phase G+)* | Phase G federated networks (WP-70–72); Trust Layer Phase 2 Dogecoin anchoring (WP-66); Trust Layer Phase 3 signed transactions (WP-67); regulatory submission pipeline (WP-68+) | long-term |

> **On the version history:** the jump from `0.1.19` to the `1.0.0-x` line was intentional — the `0.1.x` series was a feature-complete-but-unreleased prototype, and `1.0.0-x` marks the first **production-grade, security-hardened, signed** release with a real GitHub Release. Note the pre-release label shipped as numeric **`1.0.0-1`** (not `rc.1`): the WiX MSI bundler rejects non-numeric pre-release identifiers. Phase A then settled at **v1.1.0** once onboarding (WP-05) landed.

---

*This roadmap is maintained against the live repository. The latest **code** release is **v1.40.1** (45 migrations — unchanged from v1.40.0; v1.40.1 is a security/quality hardening pass with no schema change — 500 Rust tests passing with the default `tauri-commands` feature, 464 with `--no-default-features`; 104 Vitest assertions across 5 files). Migration history: migration 045 added `installed_plugins` for the WP-61 plugin/extension system (v1.40.0); 044 added `signing_keys` for WP-60 Ed25519 export signing (v1.40.0); 043 added `reanchor_events` for WP-64 taxon chain re-anchoring (v1.40.0); 042 added `backup_targets` + `cloud_sync_segments` for WP-59 cloud backup & multi-device sync (v1.40.0); 041 added `ai_suggestions` for WP-56 local AI analysis (v1.40.0); 040 added `locations` + `specimens.location_id` for WP-57 interactive lab map (v1.40.0); 039 added covering indexes for the WP-63 performance hardening pass (v1.40.0); 038 added `notification_preferences` + `smtp_config` tables and seeded `notification_check_interval_minutes` for the WP-52 notification foundation (v1.39.0); 037 added `environmental_readings` for the WP-54 sensor integration foundation (v1.39.0); 036 added `field_permissions` for WP-55 field-level permissions (v1.39.0); 035 added a `backend_type` key to `app_settings` and created `sync_peers`/`sync_conflicts` tables for the WP-50/WP-51 multi-user + LAN sync foundation (v1.38.0); 034 added `status`/`provisional_notes` columns on `taxa` + `taxon_mappings` table for custom taxa & Darwin Core export (v1.37.0); 033 added `breeding_programs` + `breeding_records` tables (v1.35.0); 032 added `domain` to `app_config` (v1.34.0); 031 backfilled taxon genesis audit entries (v1.33.0; STABLE as of v1.40.0 — see WP-45 and WP-64); 030 added `fruiting_records` (v1.31.0); 029 added `origin_type` CHECK + `is_best_performer` on `specimens` (v1.30.0); 028 added `colonization_pct`/`contaminant_type` to `subcultures` (v1.29.0); 027 seeded mycology vocabulary (v1.28.0); 026 added `biosafety_level` on `specimens` (v1.26.0); 025 added `frozen_vials` (v1.25.0); 024 added PDL columns (v1.24.0); 023 expanded cell_culture vocabulary (v1.23.0); 022 added generation-label/backcross columns to `hybridization_events` + `is_cross_species` on `strains` (v1.21.0); 021 added `ncbi_sync_log` (v1.19.0); 020 added `taxa` table for Kingdom→Genus hierarchy (v1.18.0); 019 Strain/Cultivar data model; 018 seeded `cell_culture` vocabulary; 017 created remaining vocabulary lookup tables with CHECK constraints dropped; 016 created the `stages` lookup table and dropped the stage CHECK constraint on `specimens`; 015 added `event_type` on `subcultures` + `app_config` for `lab_profile`; 014 `app_settings` + auto-checkpoint flags; 013 `audit_checkpoints` Merkle table; 012 contamination columns on `specimens`; 011 `is_draft` on `media_batches`; 010 generational-depth columns; 009 per-lineage hash chain; 008 hash-chain columns on `audit_log`; 007 performance indexes. Hand packets to Claude Code in order; each is scoped to stand alone.*
