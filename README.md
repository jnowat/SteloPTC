# SteloPTC — Plant Tissue Culture Tracking System

A desktop and Android application for tracking plant tissue culture specimens in commercial and research laboratories. Built with Rust, Tauri v2, and Svelte 5 for native performance on Windows, Linux, macOS, and Android.

---

## Downloads

> Builds are produced automatically by GitHub Actions on every push and GitHub Release — no local toolchain required to get a binary.

| Platform | Artifact | Notes |
|---|---|---|
| **Windows** | [Latest Release →](../../releases/latest) | `.msi` installer + standalone `.exe` |
| **Android** | [Latest Actions run →](../../actions/workflows/build-android.yml) | Grab the `SteloPTC-Android-Debug` artifact |

On every **GitHub Release**, both the Windows MSI and the Android APK are attached directly to the release assets. Android release APKs are signed with the repository keystore secrets (or fall back to debug signing if secrets are not configured — the APK still installs fine via ADB).

### Android APK

The debug APK is built on **every push** and available as a workflow artifact for 30 days. Release APKs are attached to [GitHub Releases](../../releases).

**Steps to download the latest debug APK:**

1. Go to [Actions → Build Android APK](../../actions/workflows/build-android.yml)
2. Click the most recent passing (green) run
3. Scroll to the **Artifacts** section
4. Click **SteloPTC-Android-Debug** to download the `.apk`

**Install on Android:**

1. Transfer the `.apk` to your device (USB, email, cloud storage, etc.)
2. Open the file — Android will prompt to enable *Install unknown apps*
3. Allow installation from this source and proceed
4. Launch **SteloPTC** from your app drawer

> **Requirements:** Android 7.0 (API 24) or later. The APK contains native libraries for all supported architectures (arm64-v8a, armeabi-v7a, x86, x86_64).

---

## Overview

SteloPTC manages the full lifecycle of plant tissue culture specimens — from initiation through subculture, acclimatization, and compliance reporting. It supports multi-user access with role-based permissions, regulatory compliance tracking (USDA APHIS, TX Ag, FL FDACS), and data export for statistical analysis.

### Feature Summary

- **Specimen Tracking** — Unique accession numbers (YYYY-MM-DD-SPECIESCODE-SEQ), provenance, lineage trees (split culture parent/child), health/disease status (0–4 color-coded slider with "Unknown/Awaiting" option), quarantine flags, and IP protection markers. Stages: explant, callus, shoot, shoot meristem, apical meristem, root, root meristem, embryogenic, plantlet.
- **Structured Location Entry** — Room / Rack / Shelf / Tray dropdowns auto-populated with last-used values for fast data entry.
- **Subculture History & Timeline** — Full passage log displayed as a vertical timeline (newest first). Each card shows media batch, vessel, location, and environment; click to expand. Supports **split culture** — one passage creates N linked child specimens with full lineage tracking. A lineage banner shows parent/child chips as clickable navigation links.
- **Contamination Tracking** — Per-passage contamination flag and notes. Dashboard **Contamination Overview** panel shows lab-wide rate (%), affected specimens, vessel-type breakdown, and the 10 most recent events (v0.1.15).
- **Subculture Scheduling** — Dashboard **Subculture Schedule** widget lists overdue and due-within-7-days specimens by species interval with day counts and direct links (v0.1.15).
- **Media Logs** — Batch database supporting MS and related formulations (MS, 1/2 MS, WPM, B5, N6, LS, White's, DKW). Tracks basal salts (auto-calculated g/L from weight + volume), hormones (auxins/cytokinins/gibberellins) with concentrations, pH, sterilization, vessel count, QC notes, and expiration. Stock reagent traceability with lot numbers and auto-depletion from inventory on batch creation.
- **Inventory Management** — Full supply tracking with category organization (media ingredients, vessels, hormones, chemicals, consumables, equipment). Stock levels, reorder thresholds, physical state (solid/liquid with concentration units), stock adjustments with audit trail, low-stock dashboard alerts, and expiration tracking.
- **Prepared Stock Solutions** — Track stock solutions made from solid reagents: source item, concentration, volume prepared/remaining, prep date, preparer, and inline volume updates.
- **QR Codes** — Per-specimen QR code generation (256×256, Error Correction M), 2×3-inch print labels for lab label printers, camera-based scanning (rear camera on Android, webcam on desktop), and scan event logging in SQLite (v0.1.14+).
- **Compliance** — Auto-flagging rules for expired permits, citrus HLB testing, quarantine without release date, and positive tests without quarantine. Agency tracking: USDA APHIS, TX Ag, FL FDACS.
- **Reminders** — User-configurable rules and calendar reminders with urgency levels (low/normal/high/critical), snooze with auto-escalation after 2 snoozes, recurring support, and a 7-day upcoming dashboard widget.
- **Error Log** — Persistent, searchable error tracking (all roles). Every error captured with severity badge, module, username, form payload JSON, and stack trace. Sidebar badge shows live unread count; toasts are clickable and navigate directly to the log (v0.1.10+).
- **User Management & Audit** — Roles: Admin, Supervisor, Tech, Guest. bcrypt password hashing. Immutable audit trail for all create/update/delete/archive/login actions, filterable by entity, action, user, and date range.
- **Photo Attachments** — Attach images directly to specimen records. Upload via OS file picker (desktop) or rear camera (Android). Responsive gallery grid with lightbox viewer and in-memory thumbnail cache. Images stored on disk under `<appDataDir>/attachments/`.
- **Export & Backup** — Dedicated Export Data page with Excel (`.xlsx`) multi-sheet workbook (Specimens, Subcultures, Media Batches, Prepared Solutions, Inventory, Compliance), plus CSV and JSON. On-demand database backup from the dashboard (supervisor/admin) with WAL checkpointing.
- **Mobile-First UI** — Hamburger + slide-out drawer on all screens < 1024 px, 48 px touch targets (WCAG 2.5.5), safe-area insets for notches and home indicators (v0.1.11+).
- **Keyboard Shortcuts** — Ctrl+1–5: Dashboard, Specimens, Media, Reminders, Error Log.
- **Contextual Tooltips** — "?" badge on every form field and action button with help text (v0.1.15).
- **Dark Mode** — System-aware with manual toggle. Inter font throughout.

### Species Registry

Pre-configured for asparagus, nandina, and citrus varieties. Any species can be added through the admin species manager.

---

## QR Codes (v0.1.14+)

| Action | Where | How |
|---|---|---|
| **Generate QR** | Specimen List row · Specimen Detail header | Click `⬡ QR` to open a modal with the QR image |
| **Print Label** | Inside QR modal | Opens a 2×3 inch print window optimised for label printers |
| **Download PNG** | Inside QR modal | Saves `QR-{accession}.png` locally |
| **Scan QR** | Specimens list header · Specimen Detail header | Opens the device camera to decode any SteloPTC QR and navigate to the specimen |

**QR payload format** (JSON, Error Correction M):
```json
{
  "app": "SteloPTC",
  "accession": "2026-02-15-CIT-SIN-001",
  "species": "CIT-SIN",
  "stage": "shoot",
  "location": "Room 2 / Rack B / Shelf 3 / Tray A",
  "id": "uuid-of-specimen"
}
```

All scan events are stored in the `qr_scans` SQLite table with raw data, accession number, scanned-by user, and timestamp.

---

## Tech Stack

| Layer     | Technology                                      |
|-----------|-------------------------------------------------|
| Backend   | Rust 1.75+                                      |
| Framework | Tauri v2 (native desktop + Android mobile)      |
| Frontend  | Svelte 5, TypeScript, Vite 6                    |
| Database  | SQLite (bundled, WAL mode)                      |
| Auth      | bcrypt password hashing, session tokens         |
| Mobile    | Android 7.0+ (API 24–35), Tauri 2 mobile        |
| QR Codes  | qrcode 1.5.4 (generation), html5-qrcode 2.3.8 (scanning) |
| Excel     | xlsx 0.18.5 (SheetJS — multi-sheet workbook export)      |

---

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
- Xcode Command Line Tools + CLang

**Android** (v0.1.11+):
- JDK 17
- Android SDK (API 35) + NDK r27 (27.2.12479018)
- Rust Android targets (installed automatically by `setup-android.sh`)
- `ANDROID_HOME` and `ANDROID_NDK_HOME` environment variables

---

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

Starts the Vite dev server and launches the Tauri window with hot-reload.

### 3. Build for Production

```bash
cargo tauri build --bundles msi
```

- **Windows**: `.exe` in `src-tauri/target/release/` and `.msi` in `src-tauri/target/release/bundle/msi/`.
- **Linux**: `.deb` and `.AppImage` in `src-tauri/target/release/bundle/`.

> **CI Note**: The GitHub Actions workflow builds MSI only (`--bundles msi`) to avoid transient NSIS download failures. Both the standalone `.exe` and `.msi` are uploaded as build artifacts.

### 4. Build for Android

1. Run once: `./scripts/setup-android.sh`
2. The Android project is already committed — no `cargo tauri android init` needed.
3. Build debug APK: `npm run android:build-debug`
4. Build release APK: `npm run android:build`

First build downloads Gradle + SDK (~5–10 min). Output APK: `src-tauri/gen/android/app/build/outputs/apk/`.

```bash
# Live-reload on a connected device/emulator
npm run android:dev

# Debug APK (unsigned, sideloadable)
npm run android:build-debug

# Release APK (requires signing env vars)
npm run android:build
```

**Release signing environment variables:**

| Variable | Description |
|---|---|
| `ANDROID_KEY_STORE_PATH` | Path to your `.jks` keystore file |
| `ANDROID_KEY_STORE_PASSWORD` | Keystore password |
| `ANDROID_KEY_ALIAS` | Key alias inside the keystore |
| `ANDROID_KEY_PASSWORD` | Key password |

**Android requirements:**
- Minimum SDK: API 24 (Android 7.0)
- Target SDK: API 35 (Android 15)
- NDK: r27 (27.2.12479018)

### Customizing Android Config

`targetSdk` and `ndkVersion` are not valid Tauri v2 `tauri.conf.json` properties — set them in `src-tauri/gen/android/app/build.gradle.kts`:

```kotlin
android {
    compileSdk = 35
    ndkVersion = "27.2.12479018"
    defaultConfig {
        targetSdk = 35
        minSdk = 24
        versionCode = 15
        versionName = "0.1.15"
    }
}
```

### Default Login

| Username | Password | Role  |
|----------|----------|-------|
| `admin`  | `admin`  | Admin |

**Change the default password immediately after first login.**

---

## Project Structure

```
SteloPTC/
├── src/                              # Svelte 5 frontend
│   ├── App.svelte                    # Main app shell with routing and dark mode
│   ├── main.ts                       # Entry point — mounts app, restores session
│   └── lib/
│       ├── api.ts                    # Typed Tauri command bindings
│       ├── components/
│       │   ├── Login.svelte
│       │   ├── Dashboard.svelte      # Stats, schedule, contamination, reminders
│       │   ├── Sidebar.svelte        # Navigation with hamburger drawer on mobile
│       │   ├── SpecimenList.svelte   # Search, filter, QR scan entry point
│       │   ├── SpecimenDetail.svelte # Passage timeline, lineage, QR, record passage
│       │   ├── SpecimenForm.svelte   # New/edit specimen form
│       │   ├── MediaList.svelte      # Media batch CRUD
│       │   ├── ReminderList.svelte   # Reminder management
│       │   ├── ComplianceView.svelte # Compliance records and auto-flagging
│       │   ├── SpeciesManager.svelte # Species registry
│       │   ├── UserManager.svelte    # User accounts and roles
│       │   ├── AuditLog.svelte       # Immutable audit trail viewer
│       │   ├── ErrorLog.svelte       # Error tracking with payload capture
│       │   ├── InventoryManager.svelte # Supply inventory CRUD
│       │   ├── QrModal.svelte        # QR generation, print label, download
│       │   ├── QrScanner.svelte      # Camera-based QR scanning
│       │   ├── ExportManager.svelte  # Excel/CSV/JSON export hub
│       │   ├── Tooltip.svelte        # Reusable "?" contextual help badge
│       │   └── Notifications.svelte  # Toast notification renderer
│       └── stores/
│           ├── auth.ts               # Auth state, session restore
│           └── app.ts                # Notifications, error logger
│
├── src-tauri/                        # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs
│       ├── lib.rs                    # App setup, command registration
│       ├── auth/mod.rs               # bcrypt + session management
│       ├── db/
│       │   ├── mod.rs                # Connection pool, init
│       │   ├── migrations.rs         # 5 schema migrations
│       │   └── queries.rs            # SQL helpers
│       ├── models/                   # Rust data structures
│       │   ├── user.rs, specimen.rs, media.rs
│       │   ├── subculture.rs, species.rs, reminder.rs
│       │   ├── compliance.rs, inventory.rs
│       │   ├── audit.rs, error_log.rs
│       │   └── mod.rs
│       └── commands/                 # Tauri command handlers
│           ├── auth.rs, specimens.rs, media.rs
│           ├── subcultures.rs        # Passages, contamination stats, schedule
│           ├── reminders.rs, compliance.rs
│           ├── species.rs, audit.rs
│           ├── error_logs.rs, export.rs
│           ├── inventory.rs, backup.rs
│           ├── qr_scans.rs, admin.rs
│           ├── attachments.rs            # Photo attach/fetch/delete
│           └── mod.rs
│
├── .github/workflows/
│   ├── build-windows.yml             # MSI + exe on push and release
│   └── build-android.yml             # APK on push and release
│
├── scripts/
│   └── setup-android.sh              # Android build prerequisite installer
│
├── CHANGELOG.md
├── README.md
└── LICENSE
```

---

## Database

SQLite, stored at:

- **Windows**: `%APPDATA%\SteloPTC\stelo_ptc.db`
- **Linux/macOS**: `~/.steloptc/stelo_ptc.db`
- **Android**: internal app storage (managed by the OS)

### Schema

| Table                | Purpose                                                   |
|----------------------|-----------------------------------------------------------|
| `users`              | User accounts and roles                                   |
| `sessions`           | Auth session tokens                                       |
| `species`            | Master species registry with codes and subculture intervals |
| `projects`           | Project/experiment groupings                              |
| `specimens`          | Core specimen records with accession numbers              |
| `tags`               | Hierarchical tag definitions                              |
| `specimen_tags`      | Tag assignments to specimens                              |
| `media_batches`      | Media preparation log with batch IDs                      |
| `media_hormones`     | Hormone details per media batch                           |
| `subcultures`        | Passage history with contamination flags and notes        |
| `prepared_solutions` | Stock solutions prepared from solid reagents              |
| `attachments`        | File attachment metadata                                  |
| `reminders`          | Scheduled reminders and rules                             |
| `compliance_records` | Regulatory tests, permits, inspections                    |
| `inventory_items`    | Supply inventory with reorder alerts                      |
| `audit_log`          | Immutable audit trail                                     |
| `error_logs`         | Persistent error tracking with form payloads              |
| `qr_scans`           | QR scan events with timestamp and user                    |

### Migrations

| # | Applied in | Changes |
|---|---|---|
| 001 | v0.1.0 | Initial schema — all core tables |
| 002 | v0.1.9 | Extended stages (meristem), employee IDs, inventory physical state, prepared_solutions |
| 003 | v0.1.10 | Fixed specimen stage CHECK constraint; added error_logs table |
| 004 | v0.1.14 | Added qr_scans table |
| 005 | v0.1.15 | Added contamination_flag and contamination_notes to subcultures |

### Backup

On-demand backup from the Dashboard (supervisor/admin only). Backups are stored in a `backups/` subdirectory alongside the database file, with timestamped filenames. The process checkpoints the WAL first to ensure a consistent copy.

---

## User Roles

| Role       | Permissions                                                                   |
|------------|-------------------------------------------------------------------------------|
| Admin      | Full CRUD, user management, role changes, species management, audit access, dev tools |
| Supervisor | Oversight, approvals, reports, species management, audit access, backup, export |
| Tech       | Data entry, edit own specimens/subcultures, create media batches, inventory adjustments |
| Guest      | View-only access to specimen summaries                                         |

---

## Compliance

Built-in auto-flagging rules:

- **Expired permits** — flags specimens with past-due permit expiry dates.
- **Citrus HLB testing** — flags CIT-* specimens missing an HLB test within the last 12 months.
- **Quarantine without release** — flags quarantined specimens with no scheduled release date.
- **Positive tests without quarantine** — flags specimens with positive disease results not under quarantine.

Additional rules can be added in `src-tauri/src/commands/compliance.rs`.

---

## Roadmap

### v0.1.x — Completed

- [x] Core specimen tracking, subculture history, media logs, compliance, reminders (v0.1.0)
- [x] Inventory management with reorder alerts (v0.1.2)
- [x] On-demand database backup from dashboard (v0.1.2)
- [x] Stable release builds on Windows (MSI/exe via GitHub Actions) (v0.1.3–v0.1.5)
- [x] Health status 0–4 slider, structured location entry (Room/Rack/Shelf/Tray) (v0.1.6)
- [x] Inter font, media batch edit, basal salts auto-calculator, database reset (v0.1.7)
- [x] Vertical passage timeline, split culture, lineage banner (v0.1.8)
- [x] Apical meristem stage, employee IDs, developer mode, prepared stock solutions, stock depletion on media creation (v0.1.9)
- [x] Specimen stage CHECK constraint fix, persistent error log system with form payload capture (v0.1.10)
- [x] Android mobile support — hamburger/drawer nav, touch targets, safe-area insets (v0.1.11)
- [x] Full Android APK build (committed project, CI/CD pipeline) (v0.1.12–v0.1.13)
- [x] QR code generation, 2×3-inch print labels, camera scanning, scan logging (v0.1.14)
- [x] Contamination tracking per passage, subculture scheduling with overdue alerts, dashboard panels (v0.1.15)
- [x] Contextual "?" tooltips on all form fields and action buttons (v0.1.15)
- [x] Batch operations — multi-select checkboxes on Specimens list; bulk Transfer Location, Update Stage, and Archive with per-specimen audit logging (v0.1.16)
- [x] PDF report generation — Culture Certificate (specimen detail + full passage history + compliance) and Specimens Summary (filtered list view); print-ready via browser print API (v0.1.17)
- [x] Excel workbook export — six-sheet `.xlsx` file (Specimens, Subcultures, Media Batches, Prepared Solutions, Inventory, Compliance) via SheetJS; dedicated Export Data page in sidebar (v0.1.18)
- [x] Photo attachments — upload images per specimen via OS file picker or Android rear camera; gallery grid with lightbox viewer, delete, and in-memory cache; stored on disk under appDataDir (v0.1.19)

### v0.1.x — Upcoming Patches

- [ ] **Excel import** — parse `.xlsx` workbooks to create or update specimens and subculture records
- [ ] **Interactive lab map** — floor plan overlay with specimen location heat-map and drag-to-move

### v0.2.x — Multi-User & Network

- [ ] **PostgreSQL backend** — drop-in replacement for the SQLite connection for LAN/server deployments with concurrent multi-user writes
- [ ] **Network sync** — real-time specimen and inventory updates across multiple desktop and Android clients on the same LAN
- [ ] **Email / push notifications** — reminder delivery and overdue subculture alerts via SMTP or push service
- [ ] **Environmental monitoring integration** — link temperature/humidity sensor readings directly to passage records
- [ ] **iOS support** — Tauri 2 iOS target with the same responsive UI as Android
- [ ] **Role-based field-level permissions** — hide or lock sensitive fields (IP flags, provenance) by role

### v0.3.x and Beyond

- [ ] **Species-level analytics** — growth curves, passage success rates, and media comparison charts across experiments
- [ ] **Local AI analysis** — NLP summaries of observation notes; image-based contamination detection from passage photos
- [ ] **Offline-first with sync** — full local operation with background sync when a server is available
- [ ] **Web deployment** — optional web frontend for read-only dashboards and report sharing
- [ ] **Protocol templates and SOPs** — attach standard operating procedure documents to species and media recipes

---

## License

This software is proprietary. See [LICENSE](./LICENSE) for the full commercial license agreement. Contact licensing@stelolab.local for purchasing information.
