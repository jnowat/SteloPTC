# Changelog

All notable changes to SteloPTC will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.7.0] - 2026-06-17

### Added

- **Generational depth tracking on specimens**
  - New `generation` column on `specimens` (migration 010, default 0). Root specimens stay at generation 0. Each split increments the generation counter by 1 on every child, so a third-generation derived culture shows `generation = 3`.
  - A **"Gen N" badge** appears in the specimen detail header for any non-root specimen so the generational depth is visible at a glance.

- **Cumulative passage tracking across the full lineage**
  - New `lineage_passage_offset` column on `specimens` (default 0). When a specimen is split, each child receives `lineage_passage_offset = parent's offset + parent's subculture_count` at the time of the split. This encodes how many passages occurred across all ancestor specimens before this one was created.
  - The "Passages" row in the specimen info card now shows **"N (P{total} from root)"** for derived specimens, where total = `lineage_passage_offset + subculture_count`. Example: a specimen split after 5 parent passages, now on its 3rd passage, displays "3 (P8 from root)".

- **Shared-root identifier for family queries**
  - New `root_specimen_id` column on `specimens` (nullable). NULL for root specimens; set to the absolute root ancestor ID for all derived specimens, regardless of nesting depth. Enables efficient "give me the whole family tree" queries without recursive CTEs.
  - New `get_specimen_family(id)` backend command returns all members of a specimen's family tree (root + all descendants, active and archived) as lightweight `FamilyMember` records including generation, passage offset, and health status.

- **Sibling display in the lineage banner**
  - When a specimen has siblings (other specimens split from the same parent that are still active), a "Siblings" row now appears in the lineage banner using purple chips. Click any sibling to navigate directly to it.

- **`badge-purple` global CSS class** added to App.svelte for generation and sibling indicators; includes dark-mode variant.

### Changed

- `split_specimen` backend command now sets `generation`, `lineage_passage_offset`, and `root_specimen_id` on every child specimen in the same atomic transaction.
- `SpecimenDetail` `loadAll` now calls `get_specimen_family` instead of filtering a full `listSpecimens(1, 500)` to populate child and sibling lists — more efficient and includes archived family members.

## [1.6.4] - 2026-06-17

### Changed

- **Species creation now anchors its chain at `chain_seq = 0`**
  - `create_species` now calls `log_audit_at_seq_zero` instead of `log_audit`. The species entry is inserted at seq=0 with `prev_hash = ZERO_HASH`. This marks the "birth" of the species lineage and its `entry_hash` serves as the cryptographic seed for any specimens created from it.

- **New root specimens are cryptographically seeded from their species**
  - `create_specimen` (root, no parent) now calls `log_audit_seeded_by_species`, which starts the specimen's chain at seq=1 with `prev_hash` set to the species' latest `entry_hash`. Falls back to `ZERO_HASH` for default/seeded species that predate the hash chain.

- **Split is now a dedicated atomic backend command (`split_specimen`)**
  - Previously, splitting was done in the frontend by calling `createSubculture` on the parent and then N `createSpecimen` calls. This was non-atomic and produced incorrect audit chains.
  - `split_specimen` handles the full operation in a single SQLite transaction: archives the parent, appends a "split" event to the parent's chain, creates each child specimen, and creates passage 1 for each child — all in one commit.
  - The parent's "split" audit entry is the last entry in its chain. Every child inherits that entry's `entry_hash` as its `prev_hash`, making the fork cryptographically unambiguous.

- **Per-child media batch, vessel, location, and notes for splits**
  - The split form now shows one configuration row per child instead of a shared form. Each child can be assigned a different media batch, vessel type, location, and optional notes.

- **Passaging now increments the specimen's own chain**
  - `create_subculture` now audits using `("subcultured", "specimen", specimen_id)` instead of `("create", "subculture", subculture_id)`. Each passage event extends the specimen's lineage chain (seq increments within the specimen), rather than starting a new per-subculture chain.

- **Passaging now updates the specimen's health status**
  - `create_subculture` now writes the observed health status back to the specimen record in the same transaction. A single `UPDATE` handles `subculture_count`, `location`, and `health_status` with `CASE WHEN` guards so unset fields are preserved.

## [1.6.3] - 2026-06-17

### Changed

- **`load_demo_data` now generates fully hash-chained records**
  - Previously, demo data was inserted via raw SQL with no audit calls, leaving every demo audit row with `NULL` in `lineage_id`, `chain_seq`, `prev_hash`, and `entry_hash` — making it useless for testing the hash chain feature.
  - Rewrote the function to call `log_audit` for every record (media batch, each specimen, each subculture passage) and `log_audit_for_child` for split specimens. Every audit entry is now fully chained from the moment it is inserted.
  - Added a cryptographic split demonstration: the first species' root specimen (`DEMO-{CODE}-001`) is split into two children (`002A` and `002B`). Both children receive `chain_seq = 1` with `prev_hash` set to the parent's last `entry_hash`, making the fork visible in the Audit Log and exercisable via the Verify buttons.

## [1.6.2] - 2026-06-17

### Fixed

- **`reset_database` now available in release builds**
  - The command was previously guarded by `#[cfg(debug_assertions)]` in both `admin.rs` and `lib.rs`, making it disappear entirely in production. Removed the compile-time guard. The command remains protected by an admin-only role check and requires the exact confirmation phrase `"RESET DATABASE"`.

- **Split audit: `entity_id` fallback in parent hash lookup**
  - When creating a split/derived specimen, the parent's last `entry_hash` is fetched to anchor the child's chain. If the parent's audit row was written before migration 009 (no `lineage_id` back-fill), the lookup now falls back to matching on `entity_id` so the fork is still cryptographically linked rather than silently starting from `ZERO_HASH`.

- **Specimen create: INSERT + audit wrapped in a transaction**
  - The specimen `INSERT` and its corresponding audit log write are now executed within a single `unchecked_transaction`. A failure in either step rolls back both, preventing orphaned specimens with no audit trail or phantom audit entries with no specimen.

## [1.6.1] - 2026-06-16

### Fixed

- **`verify_audit_lineage` always failed for split/forked specimens**
  - The verification loop was anchored to `ZERO_HASH` as the initial `prev_entry_hash`. Root lineages (no parent) have `prev_hash = ZERO_HASH` on their first row, so this worked. But forked lineages (split specimens) have `prev_hash = parent's last entry_hash ≠ ZERO_HASH` on their first row, so the link check `row.prev_hash != ZERO_HASH` immediately returned "Chain broken at seq 1" for every split specimen — making lineage verification completely unusable for the core split use case.
  - Fixed by initializing `prev_entry_hash = rows[0].prev_hash` instead of `ZERO_HASH`. The anchor row's link check now trivially passes; all subsequent links and all entry hashes are still fully verified.

- **`verify_audit_entry` silently returned "Entry not found" for chained rows**
  - The function used non-`Option` Rust types (`String`, `i64`) for nullable database columns (`lineage_id`, `chain_seq`, `prev_hash`, `entry_hash`). When rusqlite encounters a NULL in a non-`Option` field it returns an error, which `.ok()` silently converts to `None`, making the function appear to not find the entry.
  - Fixed by using `Option<String>` / `Option<i64>` for all nullable columns. Rows with no chain data (pre-v1.5.0) now receive a clear message: *"This entry has no chain data (written before the hash chain was introduced in v1.5.0)."*

- **Added regression test** for fork lineage verification to prevent the `ZERO_HASH` anchor bug from re-appearing.

## [1.6.0] - 2026-06-16

### Changed (Breaking — reworks WP-18)

- **Per-lineage hash chain replaces global sequence**
  - The audit hash chain now uses a **per-lineage sequence** instead of a single global counter. Each entity (specimen, media batch, etc.) maintains its own independent chain: `lineage_id` = `entity_id` (or `"system"` for entity-less events), and `chain_seq` counts from 1 within that lineage.
  - **Split/derived specimens** now receive `chain_seq = 1` on their first audit entry, with `prev_hash` set to the parent specimen's last `entry_hash`. Both children of a split share the same `prev_hash`, making the fork cryptographically visible — the canonical data proves they diverged from the same parent state.
  - The canonical serialization format is updated to include `lineage_id`:
    `lineage_id|chain_seq|timestamp|user_id|entity_type|entity_id|action|details`
  - `log_audit()` call sites are **unchanged**. A new `log_audit_for_child(…, parent_lineage_id)` function handles the split case and is called automatically by `create_specimen` when `parent_specimen_id` is present.

### Added

- **Migration 009** adds the `lineage_id TEXT` column to `audit_log` with a composite index on `(lineage_id, chain_seq)`. Rows from migration 008 (global chain) are back-filled with their `entity_id` as `lineage_id` so per-entity verification still works on them.

- **`verify_audit_entry` Tauri command** — recomputes the SHA-256 hash for a single audit row and returns `{ ok, message, stored_hash, computed_hash }`. Any mismatch indicates tampering.

- **`verify_audit_lineage` Tauri command** — walks the entire chain for a `lineage_id`, checking both that each row's hash is correct and that each `prev_hash` matches the preceding row's `entry_hash`. Returns `{ ok, checked, first_break_seq, message }`.

- **Audit Log UI verification**
  - Each chained row now has **Row** and **Chain** verify buttons. **Row** checks just that single entry; **Chain** verifies every entry in the entity's lineage.
  - Verification results appear inline beneath the row (✓ green / ✗ red) and can be dismissed.
  - A **chain integrity banner** at the top of the page shows how many of the visible entries are chained vs legacy, and explains how to use verification.
  - The `🔒 seq` badge in the **#** column now shows a tooltip with the full `lineage_id` on hover.

- **Unit tests** for the new hash-chain logic in `queries.rs`:
  - `chain_seq` increments per-lineage, not globally
  - Child lineage starts at `chain_seq = 1` with parent's last `entry_hash` as `prev_hash`
  - Split siblings share the same `prev_hash`
  - `compute_entry_hash` is deterministic

## [1.5.1] - 2026-06-16

### Added

- **Audit Log UI: hash chain columns now visible**
  - The Audit Log table now shows three new columns introduced in v1.5.0: **#** (chain sequence), **Prev Hash**, and **Entry Hash**.
  - Hashes are truncated to 10 characters in the table cell. Hovering shows the full 64-character SHA-256 hex string as a tooltip. Clicking copies the full hash to the clipboard (brief "✓ copied" confirmation in-cell).
  - Rows written after v1.5.0 display a 🔒 badge in the **#** column and have a faint green background to signal they are part of the verified chain. Legacy rows (written before the migration) show `—` in all three chain columns with no background tint.
  - Existing filters, pagination, and all other columns are unchanged.

## [1.5.0] - 2026-06-16

### Added

- **Hash-chain tamper-evident audit log (WP-18)**
  - Migration 008 adds three new columns to `audit_log`: `chain_seq INTEGER`, `prev_hash TEXT`, and `entry_hash TEXT`. Existing rows retain `NULL` in these columns; all new rows are fully chained.
  - Every new audit entry now computes `entry_hash = SHA-256(canonical_bytes || prev_hash)` where `canonical_bytes` is the pipe-separated serialization `chain_seq|timestamp|user_id|entity_type|entity_id|action|details`.
  - The first chained entry uses a fixed 64-character zero-hash as `prev_hash`; subsequent entries chain off the previous row's `entry_hash`.
  - Hash computation and insert are performed atomically within a single `INSERT` statement — no intermediate state is visible.
  - `AuditEntry` model now exposes `chain_seq`, `prev_hash`, and `entry_hash` fields (nullable, for backward compatibility with pre-migration rows).
  - Added `sha2 = "0.10"` direct dependency to `Cargo.toml`.
  - All existing `log_audit()` call sites continue to work unchanged — the chain fields are computed internally.

## [1.4.1] - 2026-06-14

### Fixed

- **Critical: print dialog never fired in Tauri (all three print functions)**
  - `printCultureReport` and `printLabel` contained `<script>window.onload=function(){window.print()}</script>` inline in the popup HTML. Tauri's CSP (`script-src 'self'`) silently blocks all inline scripts in popup windows, so the print dialog never opened automatically and users who pressed Ctrl+P on the main window printed the raw app UI instead of the report.
  - All three print functions (`printSummaryReport`, `printCultureReport`, `printLabel`) now call `win.print()` from the parent WebView context after `document.close()`, which is not subject to the popup's CSP.

### Changed

- **Print delivery extracted to shared `src/lib/printUtils.ts`**
  - The popup-window + in-page-fallback delivery pattern was duplicated verbatim in all three print functions (~60 lines each). It now lives in a single `deliverPrint()` function imported by all three components.
  - Each component passes `{ frameId, title, css, body, margin?, pageSize?, onError }` — the delivery mechanism is no longer tangled with report-building logic.
  - `ageDays()`, `fmtAge()`, and `healthNum()` (previously private inline functions in `printSummaryReport`) moved into `printUtils.ts` so they are testable and reusable.

- **Removed duplicate utility definitions from components**
  - `SpecimenList.svelte`, `SpecimenDetail.svelte`, and `QrModal.svelte` each had their own copies of `escHtml`/`stageFmt`/`healthLabel`. All three now import directly from `utils.ts`. Eliminated ~30 lines of duplicated code.

- **Print options panel accessibility** (`SpecimenList`)
  - Added `aria-modal="true"` to the popover.
  - Added `role="radiogroup"` + `aria-labelledby` to the radio group.
  - Pressing **Escape** while focused anywhere inside the panel now closes it.

- **`printCultureReport` body HTML readability** (`SpecimenDetail`)
  - The entire report body was a single 400-character concatenated string. Refactored into clearly-named intermediate variables (`infoRows`, `passageTable`, `complianceSection`) and a multi-line template literal.

- **`printLabel` uses `stageFmt` from utils** (`QrModal`)
  - Replaced the inline `.replace(/_/g, ' ').replace(...)` with the shared `stageFmt()` function.

### Tests

- Added `datestamp()` tests to `utils.test.ts` (was the only exported util without test coverage).
- Added tests for `ageDays`, `fmtAge`, and `healthNum` from `printUtils.ts` (17 new test cases, 50 total — all passing).

## [1.4.0] - 2026-06-13

### Changed

- **WP-19 — Professional Specimen Inventory Report**
  - **Print options panel** — clicking "Print Summary" now opens a lightweight options popover (no modal) before generating the report. Users choose a grouping strategy; the selection is remembered for the session.
  - **Selectable grouping**: three modes:
    - **By Development Stage** (default) — specimens grouped in canonical tissue-culture pipeline order (Explant → Callus → Shoot → … → Stock); unknown stages fall to the end.
    - **By Health / Urgency** — four priority bands: Critical (health 0–1), Fair (health 2), Good/Healthy (health 3–4), Unknown/Pending. The Critical band renders with a red left-border so it is immediately visible when the report is opened.
    - **Flat list** — single un-grouped table with a Stage column, identical pagination to the on-screen view.
  - **Executive Summary section** — appears at the top of every report:
    - Four stat boxes: Specimens Shown, Needs Attention (health ≤ 1 OR quarantine OR contamination), In Quarantine, and either Contaminated count (if > 0) or Average Health Score.
    - Attention and quarantine boxes switch to colored borders/values when non-zero.
    - Stage distribution chips (sorted by frequency).
    - Health distribution chips (color-coded Dead → Healthy).
  - **Per-group sub-headers** — each group shows its specimen count, average health score, and inline warning chips for quarantine / contamination counts. Critical and Fair groups carry distinguishing background colors.
  - **Enhanced table columns**: Accession (monospace), Species, Location, Passages, Initiated date, **Age** (computed from initiation date, formatted as `Xd` or `Xmo Xd`), Health (plain text, bold red for critical), Status tags. Cultures older than 730 days display an "Old" warning tag. Critical-health rows have a pink row tint for easy scanning.
  - **Filter bar** — clearly shows all active filters and total record counts (shown vs. total active).
  - **Typography & layout** — dark navy table headers, tight but readable 8.5 px body type, inter-group spacing, `page-break-inside: avoid` on group sections, `thead { display: table-header-group }` so column headings repeat across page breaks.
  - **Print delivery** — same popup → in-page fallback strategy from WP-09; all new content works in both paths.
- Version bumped to **1.4.0** across `package.json`, `Cargo.toml`, and `tauri.conf.json`.

## [1.3.1] - 2026-06-13

### Fixed

- **WP-18 — Print reliability audit & confirmation**
  - Audited all three print functions (`printSummaryReport` in `SpecimenList.svelte`, `printLabel` in `QrModal.svelte`, `printCultureReport` in `SpecimenDetail.svelte`) against the WP-06/WP-09 requirements.
  - Confirmed that no silent `if (!win) return` failures remain. All functions follow a consistent two-path strategy:
    1. **Popup path** — attempts `window.open()` inside a `try/catch`; if the window opens, the report HTML is written and `window.print()` is called in the popup.
    2. **In-page fallback** — if the popup is blocked or returns `null` (common in Tauri/WebView2), a hidden `<div>` and scoped `<style>` are injected into the current page, `window.print()` is called directly, and an `afterprint` listener removes both elements when done.
  - All failure paths surface a clear `addNotification(…, 'error')` toast so the user is never left guessing why nothing happened.
  - Print styling from WP-13 (professional header/footer, page counters, typography) is preserved in both paths.
- Version bumped to **1.3.1** across `package.json`, `Cargo.toml`, and `tauri.conf.json`.

## [1.3.0] - 2026-06-13

### Added

- **WP-17 — Excel import**
  - New `ImportManager.svelte` component accessible from the sidebar ("Import Data").
  - Users can select any `.xlsx` file produced by the SteloPTC Excel export.
  - SheetJS parses the six-sheet workbook (Specimens, Subcultures, Media Batches, Prepared Solutions, Inventory, Compliance) entirely on the frontend.
  - A **dry-run preview** is always shown first: the backend processes every row inside a transaction, returns per-sheet counts (creates / updates / skips) and a list of row-level errors, then rolls back — no data is changed.
  - After reviewing the preview the user clicks **Confirm Import**; the same payload is committed in a single atomic transaction (rolled back automatically on any database-level failure).
  - **Upsert semantics** — rows are matched by their natural key:
    - Specimens → `accession_number`
    - Media Batches → `batch_id` (Batch Code column), falling back to `name`
    - Prepared Solutions / Inventory → `name`
    - Subcultures → `(specimen_id, passage_number)`; `Specimen ID` is resolved against `specimens.id` (UUID) first, then `specimens.accession_number`
    - Compliance → always inserted (no unique natural key)
  - Species referenced by an unknown `Species Code` are auto-created as stub entries using the exported `Species` (genus + name) value.
  - Malformed or invalid rows are reported precisely (sheet name + 1-based row number + reason) rather than silently skipped.
  - Exporting a lab, wiping specimen/inventory/media data, then importing the same file fully restores specimens, media batches, prepared solutions, and inventory. Subcultures link to specimens via UUID or accession number, so they also restore correctly when specimens are imported in the same file.
  - New Tauri command `import_xlsx` in `src-tauri/src/commands/import.rs`; requires `can_write` permission.
- **WP-16 — Backup → Restore (close the loop)**
  - **`restore_backup` Tauri command** (`src-tauri/src/commands/backup.rs`): Admin-only command that validates a backup file (filename pattern + SQLite magic bytes), checkpoints and flushes the live WAL, overwrites the database file with the selected backup, cleans up stale `-wal`/`-shm` sidecar files, logs an audit record, then restarts the application so the restored data loads immediately.
  - **Restore UI** (Dashboard): Admin-only "Restore from Backup" danger panel lists all available backups. Restoring requires two explicit confirmation steps — an initial "Yes, continue" acknowledgement followed by typing `RESTORE` before the destructive action is permitted.
  - **`restoreBackup` API wrapper** (`src/lib/api.ts`): Frontend helper that invokes the new command with the selected backup path.
- Version bumped to **1.3.0** across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [1.2.7] - 2026-06-13

### Changed

- **WP-15 — Query performance & indexing audit**
  - **Migration 007** adds six new indexes to eliminate full-table scans on large datasets:
    - `idx_specimens_created_at` — covers `ORDER BY created_at DESC` in list and search views
    - `idx_specimens_parent` — covers `parent_specimen_id` lineage lookups
    - `idx_specimens_archived_created` — composite covering the common `is_archived = 0 … ORDER BY created_at DESC` path
    - `idx_subcultures_specimen_passage` — composite covering per-specimen history queries ordered by `passage_number`
    - `idx_subcultures_created_at` — covers recent-subculture stats
    - `idx_subcultures_contamination_specimen` — composite covering contamination stats join
  - **Eliminated N+1 correlated subquery** in `list_specimens`, `search_specimens`, and `get_specimen`: the per-row `(SELECT MAX(contamination_flag) FROM subcultures WHERE specimen_id = s.id)` is replaced by a single aggregating `LEFT JOIN` that executes once per query regardless of result-set size.
  - **`list_subcultures` now returns `PaginatedResponse<Subculture>`** — accepts optional `page` / `per_page` parameters (defaults: page 1, 50 per page). The frontend API wrapper preserves the existing `any[]` contract for call sites that do not need pagination.
- Version bumped to **1.2.7** across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [1.2.6] - 2026-06-13

### Added

- **WP-12 — Accessibility & keyboard pass (WCAG 2.1 AA target)**
  - **Skip-to-content link** (`App.svelte`): A visually-hidden "Skip to main content" link appears at the top of the layout and becomes visible on keyboard focus, letting keyboard-only users bypass the sidebar navigation (WCAG 2.4.1).
  - **Visible focus indicators** (`App.svelte`): `:focus-visible` global rule adds a `2px solid #2563eb` outline with 2px offset on all focusable elements so keyboard users always see where focus is; `:focus:not(:focus-visible)` suppresses the outline for mouse clicks only (WCAG 2.4.7).
  - **Sidebar ARIA improvements** (`Sidebar.svelte`): Navigation landmark now has `aria-label="Main navigation"` and `id="sidebar-nav"`. Hamburger button gains `aria-expanded` (reflects drawer open/closed state) and `aria-controls="sidebar-nav"`. Each nav-item button gains an `aria-label` that includes the keyboard shortcut where applicable (e.g., `"Dashboard — overview of all key metrics (Ctrl+1)"`). Active nav item carries `aria-current="page"`. Dark mode toggle button carries `aria-label` describing the action ("Switch to light theme" / "Switch to dark theme"). Logout button carries `aria-label="Log out"`.
  - **QR Modal focus trap** (`QrModal.svelte`): On open, focus moves to the close button automatically. Tab/Shift+Tab cycle is constrained within the modal dialog — focus cannot escape to the backdrop or page behind (WCAG 2.1.2). `aria-labelledby` points to the modal heading; heading carries the matching `id`.
  - **Lightbox accessibility** (`SpecimenDetail.svelte`): Photo lightbox is now a proper `role="dialog"` with `aria-modal="true"` and `aria-label="Photo viewer"`. Focus moves to the close button when the lightbox opens (`$effect` watches `lightboxSrc`). Pressing Escape closes the lightbox. Close button carries `aria-label="Close photo viewer"`.
  - **Health slider ARIA** (`SpecimenForm.svelte`, `SpecimenDetail.svelte`): Both the New Specimen form and the Record Passage form health sliders now carry `aria-label="Health status"`, `aria-valuemin`, `aria-valuemax`, `aria-valuenow`, and `aria-valuetext` (e.g., "2 – Fair"), giving screen readers meaningful feedback as the slider moves.
  - **Keyboard shortcuts documented** in nav item `aria-label` attributes and existing sidebar `title` tooltips. Shortcuts: Ctrl+1 Dashboard, Ctrl+2 Specimens, Ctrl+3 Media Logs, Ctrl+4 Reminders, Ctrl+5 Error Log.
- Version bumped to **1.2.6** across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [1.2.5] - 2026-06-13

### Fixed — WP-09: Tauri-reliable print invocation

- **Print Summary** (`SpecimenList.svelte`) — the "Print Summary" button now works on the Windows desktop (Tauri/WebView2) without the "Could not open print window" error. The function first attempts `window.open` (preserving the existing behavior for web/browser builds); if the popup is blocked (as it consistently is in WebView2), it falls back to injecting a hidden `<div>` with the report HTML and a `<style>` element with all print CSS scoped to `@media print`, then calls `window.print()` directly. The `afterprint` event cleans up both elements. The WP-13 output quality (professional header, footer, page counters, typography) is identical in both paths.
- **Culture Certificate** (`SpecimenDetail.svelte`) — `printCultureReport` receives the same popup/fallback treatment. Also fixes a pre-WP-06 silent-fail bug: the function previously did `if (!win) return` with no user notification; it now shows a proper error toast and uses the in-page fallback.
- **QR Label** (`QrModal.svelte`) — `printLabel` receives the same in-page fallback. The `@page { size: 2in 3in; margin: 0 }` rule is placed at the top level of the injected `<style>` element to correctly set the label page size.
- **Version alignment** — `tauri.conf.json` was at `1.2.3` while `package.json` and `Cargo.toml` were at `1.2.4`; all three are now aligned at `1.2.5`.

## [1.2.4] - 2026-06-13

### Added — WP-13: Print / PDF Polish

- **Professional print header band**: every print window (Culture Certificate, Specimens Summary) now opens with a consistent three-column header — a reserved logo area (64 px placeholder), the lab brand + report name, and a right-aligned metadata block (accession, generated date, prepared-by user).
- **Professional print footer band**: footer displays `SteloPTC · Tissue Culture Management System · {date}` on the left and a CSS page-counter (`Page N of M` in print, `Page N` in preview) on the right.
- **A4 + US Letter support**: both documents use `@page { size: auto; }` so the browser uses whatever paper size is selected in the print dialog (A4 or US Letter) without clipping content.
- **Typography and spacing polish**: font stack updated to `'Segoe UI', -apple-system, Helvetica, Arial, sans-serif`; header brand is 21–22 px/900 weight; metadata is 9.5 px/1.8 line-height; table cells use 9–10.5 px body type with wider column padding; `thead { display: table-header-group }` ensures column headers repeat across page breaks.
- **Page-break hygiene**: `tr { page-break-inside: avoid }` on all table rows and `page-break-inside: avoid` on the specimen info grid prevent orphaned rows and torn table cells.
- **Specimen info grid**: label column widened to 155 px and label font dropped to 9.5 px for a cleaner two-column layout in the Culture Certificate.
- **Filter highlight bar**: the Specimens Summary filter line is now displayed as a left-bordered highlight bar (`border-left: 3px solid #e2e8f0; background: #f8fafc`) for better visual hierarchy.

### Added — WP-14: First Test Harness

- **`src/lib/utils.ts`**: new shared TypeScript module containing pure, testable utility functions — `escHtml`, `healthLabel`, `stageFmt`, `composeLocation`, `formatAccessionNumber`, `computeStockAdjustment`, `datestamp` — extracted and suitable for use across print functions and form validation.
- **`src/lib/utils.test.ts`**: Vitest test suite with 30 passing assertions covering all utility functions (null/undefined handling, HTML escaping, health level mapping and clamping, stage formatting, location composition, accession formatting, stock adjustment bounds).
- **`vitest.config.ts`**: Vitest configuration with jsdom environment and `@sveltejs/vite-plugin-svelte` integration. Test glob: `src/**/*.test.ts`.
- **`npm test` / `npm run test:watch`**: added `test` and `test:watch` scripts to `package.json`; `vitest`, `@testing-library/svelte`, `@testing-library/jest-dom`, and `jsdom` added to `devDependencies`.
- **Rust unit tests — `db::queries`**: `#[cfg(test)]` module with 8 tests covering `generate_accession_number` (first specimen gets `001`, second gets `002`, different species resets sequence, different date resets sequence, 3-digit zero-padding) and `PaginationParams` (offset on page 1, offset on page 2, no underflow on page 0).
- **Rust unit tests — `commands::inventory`**: extracted `apply_stock_adjustment(current, adjustment) -> Result<f64, String>` and `is_low_stock(current, minimum) -> bool` as `pub` helper functions; 8 `#[cfg(test)]` assertions covering positive/negative adjustment, exact-zero, below-zero error, and low-stock threshold comparisons.
- **Rust unit tests — `commands::compliance`**: `#[cfg(test)]` module with 10 assertions using in-memory SQLite (minimal schema created inline); tests cover all four auto-flag rules — expired permit (detected/not-detected), quarantine-no-release (detected/with-date-excluded), positive-not-quarantined (detected/already-quarantined-excluded), citrus HLB missing/recent test, and the cross-cutting rule that archived specimens are excluded from every flag query.
- **`.github/workflows/test.yml`**: new CI workflow that runs `npm test` (Vitest) and `cargo test --lib` (Rust) on every push and pull request to `master`. Runs on separate jobs (`frontend-tests` and `rust-tests`) so failures are reported independently. Blocks merges on any test failure.

## [1.2.3] - 2026-06-13

### Fixed
- Dark mode table text invisible: `td` elements now have an explicit `color: #e2e8f0` override in `:global(.dark td)` so text is always readable regardless of CSS variable resolution order.
- `SkeletonLoader.svelte`: replaced `@media (prefers-color-scheme: dark)` with `:global([data-theme="dark"])` so the skeleton respects the app's manual dark-mode toggle rather than the system preference.
- `EmptyState.svelte`: same `@media` → `[data-theme="dark"]` fix; title and message colors now use `var(--color-text-muted)` / `var(--color-text-faint)` tokens.

### Added
- `DataState.svelte`: new reusable component that unifies all four data-fetch states — `loading` (animated skeleton), `error` (inline retry panel), `empty` (friendly message + optional CTA), and `ready` (renders children). Replaces ad-hoc `{#if loading}` / `{:else if}` branches throughout list views.

### Changed
- `SpecimenList.svelte`: integrated `DataState` for loading and error states; inline error now shows a retry button; FirstRun and filter-empty states remain within the ready slot.
- `MediaList.svelte`: loading, error, and empty states replaced with `DataState`.
- `InventoryManager.svelte`: loading, error, and empty states (inventory items + prepared solutions) replaced with `DataState`.
- `ReminderList.svelte`: loading, error, and empty states replaced with `DataState`.
- `ComplianceView.svelte`: loading, error, and per-tab empty states replaced with `DataState`; flags tab shows a "✅ All clear" empty state; records tab shows an empty state with a "+ New Record" CTA.
- `AuditLog.svelte`: plain-text loading/empty replaced with `DataState` skeleton + descriptive empty state.
- `ErrorLog.svelte`: fetch errors are now surfaced inline (with retry) instead of silently swallowed; existing custom loading/empty UI preserved.

## [1.2.2] - 2026-06-13

### Added
- `src/lib/styles/tokens.css`: central design-token file defining CSS custom properties for colors (light + dark), spacing scale, typography sizes, border radii, shadows, z-index layers, and transitions. Single source of truth for all visual constants.

### Changed
- `app.ts`: dark mode subscriber now sets a `data-theme="dark"` / `data-theme="light"` attribute on `<html>` in addition to the existing `.dark` class, enabling token-based theming.
- `App.svelte`: imports `tokens.css` once at the top of the style block; `.app` background and color now reference `--color-bg` / `--color-text` tokens.
- `Dashboard.svelte`: all hardcoded color, spacing, radius, and typography values replaced with design tokens; `:global(.dark)` overrides removed in favour of automatic token switching via `data-theme`.
- `Sidebar.svelte`: all hardcoded values replaced with sidebar-scoped tokens (`--color-sidebar-*`) and shared tokens for spacing, radii, shadows, z-index, and transitions.

## [1.2.1] - 2026-06-13

### Added
- `SkeletonLoader.svelte`: reusable animated shimmer skeleton for table list views, supports configurable row/column counts and both light and dark mode.
- `EmptyState.svelte`: reusable friendly empty state with icon, title, message, and optional call-to-action button.

### Changed
- `SpecimenList.svelte`: loading state now shows an animated skeleton; filtered-empty state uses the new `EmptyState` component with a search icon and helpful message.
- `MediaList.svelte`: loading state shows skeleton; empty state shows a friendly prompt with a "New Batch" CTA.
- `InventoryManager.svelte`: loading state shows skeleton; empty inventory shows a "New Item" CTA; filter-empty and no-solutions states use `EmptyState`.
- `ReminderList.svelte`: loading state shows skeleton; empty state shows a "New Reminder" CTA.

## [1.2.0] - 2026-06-13

### Added

- **WP-08 — Specimen Work Queue / Daily Task View**
  - **`get_work_queue` Tauri command** (`work_queue.rs`) — returns a prioritized list of specimens needing attention. Detects five conditions: (1) subculture due or overdue (based on species default interval), (2) media batch expired on most recent subculture, (3) contamination flag on most recent passage, (4) no recorded passages, (5) unresolved quarantine (no release date or release date passed). Results are sorted by urgency (critical → high → normal) then by days overdue descending.
  - **`WorkQueue.svelte`** — new read-only view showing a prioritized table of specimens that need attention. Each row displays urgency badge (Critical/High/Normal), accession number, species, location, and a plain-language description of the issue. Clicking any row navigates to the specimen detail. Shows a summary badge row (count by urgency level) and an "All clear" state when no items are pending.
  - **`getWorkQueue` API function** (`api.ts`) wraps the new command.
  - **`workQueueCount` store** (`app.ts`) — writable integer store, populated on login and refreshed whenever the Work Queue view loads. Persists in memory for the session lifetime.
  - **Work Queue nav entry** (`Sidebar.svelte`) — added between Dashboard and Specimens with a ✅ icon and an amber count badge matching the error-log badge style. Badge animates in when count > 0.
  - **`work-queue` view route** (`App.svelte`, `app.ts`) — added to the View union type and the view-switcher conditional block.
- Version bumped to **1.2.0** across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [1.1.1] - 2026-06-13

### Fixed

- **WP-06 — Print Summary & Print Label error handling**
  - `SpecimenList.svelte` — `printSummaryReport` now wraps the popup open and document write in try/catch blocks. A blocked or failed popup shows a user-facing error notification ("Could not open print window. Please allow popups for this site and try again.") instead of silently doing nothing.
  - `QrModal.svelte` — `printLabel` receives the same treatment: popup-blocked and write failures both surface a clear error notification. Import of `addNotification` added.

### Changed

- **WP-07 — QR Scanner: Reject Non-SteloPTC Codes Gracefully**
  - **`QrScanner.svelte`** — added `looksLikeNonSteloPTC()` helper that flags payloads starting with `http://`, `https://`, `ftp://`, or matching an email pattern. When a non-SteloPTC code is detected, a new `invalidQr` state is set and a yellow warning card ("This QR code is not a SteloPTC specimen label") is shown instead of the green result card. The "Open Specimen" button and `parsedAccession` are suppressed for invalid codes. The scan event is still recorded in the database for audit purposes. Valid SteloPTC JSON payloads and plain-text accession numbers continue to work normally.
- Version bumped to **1.1.1** across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [1.1.0] - 2026-06-11

### Added

- **WP-05 — Onboarding empty state + seed-data toggle**
  - **`FirstRun.svelte`** — new component shown whenever the lab has zero specimens. Displays a two-step guide ("Configure your species registry" → "Accession your first specimen"), with direct navigation buttons for each step. Supervisors and admins also see a **Load Sample Data** button (green, clearly labelled).
  - **`Dashboard.svelte`** — shows `FirstRun` instead of the normal stats grid when `total_specimens === 0`. Returns to the full dashboard automatically once specimens exist (or after demo data is loaded).
  - **`SpecimenList.svelte`** — shows `FirstRun` (with an inline "Add First Specimen" shortcut that scrolls to and opens the specimen form) when the list is genuinely empty (no search/filter active and `total === 0`). Filtered-but-empty searches still show the concise "No specimens found" message.
  - **`load_demo_data` Tauri command** (`admin.rs`) — creates 1 demo MS media batch, 3 demo specimens (Asparagus, Nandina, Citrus) using the seeded species registry, each with 3 passages of subculture history, all in a single atomic transaction. Guard: returns an error if any specimens already exist so it can't clobber an active lab. Supervisors and admins only.
  - **`loadDemoData` API function** (`api.ts`) wraps the new command.
  - Removing demo data: use existing **Admin → Dev Tools → Reset Database** (preserves species/users).
- Version bumped to **1.1.0** across `package.json`, `Cargo.toml`, `tauri.conf.json` (versionCode 24), `app/build.gradle.kts` (versionCode 24), and sidebar display.

## [1.0.0-2] - 2026-06-11

### Changed

- **WP-04 — Crash-proofing & data-integrity pass**
  - Replaced `.unwrap()` on `path.parent()` in `attachments_dir` with a proper `Result` return; callers propagate the error through the existing error-log + toast system instead of panicking.
  - Wrapped `create_subculture` in a SQLite transaction: the subculture INSERT, specimen `subculture_count` UPDATE, and optional location UPDATE are now atomic — a failure on any step rolls back all changes.
  - Wrapped `create_media_batch` in a SQLite transaction: the media batch INSERT, all hormone/reagent INSERTs, and all inventory stock deduction UPDATEs are now atomic — no partial batch is committed if a hormone or stock update fails.
  - `create_backup` now verifies the WAL checkpoint result; if active readers prevented a full checkpoint (`busy_frames > 0`), the backup is aborted with a descriptive error rather than silently copying an incomplete snapshot.
- Version bumped to **1.0.0-2** across `package.json`, `Cargo.toml`, `tauri.conf.json` (versionCode 23), and `app/build.gradle.kts` (versionCode 23).

## [1.0.0-1] - 2026-06-11

### Added

- **First signed GitHub Release** — both the Windows MSI and the Android APK are now attached directly to GitHub Release assets on every `release` event.
  - Windows workflow (`build-windows.yml`) now fires on `release: types: [published]` and uploads the `.msi` via `softprops/action-gh-release@v2`.
  - Android workflow (`build-android.yml`) decodes a base64-encoded release keystore from the `ANDROID_KEYSTORE_BASE64` secret, writes it to a temp path, and passes the path to Gradle as `ANDROID_KEY_STORE_PATH`.
- **Hard-fail Android release signing** — the release APK build no longer falls back to debug signing if keystore secrets are absent; it fails immediately with a descriptive error. All four secrets (`ANDROID_KEYSTORE_BASE64`, `ANDROID_KEY_STORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`) are validated before the build starts.
- **Release keystore** (`steloptc`, RSA 4096, valid ~27 years) generated and documented in `.github/SIGNING.md`. The same key must be used for all future releases to allow in-place APK upgrades on Android.
- **`build.gradle.kts` signing patch step** — after `cargo tauri android init` regenerates `gen/android/`, CI injects the signing config so the committed file and the CI-generated file stay in sync.

### Changed

- Version bumped to **1.0.0-1** across `package.json`, `Cargo.toml`, `tauri.conf.json` (versionCode 22), and `app/build.gradle.kts` (versionCode 22). The version uses a numeric pre-release suffix (`-1`) rather than `rc.1` because the WiX MSI bundler requires pre-release identifiers to be numeric-only (≤ 65535); `rc` is non-numeric and rejected at bundle time.
- README Downloads table updated: both Windows and Android rows now point to GitHub Releases for release binaries.

## [0.1.21] - 2026-06-11

### Changed

- **Content-Security-Policy hardened** — replaced `"csp": null` (no policy) with a locked-down policy in `tauri.conf.json`:
  - `default-src 'self' ipc: http://ipc.localhost` — baseline; covers Tauri IPC for all unspecified directive fallbacks.
  - `script-src 'self'` — no remote or inline scripts; all JS is Vite-bundled.
  - `style-src 'self' 'unsafe-inline' https://fonts.googleapis.com` — Svelte injects inline styles; Inter font CSS loaded from Google Fonts.
  - `font-src 'self' https://fonts.gstatic.com` — Google Fonts glyph files.
  - `img-src 'self' data: blob:` — `data:` for base64 photo lightbox round-trip and QR canvas output; `blob:` for canvas-generated blobs.
  - `connect-src 'self' ipc: http://ipc.localhost` — explicit Tauri IPC allowance (required for `invoke()` calls).
  - `worker-src blob:` — html5-qrcode/ZXing creates its decoder web worker from a `blob:` URI; without it the camera scanner fails.
  - No remote script origins; no `'unsafe-eval'`.
- Version bumped to **0.1.21**.

## [0.1.20] - 2026-06-11

### Added

- **Forced password change on first login** — fresh installations (or any account with `must_change_password = 1`) are blocked from accessing the application until a new password is set.
  - New DB migration (006): adds `must_change_password BOOLEAN NOT NULL DEFAULT 0` to the `users` table; the seeded `admin` row is set to `1` so the default `admin/admin` credential can never grant unguarded access.
  - Login response now carries a `must_change_password` flag; if `true`, the front end routes to a full-screen **Set a New Password** overlay before the app shell renders. All other navigation is blocked until the change is complete.
  - New `ForceChangePassword.svelte` component: validates minimum 8-character length and confirmation match, calls the new `change_password` Tauri command, then clears the gate.
  - New `change_password` Tauri command: validates the new password (≥ 8 chars), bcrypt-hashes it, clears `must_change_password`, and writes an audit entry.
  - `mustChangePassword` Svelte store added to `auth.ts`; `setAuth` now accepts an optional third argument to set it; `clearAuth` resets it.

### Changed

- Login hint updated: "First login: admin / admin (you will be prompted to set a new password)".
- Version bumped to **0.1.20** across `package.json`, `Cargo.toml`, `tauri.conf.json` (versionCode 20), `app/build.gradle.kts` (versionCode 20), and sidebar display.
