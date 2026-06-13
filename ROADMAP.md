# SteloPTC → Stelo Lab Suite — Engineering Roadmap

**Status as of June 2026:** **v1.1.0** (`tauri.conf.json` + latest `CHANGELOG`) · Tauri 2 + Svelte 5 + Rust/SQLite · Windows + Android CI · **Phase A shipped — first signed GitHub Releases published**
**Schema:** **6 migrations** total, latest is `migration_006_force_password_change` (added by WP-01); `migration_005_contamination_schedule` precedes it. The stage `CHECK` constraint was expanded in **migration 002** and defensively rebuilt in **migration 003** — the table-rebuild pattern WP-23 will use one final time.
**Security:** `csp` is now a locked-down policy (no longer `null`, WP-02); the default `admin/admin` credential is now gated behind a forced password change on first login (WP-01).
**Recent:** Photos/attachments landed in v0.1.19; Phase A (security hardening, signed releases, crash-proofing, onboarding) shipped across v0.1.20 → v1.1.0.
**In progress (Phase B):** a **Trust(less) and Audit Layer** — a cryptographically tamper-evident audit history (hash-chain + Merkle checkpoints now; optional Dogecoin anchoring later).
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
3. **Phase C — De-harden the domain (Section 4).** Convert the baked-in vocabulary (CHECK constraints, enums, labels) into data. This is the keystone. It's invisible to users but it's what makes one codebase serve three labs. Ships as v1.4 (still PTC-only behaviorally).
4. **Phase D — Cell Culture vertical (Section 5)** and **Phase E — Mycology vertical (Section 6)**, built as *profiles* on the shared engine.

> **Why de-harden before building verticals?** Your schema encodes plant vocabulary as SQL `CHECK` constraints — e.g. `stage CHECK(stage IN ('explant','callus','shoot_meristem',...))` at `migrations.rs:391`. The stage constraint was already **expanded in migration 002 and defensively rebuilt in migration 003** — that's two migrations whose job was to widen one constraint via a full table rebuild. WP-23 will run this table-rebuild pattern **one final time** to drop the constraint entirely. Cell lines don't have an "explant" stage; mushroom cultures don't "acclimatize." If you fork now, every vocabulary change is three migrations and three CHECK-constraint rebuilds forever. Lookup tables make vocabulary *data*, and data is cheap to vary per profile.

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

### Immediate fixes

### WP-06 — Bug/polish backlog clearance
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

### WP-07 — QR scanner: reject non-SteloPTC codes gracefully
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

### WP-08 — Specimen Work Queue / Daily Task View
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

### Looking great — design system & polish

### WP-10 — Extract a central design-token system
- **Goal:** One source of truth for color, spacing, type, radius, shadow — instead of 15 component `<style>` blocks + a 282-line block in `App.svelte`.
- **Files:** new `src/lib/styles/tokens.css` (imported once in `App.svelte`), then incremental refactors per component.
- **Steps:**
  1. Define `:root` CSS custom properties for the existing palette (light + dark), spacing scale, font sizes, radii, shadows, z-index layers.
  2. Map the current dark-mode toggle to swap a `data-theme` attribute on `<html>` that flips the token values (cleaner than per-component dark rules).
  3. Migrate components to tokens **one per packet** (don't do all 15 at once — scope creep risk). Start with `Dashboard.svelte` and `Sidebar.svelte`.
- **Acceptance:** Changing one token (e.g. accent color) restyles the whole app; dark mode flips via the single attribute.
- **Preserve:** Current visual appearance — this is a refactor, not a redesign. Pixel-diff before/after on the dashboard.
- **Bump:** patch each.

### WP-11 — Loading, empty, and error states everywhere
- **Goal:** Every list/detail view has a skeleton-loading state, a friendly empty state, and an inline error state.
- **Files:** all list components (`SpecimenList`, `MediaList`, `InventoryManager`, `ReminderList`, `ComplianceView`, `AuditLog`, `ErrorLog`).
- **Steps:** Add a tiny shared `<DataState>` wrapper (loading / empty / error / ready). Replace bare table renders.
- **Acceptance:** Throttle the backend and watch each view show a skeleton, then data; empty filters show "no results," not a blank table.
- **Preserve:** Existing data fetching.
- **Bump:** patch.

### WP-12 — Accessibility & keyboard pass (WCAG 2.1 AA target)
- **Goal:** Usable by keyboard and screen reader; contrast verified.
- **Files:** global + per-component.
- **Steps:** Audit focus order, visible focus rings, `aria-label`s on icon-only buttons (the sidebar uses emoji icons), color-contrast on the health-status slider, modal focus trapping (QR modal, lightbox), and that the existing Ctrl+1–5 shortcuts are documented in-app.
- **Acceptance:** Full create-specimen → record-passage flow completable with keyboard only; axe-core run shows no critical violations.
- **Preserve:** The 48px touch targets already added for mobile (WCAG 2.5.5).
- **Bump:** patch.

### WP-13 — Print / PDF polish
- **Goal:** The Culture Certificate and Specimens Summary look like lab documents, not browser printouts.
- **Depends on:** WP-06 (Print Summary must be working before polishing its output).
- **Files:** `src/lib/components/SpecimenList.svelte`, `src/lib/components/SpecimenDetail.svelte`, `src/lib/components/QrModal.svelte`.
- **Steps:** Add a print stylesheet with proper margins, a header/footer band (lab name, accession, generated date, page numbers), and a place for a lab logo.
- **Acceptance:** A printed certificate is clean on A4 and US Letter; the Specimens Summary prints cleanly in landscape.
- **Preserve:** Existing print-API approach; do not change the HTML structure in ways that break the fix from WP-06.
- **Bump:** patch.

### Working great — stability, performance, tests

### WP-14 — First test harness (the highest-leverage packet here)
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

### WP-15 — Query performance & indexing audit
- **Goal:** Stays fast at 10k+ specimens.
- **Files:** `src-tauri/src/db/migrations.rs` (indexes), `commands/specimens.rs`, `commands/subcultures.rs`.
- **Steps:** Verify indexes exist on every column used in `WHERE`/`JOIN`/`ORDER BY` (species_id, stage, project_id, parent_specimen_id, subculture.specimen_id, created_at). Confirm list endpoints paginate (the `PaginatedResponse` type exists — make sure every list uses it, including the dashboard panels). Replace any N+1 patterns (the changelog already shows you fixed one with `list_all_subcultures` — audit for others).
- **Acceptance:** Seed 10k specimens + 50k subcultures; list/search/dashboard load under ~200ms.
- **Preserve:** Existing pagination contract.
- **Bump:** patch.

### WP-16 — Backup → Restore (close the loop)
- **Goal:** Backups are only half a feature without restore.
- **Files:** `src-tauri/src/commands/backup.rs`, Dashboard.
- **Steps:** Add a "Restore from backup" action (admin only) that validates the file, checkpoints/closes the live DB, swaps it, and reloads. Confirm-twice UX given destructiveness.
- **Acceptance:** Backup → mutate data → restore → data matches the backup point.
- **Preserve:** WAL-checkpoint-before-copy logic.
- **Bump:** minor.

### WP-17 — Excel import (already on your list)
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

### WP-19 — Chain verification command + integrity panel
- **Goal:** A backend command that re-walks the chain and reports the first broken link, surfaced in a small admin/supervisor panel.
- **Files:** `src-tauri/src/commands/audit.rs` (`verify_audit_chain`), `src/lib/api.ts`, a new `AuditIntegrity.svelte` panel (reachable from the existing Audit Log view).
- **Steps:**
  1. `verify_audit_chain` recomputes each `entry_hash` from stored content + stored `prev_hash`, compares to the stored `entry_hash` and to the next row's `prev_hash`, and returns `{ verified, total_entries, first_broken_seq, detail }`.
  2. UI: a **"Verify history"** button showing ✓ *History verified (N entries)* or a red flag pinpointing the first broken `chain_seq`, plus a last-verified timestamp.
- **Acceptance:** A clean DB verifies green; a manual out-of-band row edit is detected and the breaking `chain_seq` is reported.
- **Preserve:** Verification is strictly read-only; the existing audit viewer is untouched apart from the added entry point.
- **Bump:** minor.

### WP-20 — Periodic Merkle checkpoints over audit batches
- **Goal:** Roll ranges of audit entries into a Merkle tree and store the root, so verification is efficient and roots are ready to anchor later — without redesign.
- **Files:** new migration (008) for an `audit_checkpoints` table, `src-tauri/src/commands/audit.rs` (`build_checkpoint`, `list_checkpoints`).
- **Steps:**
  1. Migration 008: `audit_checkpoints (id, from_seq, to_seq, merkle_root, entry_count, created_at, created_by, anchored_txid TEXT NULL)`. The `anchored_txid` column is the **Phase 2 hook** — created now, unused until on-chain anchoring exists.
  2. Implement a **deterministic Merkle tree** over the `entry_hash` leaves for the range. **Locked construction rule:** for odd node counts, duplicate the last leaf before pairing (the same rule used by Bitcoin's Merkle tree). Document this rule precisely in code comments and in `docs/CRYPTO_AUDIT.md` (WP-21), because an off-app or on-chain verifier must reproduce it byte-for-byte. This choice is permanent — changing it after checkpoints exist invalidates all prior proofs.
  3. Checkpoint creation is triggered in three ways:
     - **Event-driven:** automatically after any high-value audit event — new subculture, new media batch, contamination flag, or specimen location change.
     - **Manual:** admin/supervisor can trigger a checkpoint on demand from the Audit Log view.
     - **On backup:** `create_backup` triggers a checkpoint before copying the WAL, so every backup file contains a complete, sealed checkpoint covering all records at the time of backup.
  4. Store root + range + entry count for each checkpoint.
- **Acceptance:** Building a checkpoint over a known entry set reproduces the **same** root deterministically across runs; a per-entry Merkle proof verifies against the stored root; the "duplicate-last" rule is visibly enforced in the implementation.
- **Preserve:** The WP-18 hash chain — Merkle leaves *are* the `entry_hash` values, not a parallel hash. The WP-16 backup flow must still succeed after the pre-backup checkpoint step is added.
- **Bump:** minor.

### WP-21 — Merkle proof export & standalone re-verification
- **Goal:** Export one record's audit history plus its Merkle proof to a checkpoint root as portable JSON, with a documented standalone verifier so a third party can confirm tamper-evidence without running SteloPTC.
- **Files:** `src-tauri/src/commands/audit.rs` (`export_audit_proof`), `src/lib/components/ExportManager.svelte` hook, new `docs/CRYPTO_AUDIT.md`.
- **Steps:**
  1. `export_audit_proof(entity_id)` produces `{ record, entries[], leaf_hashes[], proof_path[], merkle_root, checkpoint_range }`.
  2. Write `docs/CRYPTO_AUDIT.md` specifying the canonical serialization (WP-18), the hash algorithm (SHA-256), and the Merkle construction (WP-20, including the duplicate-last odd-node rule) precisely enough to reimplement independently.
  3. Ship a minimal standalone verifier (a short Node or Python script in `docs/`) that takes the exported JSON and confirms the proof against the root.
- **Acceptance:** An exported proof verifies with the standalone script against the stored root; tampering with any field of the exported record fails verification.
- **Preserve:** The WP-20 checkpoint format (this consumes it); export must not mutate any audit data.
- **Bump:** minor → ships the Phase-1 Trust layer as **v1.3.0**.

#### Phase 2 — On-Chain Anchoring (Dogecoin first) — *future, not yet scoped*

When external verifiability is actually needed (regulatory evidence, IP-priority proof, cross-party collaboration), publish a checkpoint's `merkle_root` to Dogecoin (e.g. via an `OP_RETURN` output), store the returned txid in `audit_checkpoints.anchored_txid`, and add a verification path that confirms a root on-chain. This is intentionally left un-packetized for now; the Phase-1 design (stable canonical form, deterministic Merkle root, nullable `anchored_txid`) already makes it a drop-in rather than a rewrite. *Reserved: WP-65+.*

#### Phase 3 — Specimen Events as Transactions — *longer-term, deprioritized*

A more formal model in which specimen lifecycle events are individually signed and ordered like ledger transactions. Recorded here only to keep the Phase-1 foundation from foreclosing it. Not a near-term priority. *Reserved: WP-66+.*

---

## 4. PHASE C — De-harden the domain (the keystone refactor)

This is the work that turns one product into a platform. It is **behavior-preserving for PTC** — after this phase, the plant app looks and works identically, but the vocabulary lives in data instead of in `CHECK` constraints, Rust enums, and hardcoded labels. Do it in this order.

### WP-22 — Introduce the `lab_profile` concept
- **Goal:** One app-level setting that says which kind of lab this install is.
- **Files:** new migration, `src-tauri/src/commands/admin.rs`, a new `src/lib/profile.ts`.
- **Steps:**
  1. Migration: a single-row `app_config` table (if not present) with `lab_profile TEXT NOT NULL DEFAULT 'plant_tissue_culture'`. Allowed values: `plant_tissue_culture | cell_culture | mycology`.
  2. Backend command to read/write the profile (admin only; set at first-run, hard to change after data exists).
  3. Front end `profile.ts` exposes the active profile to all components.
- **Acceptance:** Profile is readable app-wide; defaults to PTC so nothing changes.
- **Preserve:** Everything — this packet adds, removes nothing.
- **Bump:** minor.

### WP-23 — Convert stage `CHECK` constraints → a `stages` lookup table
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

### WP-24 — Same treatment for the other hardcoded vocabularies
- **Goal:** Generalize `propagation_method`, `hormone_type`, compliance `record_type`/`agency`, and inventory `category` the same way.
- **Files:** migration, the corresponding models/commands/components.
- **Steps:** For each, create a profile-scoped lookup table seeded with today's plant values; drop the `CHECK` constraint; drive the UI from the table. Group related ones to minimize table-rebuild migrations.
- **Acceptance:** PTC unchanged; each vocabulary now varies by profile.
- **Preserve:** All existing enum values as seed data.
- **Bump:** minor.

### WP-25 — Extract a UI "profile manifest" for labels & terminology
- **Goal:** The words on screen (entity names, page titles, field labels) come from a per-profile manifest, not hardcoded strings.
- **Files:** `src/lib/profile.ts` → add a `manifest` object keyed by profile; `Sidebar.svelte`, `index.html` title, all component headings.
- **Steps:**
  1. Define a `ProfileManifest` type: `{ appName, primaryEntity (e.g. "Specimen" / "Cell Line" / "Culture"), primaryEntityPlural, registryEntity ("Species" / "Cell Line" / "Strain"), passageVerb ("Subculture" / "Passage" / "Transfer"), mediaNoun ("Media" / "Medium" / "Substrate"), ... }`.
  2. Replace hardcoded user-facing strings with `manifest.x` lookups. (Internal code/table names stay as-is — only the *display* layer changes.)
- **Acceptance:** Flipping the profile string in a dev build changes the sidebar labels, page title, and headings coherently — without touching components.
- **Preserve:** PTC manifest must reproduce today's exact wording.
- **Bump:** minor.

### WP-26 — Profile-scoped compliance rules
- **Goal:** The auto-flag engine (currently citrus-HLB / USDA-specific in `compliance.rs:252`) becomes a profile-pluggable rule set.
- **Files:** `src-tauri/src/commands/compliance.rs`.
- **Steps:** Move each rule (expired permit, citrus HLB, quarantine-without-release, positive-without-quarantine) behind a profile gate. PTC keeps all four. Define the rule interface so cell-culture and mycology can register their own (Section 5/6) without editing the plant rules.
- **Acceptance:** PTC flags identically; the rule registry is profile-aware.
- **Preserve:** All four current plant rules and their flag messages/types.
- **Bump:** minor → ship as **v1.4.0** (still a PTC product, now profile-ready underneath).

### WP-27 — Per-vertical app identity (build-time)
- **Goal:** Three installable apps from one repo, differentiated at build time, sharing 95%+ of the code.
- **Files:** `tauri.conf.json` (+ per-profile overrides), CI workflows, package metadata.
- **Steps:**
  1. Parameterize `productName`, `identifier` (`com.steloptc.app` → `com.stelocc.app`, `com.stelomyco.app`), window title, icons, and **default `lab_profile`** via a build env var / profile-specific config overlay.
  2. CI matrix builds all three (or one per tag prefix). Each installs side-by-side with its own data dir.
  3. **Locked product names:** **SteloPTC** (plant tissue culture), **SteloCC** (cell culture), and **SteloMyco** (mycology), shipped under the **Stelo Lab Suite** umbrella. Use these exact names for `productName`, identifiers, store listings, and docs.
- **Acceptance:** `npm run build` with `PROFILE=cell_culture` produces a distinct app, distinct identifier, defaulting to the cell-culture profile and vocabulary.
- **Preserve:** The existing PTC build output exactly when `PROFILE` is unset/`plant_tissue_culture`.
- **Bump:** minor.

---

## 5. PHASE D — Cell Culture vertical (SteloCC)

Built entirely as profile data + a handful of cell-specific features on the shared engine. Mammalian/insect/cell-line work, not plants.

**What "species" becomes:** a **Cell Line** registry — line name, organism, tissue/origin, ATCC/ECACC/DSMZ catalog #, biosafety level, morphology (adherent/suspension), and recommended split ratio + interval.

### WP-30 — Seed the cell-culture profile vocabulary
- Stages → cell-culture lifecycle: `thawed → adherent/suspension → confluent → passaged → frozen/cryopreserved → contaminated → discarded`.
- Passage verb → **"Passage"** (not subculture). Each passage records **passage number** (P-number), split ratio (e.g. 1:4), confluence % at split, viable cell count / viability %, and counting method (hemocytometer / automated).
- Media → **"Medium"**; basal types become `DMEM, RPMI-1640, MEM, F-12, IMDM, ...` with supplements (FBS %, L-glutamine, antibiotics, growth factors) replacing the auxin/cytokinin/gibberellin hormone model.
- **Files:** seed migration for `profile = 'cell_culture'`, cell-culture manifest in `profile.ts`.

### WP-31 — Passage-number lineage & doubling time
- Extend the existing parent/child lineage (already strong for split culture) to track **cumulative passage number** and **population doubling level (PDL)**. Compute **doubling time** from seed/harvest counts + elapsed time. Reuse the lineage banner UI.

### WP-32 — Cryopreservation & LN2 inventory
- The structured location entry (Room/Rack/Shelf/Tray) maps cleanly to **freezer/tower/box/position** for LN2 / -80°C storage. Add a frozen-vial record: cell line, passage #, vial count, freeze date, freeze medium (e.g. 10% DMSO), location, and a thaw action that decrements vial count and creates a new active culture with the carried-forward passage number.

### WP-33 — Mycoplasma & contamination testing (compliance rule)
- Register a cell-culture compliance rule: flag lines with no **mycoplasma test** in the last N passages/days (mirrors the citrus-HLB rule pattern from WP-26). Add biosafety-level tracking to the cell-line registry.

### WP-34 — Cell-culture dashboard panels
- Reuse the contamination-overview + schedule widgets: "passages due," "lines overdue for mycoplasma test," "vials in storage by line," "low-confluence alerts."

---

## 6. PHASE E — Mycology vertical (SteloMyco)

Contamination is even more central here than in PTC — the engine's contamination tracking is a real advantage. Built as profile data + a few mycology-specific features.

**What "species" becomes:** a **Strain/Culture** registry — genus/species (e.g. *Pleurotus ostreatus*), strain name/code, source (spore print / tissue clone / commercial culture), and dikaryon vs monokaryon status.

### WP-40 — Seed the mycology profile vocabulary
- Stages → mycology lifecycle: `spore/clone → agar (mycelium on plate) → liquid culture → grain spawn → bulk substrate → colonizing → fruiting → senescent → contaminated`.
- Passage verb → **"Transfer"** (agar-to-agar, agar-to-grain, grain-to-grain, grain-to-bulk). Each transfer records source→target medium type and inoculation rate.
- Media → **"Substrate / Medium"**: `MEA, PDA, MYA agar`, `liquid culture (honey/LME/malt)`, `grain (rye/millet/WBS)`, `bulk (CVG/masters mix/manure/straw)`. Replace the basal-salts/hormone model with substrate composition + supplementation (gypsum, bran).

### WP-41 — Colonization & contamination front-and-center
- Add a **colonization %** field and a colonization timeline per culture. Elevate the existing per-passage contamination flag to a prominent culture-level health signal with contaminant type (Trichoderma/bacterial/cobweb/etc.). The lab-wide contamination-rate panel is directly reusable and is arguably the headline feature for this audience.

### WP-42 — Genetic lineage & strain isolation
- The parent/child lineage tree maps perfectly to **clone lineage** (which plate came from which). Add multi-spore vs isolated-dikaryon markers and a "best performer" flag for strain selection over generations.

### WP-43 — Fruiting conditions & yield
- Per-culture environmental targets (temp, RH, FAE, light) — reuse `environmental_notes` + structured fields. Record **yield** at harvest (fresh/dry weight, flush number) to compare strains and substrates over time.

### WP-44 — Mycology compliance/QC rules
- Lighter regulatory load: repurpose the rule engine for **QC** instead — flag cultures colonizing too slowly, overdue for transfer (senescence risk), or with open contamination not yet discarded.

---

## 7. PHASE F — Cross-cutting & beyond (post-vertical)

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

## 8. Risk register & guardrails

| Risk | Mitigation |
|---|---|
| Phase C drops `CHECK` constraints → bad data could slip in | Replace DB-level constraints with **app-level validation** in the command layer + seed codes that exactly match existing values; add WP-14 tests asserting only valid stages persist. |
| Three verticals drift into three forks anyway | All vertical work is **profile data + manifest entries**, never `if profile == X` branches scattered through components. Enforce in review: vocabulary lives in tables/manifests, not in component logic. |
| Shipping slips chasing verticals | **Cut v1.0.0 at end of Phase A** before any vertical work. The verticals are additive; the plant product must not wait on them. |
| Migration mistakes on table rebuilds | Every rebuild migration ships with WP-14 coverage that loads a pre-migration fixture DB and asserts row counts + values survive. |
| Default-credential / CSP issues reach users | WP-01 and WP-02 are gates on v1.0 — non-negotiable. |
| Cryptographic invariants broken by future audit writes | WP-14 tests must encode the canonical serialization and chain-continuity invariants from WP-18 before they are shipped. The chain must be verified by the CI test suite on every push. |

---

## 9. Versioning plan

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
| v1.2.x | WP-15–17 perf/indexing, backup restore, Excel import | planned |
| **v1.3.0** | **Trust(less) & Audit Layer — Phase 1** (hash-chain + Merkle checkpoints + proof export, WP-18–21) | planned |
| v1.3.x | *Buffer for Phase B overflow, additional patch releases, or Trust Layer follow-up work.* | planned |
| **v1.4.0** | Phase C — profile-ready engine (PTC behavior unchanged, WP-22–27) | planned |
| v2.0.0 | First multi-app release: SteloPTC + **SteloCC** | planned |
| v2.1.0 | **SteloMyco** | planned |
| v2.x+ | Phase F cross-cutting features; Trust Layer **Phase 2** (Dogecoin anchoring, WP-65+) when external proof is needed | future |

> **On the version history:** the jump from `0.1.19` to the `1.0.0-x` line was intentional — the `0.1.x` series was a feature-complete-but-unreleased prototype, and `1.0.0-x` marks the first **production-grade, security-hardened, signed** release with a real GitHub Release. Note the pre-release label shipped as numeric **`1.0.0-1`** (not `rc.1`): the WiX MSI bundler rejects non-numeric pre-release identifiers. Phase A then settled at **v1.1.0** once onboarding (WP-05) landed.

---

*This roadmap is grounded in the live repository at **v1.1.0** (Phase A shipped): 6 migrations with `migration_006_force_password_change` latest, stage CHECK-constraint rebuilds in migrations 002/003 at `db/migrations.rs`, the now-active CSP in `tauri.conf.json`, compliance rules in `commands/compliance.rs`, and the species/specimen models. Hand packets to Claude Code in order; each is scoped to stand alone.*
