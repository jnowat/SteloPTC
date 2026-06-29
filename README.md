# SteloPTC — Plant Tissue Culture Tracking System

A desktop and Android application for tracking plant tissue culture specimens in commercial and research laboratories. Built with Rust, Tauri v2, and Svelte 5 for native performance on Windows, Linux, macOS, and Android.

---

## Downloads

> Builds are produced automatically by GitHub Actions on every push and GitHub Release — no local toolchain required to get a binary.

| Platform | Artifact | Notes |
|---|---|---|
| **Windows** | [Latest Release →](../../releases/latest) | `.msi` installer attached to every GitHub Release |
| **Android** | [Latest Release →](../../releases/latest) | Release-signed `.apk` attached to every GitHub Release |
| **Android (debug)** | [Latest Actions run →](../../actions/workflows/build-android.yml) | `SteloPTC-Android-Debug` artifact — every push, 30-day retention |

On every **GitHub Release**, both the Windows MSI and the Android release APK are attached directly to the release assets. The Android release APK is signed with the project release keystore (see `.github/SIGNING.md`) — **not** debug-signed — so it supports in-place upgrades on Android.

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

- **Specimen Tracking** — Unique accession numbers (YYYY-MM-DD-SPECIESCODE-SEQ), provenance, lineage trees (split culture parent/child), health/disease status (0–4 color-coded slider with "Unknown/Awaiting" option), quarantine flags, and IP protection markers. Stages: explant, callus, shoot, shoot meristem, apical meristem, root, root meristem, embryogenic, plantlet. Each specimen tracks its **generation depth** (Gen N badge in the detail header) and **cumulative passage count from root** (`lineage_passage_offset + subculture_count`), enabling precise genealogical history across any number of splits. A `root_specimen_id` column links every derived specimen back to its absolute ancestor for efficient family-tree queries (v1.7.0). **Population Doubling Level (PDL)** is optionally tracked per passage via seed/harvest cell counts or split ratio; cumulative PDL is inherited by children at split time and displayed in the specimen info card (v1.24.0).
- **Structured Location Entry** — Room / Rack / Shelf / Tray dropdowns auto-populated with last-used values for fast data entry.
- **Subculture History & Timeline** — Full passage log displayed as a vertical timeline (newest first). Each card shows media batch, vessel, location, and environment; click to expand. **Splitting** is handled by an atomic `split_specimen` backend command: the parent is archived, a "split" event is appended to its chain, and all children are created in a single SQLite transaction — no partial state is possible. Split children receive **letter-suffix accessions** (`001` → `001A`, `001B`; recursive splits chain the letter: `001A` → `001AA`). Each child has its own configuration card (health slider, stage, location, media batch, vessel type, notes, and optional check-in reminder). A **safety confirmation dialog** lists all proposed children with accessions before the split executes. **Draft media batches** can be created inline during the split and completed later in Media Management. Synthetic "Split from" / "Split into N children" cards appear in the passage timeline (purple dashed style). The lineage banner shows parent, all children, and **siblings** (including archived ones with strikethrough) as clickable chips. A **navigation history stack** lets you press Back to return to the prior specimen after navigating a lineage chip. A **"Gen N" badge** in the detail header shows the generational depth of any derived specimen. Each **passage event** extends the specimen's own cryptographic lineage chain (`chain_seq` increments within the specimen's lineage), so the audit history of a specimen and its passage history are a single continuous, verifiable sequence (v1.6.4, v1.7.0, v1.8.0).
- **Contamination Tracking** — Per-passage contamination flag and notes. Dashboard **Contamination Overview** panel shows lab-wide rate (%), affected specimens, vessel-type breakdown, contaminant-type breakdown (mycology), and the 10 most recent events; all figures are scoped to the active lab profile. In the mycology profile, a **Contaminant Type** dropdown (Trichoderma, Wet Rot / Bacterial, Cobweb Mold, Pin Mold, Mycelium Abort, Other) can be set per passage; the type is shown in the passage timeline badge and contamination dashboard (v0.1.15, v1.13.0, v1.29.0).
- **Subculture Scheduling** — Dashboard **Subculture Schedule** widget lists overdue and due-within-7-days specimens by species interval with day counts and direct links; only specimens whose stage belongs to the active lab profile are included (v0.1.15, v1.13.0).
- **Work Queue** — Daily task view listing every specimen that needs immediate attention. Detects five conditions: subculture due/overdue (per species interval), media batch expired, contamination flag on most recent passage, no passages recorded, and unresolved quarantine. Items are sorted by urgency (Critical → High → Normal) then by days overdue. The sidebar shows an amber count badge when items are pending. Read-only in v1.2.0 — click any row to open the specimen and take action.
- **Consistent Loading & Empty States** — All list views (Specimens, Media, Inventory, Reminders, Compliance, Audit Log, Error Log) display an animated shimmer skeleton while data loads and a friendly, icon-led empty state with a contextual call-to-action when there is no data to show (v1.2.1, v1.2.3).
- **Media Logs** — Batch database supporting MS and related formulations (MS, 1/2 MS, WPM, B5, N6, LS, White's, DKW). Tracks basal salts (auto-calculated g/L from weight + volume), hormones (auxins/cytokinins/gibberellins) with concentrations, pH, sterilization, vessel count, QC notes, and expiration. Stock reagent traceability with lot numbers and auto-depletion from inventory on batch creation.
- **Inventory Management** — Full supply tracking with category organization (media ingredients, vessels, hormones, chemicals, consumables, equipment). Stock levels, reorder thresholds, physical state (solid/liquid with concentration units), stock adjustments with audit trail, low-stock dashboard alerts, and expiration tracking.
- **Prepared Stock Solutions** — Track stock solutions made from solid reagents: source item, concentration, volume prepared/remaining, prep date, preparer, and inline volume updates.
- **QR Codes** — Per-specimen QR code generation (256×256, Error Correction M), 2×3-inch print labels for lab label printers, camera-based scanning (rear camera on Android, webcam on desktop), and scan event logging in SQLite. The scanner validates the payload: non-SteloPTC codes (URLs, plain text, vCards) show a distinct warning and suppress the "Open Specimen" action while still recording the scan event for audit (v0.1.14+, v1.1.1).
- **Compliance** — Auto-flagging rules for expired permits, citrus HLB testing, quarantine without release date, and positive tests without quarantine. Agency tracking: USDA APHIS, TX Ag, FL FDACS.
- **Reminders** — User-configurable rules and calendar reminders with urgency levels (low/normal/high/critical), snooze with auto-escalation after 2 snoozes, recurring support, and a 7-day upcoming dashboard widget.
- **Error Log** — Persistent, searchable error tracking (all roles). Every error captured with severity badge, module, username, form payload JSON, and stack trace. Sidebar badge shows live unread count; toasts are clickable and navigate directly to the log (v0.1.10+).
- **Settings** — Admin-only Settings view (sidebar gear icon) for configuring lab-wide options. Currently: lab profile switcher (`plant_tissue_culture | cell_culture | mycology`) with a warning banner and mandatory `CHANGE PROFILE` confirmation when data already exists; profile change updates the Svelte store immediately so all components react without restart. All three profiles (`plant_tissue_culture`, `cell_culture`, and `mycology`) have fully seeded vocabulary (stages, propagation methods, hormone types / substrate supplements, compliance types, agencies, and inventory categories); the `cell_culture` profile includes an expanded stage set covering common lifecycle states (thawed, adherent, confluent, passaged, cryopreserved, contaminated, discarded) in addition to phase-based stages; the `mycology` profile covers the full mushroom cultivation lifecycle (spore/clone → agar → liquid culture → grain spawn → bulk substrate → colonizing → fruiting → senescent, with contaminated and discarded as terminal stages); mycology-specific colonization % and contaminant type fields are available in the passage form (v1.14.0, v1.15.0, v1.23.0, v1.28.0, v1.29.0).
- **User Management & Audit** — Roles: Admin, Supervisor, Tech, Guest. bcrypt password hashing. Append-only audit trail for all create/update/delete/archive/login actions, filterable by entity, action, user, and date range. Every audit entry is **cryptographically hash-chained** (see Cryptographic Audit Chain below).
- **Cryptographic Audit Chain** — Every audit entry carries a SHA-256 `entry_hash` computed over its canonical content plus the previous entry's hash, forming a **per-lineage append-only chain** (v1.5.0–v1.6.0). Species creation anchors a chain at seq 0; each new root specimen seeds its chain from its species' last hash; split children inherit the parent's last `entry_hash` as `prev_hash`, making fork points cryptographically unambiguous. The Audit Log UI shows chain columns (`#`, Prev Hash, Entry Hash) with truncated display and full-hash tooltips; each chained row has **Row** and **Chain** verify buttons that re-compute hashes on demand and report the first broken link. A chain-integrity banner shows chained vs. legacy entry counts at a glance (v1.5.1, v1.6.0). **Merkle Checkpoints (v1.9.0)** — a sealed range of any lineage can be committed to a single Merkle root stored in `audit_checkpoints`; three-stage verification (count → Merkle root → per-entry content hash) catches deletions, hash tampering, and content-only edits. See [`docs/merkle-checkpoints.md`](docs/merkle-checkpoints.md) for the full specification. **Phase TX extends this chain downward:** strain creation is seeded from the species hash; strain-bound specimen creation is seeded from the strain hash, creating an unbroken cryptographic path Species → Strain → Specimen (v1.9.0 target).
- **Strain & Cultivar Registry** *(v1.17.0)* — Named Strains or Cultivars under each species, each with its own hash chain seeded from the parent species hash. Four-value status model: `Unverified` (default) → `Claimed` (explicit assertion, no friction) → `Confirmed — Manual` (documented basis required + blocking acknowledgment modal; `⚠ Manual ID` badge permanent) → `Confirmed — Genomic` (fingerprint data required; gold standard). Status downgrades permanently rejected. Hybrid strains created via dedicated 8-step wizard with bidirectional `used_as_parent` audit entries on both parent chains. Specimens cryptographically version-bound to a specific strain state at creation (`strain_chain_seq`). Strain pill shown in specimen detail header showing `[CODE · v{chain_seq} · STATUS]`. Accession numbers never encode strain — final design decision.
- **Taxonomy Navigator** *(v1.17.0 → v1.22.0)* — **Advanced multi-column browser** (Kingdom → Phylum → Class → Order → Family → Genus → Species → Strains), each column independently scrollable; 5–6 columns visible on desktop, horizontal-scroll on mobile. Every node shows aggregated descendant counts (`N strains · M specimens`) computed on the backend via correlated `taxon_path` subqueries. **Global search** (300 ms debounced) finds taxa, species, strains, and specimen accessions with grouped dropdown results that jump directly to the matching node. **Keyboard navigation**: Arrow keys move focus across and within columns, Enter selects, Escape closes panel or resets to root, `/` focuses search. **Strain quick-action panel** opens inline when a strain is selected, listing live specimen rows (stage, health, accession) with click-through to Specimen Detail. StrainDetail slide-over integration for full strain inspection. **localStorage path persistence** restores the last-selected taxon/species/strain across sessions. Strain status filter (active / archived / all) and per-kingdom quick-jump buttons.
- **Multi-generational Pedigree Tools** *(v1.20.0)* — Full ancestry and descendant traversal of the strain hybridization graph (`strain_parents`). Three backend commands: `get_strain_ancestry` and `get_strain_descendants` walk up/down through hybrid generations (default depth 5, capped at 10) returning recursive `PedigreeNode` trees with edge metadata (parent role, audit chain seq at creation, event notes); `get_strain_specimen_tree` aggregates live specimens across any strain and optionally its entire descendant subtree. A fourth `export_strain_pedigree` command bundles all reachable strains and hybridization events into portable JSON. The reusable `PedigreeChart.svelte` component renders both views as an indented node list with status badges, specimen counts, Hybrid/role labels, and a one-click JSON export. Cycle detection in both traversal directions rejects any circular pedigree with a descriptive error. **Conceptually distinct from specimen culture lineage** — only the hybridization graph is traversed here.
- **Advanced Hybridisation Tools** *(v1.21.0)* — Three major extensions to the hybrid strain system: **(1) Generation Labeling** — F/BC generation labels (F1, F2, F3, F4, BC1F1, BC1F2, …) are stored on `hybridization_events.generation_label`; the backend automatically suggests a label from parent labels (F1×F1→F2) and overrides with backcross notation when one parent is an ancestor of the other. The 9-step Hybrid Wizard (formerly 8) includes a new Generation Label step with async suggestion display and free-text override. **(2) Backcross Notation** — DFS ancestor search (`detect_backcross`) checks both parent directions; depth 1 → `BC1F1`, depth 2 → `BC2F1`, etc. Backcross depth is stored on `hybridization_events.backcross_depth`. **(3) Cross-Species Guard Hardening** — Normal users are hard-blocked from cross-species pairings with a clear error. Admin users see an override panel requiring a non-empty scientific justification and an explicit acknowledgement checkbox; the backend writes a **permanent, non-removable** `cross_species_override` audit entry and sets `is_cross_species = 1` on the resulting strain. `StrainDetail.svelte` (new slide-over opened by clicking any strain name) shows a red permanent warning banner on any cross-species hybrid. **(4) Generational Stats** — `get_generational_stats` returns per-generation specimen counts with healthy/problem breakdown for all descendants; displayed in the Generations tab of the new Strain Detail slide-over.
- **Taxonomy Backbone** *(v1.18.0)* — Hierarchical `taxa` table for ranks above Species (Kingdom → Phylum → Class → Order → Family → Genus). Each taxon stores a `taxon_path` JSON array of ancestor IDs for efficient tree traversal. A `get_taxon_descendants` command returns a full `TaxonNode` tree with aggregate strain and specimen counts at every level, ready for the advanced Taxonomy Navigator (WP-39) and NCBI integration (WP-36). Existing species data is automatically back-filled with genus-level taxa on upgrade. **No audit-chain involvement** — taxa are classification data only.
- **Photo Attachments** — Attach images directly to specimen records. Upload via OS file picker (desktop) or rear camera (Android). Responsive gallery grid with lightbox viewer and in-memory thumbnail cache. Images stored on disk under `<appDataDir>/attachments/`.
- **Export & Backup** — Dedicated Export Data page with Excel (`.xlsx`) multi-sheet workbook (Specimens, Subcultures, Media Batches, Prepared Solutions, Inventory, Compliance), plus CSV and JSON. On-demand database backup from the dashboard (supervisor/admin) with WAL checkpointing. Admins can restore from any listed backup via a two-step confirmation flow; the app restarts automatically after a successful restore.
- **Excel Import** — Dedicated Import Data page that accepts any `.xlsx` file produced by SteloPTC's export. SheetJS parses the workbook in-browser; a dry-run preview shows per-sheet create/update/skip counts and row-level errors before any data is written. Confirmed imports run in a single atomic transaction. Upserts specimens (by accession number), media batches (by batch code), prepared solutions and inventory (by name), and subcultures (by specimen + passage); compliance records are appended. Missing species are auto-created. Round-trip tested: export → wipe → import restores the lab (v1.3.0).
- **Mobile-First UI** — Hamburger + slide-out drawer on all screens < 1024 px, 48 px touch targets (WCAG 2.5.5), safe-area insets for notches and home indicators (v0.1.11+).
- **Keyboard Shortcuts** — Ctrl+1: Dashboard, Ctrl+2: Specimens, Ctrl+3: Media Logs, Ctrl+4: Reminders, Ctrl+5: Error Log. Ctrl/Cmd both work on macOS.
- **Contextual Tooltips** — "?" badge on every form field and action button with help text (v0.1.15).
- **Dark Mode** — System-aware with manual toggle, driven by a `data-theme` attribute on `<html>`. Inter font throughout.
- **Design Token System** — All colors, spacing, typography, radii, shadows, and z-index layers are defined as CSS custom properties in `src/lib/styles/tokens.css`. Changing a single token updates the UI consistently across all token-aware components (v1.2.2).
- **Consistent Data States** — `DataState.svelte` provides a unified skeleton loading state, friendly empty state with optional CTA, and inline error state with retry across all list views (v1.2.3).
- **Professional Print / PDF Output** — Culture Certificates and Specimens Summary reports open as formatted print windows with consistent header (logo space, lab name, accession, date, prepared-by) and footer (lab name, page numbers). Layouts are clean and well-organized on both A4 and US Letter paper, portrait and landscape. The Specimens Summary supports three grouping modes: by development stage, by health/urgency, or flat list — with an executive summary section (stats, stage distribution, health distribution) at the top. All three print functions (`printSummaryReport`, `printCultureReport`, `printLabel`) use a shared `printUtils.ts` delivery module: popup path for browsers, in-page DOM fallback for Tauri/WebView2 where `window.open` is blocked (v1.2.4, v1.4.0, v1.4.1).
- **Cryopreservation & LN2 Inventory** *(v1.25.0)* — First-class `frozen_vials` table stores vial lots with location (Freezer / Tower / Box / Position, mirroring the specimen Room/Rack/Shelf/Tray structure), freeze medium, passage number, cumulative PDL at freeze, and status (`active | depleted | discarded`). A DB-level `CHECK(vial_count >= 0)` prevents negative inventory. The **Thaw** action is atomic: it decrements the vial count (auto-marking the lot as `depleted` when it reaches zero) and creates a new `specimens` record that inherits `lineage_passage_offset` and `cumulative_pdl` from the frozen vial, preserving passage-lineage data across the freeze–thaw cycle. Audit entries are written for both the vial and the new specimen. The **Cryostorage** sidebar entry (❄) opens a filterable inventory table with "Record Vials" and inline Thaw/Discard actions.
- **Mycoplasma Compliance & Biosafety Level** *(v1.26.0)* — A new compliance flag rule (`missing_mycoplasma_test / severity: high`) fires for every non-archived specimen in `cell_culture` mode that has no mycoplasma test result within the configurable interval (`mycoplasma_test_interval_days`, default 90 days). The last test date is stored on the flag and shown in the Compliance flags table. Specimens gain a nullable `biosafety_level` column (BSL-1 / BSL-2 / BSL-2+ / BSL-3, DB-level CHECK constraint) displayed as a colour-coded badge in the SpecimenDetail info card. A `get_mycoplasma_status` backend command returns per-specimen mycoplasma test status for display.
- **Cell-culture Dashboard Panels** *(v1.27.0)* — Four dedicated panels appear on the Dashboard when the active lab profile is `cell_culture` (all other profiles are unaffected): (1) **Passages Due / Overdue** — overdue passages (red) and those due within 3 days (yellow), reusing the existing subculture schedule data with cell-culture terminology; (2) **Lines Overdue for Mycoplasma Test** — filters compliance flags for `missing_mycoplasma_test` and shows each line's last test date; (3) **Vials in Storage by Line** — per-species frozen vial inventory from `get_vial_summary_by_line`, with amber highlighting for lines at ≤ 5 total vials; (4) **Cultures Needing Attention** — specimens in active non-terminal stages not passaged in ≥ 7 days, ordered by staleness and colour-coded (red ≥ 14 d, yellow 7–13 d). New `query_vial_summary_by_line` and `query_culture_maintenance_alerts` backend helpers live in `db/dashboard.rs` with 9 new unit tests.
- **Mycology Profile Vocabulary** *(v1.28.0)* — Migration 027 seeds all six profile-scoped vocabulary tables for the `mycology` profile via `INSERT OR IGNORE` (idempotent, purely additive — existing PTC and cell_culture rows are untouched). Stages cover the full mushroom/fungal cultivation lifecycle: `spore_clone`, `agar`, `liquid_culture`, `grain_spawn`, `bulk_substrate`, `colonizing`, `fruiting`, `senescent`, `contaminated` (terminal), `discarded` (terminal). Propagation methods reflect common transfer techniques (`agar_to_agar`, `agar_to_grain`, `grain_to_grain`, `grain_to_bulk`, `liquid_inoculation`, `spore_syringe`, `culture_restart`). Supplement types (`gypsum`, `bran`, `calcium_carbonate`, `activated_carbon`, `coconut_coir`, `vermiculite`) reuse the `hormone_types` table. Inventory categories include `agar_media`, `grain_spawn`, `bulk_substrate`, `liquid_culture`, `substrate_amendment`, `syringes_needles`, and more. 12 new Rust unit tests cover counts, terminal/non-terminal split, expected codes, profile isolation, and idempotency.
- **Mycology Colonization & Contamination Tracking** *(v1.29.0)* — Migration 028 adds `colonization_pct REAL` (0–100, CHECK-constrained) and `contaminant_type TEXT` to `subcultures`. In the mycology profile, the passage form shows a **Colonization %** input for recording mycelium spread and a **Contaminant Type** dropdown (Trichoderma, Wet Rot / Bacterial, Cobweb Mold, Pin Mold, Mycelium Abort, Other) when contamination is flagged. A **Colonization Progress** bar-chart displays below the passage timeline for any specimen with recorded readings (color-coded: green ≥ 80%, amber ≥ 50%, red < 50%). The passage timeline badge shows the specific contaminant type instead of a generic "Contaminated" label. Contamination dashboard gains a `by_contaminant_type` breakdown alongside the existing `by_vessel_type` breakdown. New `get_colonization_history` Tauri command returns ordered colonization readings per specimen. 8 new Rust unit tests (4 migration, 4 dashboard). Total: 225 Rust tests.
- **Genetic Lineage & Strain Isolation** *(v1.30.0)* — Migration 029 adds `origin_type TEXT` (CHECK-constrained to `multi_spore | isolated_dikaryon | tissue_clone`, NULL = unspecified) and `is_best_performer INTEGER NOT NULL DEFAULT 0` to `specimens`. In the mycology profile: the **New Specimen** form exposes a **Culture Origin Type** dropdown; the **Specimen Detail** info card shows a colour-coded **Culture Origin** badge and a **Best Performer** toggle button (★/☆) that calls `updateSpecimen` on click and refreshes the record. `split_specimen` inherits `origin_type` from the parent into every child and resets `is_best_performer = 0` (selection is re-evaluated per generation). `search_specimens` supports a `best_performer_only` filter. 5 new Rust unit tests (migration 029 column existence, CHECK constraint, flag insert).
- **Fruiting Conditions & Yield Tracking** *(v1.31.0)* — New `fruiting_records` table (migration 030) stores per-flush harvest data for mycology cultures: `flush_number`, `harvest_date`, `fresh_weight_g`, `dry_weight_g`, `fruiting_temp_c`, `fruiting_rh_percent`, `fae_rate`, and `light_hours_per_day`. Two Tauri commands (`create_fruiting_record`, `list_fruiting_records`) expose the data to the frontend. The Specimen Detail **History** tab (mycology only) gains a **Fruiting Records** section with a collapsible flush-entry form and a scrollable table of all recorded flushes. 7 new Rust unit tests.
- **Mycology Compliance / QC Rules** *(v1.32.0)* — Three new mycology-specific QC flags added to the existing compliance rule engine (`get_compliance_flags`): **`myco_open_contamination`** (high) flags non-terminal cultures with contamination set but not discarded; **`myco_overdue_transfer`** (normal) flags cultures with no passage in the last N days (configurable via `myco_transfer_interval_days`, default 21); **`myco_slow_colonization`** (normal) flags colonizing-stage cultures whose most recent `colonization_pct` is below a threshold (default 30%) after at least N days (default 7). All three rules are profile-gated — PTC and cell_culture are unaffected. Dashboard gains a **Mycology QC Alerts** panel (mycology profile only) surfacing open flags with severity badges. 8 new Rust unit tests. Total: 245 Rust tests.
- **Full Taxonomic Hash Chain (Experimental)** *(v1.33.0)* — Migration 031 backfills genesis audit entries for all existing taxa in rank order (kingdom → genus), enabling the full **Kingdom → Phylum → Class → Order → Family → Genus → Species → Strain → Specimen** cryptographic provenance chain. New `log_audit_taxon_genesis` and `log_audit_species_genesis` helpers extend the audit system upward. Strain genesis is now anchored to the genus taxon's `entry_hash`; species genesis is seeded from the genus taxon. `create_taxon` and `update_taxon` write audit entries. **Warning:** this is an experimental feature — reclassifying a taxon after its genesis entry is written breaks the chain for all descendants. See ROADMAP.md §WP-45 for the full risk description and known limitations. 6 new/updated Rust unit tests. Total: 250 Rust tests.
- **Cross-Domain Taxonomy Support** *(v1.34.0)* — Migration 032 adds a `domain` column to `app_config` mapping each lab profile to its biological domain: `plant_tissue_culture` → `Plantae`, `cell_culture` → `Animalia`, `mycology` → `Fungi`. No CHECK constraint so future domains (Bacteria, Archaea) can be stored without another migration. New `active_domain(conn)` backend helper returns the current domain (falls back to `'Plantae'`). Frontend `profile.ts` gains `LabDomain` type, `DomainManifest` interface (rank order, strain type labels, confirmation method labels), `PROFILE_DOMAIN` map, `DOMAIN_MANIFESTS` (per-domain UI manifests for all three supported domains), and `activeDomainManifest()` helper. 8 new Rust tests, 16 new frontend tests. Total: 258 Rust tests.
- **Breeding Programs & Multi-Generational Selection Tracking** *(v1.35.0)* — Migration 033 adds `breeding_programs` and `breeding_records` tables (with cascade delete and indexes). Users can create named breeding programs with goals, start dates, target traits, and founder strain IDs, then record per-generation selection events with strain, generation number, fitness score, and selection notes. `get_generational_summary` aggregates per-generation selection counts and average fitness. Seven backend Tauri commands (create/list/get programs, add/list records, generational summary). A **Breeding Programs** sidebar entry opens the `BreedingProgramManager` component with split list/detail layout, create-program form, add-selection form, generational performance table, and selection record cards. 13 new Rust tests. Total: 271 Rust tests.

- **Automated Tests** — Vitest test suite covers utility functions, export row builders, import validation, and lab profile store (including domain manifest exports) — ~123 assertions across 4 test files. Rust unit tests (271 test functions) cover accession number generation, pagination helpers, stock depletion rules, all compliance auto-flag SQL rules (including the new mycoplasma rule), **hash-chain invariants** (per-lineage `chain_seq`, fork invariants, `compute_entry_hash` determinism), Merkle checkpoint correctness, death workflow invariants, profile-aware dashboard queries, check_profile_change_allowed confirmation logic, cell_culture vocabulary seeding (expanded stage set, propagation methods, supplement types, biomanufacturing compliance terms), **mycology vocabulary seeding** (stage lifecycle, terminal stages, propagation methods, supplement types, compliance terms, inventory categories, profile isolation, idempotency), **mycology colonization & contaminant tracking** (migration 028 column existence, CHECK constraint, NULL defaults, contaminant-type grouping, empty-when-NULL, recent-events inclusion), **genetic lineage markers** (migration 029 `origin_type` column existence, CHECK constraint acceptance/rejection, `is_best_performer` default and filter), **strain/cultivar data model** invariants (status transitions, hash-chain seeding, hybridization guard), **taxonomy backbone** (taxa table structure, rank constraint, backfill idempotency), **pedigree traversal** (ancestry, descendants, max-depth capping, cycle detection in both directions, specimen tree with/without descendants, export bundle), **PDL/doubling-time calculations** (typical growth, decline, no-growth, and invalid-input cases), **cryopreservation** (location composition, insert, zero-count rejection, thaw decrement, depleted status, PDL/passage inheritance, overdraw rejection, discard, list filtering), **mycoplasma status queries** (active-only filter, last-test-date selection, null-when-no-record, non-mycoplasma exclusion), **cell-culture dashboard queries** (vial summary grouping, depleted/discarded exclusion, maintenance alert thresholds, profile scoping, terminal-stage exclusion, created-at fallback), and **fruiting records** (insert + get round-trip, list per specimen, FK rejection of unknown specimen, migration 030 table/index existence, flush_number DEFAULT), **mycology QC compliance rules** (open_contamination detected/not-raised-for-terminal, overdue_transfer no-subculture/recent-passage-ok, slow_colonization triggered/recent-ok/above-threshold-ok, archived-excluded), **taxon hash chain** (root taxon uses ZERO_HASH, child seeds from parent, update appends correctly, species genesis seeds from genus, strain falls back when no genus entry), **cross-domain taxonomy** (migration 032 domain column existence and per-profile defaults, `active_domain` reads/falls-back correctly), and **breeding programs** (migration 033 table/index existence, cascade delete, create/get/list program round-trips, add/list record round-trips, generational summary aggregation, FK rejection for unknown program). Both `npm test` and `cargo test` run in CI on every push and block merges on failure (v1.2.4, v1.4.1, v1.6.0, v1.11.0, v1.13.0, v1.14.0, v1.15.0, v1.16.0, v1.17.0, v1.18.0, v1.20.0, v1.21.0, v1.22.0, v1.23.0, v1.24.0, v1.25.0, v1.26.0, v1.27.0, v1.28.0, v1.29.0, v1.30.0, v1.31.0, v1.32.0, v1.33.0, v1.34.0, v1.35.0).
- **Accessibility** — WCAG 2.1 AA target: visible `:focus-visible` keyboard indicators, skip-to-content link, ARIA landmarks and `aria-current` on sidebar navigation, focus trapping in QR modal and photo lightbox, `aria-label` on all icon-only buttons, ARIA attributes (`aria-valuenow`/`aria-valuetext`) on health status sliders (v1.2.6).
- **Query Performance** — Six composite and covering indexes added (migration 007) on `specimens(created_at)`, `specimens(parent_specimen_id)`, `specimens(is_archived, created_at)`, `subcultures(specimen_id, passage_number)`, `subcultures(created_at)`, and `subcultures(contamination_flag, specimen_id)`. Correlated per-row contamination subqueries in specimen list/search replaced with a single aggregating LEFT JOIN. Subculture history list endpoint now uses the `PaginatedResponse` pattern. Scales to 10k+ specimens and 50k+ subcultures (v1.2.7).

### Species Registry

Pre-configured for asparagus, nandina, and citrus varieties. Any species can be added through the admin species manager. Species act as the cryptographic root for all derived specimens — every specimen's provenance chain traces back to its species genesis hash.

**Strain & Cultivar Registry (Phase TX-1 · shipped v1.16.0–v1.17.0):** Each species supports any number of named Strains or Cultivars as first-class entities with their own hash chains (seeded from the species hash). Strains use a **four-value, three-tier status model**: `Unverified` (default — no assertion made) → `Claimed` (explicit identity assertion, low friction) → `Confirmed — Manual` (professional assessment, requires documented basis + blocking acknowledgment modal, `⚠ Manual ID` badge permanent, never equivalent to genomic) → `Confirmed — Genomic` (fingerprint data required, gold standard). Status downgrades are permanently rejected. Specimens are cryptographically bound to a specific strain version at creation (`strain_chain_seq`). **Accession numbers never encode strain** — this is a final design decision; the accession identifies the culture lineage only. Hybrid strains are created via a dedicated wizard that records a `hybridization_events` entry and writes bidirectional `used_as_parent` audit entries on both parent strain chains. A Taxonomy Navigator provides hierarchical browsing from Species → Strains → Specimens.

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
| Auth      | bcrypt password hashing, session tokens, forced first-login password change |
| Security  | Tauri CSP: `script-src 'self'`; no remote scripts; `data:`/`blob:` image/worker sources scoped explicitly |
| Mobile    | Android 7.0+ (API 24–35), Tauri 2 mobile        |
| QR Codes  | qrcode 1.5.4 (generation), html5-qrcode 2.3.8 (scanning) |
| Excel     | xlsx 0.18.5 (SheetJS — multi-sheet workbook export and import) |
| Crypto    | sha2 0.10 (Rust — SHA-256 for per-lineage audit hash chain) |

---

## Testing

SteloPTC ships with both frontend (TypeScript/Vitest) and backend (Rust/cargo) test suites.

### Frontend tests

```bash
npm test          # run once (CI mode)
npm run test:watch  # watch mode (development)
```

Tests live in `src/**/*.test.ts`. The current suite covers ~107 assertions across four test files:

**`src/lib/utils.test.ts`** — core utility functions:

| Area | Coverage |
|---|---|
| `escHtml` | null/undefined/empty, HTML entities, number coercion |
| `healthLabel` | null, NaN, all 5 health levels, clamping, rounding |
| `stageFmt` | underscore → Title Case, empty input |
| `composeLocation` | full 4-part, partial, empty |
| `formatAccessionNumber` | zero-padding, 3-digit sequences |
| `computeStockAdjustment` | positive/negative deltas, exact-zero, below-zero guard |
| `datestamp` | formatted date output |
| `ageDays` / `fmtAge` / `healthNum` / `effectiveHealth` | age, formatting, and health extraction edge cases |

**`src/lib/exportUtils.test.ts`** — export row builders (v1.3.0+):

| Area | Coverage |
|---|---|
| `specimenRows` / `subcultureRows` / `mediaRows` | Core export sheet row construction |
| `inventoryRows` / `complianceRows` / `prepSolutionRows` | Supplementary export sheets |

**`src/lib/importUtils.test.ts`** — import validation:

| Area | Coverage |
|---|---|
| `REQUIRED_SHEET_NAMES` | Expected sheets present |
| `findMissingSheets` | Detects missing required sheets |

**`src/lib/profile.test.ts`** — lab profile Svelte store (v1.14.0+):

| Area | Coverage |
|---|---|
| `labProfile` store | Default value, reactive updates, immediate reflection after profile switch |
| `currentLabProfile()` | Synchronous accessor returns current store value |
| `LAB_PROFILE_LABELS` | Completeness and human-readable label for the default profile |

### Rust tests

```bash
cd src-tauri && cargo test --lib
```

Requires Linux GTK system libraries (installed automatically in CI):

| Module | Tests |
|---|---|
| `db::queries` | Accession number format, first/second/different-species/different-date sequences, zero-padding, pagination offset/limit calculations; **hash-chain invariants** — per-lineage `chain_seq` increments, child chains start at seq 1 with parent's `entry_hash` as `prev_hash`, split siblings share the same `prev_hash`, `compute_entry_hash` is deterministic (v1.6.0); **Merkle checkpoint tests** — empty/single/two/three-leaf trees, determinism, mutation detection, checkpoint creation, intact verification, tamper detection, removal detection, entries-beyond-end-seq, out-of-range seq windows (v1.9.0); **profile switcher** — `check_profile_change_allowed` — empty lab always allowed, correct confirmation accepted, wrong/missing confirmation rejected, plural grammar (v1.14.0); **strain hash-chain seeding** — strain genesis `prev_hash` equals species `entry_hash`, specimen with strain seeds from strain `entry_hash`, `strain_chain_seq` matches at creation, `create_hybridization_event` cross-species guard, bidirectional `used_as_parent` entries, fork invariant preserved for split siblings with a strain (v1.16.0); **mycology QC compliance rules** (v1.32.0) — 62 tests |
| `db::dashboard` | Vocabulary labels returned for PTC, cross-profile stage exclusion, empty result for unseeded profile, database-wide aggregate counts, contamination scoping and rate, vessel-type breakdown, schedule filtering — 12 tests (v1.13.0) |
| `db::vocabulary` | Stage list returns correct count and order for active profile; vocabulary isolation between profiles — 9 tests |
| `db::migrations` | Migration fixture correctness; cell_culture stage count (12), single terminal stage, propagation method count (7), hormone type count (4), compliance record type count (9), compliance agency count (4), inventory category count (7), isolation from PTC vocabulary (v1.15.0); **taxa backbone** — `taxa` table created with correct columns, rank CHECK constraint enforced, `get_taxon_descendants` returns empty tree for unknown taxon, backfill idempotent, `ncbi_taxon_id` nullable on `taxa` and `species`, `taxon_path` JSON stored correctly (v1.18.0); **mycology colonization/contamination** — migration 028 column existence, CHECK constraint, NULL defaults (v1.29.0); **genetic lineage markers** — migration 029 `origin_type` column existence, CHECK constraint acceptance/rejection, `is_best_performer` default and filter (v1.30.0); **fruiting records** — migration 030 table/index existence, FK rejection, `flush_number` DEFAULT (v1.31.0) — 39 tests |
| `commands::inventory` | `apply_stock_adjustment` — positive delta, negative delta, to-zero, below-zero error; `is_low_stock` — at/below/above minimum |
| `commands::compliance` | Expired permit detected/not-detected; quarantine-no-release detected/not-detected; positive-not-quarantined detected/not-detected; HLB missing/recent; archived specimens excluded from all flags |
| `commands::audit` | Checkpoint tamper-detection and verification invariants — 4 tests |
| `commands::specimens` | Death archives specimen and zeroes health; `event_type` stored as `'death'`; archived specimen blocks further passages; normal passages retain `'passage'` event_type; `app_config` seeded with default profile — 5 tests (v1.11.0) |

### CI

The `test.yml` GitHub Actions workflow runs both test suites on every push and pull request to `master`. Merges are blocked if either suite fails.

---

## Building from Source

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | 18+ | [nodejs.org](https://nodejs.org) or `nvm install 20` |
| Tauri CLI | latest | `cargo install tauri-cli` |

**Linux additionally requires:**

```bash
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev libssl-dev
```

### 1. Clone and install dependencies

```bash
git clone https://github.com/jnowat/steloptc.git
cd steloptc
npm install --legacy-peer-deps
```

### 2. Run in development mode

```bash
cargo tauri dev
```

Opens a Tauri window with Vite hot-reload. The first build downloads Rust dependencies and may take a few minutes.

### 3. Run the test suites

**Frontend (Vitest):**

```bash
npm test               # single run
npm run test:watch     # watch mode
```

**Backend (Rust):**

```bash
cd src-tauri && cargo test --lib
```

### 4. Type-check the frontend

```bash
npm run check          # svelte-check + TypeScript
```

### 5. Build a release binary

```bash
# Windows MSI + exe
cargo tauri build --bundles msi

# Linux .deb / .AppImage
cargo tauri build
```

Output lands in `src-tauri/target/release/bundle/`.

### Default login

On a fresh database the only account is `admin` / `admin`. The app immediately forces a password change before any other action is possible.

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
        versionCode = 24        // committed baseline; patched to the release value by CI at build time
        versionName = "1.1.0"  // committed baseline; patched to the release value by CI at build time
    }
}
```

> **Note:** `versionCode` and `versionName` in the committed file are a stable baseline. The release CI workflow patches both fields to the correct release values before building the signed APK, so the committed values are intentionally behind the current release version.

### Default Login

| Username | Password | Role  |
|----------|----------|-------|
| `admin`  | `admin`  | Admin |

**On a fresh installation, logging in as `admin` immediately redirects to a mandatory password-change screen. The application cannot be used until a new password is set.** This prevents deployments from running with the default credential.

---

## Project Structure

```
SteloPTC/
├── src/                              # Svelte 5 frontend
│   ├── App.svelte                    # Main app shell with routing and dark mode
│   ├── main.ts                       # Entry point — mounts app, restores session
│   └── lib/
│       ├── api.ts                    # Typed Tauri command bindings
│       ├── printUtils.ts             # Shared print delivery (popup + in-page DOM fallback), age/health helpers
│       ├── styles/
│       │   └── tokens.css            # Central CSS custom properties — colors, spacing, type, radii, shadows, z-index
│       ├── components/
│       │   ├── Login.svelte
│       │   ├── Dashboard.svelte      # Stats, schedule, contamination, reminders
│       │   ├── Sidebar.svelte        # Navigation with hamburger drawer on mobile
│       │   ├── SpecimenList.svelte   # Search, filter, QR scan entry point
│       │   ├── SpecimenDetail.svelte # Passage timeline, lineage (parent/children/siblings), QR, record passage
│       │   ├── SpecimenForm.svelte   # New/edit specimen form
│       │   ├── MediaList.svelte      # Media batch CRUD
│       │   ├── ReminderList.svelte   # Reminder management
│       │   ├── ComplianceView.svelte # Compliance records and auto-flagging
│       │   ├── SpeciesManager.svelte # Species registry
│       │   ├── UserManager.svelte    # User accounts and roles
│       │   ├── AuditLog.svelte       # Audit trail with hash-chain columns, Row/Chain verify buttons
│       │   ├── ErrorLog.svelte       # Error tracking with payload capture
│       │   ├── InventoryManager.svelte # Supply inventory CRUD
│       │   ├── ImportManager.svelte  # Excel import with dry-run preview
│       │   ├── QrModal.svelte        # QR generation, print label, download
│       │   ├── QrScanner.svelte      # Camera-based QR scanning
│       │   ├── ExportManager.svelte  # Excel/CSV/JSON export hub
│       │   ├── Tooltip.svelte        # Reusable "?" contextual help badge
│       │   ├── Notifications.svelte  # Toast notification renderer
│       │   ├── WorkQueue.svelte      # Prioritized daily task view
│       │   ├── ForceChangePassword.svelte # First-login mandatory password change
│       │   ├── FirstRun.svelte       # Onboarding guide for fresh installs
│       │   ├── SkeletonLoader.svelte # Animated shimmer skeleton for loading states
│       │   ├── EmptyState.svelte     # Friendly empty state with icon and CTA
│       │   └── DataState.svelte      # Unified loading/error/empty/ready state wrapper
│       └── stores/
│           ├── auth.ts               # Auth state, session restore, mustChangePassword gate
│           └── app.ts                # View routing, notifications, error logger, work queue count
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
│       │   ├── migrations.rs         # 29 schema migrations
│       │   └── queries.rs            # SQL helpers
│       ├── models/                   # Rust data structures
│       │   ├── user.rs, specimen.rs, media.rs
│       │   ├── subculture.rs, species.rs, reminder.rs
│       │   ├── compliance.rs, inventory.rs
│       │   ├── audit.rs, error_log.rs
│       │   └── mod.rs
│       └── commands/                 # Tauri command handlers
│           ├── auth.rs, specimens.rs, media.rs
│           ├── subcultures.rs        # Passages, contamination stats, schedule; split_specimen atomic command
│           ├── reminders.rs, compliance.rs
│           ├── species.rs, audit.rs  # audit.rs: log_audit, verify_audit_entry, verify_audit_lineage
│           ├── error_logs.rs, export.rs
│           ├── import.rs             # Excel import with dry-run + atomic commit
│           ├── inventory.rs, backup.rs
│           ├── qr_scans.rs, admin.rs
│           ├── work_queue.rs         # get_work_queue — prioritized overdue-specimen list
│           ├── attachments.rs        # Photo attach/fetch/delete
│           └── mod.rs
│
├── .github/workflows/
│   ├── build-windows.yml             # MSI + exe on push and release
│   ├── build-android.yml             # APK on push and release
│   └── test.yml                      # Vitest + cargo test on every push; blocks merges on failure
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
| `specimens`          | Core specimen records with accession numbers; `generation`, `lineage_passage_offset`, `root_specimen_id` for genealogy tracking (v1.7.0) |
| `tags`               | Hierarchical tag definitions                              |
| `specimen_tags`      | Tag assignments to specimens                              |
| `media_batches`      | Media preparation log with batch IDs; `is_draft` flag for placeholder batches created during split workflow (v1.8.0) |
| `media_hormones`     | Hormone details per media batch                           |
| `subcultures`        | Passage history with contamination flags and notes        |
| `prepared_solutions` | Stock solutions prepared from solid reagents              |
| `attachments`        | File attachment metadata                                  |
| `reminders`          | Scheduled reminders and rules                             |
| `compliance_records` | Regulatory tests, permits, inspections                    |
| `inventory_items`    | Supply inventory with reorder alerts                      |
| `audit_log`          | Append-only audit trail; `chain_seq`, `prev_hash`, `entry_hash` for SHA-256 per-lineage hash chain; `lineage_id` for per-entity chain isolation (v1.5.0, v1.6.0) |
| `audit_checkpoints`  | Merkle checkpoint records per lineage range; `is_auto`, `auto_source` for auto-checkpointing; Dogecoin anchor hook (v1.9.0, v1.10.0) |
| `app_settings`       | Key-value settings store; seeded with auto-checkpoint defaults (v1.10.0) |
| `app_config`         | Single-row config table; `lab_profile` constrained to `plant_tissue_culture \| cell_culture \| mycology` (v1.11.0) |
| `stages`             | Profile-scoped specimen stage vocabulary; replaces `CHECK(stage IN (...))` constraint on `specimens` (v1.12.0) |
| `hormone_types`      | Profile-scoped hormone type vocabulary for `media_hormones` (v1.12.0) |
| `compliance_record_types` | Profile-scoped compliance record type vocabulary (v1.12.0) |
| `compliance_agencies` | Profile-scoped compliance agency vocabulary (v1.12.0) |
| `inventory_categories` | Profile-scoped inventory category vocabulary (v1.12.0) |
| `strains`            | Named strains/cultivars per species with four-value status model (`unverified → claimed → confirmed_manual → confirmed_genomic`); hash chain seeded from parent species (v1.16.0) |
| `strain_parents`     | Multi-parent hybrid pedigree links; `parent_chain_seq_at_creation` records strain chain version at hybridization time (v1.16.0) |
| `hybridization_events` | Records both parent strains and their exact chain sequence snapshots at hybridization time; one record per hybrid strain (v1.16.0) |
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
| 006 | v0.1.20 | Added must_change_password to users; seeded admin row with flag set |
| 007 | v1.2.7 | Six composite/covering indexes on specimens and subcultures for query performance |
| 008 | v1.5.0 | Added chain_seq, prev_hash, entry_hash columns to audit_log (hash-chain tamper evidence) |
| 009 | v1.6.0 | Added lineage_id column to audit_log; composite index on (lineage_id, chain_seq); per-lineage back-fill |
| 010 | v1.7.0 | Added generation, lineage_passage_offset, root_specimen_id columns to specimens |
| 011 | v1.8.0 | Added is_draft column to media_batches for placeholder batches created during split workflow |
| 012 | v1.8.x | Added contamination_flag and contamination_notes columns to specimens (archived contamination state) |
| 013 | v1.9.0 | Added audit_checkpoints table with Merkle root, seq range, entry count, and Dogecoin anchor hook |
| 014 | v1.10.0 | Added `is_auto` and `auto_source` to `audit_checkpoints`; created `app_settings` key-value table with seeded auto-checkpoint defaults |
| 015 | v1.11.0 | Added `event_type TEXT NOT NULL DEFAULT 'passage'` to `subcultures` (with index); created `app_config` single-row table with `lab_profile` (constrained to `plant_tissue_culture \| cell_culture \| mycology`) |
| 016 | v1.12.0 | Created `stages` lookup table (`profile`, `code`, `label`, `sort_order`, `is_terminal`); seeded 15 PTC stage codes; rebuilt `specimens` to drop the `CHECK(stage IN (...))` constraint — the final vocabulary-driven table rebuild |
| 017 | v1.12.0 | Created `hormone_types`, `compliance_record_types`, `compliance_agencies`, `inventory_categories` lookup tables (profile-scoped, seeded with PTC values); rebuilt `media_hormones`, `compliance_records`, `inventory_items` to drop their respective `CHECK` constraints |
| 018 | v1.15.0 | Seeded `cell_culture` profile vocabulary into all six lookup tables via `INSERT OR IGNORE` — 12 stages, 7 propagation methods, 4 hormone types, 9 compliance record types, 4 compliance agencies, 7 inventory categories; no schema changes, no table rebuilds |
| 019 | v1.16.0 | Created `strains`, `strain_parents`, and `hybridization_events` tables; added `strain_id` (FK, nullable) and `strain_chain_seq` (nullable) to `specimens`; six covering indexes; purely additive — existing specimen rows receive `NULL` for both new columns |
| 020 | v1.18.0 | Created `taxa` hierarchical classification table (ranks: kingdom–genus) with self-referential `parent_id`, `ncbi_taxon_id`, `local_override`, and `taxon_path` columns; added `taxon_path` and `ncbi_taxon_id` to `species`; idempotent genus backfill (`backfill_genus_taxa`) runs automatically |
| 021 | v1.19.0 | Created `ncbi_sync_log` table to record NCBI taxonomy import events, data updates, and name/rank conflicts; four indexes for fast conflict and type queries; `resolution` constrained to `kept_local \| accepted_ncbi \| merged` (nullable) |
| 027 | v1.28.0 | Seeded `mycology` profile vocabulary into all six lookup tables via `INSERT OR IGNORE` — 10 stages (8 non-terminal + contaminated + discarded), 8 propagation methods, 7 substrate supplement types, 6 compliance record types, 4 compliance agencies, 10 inventory categories; no schema changes, no table rebuilds; existing PTC and cell_culture rows untouched |
| 028 | v1.29.0 | Added `colonization_pct REAL CHECK (0–100)` and `contaminant_type TEXT` to `subcultures`; both columns are nullable so existing rows are unaffected; `colonization_pct` carries a CHECK constraint enforcing the 0–100 range |
| 029 | v1.30.0 | Added `origin_type TEXT CHECK(origin_type IN ('multi_spore','isolated_dikaryon','tissue_clone'))` (nullable) and `is_best_performer INTEGER NOT NULL DEFAULT 0` to `specimens`; purely additive — existing specimen rows receive `NULL` / `0` for both new columns |
| 030 | v1.31.0 | Created `fruiting_records` table with FK to `specimens`; columns: `flush_number INTEGER NOT NULL DEFAULT 1`, `harvest_date TEXT NOT NULL`, optional yield fields (`fresh_weight_g`, `dry_weight_g`) and environment fields (`fruiting_temp_c`, `fruiting_rh_percent`, `fae_rate`, `light_hours_per_day`); index on `specimen_id` |

### Backup

On-demand backup from the Dashboard (supervisor/admin only). Backups are stored in a `backups/` subdirectory alongside the database file, with timestamped filenames. The process checkpoints the WAL first to ensure a consistent copy.

Admins can restore from any previously created backup via the "Restore from Backup" panel on the Dashboard. The restore flow requires two explicit confirmations before the destructive action is taken. On success the current database is replaced with the backup and the application restarts automatically to load the restored data.

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

### v1.0.0-x — v1.2.6 — Completed

- [x] First signed GitHub Release; Windows MSI and Android APK attached to release assets (v1.0.0-1)
- [x] **Crash-proofing & data-integrity pass** — all `.unwrap()` calls in command handlers converted to returned errors; `create_subculture` and `create_media_batch` wrapped in SQLite transactions; WAL checkpoint verified before backup copy (v1.0.0-2)
- [x] **Onboarding empty state + seed-data toggle** — guided first-run panel; supervisors/admins can load a ready-made sample lab in one click (v1.1.0)
- [x] **QR scanner rejects non-SteloPTC codes gracefully** — non-specimen QR codes show a clear warning instead of triggering a failed specimen lookup (v1.1.1)
- [x] **Print error handling** — "Print Summary" and "Print Label" surface a clear error notification when the browser blocks the popup (v1.1.1)
- [x] **Work Queue** — daily task view listing every specimen needing immediate attention, sorted by urgency; amber sidebar badge (v1.2.0)
- [x] **Skeleton loaders & empty states** — animated shimmer skeleton for loading; friendly icon-led empty states with CTAs across all list views (v1.2.1)
- [x] **Design Token System** — all colors, spacing, typography, radii, shadows, and z-index values defined as CSS custom properties in `tokens.css`; automatic light/dark switching via `data-theme` attribute (v1.2.2)
- [x] **Unified data states** — `DataState.svelte` provides skeleton loading, inline error-with-retry, and descriptive empty states across all list views; dark mode text visibility fixed (v1.2.3)
- [x] **Professional print / PDF output** — consistent header/footer, A4 + US Letter support, page-break hygiene, typography polish (v1.2.4)
- [x] **First test harness** — Vitest + Rust unit tests; CI workflow blocks merges on failure (v1.2.4)
- [x] **Tauri-reliable print invocation** — popup + in-page DOM fallback for all three print functions; no silent failures in WebView2 (v1.2.5)
- [x] **Accessibility pass (WCAG 2.1 AA target)** — visible keyboard focus indicators, skip-to-content link, ARIA landmarks, `aria-current` navigation, focus traps in QR modal and photo lightbox, `aria-label` on all icon-only buttons, health slider ARIA attributes (v1.2.6)

### v1.2.7 — v1.8.0 — Completed

- [x] **Query performance & indexing** — six composite indexes on specimens and subcultures (migration 007); N+1 contamination subquery eliminated; subculture list paginated (v1.2.7)
- [x] **Backup restore** — admin-only restore from any listed backup with two-step confirmation; app restarts automatically on success (v1.3.0)
- [x] **Excel import** — dry-run preview with per-sheet counts and row-level errors; atomic commit on confirm; round-trip restore verified (v1.3.0)
- [x] **Print reliability & refactor** — `printUtils.ts` extraction; all three print functions audited; no silent `if (!win) return` failures remain (v1.3.1)
- [x] **Professional Specimen Inventory Report** — three grouping modes (by stage / by health+urgency / flat); executive summary with stat boxes, stage and health distribution chips; per-group sub-headers with contamination/quarantine counts (v1.4.0)
- [x] **CSP print-dialog fix** — `win.print()` called from parent WebView context instead of inline script, which Tauri's CSP blocks; test coverage for print utils expanded to 50 assertions (v1.4.1)
- [x] **Hash-chain tamper-evident audit log** — every audit entry carries SHA-256 `entry_hash = SHA256(canonical_bytes ∥ prev_hash)`; migration 008 adds `chain_seq`, `prev_hash`, `entry_hash` to `audit_log` (v1.5.0)
- [x] **Audit Log UI for hash chain** — chain columns visible with truncated display and full-hash tooltips; 🔒 badge on chained rows; click to copy full hash (v1.5.1)
- [x] **Per-lineage hash chain** — chain becomes per-entity (`lineage_id`) rather than global; split/fork specimens inherit parent's last `entry_hash` as `prev_hash`, making fork points cryptographically visible; `verify_audit_entry` and `verify_audit_lineage` Tauri commands; Row + Chain verify buttons in Audit Log UI; migration 009 (v1.6.0)
- [x] **Hash-chain hardening** — fork lineage verification fix; nullable column types for pre-chain rows; atomic specimen INSERT + audit; `reset_database` available in release builds with admin guard; species creation anchors chain at seq 0; root specimens seeded from species hash; `load_demo_data` generates fully chained records with a cryptographic split demonstration (v1.6.1–v1.6.4)
- [x] **Generational depth & genealogy tracking** — `generation` badge in specimen detail header; `lineage_passage_offset` for cumulative passage count from root; `root_specimen_id` for efficient family-tree queries; `get_specimen_family` command; sibling display in lineage banner; migration 010 (v1.7.0)
- [x] **Split workflow overhaul** — letter-suffix accessions (001A/001B/001AA…), per-child configuration cards (health, stage, location, media batch, vessel, notes, reminder), draft media batches (migration 011), safety confirmation dialog, synthetic split timeline events (purple dashed cards), lineage bar includes archived children, Back button navigation history stack (v1.8.0)

### v1.9.0 — v1.11.0 — Completed

- [x] **Trust Layer Polish (WP-19)** — contamination inheritance on split (children inherit parent's `contamination_flag` + notes; audit entry reflects inheritance); **Verify All Lineages** batch button in Audit Log; cleaner per-lineage verification messages (v1.9.0)
- [x] **Merkle checkpoints (WP-20)** — binary Merkle tree over per-lineage `entry_hash` values; create/verify/list via admin UI and Tauri commands; three-stage tamper detection (count → root → per-entry content); [`docs/merkle-checkpoints.md`](docs/merkle-checkpoints.md) (v1.9.0)
- [x] **Merkle proof export & auto-checkpointing (WP-21)** — portable `PortableMerkleProof` JSON per checkpoint; `verify_exported_proof` command for offline auditors; standalone Python verifier; configurable auto-checkpointing on entry-count threshold and pre-backup; [`docs/merkle-proofs.md`](docs/merkle-proofs.md) — **Trust Layer Phase 1 complete** (v1.10.0)
- [x] **Lab Profile concept (WP-22)** — `app_config` single-row table with `lab_profile` (`plant_tissue_culture | cell_culture | mycology`); admin-only write; `profile.ts` Svelte store (v1.11.0)
- [x] **Dead specimen workflow** — "☠ Record Death & Archive" action when health slider hits 0; `record_specimen_death` command (archives specimen + inserts `event_type = 'death'` subculture row + writes audit entry); death event card in passage timeline with skull icon; "Dead / Archived" red badge; passage count excludes death events (v1.11.0)
- [x] **Profile-scoped vocabulary tables (WP-23 / WP-24)** — `stages`, `propagation_methods`, `hormone_types`, `compliance_record_types`, `compliance_agencies`, `inventory_categories` tables; all six `list_*` Tauri commands; form dropdowns driven from vocabulary (v1.12.0)
- [x] **Profile-aware dashboard statistics (WP-25)** — "Specimens by Stage" breakdown, Contamination Overview, and Subculture Schedule all filtered through the `stages` vocabulary for the active lab profile; vocabulary labels (e.g. "Shoot Meristem") replace raw stage codes in the stage chart; `db::dashboard` module with 11 unit tests; no hardcoded stage lists remain in dashboard queries (v1.13.0)
- [x] **Lab Profile Switcher in Settings (WP-26)** — new admin-only Settings view (sidebar gear icon) with a profile dropdown, warning banner, and mandatory `CHANGE PROFILE` confirmation before switching; when the lab is empty no confirmation is required; after a successful switch the `labProfile` Svelte store updates immediately so the whole app reacts; `check_profile_change_allowed` helper with 7 Rust unit tests; 6 TypeScript store tests (v1.14.0)

### Planned

- [ ] **Interactive lab map** — floor plan overlay with specimen location heat-map and drag-to-move

### v1.16.0–v1.17.0 — Taxonomic & Provenance Module Phase 1 (Phase TX-1) — ✅ Complete

- [x] **Strain/Cultivar Registry** — Strains as first-class entities under each species with their own SHA-256 hash chains seeded from the parent species hash (WP-28 · v1.16.0). Accession numbers never encode strain.
- [x] **Strain version binding** — specimens cryptographically bound to a specific strain version (`strain_chain_seq`) at creation time; strain version pill in specimen detail header `[CODE · v{chain_seq} · STATUS]` (WP-28 backend v1.16.0 · WP-29 UI v1.17.0)
- [x] **Strain status workflow** — four-value model: `Unverified` (default) → `Claimed` (one-click assertion) → `Confirmed — Manual` (requires documented basis; downgrades permanently rejected) → `Confirmed — Genomic` (requires fingerprint data; gold standard); full audit trail per status change **(WP-28 v1.16.0)**
- [x] **Strain management UI** — per-species strain list with status badges, specimen counts, nudge behavior (30-day unverified pulse), full CRUD, `confirmed_manual` non-dismissible blocking modal **(WP-29 v1.17.0)**
- [x] **Hybrid creation wizard** — `hybridization_events` model + 8-step `HybridWizard.svelte`; intraspecific-only; atomic transaction writes hybrid strain + 2 parent rows + 4 audit entries; pedigree preview (WP-28 backend v1.16.0 · WP-29 UI v1.17.0)
- [x] **Taxonomy Navigator** — two-column `TaxonomyNavigator.svelte` (Species → Strains → Specimens) with live search, status filter, slide-in specimen panel, inline StrainManager, and "Taxonomy" sidebar entry **(WP-29 v1.17.0)**
- [x] **Hybrid pedigree foundation** — `strain_parents` table supporting multi-parent pedigree from day one; `strain_chain_seq` on specimens version-binds each specimen to the strain state at creation **(WP-28 v1.16.0)**

### v2.1.0+ — Multi-Vertical & Taxonomy Expansion

- [x] **Phase C WP-26–27 complete** — lab profile switcher in Settings with confirmation flow (v1.14.0); `cell_culture` vocabulary seeded across all six lookup tables (v1.15.0) — Phase C fully complete
- [x] **Phase TX-1 complete** — Strain Manager UI, Hybrid Wizard, and Taxonomy Navigator shipped (WP-28 backend v1.16.0 + WP-29 UI v1.17.0) — Phase TX-1 fully complete
- [x] **Phase TX-2 WP-35 — Expanded taxonomy backbone (Genus → Kingdom)** — `taxa` table (migration 020) for Kingdom → Phylum → Class → Order → Family → Genus hierarchy; `taxon_path` JSON + `ncbi_taxon_id` on `species`; automatic genus backfill from existing species; `get_taxon_descendants` recursive tree command; TypeScript `TaxonNode` interface; no hash chains above Species by design **(v1.18.0)**
- [ ] **Phase TX-2 remaining** — NCBI Taxonomy import + sync with conflict resolution (WP-36), multi-generational pedigree tools (WP-37), advanced hybridization (WP-38), full-rank Taxonomy Navigator with filtering and descendant counts (WP-39)
- [ ] **SteloCC (Cell Culture)** — cell line registry, passage number / PDL tracking, cryopreservation & LN2 inventory, mycoplasma compliance rules (v2.0.0 target); benefits from Phase TX generic taxonomy engine
- [ ] **SteloMyco (Mycology)** — strain/isolate registry, substrate composition (v2.1.0 target); Phase TX strain model maps directly to mycology strain concepts; `mycology` vocabulary now seeded (stages, propagation methods, substrate supplements, compliance types, agencies, inventory categories — v1.28.0); colonization % and contaminant type tracking now live (v1.29.0); genetic lineage markers (`origin_type`, `is_best_performer`) and best-performer filter now live (v1.30.0); **fruiting conditions & yield** (`fruiting_records` table, flush entry form, harvest data table in Specimen Detail) now live (v1.31.0); **mycology QC compliance rules** (open contamination, overdue transfer, slow colonization flags + dashboard panel) now live (v1.32.0)
- [ ] **PostgreSQL backend** — drop-in replacement for the SQLite connection for LAN/server deployments with concurrent multi-user writes
- [ ] **Network sync** — real-time specimen and inventory updates across multiple desktop and Android clients on the same LAN
- [ ] **Email / push notifications** — reminder delivery and overdue subculture alerts via SMTP or push service
- [ ] **iOS support** — Tauri 2 iOS target with the same responsive UI as Android
- [ ] **Environmental monitoring integration** — link temperature/humidity sensor readings directly to passage records
- [ ] **Role-based field-level permissions** — hide or lock sensitive fields (IP flags, provenance) by role

### Beyond v2.x

- [ ] **Phase TX-3 — Advanced taxonomy** — cross-domain support (Plantae/Animalia/Fungi/Bacteria profile vocabularies), breeding programs & multi-generational selection tracking, cross-species hybridization with admin override, custom taxa & Darwin Core export (WP-46–49); full Kingdom → Strain hash chain extension (WP-45) is optional/not scheduled pending resolution of the reclassification problem
- [ ] **On-chain anchoring** — publish checkpoint Merkle roots to Dogecoin via `OP_RETURN` for third-party tamper-evidence (WP-65+)
- [ ] **Species-level analytics** — growth curves, passage success rates, and media comparison charts across experiments; strain-level analytics comparing performance across cultivars
- [ ] **Local AI analysis** — NLP summaries of observation notes; image-based contamination detection from passage photos
- [ ] **Offline-first with sync** — full local operation with background sync when a server is available
- [ ] **Protocol templates and SOPs** — attach standard operating procedure documents to species and media recipes

---

## License

This software is proprietary. See [LICENSE](./LICENSE) for the full commercial license agreement. Contact licensing@stelolab.local for purchasing information.
