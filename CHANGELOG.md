# Changelog

All notable changes to SteloPTC will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
