# Changelog

All notable changes to SteloPTC will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
