# SteloPTC - Plant Tissue Culture Tracking System

A desktop application for tracking plant tissue culture specimens, designed for commercial and research laboratories. Built with Rust, Tauri, and Svelte for native performance and a modern UI.

## Overview

SteloPTC manages the full lifecycle of plant tissue culture specimens -- from initiation through subculture, acclimatization, and compliance reporting. It supports multi-user access with role-based permissions, regulatory compliance tracking (USDA APHIS, TX Ag, FL FDACS), and data export for statistical analysis.

### Key Features

- **Specimen Tracking**: Unique accession numbers (YYYY-MM-DD-SPECIESCODE-SEQ), provenance, lineage trees, health/disease status (0–4 color-coded slider), quarantine flags, and IP protection markers. Stages include explant, callus, shoot, shoot meristem, root, root meristem, embryogenic, plantlet, and more.
- **Structured Location Entry**: Specimen location is entered via Room / Rack / Shelf / Tray dropdowns with auto-populated last-used values for fast data entry.
- **Subculture History & Timeline**: Full passage logging displayed as a beautiful vertical timeline (newest first). Each card shows media batch, vessel, location, and environment at a glance; click to expand all fields. Supports **split culture** recording — one passage can create N new linked child specimens, each with a parent reference for full lineage tracking. A lineage banner on each specimen shows parent/child links as clickable chips for fast navigation between split cultures.
- **Media Logs**: Separate media batch database supporting MS and related formulations. Tracks basal salts (with auto-calculated g/L concentration from weight and volume), hormones (auxins/cytokinins), pH, sterilization, vessel count, and QC notes. Admin/supervisor edit support. Stock reagent traceability (lot numbers, amounts) recorded per batch.
- **Inventory Integration**: Media batch creation form shows a "+ Add Reagent" section linking directly to inventory items with lot numbers for full ingredient traceability. Supports both weighed-out and pre-made (commercial) basal salt solutions.
- **Compliance**: Built-in flagging rules (e.g., citrus HLB testing, expired permits, quarantine status). Tracks permits, disease tests, chain of custody, and agency-specific records for USDA APHIS, TX Ag, and FL FDACS.
- **Reminders**: User-configurable rules and calendar-based reminders with urgency levels, snooze/escalation, and recurring schedules.
- **User Roles**: Admin, Supervisor, Tech, Guest -- with granular permissions and full audit logging of all changes.
- **Inventory Management**: Full supply tracking with categories, stock levels, reorder alerts, and stock adjustments with audit trail. Unit field supports g, mg, mL, L, and other common units.
- **Database Backup**: On-demand database backup from the dashboard with WAL checkpointing for data integrity.
- **Export**: CSV and JSON export for analysis in R, Python, or SPSS.
- **Error Log**: Persistent, searchable error log (sidebar nav, all roles). Captures every application error with timestamp, severity (info/warning/error/critical), module, username, full message, form payload JSON, and stack trace. Expandable rows include Copy to Clipboard and Report on GitHub buttons. Sidebar badge shows live unread count. Error toasts are clickable and navigate directly to the log. All form submissions auto-capture the submitted payload on failure for instant reproducibility.
- **Dark Mode**: System-aware with manual toggle. Inter font for clean mixed-case rendering.
- **Keyboard Shortcuts**: Ctrl+1–5 for quick navigation (1 Dashboard, 2 Specimens, 3 Media, 4 Reminders, 5 Error Log).
- **Admin Dev Tools**: Admin-only "Reset Database" panel on the dashboard to wipe all operational data (preserves users/species) during development and setup. Requires typing `RESET DATABASE` to confirm.

### Species Supported

Pre-configured for asparagus, nandina, and citrus varieties. Any species can be added through the species registry.

## Tech Stack

| Layer     | Technology                                     |
|-----------|-------------------------------------------------|
| Backend   | Rust                                            |
| Framework | Tauri v2 (native desktop, cross-platform)       |
| Frontend  | Svelte 5, TypeScript, Vite                      |
| Database  | SQLite (bundled, WAL mode) -- PostgreSQL planned |
| Auth      | bcrypt password hashing, session tokens          |

## Requirements

### Build Requirements

- **Rust** 1.75+ (install via [rustup](https://rustup.rs))
- **Node.js** 18+ with npm
- **Tauri CLI** (`cargo install tauri-cli`)

### Platform-Specific

**Windows** (primary target):
- Visual Studio Build Tools 2022 with C++ workload
- WebView2 (included in Windows 10 1803+ and Windows 11)

**Linux**:
- `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`
- See [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

**macOS**:
- Xcode Command Line Tools
- CLang

**Android** (v0.1.11+):
- JDK 17
- Android SDK (API 34) + NDK r27
- Rust Android targets (installed automatically by `setup-android.sh`)
- `ANDROID_HOME`, `ANDROID_NDK_HOME` environment variables

## Getting Started

### 1. Clone and Install

```bash
git clone <repository-url>
cd SteloPTC
npm install
```

### 2. Development Mode

```bash
cargo tauri dev
```

This starts the Vite dev server and launches the Tauri window with hot-reload.

### 3. Build for Production

```bash
cargo tauri build --bundles msi
```

**Windows**: Produces a standalone `.exe` in `src-tauri/target/release/` and an `.msi` installer in `src-tauri/target/release/bundle/msi/`.

**Linux**: Produces `.deb` and `.AppImage` in `src-tauri/target/release/bundle/`.

> **CI Note**: The GitHub Actions workflow builds MSI only (skips NSIS) to avoid transient 502 errors from GitHub-hosted NSIS tool downloads. Both the standalone `.exe` and `.msi` installer are uploaded as build artifacts.

### 4. Build for Android

Use the automated setup script to install all prerequisites and initialise the Android project:

```bash
# Install prerequisites + init Android project
bash scripts/setup-android.sh

# Install prerequisites and build a debug APK in one step
bash scripts/setup-android.sh --build

# Install prerequisites and build a release APK
bash scripts/setup-android.sh --release
```

The script checks/installs:
- Rust Android targets (`aarch64-linux-android`, `armv7-linux-androideabi`, `i686-linux-android`, `x86_64-linux-android`)
- JDK 17 (via `apt`/`dnf`/`pacman`/`brew`)
- Android SDK command-line tools, `build-tools;34.0.0`, `platforms;android-34`
- Android NDK r27 (`27.2.12479018`)
- Tauri CLI v2

After setup, you can also build manually:

```bash
# Debug APK
cargo tauri android build

# Release APK (requires signing config)
cargo tauri android build --release
```

The APK is output to `src-tauri/gen/android/app/build/outputs/apk/universal/`.

**Android requirements:**
- Minimum SDK: API 24 (Android 7.0 Nougat)
- Target SDK: API 34 (Android 14)
- NDK: r27

> **Note**: `cargo tauri android init` must be run once before building. The setup script handles this automatically.

### Default Login

| Username | Password | Role  |
|----------|----------|-------|
| `admin`  | `admin`  | Admin |

**Change the default password immediately after first login.**

## Project Structure

```
SteloPTC/
├── src/                          # Svelte frontend
│   ├── App.svelte                # Main application shell
│   ├── main.ts                   # Entry point
│   └── lib/
│       ├── api.ts                # Tauri command bindings
│       ├── components/           # UI components
│       │   ├── Login.svelte
│       │   ├── Sidebar.svelte
│       │   ├── Dashboard.svelte
│       │   ├── SpecimenList.svelte
│       │   ├── SpecimenDetail.svelte
│       │   ├── SpecimenForm.svelte
│       │   ├── MediaList.svelte
│       │   ├── ReminderList.svelte
│       │   ├── ComplianceView.svelte
│       │   ├── SpeciesManager.svelte
│       │   ├── UserManager.svelte
│       │   ├── AuditLog.svelte
│       │   ├── InventoryManager.svelte
│       │   └── Notifications.svelte
│       └── stores/               # Svelte stores
│           ├── auth.ts
│           └── app.ts
├── src-tauri/                    # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs               # Entry point
│       ├── lib.rs                # App setup, command registration
│       ├── auth/                 # Authentication & sessions
│       ├── db/                   # Database, migrations, queries
│       ├── models/               # Data structures
│       └── commands/             # Tauri command handlers
│           ├── auth.rs
│           ├── specimens.rs
│           ├── media.rs
│           ├── subcultures.rs
│           ├── reminders.rs
│           ├── compliance.rs
│           ├── species.rs
│           ├── audit.rs
│           ├── export.rs
│           ├── inventory.rs
│           └── backup.rs
├── LICENSE                       # Commercial license
├── CHANGELOG.md
└── README.md
```

## Database

The application uses SQLite by default, stored at:

- **Windows**: `%APPDATA%\SteloPTC\stelo_ptc.db`
- **Linux/macOS**: `~/.steloptc/stelo_ptc.db`

### Schema Overview

| Table                | Purpose                                          |
|----------------------|--------------------------------------------------|
| `users`              | User accounts and roles                          |
| `sessions`           | Auth session tokens                              |
| `species`            | Master species registry with codes and intervals |
| `projects`           | Project/experiment groupings                     |
| `specimens`          | Core specimen records with accession numbers     |
| `tags`               | Hierarchical tag definitions                     |
| `specimen_tags`      | Tag assignments to specimens                     |
| `media_batches`      | Media preparation log                            |
| `media_hormones`     | Hormone details per media batch                  |
| `subcultures`        | Passage/subculture history                       |
| `attachments`        | File attachment metadata                         |
| `reminders`          | Scheduled reminders and rules                    |
| `compliance_records` | Regulatory test/permit/inspection records        |
| `inventory_items`    | Supply inventory with reorder alerts             |
| `audit_log`          | Immutable audit trail                            |

### Backup

Database backups can be created from the Dashboard (supervisor/admin only). Backups are stored in a `backups/` subdirectory alongside the database file. The backup process checkpoints the WAL first to ensure a complete, consistent copy. You can also manually copy the database file while the application is closed.

## User Roles

| Role       | Permissions                                                |
|------------|-------------------------------------------------------------|
| Admin      | Full CRUD, user management, role changes, species management, audit access |
| Supervisor | Oversight, approvals, reports, species management, audit access            |
| Tech       | Data entry, edit own specimens/subcultures, create media batches           |
| Guest      | View-only access to specimen summaries                                      |

## Compliance

SteloPTC includes auto-flagging rules for regulatory compliance:

- **Expired permits**: Flags specimens with past-due permit expiry dates.
- **Citrus HLB testing**: Flags citrus specimens (CIT-* species codes) missing HLB test results within the last 12 months.
- **Quarantine without release**: Flags quarantined specimens with no scheduled release date.
- **Positive tests without quarantine**: Flags specimens with positive disease test results that are not quarantined.

Additional compliance rules can be added by extending `src-tauri/src/commands/compliance.rs`.

## Roadmap

- [x] Android mobile support (v0.1.11) — slide-out navigation, responsive layout, touch targets, APK build pipeline
- [x] Inventory management with reorder alerts
- [x] Database backup (on-demand from dashboard)
- [x] Vertical passage timeline — scrollable history with collapsible detail cards, newest-first, replacing the old flat table
- [x] Split culture recording — one passage creates N linked child specimens with `parent_specimen_id` for full lineage tracking
- [x] Lineage banner — parent/child chips on each specimen for one-click navigation across splits
- [ ] PostgreSQL support for multi-user LAN deployment
- [ ] QR / barcode generation and webcam scanning for specimen containers
- [ ] Print label workflow — generate QR labels formatted for lab label printers
- [ ] Photo attachments with direct camera capture and per-passage image logging
- [ ] Contamination tracking — per-vessel flags, contamination notes, and lab-wide contamination rate statistics
- [ ] Subculture scheduling — due-date forecasting based on species interval with overdue alerts
- [ ] Interactive lab map with floor plan overlay and specimen location heat-map
- [ ] Excel multi-sheet import/export
- [ ] PDF report generation (export certificates, inspection logs)
- [ ] Batch operations — bulk passage, bulk location transfer, bulk status update across selected specimens
- [ ] Species-level analytics — growth curves, passage success rates, media comparison charts
- [ ] Environmental monitoring integration — link temp/humidity sensor readings to passage records
- [ ] Email / push notifications for reminders and overdue subcultures
- [ ] Local AI analysis (NLP note summaries, image contamination detection)
- [ ] Offline mode with LAN sync

## License

This software is proprietary. See [LICENSE](./LICENSE) for the full commercial license agreement. Contact licensing@stelolab.local for purchasing information.
