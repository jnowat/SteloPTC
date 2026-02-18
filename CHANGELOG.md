# Changelog

All notable changes to SteloPTC will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.9] - 2026-02-18

### Added

- **Apical Meristem stage**: New stage option "Apical Meristem" added to the specimen stage selector, distinct from "Shoot Meristem".
- **Unknown / Awaiting health grade (-1)**: Both the New Specimen form and the passage recording form now have an "Unknown / Awaiting Assessment" checkbox above the health slider. When checked, health is stored as `-1` and displayed in purple throughout the app. Intended for newly initiated AMTC cultures or small passages where health cannot yet be assessed.
- **Employee ID on specimens, subcultures, and media batches**: A new "Employee ID / Badge #" text field appears on the New Specimen form, the Record Passage form, and the Media Batch form for technician traceability.
- **Developer Mode toggle**: A new red "Dev Tools — Developer Mode" admin-only panel on the Dashboard enables/disables dev mode globally. State is persisted in `localStorage` across sessions. When active, passage cards in Specimen Detail show a red "Edit" button for inline passage editing.
- **Inline passage editing (dev mode)**: When dev mode is on, any expanded passage card shows an inline edit form (date, media batch, vessel type, health status, performer, observations, notes) with a Save/Cancel action. Requires admin role to activate dev mode.
- **Multi-expand passage timeline**: Passage cards in Specimen Detail can now be independently expanded — multiple cards can be open simultaneously. Previously only one card could be open at a time.
- **Health status in passage form**: The Record Passage form now includes the same 0–4 color-coded health slider with "Unknown / Awaiting" toggle as the New Specimen form.
- **Inventory physical state (solid / liquid)**: Inventory items now have a Physical State field. Liquids require a concentration and unit (nM, µM, mM, M, ng/mL, µg/mL, mg/mL, mg/L, g/L, %). State and concentration are shown in the inventory table.
- **Prepared Stock Solutions**: New section in Inventory Manager to track stock solutions made from solid reagents. Each solution records source item, concentration, volume prepared/remaining, prep date, and preparer (employee ID). Volume remaining can be updated inline; solutions can be deleted.
- **Solid reagent auto-calculation in Media Batch**: When adding a solid reagent to a media batch, entering the physical amount used (g or mg) auto-calculates the final concentration in the media (mg/L) based on the batch volume. The calculated value is shown in a "Final Conc." read-only field.
- **Stock depletion on media creation**: When a media batch is saved, the `amount_used` for each reagent is automatically deducted from the corresponding inventory item's `current_stock`. A warning is shown in the UI if the amount used exceeds current stock before saving.
- **Temporal date validation**: The passage form warns (in red) if the selected media batch's preparation date is after the passage date, which would be temporally impossible.
- **DB migration (v2)**: Added migration_002_v019 that extends the schema: specimens table is recreated to allow `shoot_meristem`, `apical_meristem`, `root_meristem` stages in the CHECK constraint and adds `employee_id` to specimens; adds `physical_state`, `concentration`, `concentration_unit` to inventory_items; adds `employee_id` to media_batches; adds `employee_id` and `health_status` to subcultures; adds `amount_used` and `amount_unit` to media_hormones; creates `prepared_solutions` table.

### Fixed

- **Location label capitalization**: Room, Rack, Shelf, and Tray sub-labels in the structured location field were being displayed in full uppercase due to an inherited `text-transform: uppercase` CSS rule. The `.loc-label` style is now `text-transform: none`.

### Changed

- Version bumped to 0.1.9 across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [0.1.8] - 2026-02-18

### Added

- **Passage Timeline**: Subculture history is now displayed as a beautiful vertical timeline (newest passage first) replacing the plain table. Each passage card shows passage number, date, media batch, vessel type, and destination location at a glance, and can be clicked to expand full details (temperature, pH, light cycle, from/to location, observations, notes, performer).
- **Split Culture**: When recording a passage, a "Split culture into multiple containers" toggle now appears at the bottom of the form. Enabling it reveals a count field — submitting creates that many new child specimen records, each linked to the current specimen via `parent_specimen_id`, at the transfer-to location, with an auto-generated note (e.g., "Split from ACC-001 on 2026-02-18. Container 3 of 20."). A live preview graphic shows the parent → child layout before submitting.
- **Lineage Banner**: Specimen Detail now shows a lineage banner when a specimen has a parent (split source) or children (split products). Clicking any parent/child chip navigates directly to that specimen's detail page.
- **Location Dropdowns in Passage Form**: Transfer-To Location in the Record Passage form now uses the same Room/Rack/Shelf/Tray dropdown system as the specimen creation form, with last-used values remembered via `localStorage`.

### Changed

- Version bumped to 0.1.8 across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [0.1.7] - 2026-02-18

### Added

- **Inter font**: Switched global application font to Inter (Google Fonts), providing clean mixed-case rendering throughout all UI panels.
- **Media Logs — Edit button**: Admins and supervisors can now edit existing media batches (name, expiration date, volume remaining, storage conditions, QC notes, review flag, notes) without deleting and recreating.
- **Media Logs — Basal Salts auto-calculator**: When creating a media batch, enter the weight of basal salts powder (g) and total water volume (mL); the Basal Salts Concentration (g/L) field auto-populates. A "Pre-made solution" toggle shows a direct concentration field for commercial liquid formulations.
- **Media Logs — Vessels/Jars Prepared**: New numeric field tracks how many jars/vessels were prepared per batch; displayed as a column in the media batch table.
- **Admin dev-tools — Reset Database**: Admin-only panel on the dashboard under "Dev Tools" allows a full wipe of all operational data (specimens, media, subcultures, inventory, compliance records, audit log) while preserving user accounts and species definitions. Requires typing `RESET DATABASE` to confirm. Intended for use during development/setup.
- **Health status badge**: In Specimen Detail, numeric health values (0–4) now render as colored badges (red → green) instead of raw numbers.

### Fixed

- **Label case / text justification**: Removed `text-transform: uppercase` from global label CSS. Labels now display in proper mixed case, fixing the visual misalignment caused by CAPS-only labels next to regular-case content.
- **Light Cycle label**: Subculture form now shows "Light Cycle (hrs on/hrs off)" with placeholder "16/8" for clarity.
- **Subculture environmental fields**: Temperature, pH, and Light Cycle inputs are now compact flex items (fixed widths) instead of occupying a full three-column grid row.

### Changed

- Version bumped to 0.1.7 across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [0.1.6] - 2026-02-17

### Added

- **Shoot Meristem / Root Meristem stages**: Two new stage options ("Shoot Meristem", "Root Meristem") added to the specimen stage selector.
- **Health Status slider**: Replaced the free-text health status field in New Specimen with a color-coded 0–4 range slider (0 = Dead, 1 = Poor, 2 = Fair, 3 = Good, 4 = Healthy) with a gradient track and live label display.
- **Structured location entry**: Location field now shows four dropdowns — Room (1–5), Rack (A–D), Shelf (1–5), Tray (A–F) — that compose a human-readable location string on save (e.g., "Room 2 / Rack B / Shelf 3 / Tray C").
- **Initial Media Batch field on new specimen**: Specimen creation form now includes a Media Batch dropdown; the selected batch is recorded in the specimen notes for traceability.
- **Auto-populate last-used form values**: New Specimen form remembers the last Room, Rack, Shelf, Tray, Health, Species, Stage, Propagation Method, and Media Batch selections via `localStorage` and pre-fills them on next use.
- **Media Logs – Basal Salts auto-populate**: Last-used Basal Salts type and concentration are remembered and pre-filled on the next media batch form.
- **Stock Reagent traceability section**: New Specimen Media Batch creation form includes a "+ Add Reagent" builder that lets you select inventory items, auto-fills their lot number, and records amount + unit. Reagent details are appended to batch notes as a structured traceable log.
- **Inventory unit datalist**: Unit field now provides a dropdown suggestion list (g, mg, mL, L, units, pcs, µg, µL) while still allowing free-text entry.

### Fixed

- **Sidebar icon rendering**: Dark mode toggle (☀/🌙) and logout (➤) buttons were rendering raw HTML entity strings (e.g., `&#9728;`) as literal text instead of the actual symbols. Fixed by using Svelte's `{@html ...}` interpolation. This was the "extra text in the lower left corner that changes with dark mode" bug.
- **Temperature label**: Subculture form now correctly shows "Temperature (°C)" instead of "Temperature (C)".

### Changed

- **Media Logs form layout**: pH, Agar, Sucrose, and Volume Prepared fields are now compact fixed-width inputs instead of stretching to fill the full grid column. Sterilization selector is also narrow. Notes textarea remains full width.
- **Basal Salts Concentration label**: Changed from "Concentration" to "Basal Salts Concentration (g/L)" for clarity.
- Version bumped to 0.1.6 across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [0.1.5] - 2026-02-17

### Fixed

- **`lifecycle_function_unavailable` error in release builds**: The MSI/exe showed "mount(...) is not available on the server" instead of the login screen. Root cause: the Vite production build was resolving Svelte 5's package exports to the **server** entry point (where `mount()` does not exist) instead of the **browser/client** entry point.
  - `vite.config.ts`: Added `compilerOptions: { generate: "client" }` to the Svelte plugin to force client-side code generation — this is a Tauri desktop app with no SSR.
  - `vite.config.ts`: Added `resolve: { conditions: ["browser"] }` so the bundler resolves Svelte 5's conditional package exports to the client-side APIs (`mount`, `onMount`, `onDestroy`, etc.).
  - `vite.config.ts`: Added `build.target` for modern WebView2/WebKitGTK/WKWebView engines, with conditional minification and sourcemaps based on `TAURI_DEBUG`.
  - `svelte.config.js`: Added `compilerOptions: { generate: "client" }` as a redundant safety net.
- **GitHub Actions workflow** (`build-windows.yml`):
  - **Transient 502 failures**: NSIS bundler downloads `nsis_tauri_utils.dll` from GitHub Releases, which frequently returns HTTP 502 Bad Gateway — breaking the entire CI build even though the MSI was already produced. Fixed by building MSI only (`--bundles msi`) and skipping the redundant NSIS installer.
  - Added retry logic (up to 3 attempts with 15 s/30 s backoff) around `cargo tauri build` so transient WiX download failures also recover automatically.
  - Added `swatinem/rust-cache` for Rust compilation caching (cuts repeat build times significantly).
  - Replaced `tauri-apps/tauri-action@v0` with direct `cargo tauri build` invocation for full control over retry/bundle flags.
  - Uploads both the standalone `.exe` and the `.msi` installer as artifacts.

### Changed

- Version bumped to 0.1.5 across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [0.1.4] - 2026-02-17

### Fixed

- **Blank/dark screen in release builds (.exe/.msi)**: Two root causes identified and fixed:
  1. `tauri.conf.json` had `devUrl: "http://localhost:1420"` — release builds were attempting to load the frontend from a dev server that doesn't exist in production. Set `devUrl` to `null` so production builds always load from the bundled `frontendDist` (`../dist`).
  2. `InventoryManager.svelte` had broken syntax (`function filtered: any[]` and `let filtered = $derived(filtered)`) that caused `vite build` to fail with "Unexpected token", meaning no `dist/` folder was generated for the release build to bundle.
- **InventoryManager.svelte**: Fixed `$derived` to use a proper function call (`$derived(getFilteredItems())`) instead of a circular self-reference.

### Changed

- Version bumped to 0.1.4 across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [0.1.3] - 2026-02-17

### Fixed

- **White/blank screen on startup (Windows 11)**: Complete rewrite of the application boot sequence to eliminate all blank-screen failure modes.
  - `main.ts` no longer removes the loading screen before Svelte mounts. The loader now stays visible until the app successfully renders, using a CSS class (`body.app-ready`) to hide it only after confirmed mount.
  - Added `try-catch` around the entire mount process so any JavaScript error during initialization shows a visible error message instead of a blank screen.
  - Added global `window.onerror` and `window.onunhandledrejection` handlers inline in `index.html` (before any module code) to catch and display import failures, Svelte compilation errors, or any other uncaught exceptions.
  - Loading screen is now `position: fixed` with `z-index: 99999` to guarantee visibility regardless of CSS state.
  - Error display integrated into the loader screen with styled error box, replacing the spinner when an error occurs.
- **App.svelte startup error handling**: `onMount` session restore is now wrapped in `try-catch`, and the `startupError` variable (previously declared but unused) now properly displays errors with a branded error screen.
- **Added `:global(.btn:disabled)` style**: Disabled buttons now visually indicate their state with reduced opacity.
- **InventoryManager.svelte**: Replaced no-op `$effect` and repeated `filteredItems()` function calls with proper `$derived` reactive values for better Svelte 5 compatibility.

### Changed

- Version bumped to 0.1.3 across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [0.1.2] - 2026-02-17

### Added

- **Inventory Management**: Full CRUD for lab supply inventory with category-based organization (media ingredients, vessels, hormones, chemicals, consumables, equipment).
  - Stock level tracking with minimum stock and reorder point thresholds.
  - Stock adjustment with positive/negative amounts and audit-logged reasons.
  - Low-stock alerts displayed on the dashboard.
  - Search, category filter, and low-stock-only filter on the Inventory page.
  - Expiration date tracking with visual expired indicators.
  - Supplier, catalog number, lot number, and storage location fields.
  - Role-based access: Tech+ can create/edit/adjust, Supervisor+ can delete.
- **Database Backup**: On-demand database backup from the dashboard.
  - WAL checkpoint before copy ensures backup integrity.
  - Backups stored in `backups/` subdirectory with timestamped filenames.
  - Backup list retrieval for future restore UI.
  - Audit-logged backup events.
  - Supervisor/admin role required.
- **Dashboard Enhancements**: New low-stock inventory alerts panel and backup button on the dashboard.

### Fixed

- **README database path**: Corrected Linux/macOS data directory from `.stelobtc` to `.steloptc` in documentation.

### Changed

- Version bumped to 0.1.2 across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.
- Updated README roadmap to mark inventory management and database backup as completed.
- Updated README project structure to include new files.

## [0.1.1] - 2026-02-17

### Fixed

- **White screen on startup**: Added loading screen in `index.html` so the app never shows a blank white page while JavaScript initializes.
- **Session restoration race condition**: App now shows a branded loading/spinner state while validating a saved session token, instead of prematurely rendering the authenticated UI which could cause a flash or crash.
- **Initialization state tracking**: Added `initializing` store to `auth.ts` so `App.svelte` can distinguish between "not logged in" and "checking saved session".
- **Graceful database fallback**: `lib.rs` no longer panics on database initialization failure; falls back to in-memory database so the app window still appears.
- **Setup error propagation**: Replaced `unwrap()`/`expect()` in Tauri `setup` hook with proper `Result` error propagation to avoid silent panics.
- **API error handling**: Improved error coercion in `api.ts` using `instanceof Error` checks for more reliable error messages.
- **Login error handling**: Wrapped `login()` in try-catch for consistent error messages on auth failures.
- **Database path typo**: Fixed Linux/macOS data directory from `.stelobtc` to `.steloptc`.
- **Removed invalid `$schema`**: Removed incorrect schema URL from `tauri.conf.json` that pointed to a non-official repository.
- **Loading screen cleanup**: `main.ts` now removes the HTML loading screen once Svelte mounts, preventing visual overlap.

### Changed

- Version bumped to 0.1.1 across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [0.1.0] - 2026-02-15

### Added

- **Core Application**: Tauri v2 desktop application with Svelte 5 frontend and Rust backend.
- **Database**: SQLite with WAL mode, full schema with migrations and seed data.
- **Specimen Management**:
  - Create, read, update, and archive specimens.
  - Auto-generated accession numbers (YYYY-MM-DD-SPECIESCODE-SEQ format).
  - Full-text search across accession numbers, notes, location, and provenance.
  - Filter by species, stage, project, and quarantine status.
  - Pagination for large datasets.
  - QR code data generation per specimen.
  - Parent-child lineage tracking.
- **Species Registry**:
  - Pre-seeded species: Asparagus officinalis, Nandina domestica, Citrus sinensis, C. limon, C. paradisi, C. reticulata.
  - Configurable default subculture intervals per species.
  - Admin/supervisor species management.
- **Subculture History**:
  - Passage recording with auto-incrementing passage numbers.
  - Media batch linkage per subculture.
  - Vessel type tracking (15 pre-configured common vessel types).
  - Environmental conditions: temperature, pH, light cycle, humidity.
  - Transfer logging: location changes, environmental shifts.
  - Automatic specimen location update on transfer.
- **Media Logs**:
  - Media batch creation with auto-generated batch IDs (MB-YYYYMMDD-SEQ).
  - Support for MS and related basal salt formulations (MS, 1/2 MS, WPM, B5, N6, LS, White's, DKW).
  - Hormone tracking (auxins, cytokinins, gibberellins) with concentrations.
  - pH tracking (pre/post autoclave), volume management, expiration tracking.
  - Expired media visual warnings.
  - Supervisor review flag for custom recipes.
- **Compliance**:
  - Compliance record creation for disease tests, permits, certificates, inspections.
  - Agency tracking: USDA APHIS, TX Ag, FL FDACS.
  - Auto-flagging rules:
    - Expired permits.
    - Citrus HLB test compliance (12-month window).
    - Quarantine without release date.
    - Positive disease tests without quarantine.
- **Reminders**:
  - Custom and rule-based reminders with due dates.
  - Urgency levels: low, normal, high, critical.
  - Snooze with auto-escalation (critical after 2 snoozes).
  - Recurring reminder support.
  - Dashboard widget showing upcoming (7-day) reminders.
- **User Management**:
  - Role-based access: Admin, Supervisor, Tech, Guest.
  - bcrypt password hashing.
  - Session-based authentication.
  - Admin-only user creation and role management.
- **Audit Logging**:
  - Immutable audit trail for all create, update, delete, archive, and login actions.
  - Filterable by entity type, action, user, and date range.
  - Paginated log viewer.
- **Data Export**:
  - CSV export of all active specimens.
  - JSON export of all active specimens.
  - Client-side download via blob URL.
- **User Interface**:
  - Dashboard with stats overview, upcoming reminders, compliance alerts, and distribution charts.
  - Sidebar navigation with role-aware visibility.
  - Dark mode with system preference detection and manual toggle.
  - Keyboard shortcuts (Ctrl+1-4 for navigation).
  - Toast notifications with auto-dismiss.
  - Responsive table layouts with pagination.
- **Default Data**:
  - Default admin user (admin/admin).
  - Pre-configured hierarchical tags: Health, Disease, Growth, Issue, Contamination Type, Action Needed.
  - Pre-configured species with codes and subculture intervals.

### Notes

- This is the initial MVP release focused on core tracking functionality.
- SQLite is used as the database engine; PostgreSQL support is planned.
- The application runs as a local desktop app. Network/multi-user deployment requires PostgreSQL (future release).
- Default admin credentials should be changed immediately after first login.
