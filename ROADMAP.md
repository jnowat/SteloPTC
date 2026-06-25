# SteloPTC → Stelo Lab Suite — Engineering Roadmap

**Status as of June 2026:** **v1.30.0** (`tauri.conf.json` + latest `CHANGELOG`) · Tauri 2 + Svelte 5 + Rust/SQLite · Windows + Android CI · **Trust Layer Phase 1 complete (WP-18–21) · Phase C WP-22–27 fully shipped · Phase TX-1 complete (WP-28 v1.16.0 · WP-29 v1.17.0) · Phase TX-2 fully shipped (WP-35–39, v1.18.0–v1.22.0) · Phase D (Cell Culture) fully shipped (WP-30–34, v1.23.0–v1.27.0) · Phase E (Mycology) WP-40–42 shipped (v1.28.0–v1.30.0)**
**Schema:** **29 migrations** total; latest is **migration 029** (`origin_type` and `is_best_performer` columns on `specimens` for mycology genetic lineage tracking — WP-42, v1.30.0). Migration 028 added `colonization_pct` and `contaminant_type` to `subcultures` (WP-41, v1.29.0). Migration 027 seeded the `mycology` profile vocabulary into all six lookup tables (WP-40, v1.28.0). Migration 026 added `biosafety_level` to `specimens` (WP-33, v1.26.0). Migration 025 added the `frozen_vials` table (WP-32, v1.25.0). Migration 024 added PDL and doubling-time columns to `specimens` and `subcultures` (WP-31, v1.24.0). Migration 023 expanded `cell_culture` vocabulary (WP-30, v1.23.0). Migration 022 added generation-label and backcross-depth columns to `hybridization_events` and `is_cross_species` to `strains` (WP-38, v1.21.0). Migration 021 added the `ncbi_sync_log` table (WP-36, v1.19.0). Migration 020 added the `taxa` table (Kingdom → Genus hierarchy, v1.18.0). Migration 019 was the Strain/Cultivar data model (v1.16.0). Migration 018 seeded `cell_culture` vocabulary (v1.15.0). Migration 017 created the four remaining vocabulary lookup tables and dropped their CHECK constraints (v1.12.0); 016 created `stages` lookup table and seeded all 15 PTC stage codes (v1.12.0); 015 added `event_type` on `subcultures` + `app_config` with `lab_profile` (v1.11.0); 014 added `is_auto`/`auto_source` to `audit_checkpoints` + `app_settings` (v1.10.0); 013 added the `audit_checkpoints` Merkle table (v1.9.0); 012 added `contamination_flag`/`contamination_notes` to `specimens`; 011 added `is_draft` to `media_batches` (v1.8.0); 010 added generational depth columns (v1.7.0); 009 introduced the per-lineage hash chain; 008 added hash-chain columns to `audit_log`; 007 added performance indexes. The stage `CHECK` constraint was expanded in **migration 002**, defensively rebuilt in **migration 003**, and **finally dropped for good in migration 016** (WP-23) — no more CHECK-constraint rebuilds for vocabulary changes.
**Security:** `csp` is now a locked-down policy (no longer `null`, WP-02); the default `admin/admin` credential is now gated behind a forced password change on first login (WP-01).
**Recent:** Trust(less) & Audit Layer Phase 1 (hash-chain + per-lineage genealogy, WP-18) shipped across v1.5.0 → v1.6.4; generational depth tracking, lineage passage offsets, `root_specimen_id`, and sibling display landed in v1.7.0; split workflow overhauled in v1.8.0 with letter-suffix accessions (001A/001B…), per-child controls, draft media batches, safety confirmation dialog, and synthetic split events in the passage timeline.
**In progress (Phase E — Mycology):** Phase B polish & stability (WP-06–17) fully shipped v1.1.1–v1.3.0 ✅; Trust Layer Phase 1 (WP-18–21) **fully shipped** ✅; **Phase C WP-22–27 fully shipped** ✅; **Phase TX-1 WP-28–29 fully shipped** ✅ (v1.16.0–v1.17.0); **Phase TX-2 WP-35–39 fully shipped** ✅ (v1.18.0–v1.22.0) — taxonomy backbone, NCBI sync, multi-generational pedigree, advanced hybridization tools, advanced Taxonomy Navigator; **Phase D (Cell Culture) WP-30–34 fully shipped** ✅ (v1.23.0–v1.27.0) — cell culture vocabulary expansion, PDL tracking, cryostorage, mycoplasma compliance, cell culture dashboard panels; **Phase E (Mycology) WP-40–42 shipped** ✅ — WP-40 mycology vocabulary (v1.28.0), WP-41 colonization & contamination tracking (v1.29.0), WP-42 genetic lineage & strain isolation markers (v1.30.0). **Current focus: Phase E WP-43 (fruiting conditions & yield) and WP-44 (mycology compliance/QC rules).**
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

When external verifiability is actually needed (regulatory evidence, IP-priority proof, cross-party collaboration), publish a checkpoint's `merkle_root` to Dogecoin (e.g. via an `OP_RETURN` output), store the returned txid in `audit_checkpoints.anchored_txid`, and add a verification path that confirms a root on-chain. This is intentionally left un-packetized for now; the Phase-1 design (stable canonical form, deterministic Merkle root, nullable `anchored_txid`) already makes it a drop-in rather than a rewrite. *Reserved: WP-65+.*

#### Phase 3 — Specimen Events as Transactions — *longer-term, deprioritized*

A more formal model in which specimen lifecycle events are individually signed and ordered like ledger transactions. Recorded here only to keep the Phase-1 foundation from foreclosing it. Not a near-term priority. *Reserved: WP-66+.*

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

#### Phase TX-2 — Expansion · WP-35–39 · ✅ Fully shipped (v1.18.0–v1.22.0)

**Goal:** Deeper taxonomy (Genus → Kingdom), NCBI Taxonomy import with sync and conflict resolution, multi-generational pedigree visualization, intraspecific hybridization, and a powerful full-featured taxonomy navigator. **All five WPs shipped.**

**Depends on:** Phase TX-1 complete; Phase C complete (profile-scoped lookup tables power the `strain_type` and `strain_status` vocabularies; domain-specific terminology driven by UI manifest from WP-25).

---

### WP-35 — Expanded taxonomy backbone (Genus → Kingdom) — ✅ Delivered in **v1.18.0**

- **Goal:** Model the ranks above Species as first-class classification records enabling hierarchical navigation, descendant-count queries, and NCBI sync in WP-36.
- **Files:** new migration, new `src-tauri/src/models/taxon.rs`, new `src-tauri/src/commands/taxa.rs`.
- **Steps:**
  1. Create `taxa` table: `id TEXT PRIMARY KEY`, `rank TEXT NOT NULL` (values: `kingdom | phylum | class | order | family | genus`), `name TEXT NOT NULL`, `parent_id TEXT REFERENCES taxa(id)`, `ncbi_taxon_id INTEGER NULL`, `ncbi_updated_at TEXT NULL`, `local_override BOOLEAN NOT NULL DEFAULT 0` (true = local edits take priority over NCBI sync), `created_at TEXT NOT NULL`, `updated_at TEXT NOT NULL`. Add `taxon_path TEXT` (JSON array of taxon IDs from kingdom to genus) and `ncbi_taxon_id INTEGER` to the existing `species` table.
  2. **Classification data only — no hash chain lineages:** `taxa` records are navigation and classification records. They do **not** receive audit lineages or hash chain entries. Taxonomic reclassifications (common at higher ranks) would otherwise break chains in ways that cannot be corrected without invalidating downstream strain and specimen records. Hash chain cryptographic integrity in SteloPTC is scoped to: Species → Strain → Specimen. Full-taxonomy hash chains extending above Species are a future optional consideration (see WP-45), not a current design goal.
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
  - **Migration 021** (`ncbi_sync_log` table) — records every NCBI taxonomy import event, data update, and name/rank conflict. `sync_type` (`import | update | conflict`), `conflict_details` (JSON), `resolution` (`kept_local | accepted_ncbi | merged`). Four indexes for fast conflict and type queries.
  - **`src-tauri/src/models/taxon.rs`** — six new types: `NcbiTaxonRecord`, `NcbiSyncLog`, `ImportNcbiTaxonomyRequest`, `NcbiConflictSummary`, `ImportNcbiTaxonomyResult`, `ResolveNcbiConflictRequest`.
  - **`src-tauri/src/commands/ncbi.rs`** (new) — four Tauri commands: `import_ncbi_taxonomy` (admin, two-phase dry-run/apply), `resolve_ncbi_conflict`, `sync_ncbi_taxon`, `list_ncbi_sync_log`.
  - **`src-tauri/src/db/queries.rs`** — seven new pure helpers: `normalize_ncbi_rank`, `find_taxon_by_ncbi_id`, `find_taxon_by_name_rank`, `detect_ncbi_conflict`, `insert_ncbi_sync_log`, `list_pending_ncbi_conflicts`, `list_ncbi_sync_log`.
  - **`src/lib/components/NcbiSyncPanel.svelte`** (new) — admin-only panel: JSON textarea for pasting NCBI records, Dry Run button with result preview, Confirm Import button (appears after successful dry run), pending conflicts list with Keep Local / Accept NCBI / Merged resolution buttons, recent sync log table.
- **Bump:** minor → **v1.19.0**.

---

### WP-37 — Multi-generational pedigree tools — ✅ Delivered in **v1.20.0**

- **Goal:** Visualize and export the full multi-generational pedigree of any strain, tracing both ancestor and descendant lines through all hybrid generations.
- **As built (v1.20.0):**
  - **`src-tauri/src/models/strain.rs`** — seven new model types: `StrainSummary`, `PedigreeEdge`, `PedigreeNode` (recursive with `parents` / `children`), `SpecimenSummary`, `StrainSpecimenTree`, `HybridizationEventRecord`, `PedigreeExport`.
  - **`src-tauri/src/db/queries.rs`** — eight new pure pedigree helpers: `get_strain_ancestry`, `get_strain_descendants`, `get_strain_specimen_tree`, `export_strain_pedigree`, and four private helpers (`load_strain_summary`, `load_parent_entries`, `load_child_entries`, `collect_pedigree_ids`). Both traversal functions include DFS cycle detection.
  - **`src-tauri/src/commands/strains.rs`** — four new Tauri commands: `get_strain_ancestry`, `get_strain_descendants`, `get_strain_specimen_tree`, `export_strain_pedigree`. Auth-gated; max_depth defaults to 5, capped at 10.
  - **`src/lib/components/PedigreeChart.svelte`** (new) — renders ancestry and descendant views as an indented node list (no SVG required). Each node: strain name, code, status badge, Hybrid badge, live specimen count, parent-role badge. Root node visually distinguished. Ancestors/Descendants toggle with live node counts. Export JSON button downloads the full pedigree bundle.
  - **13 new unit tests** covering: wildtype with no parents/children, 2- and 3-generation ancestry/descendants, max_depth capping, cycle detection (both directions), specimen tree with/without descendants, export bundle integrity.
- **Bump:** minor → **v1.20.0**.

---

### WP-38 — Advanced hybridization tools — ✅ Delivered in **v1.21.0**

- **Goal:** Extend the Phase TX-1 hybrid model with generation naming, backcross notation, and hardened cross-species guardrails.
- **As built (v1.21.0):**
  - **Migration 022** — three additive `ALTER TABLE ADD COLUMN` statements: `hybridization_events.generation_label TEXT`, `hybridization_events.backcross_depth INTEGER`, `strains.is_cross_species INTEGER NOT NULL DEFAULT 0`.
  - **`src-tauri/src/db/queries.rs`** — six new pure helpers (all unit-tested): `get_strain_generation_label`, `suggest_generation_label` (pure: derives F1→F2→F3→F4 from parent labels), `find_ancestor_depth_impl` (private DFS), `detect_backcross` (checks both directions), `suggest_generation_label_for_parents` (backcross overrides filial label), `get_generational_stats` (per-generation specimen counts with healthy/problem breakdown).
  - **`src-tauri/src/commands/strains.rs`** — `create_hybridization_event` fully rewritten: detects cross-species, blocks non-admin callers, writes permanent `cross_species_override` audit entry for admin overrides, resolves generation label (explicit → backcross suggestion → parent-label suggestion → null). Two new commands: `suggest_generation_label` and `get_generational_stats`.
  - **`src/lib/components/HybridWizard.svelte`** — wizard expanded from 8 to 9 steps: new Step 3 admin cross-species override panel (mandatory justification textarea + acknowledgement checkbox); new Step 5 (Generation Label) with async suggestion display and quick-select dropdown.
  - **`src/lib/components/StrainDetail.svelte`** (new slide-over) — Overview/Generations/Pedigree tabs; permanent red cross-species warning banner; generation label badge; per-generation specimen stats table.
  - **9 new unit tests** covering label inference, backcross detection at multiple depths, backcross label override, per-generation stats, empty stats for non-hybrid strains.
- **Bump:** minor → **v1.21.0**.

---

### WP-39 — Advanced taxonomy navigator — ✅ Delivered in **v1.22.0**

- **Goal:** Upgrade the Phase TX-1 two-column navigator into a full multi-rank column browser with powerful filtering, descendant counts, and keyboard navigation.
- **As built (v1.22.0):**
  - **`src-tauri/src/db/queries.rs`** — two new helpers: `get_taxon_column_items` (returns immediate taxon children decorated with `strain_count`/`specimen_count` via correlated SQL subqueries) and `search_taxonomy` (multi-entity search: taxa, species, strains, specimen accessions; up to 10 hits per entity type with breadcrumb IDs and navigation targets). Eight new unit tests.
  - **`src-tauri/src/models/taxon.rs`** — `TaxonColumnItem` and `TaxonomySearchResult` model types.
  - **`src-tauri/src/commands/taxa.rs`** — three new commands: `get_taxon_column`, `list_species_for_taxon`, `search_taxonomy`.
  - **`TaxonomyNavigator.svelte`** complete rewrite — multi-column browser (Kingdom → Phylum → Class → Order → Family → Genus → Species → Strains); each column independently scrollable; 5–6 visible on desktop, horizontal-scroll on mobile; breadcrumb trail from `$derived`; descendant count aggregates (`N strains · M specimens`) via backend correlated subqueries; global search (300 ms debounce) with grouped dropdown; keyboard navigation (↑↓ within column, ←→ between columns, Enter selects, Escape closes/resets, `/` focuses search); strain quick-action panel with specimen rows and StrainDetail slide-over integration; `localStorage` path persistence.
- **Bump:** minor → **v1.22.0** — **Phase TX-2 complete**.

---

#### Phase TX-3 — Advanced · WP-45–49 · Target: v3.x

*These packets are specified at the design level. They will be fully broken into concrete steps when Phase TX-2 is complete.*

---

### WP-45 — Full taxonomic hash chain (Kingdom → Strain → Specimen) — *Optional / Not Scheduled*

> **Status: Deprioritized.** This packet is a design placeholder and is explicitly not part of the committed TX-3 plan. Retained here for future consideration only.

If future demand warrants it: extend hash chain seeding to all taxonomy ranks so each `taxa` record's genesis is seeded from its parent taxon's current `entry_hash`. The full cryptographic path Kingdom → Phylum → Class → Order → Family → Genus → Species → Strain → Specimen would be continuously verifiable end-to-end. Highest value for IP-priority disputes requiring a dated, unbroken provenance chain from classification to culture.

**Fundamental tension that must be resolved before scheduling:** Taxonomic reclassifications (common at family, order, and class level) would break the chain at the reclassified rank and invalidate every strain and specimen record below it. No satisfactory mitigation is known at the time WP-45 was written. Until this is resolved, hash chain integrity remains intentionally scoped to Species → Strain → Specimen.

---

### WP-46 — Cross-domain taxonomy support

Define `domain` as a per-profile configuration. SteloPTC defaults to `Plantae`; SteloCC uses `Animalia`; SteloMyco uses `Fungi`; future SteloBio uses `Bacteria/Archaea`. Domain controls: default ranks shown in the Navigator, strain type vocabulary (`cultivar/variety` for plants, `breed/ecotype` for animals, `strain/isolate` for fungi/bacteria), confirmation method vocabulary (`morphological/genomic/phenotypic` per domain). The underlying tables, audit log, and all cryptographic machinery are identical across all domains — only the UI manifest and lookup table data vary.

---

### WP-47 — Breeding programs & multi-generational selection tracking

Introduce a `breeding_programs` table (name, goal, start date, target traits, founder strains). Each hybrid strain can be linked to a program. A `breeding_records` table tracks selection notes, fitness scores, and generation number per strain per program. A breeding program dashboard compares all generations produced, selection milestones met, and performance trends across generations. Enables structured crop improvement, strain stabilization, and documented selection histories for any vertical.

---

### WP-48 — Advanced hybridization (cross-species, F1/F2, backcross)

Lift the same-species constraint from WP-38 with an explicit admin override that writes a permanent warning to the audit log. Support F1/F2/F3 generation naming, backcross notation (`BC1F2`), and introgression lines. Add `hybrid_generation_code` field to strains. Optional hybrid vigor scoring (user-defined numeric metric). Full cross-species pedigree chart.

---

### WP-49 — Custom taxa & Darwin Core export

Allow labs to define provisional taxa not yet in NCBI (undescribed species, working names, lab-internal groupings). Custom taxa get `status = provisional`. A mapping table links provisional names to accepted NCBI taxa once published. Export the full taxonomy tree (or any subtree) as Darwin Core XML/JSON for community sharing, regulatory submission, or integration with herbarium and museum databases.

---

## 6. PHASE D — Cell Culture vertical (SteloCC) — ✅ WP-30–34 Fully Shipped (v1.23.0–v1.27.0)

Built entirely as profile data + a handful of cell-specific features on the shared engine. Mammalian/insect/cell-line work, not plants.

**What "species" becomes:** a **Cell Line** registry — line name, organism, tissue/origin, ATCC/ECACC/DSMZ catalog #, biosafety level, morphology (adherent/suspension), and recommended split ratio + interval.

### WP-30 — Cell culture vocabulary expansion — ✅ Delivered in **v1.23.0**
- **As built:** Migration 023 expands the `cell_culture` vocabulary tables seeded by migration 018 with additional lifecycle-state and technique terms (8 new stages including `thawed`, `adherent`, `suspension`, `confluent`, `passaged`, `cryopreserved`, `contaminated`, `discarded`; 4 new propagation methods including `trypsinization`, `mechanical_dissociation`, `dilution`, `subculturing`; 2 new supplement types; 2 new compliance record types; 2 new agencies; 2 new inventory categories). All inserts use `INSERT OR IGNORE` — purely additive and idempotent. 9 new Rust unit tests.

### WP-31 — Passage-number lineage & doubling time — ✅ Delivered in **v1.24.0**
- **As built:** Migration 024 adds `cumulative_pdl REAL` to `specimens` and `seed_cell_count`, `harvest_cell_count`, `split_ratio`, `pdl_gained`, `doubling_time_hours` to `subcultures`. Three pure calculation helpers in `queries.rs`: `calculate_doubling_time`, `calculate_pdl_from_counts`, `calculate_pdl_from_ratio`. `create_subculture` auto-calculates PDL and doubling time; `split_specimen` inherits `cumulative_pdl`. SpecimenDetail passage form gains Seed/Harvest Cell Count and Split Ratio inputs with live PDL preview; info card shows cumulative PDL. SpecimenPassageTimeline shows a PDL block per passage card. 9 new unit tests.

### WP-32 — Cryopreservation & LN2 inventory — ✅ Delivered in **v1.25.0**
- **As built:** Migration 025 adds the `frozen_vials` table with `CHECK(vial_count >= 0)` and status `active | depleted | discarded`. `commands/cryo.rs` (new file) with five Tauri commands. `CryoManager.svelte` (new): filterable vial table, Record Vials modal, Thaw modal (atomic: decrements count, creates specimen inheriting `lineage_passage_offset` + `cumulative_pdl`, writes dual audit entries), Discard modal. Cryostorage sidebar entry (❄). 13 new unit tests in `db/queries.rs`.

### WP-33 — Mycoplasma & contamination testing — ✅ Delivered in **v1.26.0**
- **As built:** Migration 026 adds `biosafety_level TEXT CHECK(biosafety_level IN ('BSL-1','BSL-2','BSL-2+','BSL-3'))` to `specimens`. New mycoplasma compliance rule in `commands/compliance.rs`: flags non-archived `cell_culture` specimens without a mycoplasma test within `mycoplasma_test_interval_days` (default 90). `get_mycoplasma_status` command. `ComplianceView.svelte` gains "Last Test" column. `SpecimenDetail` shows colour-coded BSL badge. 10 new Rust unit tests.

### WP-34 — Cell-culture dashboard panels — ✅ Delivered in **v1.27.0**
- **As built:** Four new `cell_culture`-only panels on Dashboard: (1) Passages Due / Overdue, (2) Lines Overdue for Mycoplasma Test, (3) Vials in Storage by Line (`getVialSummaryByLine`), (4) Cultures Needing Attention (`getCultureMaintenanceAlerts`, specimens ≥ 7 days without passage). Two new helpers in `db/dashboard.rs`. One new command each in `cryo.rs` and `subcultures.rs`. 9 new unit tests. **Phase D complete.**

---

## 7. PHASE E — Mycology vertical (SteloMyco) — WP-40–42 Shipped · WP-43–44 Planned

Contamination is even more central here than in PTC — the engine's contamination tracking is a real advantage. Built as profile data + a few mycology-specific features.

**What "species" becomes:** a **Strain/Culture** registry — genus/species (e.g. *Pleurotus ostreatus*), strain name/code, source (spore print / tissue clone / commercial culture), and dikaryon vs monokaryon status.

### WP-40 — Mycology profile vocabulary — ✅ Delivered in **v1.28.0**
- **As built:** Migration 027 seeds all six profile-scoped vocabulary tables for `mycology` via `INSERT OR IGNORE` (idempotent, purely additive). Stages: full mushroom lifecycle (10 entries: `spore_clone`, `agar`, `liquid_culture`, `grain_spawn`, `bulk_substrate`, `colonizing`, `fruiting`, `senescent`, `contaminated` terminal, `discarded` terminal). Propagation methods (8 entries): `agar_to_agar`, `agar_to_grain`, `grain_to_grain`, `grain_to_bulk`, `liquid_inoculation`, `spore_syringe`, `culture_restart`, `other`. Supplement types (7 in `hormone_types`): `gypsum`, `bran`, `calcium_carbonate`, `activated_carbon`, `coconut_coir`, `vermiculite`, `other`. Compliance record types (6), agencies (4), inventory categories (10). 12 new Rust unit tests covering stage counts, terminal/non-terminal split, expected codes, profile isolation, and idempotency.

### WP-41 — Colonization & contamination tracking — ✅ Delivered in **v1.29.0**
- **As built:** Migration 028 adds `colonization_pct REAL CHECK(colonization_pct BETWEEN 0 AND 100)` and `contaminant_type TEXT` to `subcultures`. In the mycology profile: passage form gains Colonization % input and Contaminant Type dropdown (Trichoderma, Wet Rot/Bacterial, Cobweb Mold, Pin Mold, Mycelium Abort, Other). `SpecimenDetail` shows a bar-chart Colonization Progress section (green ≥ 80%, amber ≥ 50%, red < 50%). Passage timeline badge shows specific contaminant type. Dashboard contamination panel gains `by_contaminant_type` breakdown. New `get_colonization_history` command. 8 new Rust unit tests (4 migration, 4 dashboard).

### WP-42 — Genetic lineage & strain isolation markers — ✅ Delivered in **v1.30.0**
- **As built:** Migration 029 adds `origin_type TEXT CHECK(origin_type IN ('multi_spore','isolated_dikaryon','tissue_clone'))` (NULL = unspecified) and `is_best_performer INTEGER NOT NULL DEFAULT 0` to `specimens`. Mycology profile only: SpecimenForm shows Culture Origin Type dropdown; SpecimenDetail info card shows Culture Origin badge and Best Performer toggle button (★/☆). `split_specimen` inherits `origin_type` from parent and resets `is_best_performer = 0`. `search_specimens` supports `best_performer_only` filter. 5 new Rust unit tests.

### WP-43 — Fruiting conditions & yield — *planned*
- Per-culture environmental targets (temp, RH, FAE, light) — reuse `environmental_notes` + structured fields. Record **yield** at harvest (fresh/dry weight, flush number) to compare strains and substrates over time.

### WP-44 — Mycology compliance/QC rules — *planned*
- Lighter regulatory load: repurpose the rule engine for **QC** instead — flag cultures colonizing too slowly, overdue for transfer (senescence risk), or with open contamination not yet discarded.

---

## 8. PHASE F — Cross-cutting & beyond (post-vertical)

These are your existing v0.2/v0.3 items, re-sequenced to run *after* the platform exists so they benefit all three verticals at once:

- **WP-50 — PostgreSQL backend option** for LAN/multi-writer deployments (drop-in behind the connection layer; the lookup-table design from Phase C makes the schema portable).
- **WP-51 — LAN network sync** across desktop + mobile clients.
- **WP-52 — Email/push notifications** for reminders and overdue passages/transfers.
- **WP-53 — iOS support** (Tauri 2 iOS target; the responsive UI already exists).
- **WP-54 — Environmental sensor integration** (temp/humidity/CO₂ → passage/fruiting records) — high value for cell culture (incubators) and mycology (fruiting chambers).
- **WP-55 — Role-based field-level permissions** (hide IP/provenance by role).
- **WP-56 — Local AI analysis** — notes summarization, image-based contamination detection (a natural fit given all three verticals already attach photos).
- **WP-57 — Interactive lab map** — floor-plan heat-map of locations (your existing structured location data feeds this directly).

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
| **v1.18.0** *(Phase TX-2)* | **WP-35 — Expanded taxonomy backbone (Genus → Kingdom):** migration 020 (`taxa` table, `taxon_path`/`ncbi_taxon_id` on `species`); `backfill_genus_taxa`; `commands/taxa.rs` (`create_taxon`, `get_taxon`, `update_taxon`, `list_taxa_by_rank`, `get_taxon_descendants`); TypeScript interfaces — **Phase TX-2 WP-35 complete** | ✅ shipped |
| **v1.19.0** *(Phase TX-2)* | **WP-36 — NCBI Taxonomy import & ongoing sync:** migration 021 (`ncbi_sync_log`); `commands/ncbi.rs` (4 commands: `import_ncbi_taxonomy`, `resolve_ncbi_conflict`, `sync_ncbi_taxon`, `list_ncbi_sync_log`); `NcbiSyncPanel.svelte` (dry-run/apply UI, conflict resolution) | ✅ shipped |
| **v1.20.0** *(Phase TX-2)* | **WP-37 — Multi-generational pedigree tools:** `get_strain_ancestry` / `get_strain_descendants` / `get_strain_specimen_tree` / `export_strain_pedigree` commands; `PedigreeChart.svelte`; DFS cycle detection; 13 new Rust tests | ✅ shipped |
| **v1.21.0** *(Phase TX-2)* | **WP-38 — Advanced hybridization tools:** migration 022 (generation_label, backcross_depth, is_cross_species); 6 new query helpers; HybridWizard 9-step with Generation Label step; cross-species admin override with permanent audit warning; `StrainDetail.svelte` slide-over (Overview/Generations/Pedigree); 9 new Rust tests | ✅ shipped |
| **v1.22.0** *(Phase TX-2)* | **WP-39 — Advanced Taxonomy Navigator:** multi-column browser (Kingdom → Strains); descendant-count bubble-up; 300 ms debounced global search; full keyboard navigation; strain quick-action panel; localStorage path persistence — **Phase TX-2 complete** | ✅ shipped |
| **v1.23.0** *(Phase D)* | **WP-30 — Cell culture vocabulary expansion:** migration 023 adds 20 new vocabulary rows across 6 tables for `cell_culture` profile; 9 new Rust tests | ✅ shipped |
| **v1.24.0** *(Phase D)* | **WP-31 — Passage-number lineage & doubling time:** migration 024 adds PDL and cell-count columns; `calculate_doubling_time` / `calculate_pdl_from_counts` / `calculate_pdl_from_ratio` pure helpers; cumulative PDL inherited at split; passage form PDL preview; 9 new Rust tests | ✅ shipped |
| **v1.25.0** *(Phase D)* | **WP-32 — Cryopreservation & LN2 inventory:** migration 025 (`frozen_vials` table); `commands/cryo.rs` (5 commands); `CryoManager.svelte`; atomic Thaw action inheriting PDL/passage lineage; 13 new Rust tests | ✅ shipped |
| **v1.26.0** *(Phase D)* | **WP-33 — Mycoplasma & contamination testing:** migration 026 (`biosafety_level` on specimens); mycoplasma compliance rule; `get_mycoplasma_status` command; BSL badge in SpecimenDetail; Last Test column in ComplianceView; 10 new Rust tests | ✅ shipped |
| **v1.27.0** *(Phase D)* | **WP-34 — Cell-culture dashboard panels:** 4 `cell_culture`-only Dashboard panels (Passages Due, Mycoplasma Overdue, Vials by Line, Cultures Needing Attention); `query_vial_summary_by_line` / `query_culture_maintenance_alerts` helpers; 9 new Rust tests — **Phase D complete** | ✅ shipped |
| **v1.28.0** *(Phase E)* | **WP-40 — Mycology profile vocabulary:** migration 027 seeds all 6 vocabulary tables for `mycology` profile (10 stages, 8 propagation methods, 7 supplement types, 6 compliance record types, 4 agencies, 10 inventory categories); 12 new Rust tests | ✅ shipped |
| **v1.29.0** *(Phase E)* | **WP-41 — Mycology colonization & contamination tracking:** migration 028 (`colonization_pct`, `contaminant_type` on subcultures); `get_colonization_history` command; Colonization Progress bar-chart; contaminant-type dashboard breakdown; 8 new Rust tests | ✅ shipped |
| **v1.30.0** *(Phase E)* | **WP-42 — Genetic lineage & strain isolation markers:** migration 029 (`origin_type`, `is_best_performer` on specimens); Culture Origin badge; Best Performer toggle; `split_specimen` inherits `origin_type`; `best_performer_only` search filter; 5 new Rust tests | ✅ shipped |
| v1.31.0+ *(Phase E)* | **WP-43 — Fruiting conditions & yield** (temp, RH, FAE, light targets + harvest yield recording) | planned |
| v1.32.0+ *(Phase E)* | **WP-44 — Mycology compliance/QC rules** (slow colonization, overdue transfer, open contamination flags) | planned |
| v2.x *(Phase TX-3)* | **Phase TX-3 — Advanced taxonomy:** WP-45 full taxonomic hash chain, WP-46 cross-domain support, WP-47 breeding programs, WP-48 advanced hybridization, WP-49 custom taxa & Darwin Core export | future |
| v2.x+ | Phase F cross-cutting features; Trust Layer **Phase 2** (Dogecoin anchoring, WP-65+) when external proof is needed | future |

> **On the version history:** the jump from `0.1.19` to the `1.0.0-x` line was intentional — the `0.1.x` series was a feature-complete-but-unreleased prototype, and `1.0.0-x` marks the first **production-grade, security-hardened, signed** release with a real GitHub Release. Note the pre-release label shipped as numeric **`1.0.0-1`** (not `rc.1`): the WiX MSI bundler rejects non-numeric pre-release identifiers. Phase A then settled at **v1.1.0** once onboarding (WP-05) landed.

---

*This roadmap is grounded in the live repository at **v1.30.0**: 29 migrations, latest being migration 029 (`origin_type` + `is_best_performer` on `specimens` for mycology genetic lineage markers — WP-42, v1.30.0); 028 added `colonization_pct`/`contaminant_type` on `subcultures` (WP-41); 027 seeded mycology vocabulary (WP-40); 026 added `biosafety_level` on `specimens` (WP-33); 025 added `frozen_vials` table (WP-32); 024 added PDL/doubling-time columns (WP-31); 023 expanded `cell_culture` vocabulary (WP-30); 022 added generation-label/backcross-depth/is_cross_species (WP-38); 021 added `ncbi_sync_log` (WP-36); 020 added the `taxa` table (WP-35); 019 was the Strain/Cultivar data model (WP-28); 018 seeded `cell_culture` vocabulary (WP-27); 017 created the four remaining vocabulary lookup tables with CHECK constraints dropped (WP-24); 016 created the `stages` lookup table and dropped the stage CHECK constraint on `specimens` (WP-23); 015 added `event_type` on `subcultures` + `app_config` for `lab_profile` (WP-22); 014 added `app_settings` + auto-checkpoint flags (WP-21); 013 added the `audit_checkpoints` Merkle table (WP-20); 012 added `contamination_flag`/`contamination_notes` to `specimens`; 011 added `is_draft` on `media_batches`; 010 added generational-depth columns; 009 introduced per-lineage hash chain; 008 added hash-chain columns to `audit_log`; 007 added performance indexes; 001–006 were the foundational schema, vocabulary, and security migrations. Hand packets to Claude Code in order; each is scoped to stand alone.*
