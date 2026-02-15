# SteloPTC - Plant Tissue Culture Tracking System

A desktop application for tracking plant tissue culture specimens, designed for commercial and research laboratories. Built with Rust, Tauri, and Svelte for native performance and a modern UI.

## Overview

SteloPTC manages the full lifecycle of plant tissue culture specimens -- from initiation through subculture, acclimatization, and compliance reporting. It supports multi-user access with role-based permissions, regulatory compliance tracking (USDA APHIS, TX Ag, FL FDACS), and data export for statistical analysis.

### Key Features

- **Specimen Tracking**: Unique accession numbers (YYYY-MM-DD-SPECIESCODE-SEQ), provenance, lineage trees, health/disease status, quarantine flags, and IP protection markers.
- **Subculture History**: Full passage logging with media linkage, vessel tracking, environmental conditions, transfer records, and per-passage observations.
- **Media Logs**: Separate media batch database supporting MS and related formulations. Tracks basal salts, hormones (auxins/cytokinins), pH, sterilization, volumes, and QC notes.
- **Compliance**: Built-in flagging rules (e.g., citrus HLB testing, expired permits, quarantine status). Tracks permits, disease tests, chain of custody, and agency-specific records for USDA APHIS, TX Ag, and FL FDACS.
- **Reminders**: User-configurable rules and calendar-based reminders with urgency levels, snooze/escalation, and recurring schedules.
- **User Roles**: Admin, Supervisor, Tech, Guest -- with granular permissions and full audit logging of all changes.
- **Export**: CSV and JSON export for analysis in R, Python, or SPSS.
- **Dark Mode**: System-aware with manual toggle.
- **Keyboard Shortcuts**: Ctrl+1-4 for quick navigation.

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
cargo tauri build
```

**Windows**: Produces an `.exe` installer and standalone binary in `src-tauri/target/release/`.

**Linux**: Produces `.deb` and `.AppImage` in `src-tauri/target/release/bundle/`.

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
│           └── export.rs
├── LICENSE                       # Commercial license
├── CHANGELOG.md
└── README.md
```

## Database

The application uses SQLite by default, stored at:

- **Windows**: `%APPDATA%\SteloPTC\stelo_ptc.db`
- **Linux/macOS**: `~/.stelobtc/stelo_ptc.db`

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
| `inventory_items`    | Basic supply inventory                           |
| `audit_log`          | Immutable audit trail                            |

### Backup

The database file can be backed up by copying it while the application is closed. WAL mode ensures read consistency during operation. Automated backup to network shares is planned for a future release.

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

- [ ] PostgreSQL support for multi-user LAN deployment
- [ ] QR code generation and webcam scanning
- [ ] Photo attachments with direct camera capture
- [ ] Visual Gantt timeline for subculture history
- [ ] Interactive lab map with floor plan overlay
- [ ] Excel multi-sheet import/export
- [ ] PDF report generation (export certificates, inspection logs)
- [ ] Local AI analysis (NLP note summaries, image contamination detection)
- [ ] Inventory management with reorder alerts
- [ ] Offline mode with LAN sync
- [ ] Automated nightly backups

## License

This software is proprietary. See [LICENSE](./LICENSE) for the full commercial license agreement. Contact licensing@stelolab.local for purchasing information.
