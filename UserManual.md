# SteloPTC User Manual

> **The end-to-end guide for lab staff.** Every workflow, step by step.
> For release history see [`CHANGELOG.md`](CHANGELOG.md); for per-feature engineering status see
> [`ROADMAP.md`](ROADMAP.md); for a one-page product overview see [`README.md`](README.md).

| | |
|---|---|
| **Applies to** | **v1.53.2** (July 2026) |
| **Covers** | The complete shipped product — core workflows, the Trust Layer, Phase F cross-cutting features, Phase G federated exchange, and Phase H compliance & integrity |
| **Audience** | Lab technicians, supervisors, and lab admins |

SteloPTC is a desktop and Android application for managing plant tissue culture, cell culture, and
mycology laboratories, with a strong focus on **provenance, traceability, and cryptographic data
integrity**. It combines traditional lab record-keeping with an immutable, hash-chained audit trail,
so you can trace — and *prove* — the complete history of any culture, even many generations and
splits later.

> **A note on honesty.** A handful of capabilities ship deliberately incomplete and are labelled
> **Current limitation** wherever they appear in this manual: the PostgreSQL backend, LAN sync
> transport, S3/SFTP cloud-backup targets, plugin WASM rule execution, iOS, and automatic on-chain
> broadcast. Everything else described here is live and usable today.

---

## Table of Contents

**Core workflows**

1. [Introduction & Overview](#1-introduction--overview)
2. [Core Concepts](#2-core-concepts)
3. [Getting Started](#3-getting-started)
4. [Managing Species](#4-managing-species)
5. [Managing Strains & Cultivars](#5-managing-strains--cultivars-phase-tx-1--shipped-v1160v1170)
6. [Taxonomy Navigator](#6-taxonomy-navigator-phase-tx-fully-shipped--v1170v1220)
7. [Working with Specimens](#7-working-with-specimens)
8. [Splitting Cultures (Detailed)](#8-splitting-cultures-detailed)
9. [Recording Passages / Subcultures](#9-recording-passages--subcultures) *(incl. dead specimen / archive workflow)*
10. [The Audit Log & Cryptographic Hash Chain](#10-the-audit-log--cryptographic-hash-chain)
11. [Provenance & Genealogy Tracking](#11-provenance--genealogy-tracking)
12. [Reminders & Follow-ups](#12-reminders--follow-ups)
13. [Importing & Exporting Data](#13-importing--exporting-data)
14. [Printing Reports](#14-printing-reports)
15. [Understanding the Hash Chain & Data Integrity (Advanced)](#15-understanding-the-hash-chain--data-integrity-advanced)
16. [Troubleshooting & Common Issues](#16-troubleshooting--common-issues)
17. [Best Practices for Tissue Culture Tracking](#17-best-practices-for-tissue-culture-tracking)
18. [Future Features & Roadmap](#18-future-features--roadmap)

**Cross-cutting features** *(Phase F — v1.38.0–v1.41.0)*

19. [Local AI Assistant (Ollama & LocalAI)](#19-local-ai-assistant-ollama--localai)
20. [Interactive Lab Map](#20-interactive-lab-map)
21. [Analytics Dashboard](#21-analytics-dashboard)
22. [Encrypted Cloud Backup & Multi-Device Sync](#22-encrypted-cloud-backup--multi-device-sync)
23. [Regulatory Compliance Exports (FDA / USDA / CITES)](#23-regulatory-compliance-exports-fda--usda--cites)
24. [Plugin Manager](#24-plugin-manager)
25. [Notifications & Environmental Monitoring](#25-notifications--environmental-monitoring)
26. [Installable Web App (PWA)](#26-installable-web-app-pwa)

**Trust Layer, federated exchange & compliance** *(v1.42.0–v1.53.2)*

27. [On-Chain Anchoring & the Signed Event Ledger](#27-on-chain-anchoring--the-signed-event-ledger)
28. [Working with Partner Labs — Passports, Taxonomy Registry & Breeding Coordination](#28-working-with-partner-labs--passports-taxonomy-registry--breeding-coordination)
29. [Compliance Flags, Rules & Waivers](#29-compliance-flags-rules--waivers)
30. [The Regulatory Submission Pipeline](#30-the-regulatory-submission-pipeline)
31. [Data Integrity Self-Check](#31-data-integrity-self-check)
32. [Mycology: The Fruiting Overview](#32-mycology-the-fruiting-overview)

---

## 1. Introduction & Overview

SteloPTC helps labs maintain rigorous, auditable records of their tissue culture work. Every meaningful action — creating a specimen, recording a passage, splitting a culture, editing a species, or registering a strain — is logged in a cryptographic hash chain. This makes the history tamper-evident and gives you strong confidence in the provenance of your cultures.

The application is designed to be practical for day-to-day lab use while providing long-term value for research traceability, compliance, quality systems, and knowledge transfer.

**Core philosophy**
- Local-first and offline-capable
- Data integrity is non-negotiable
- Species act as protected cryptographic roots
- Splits create clear lineage branches while preserving full parent history
- Strain identity is version-bound at specimen creation time
- The system should make correct work easy and incorrect work visible

---

## 2. Core Concepts

### Lab Profiles (Plant Tissue Culture · Cell Culture · Mycology)

SteloPTC runs **one lab profile at a time**, chosen in **Settings → Lab Profile**. The
profile is the single switch that adapts the whole app to your discipline: it selects the
controlled vocabularies (stages, propagation/culture methods, contaminant types, inventory
categories) and the biological **domain** (Plantae / Animalia / Fungi) the taxonomy tools
use. The same engine — accessioning, hash chain, splits & passages, compliance, audit —
serves all three; only the vocabulary and a few discipline-specific fields change.

| Profile | Domain | A "specimen" is… | Example stages | Discipline-specific tracking |
|---|---|---|---|---|
| **Plant Tissue Culture** *(default)* | Plantae | an explant/culture on media | explant, callus, suspension, shoot, plantlet | MS/WPM media batches, auxin/cytokinin hormones, subculture passages |
| **Cell Culture** | Animalia | a cell line / passage in flasks | adherent, suspension, characterization, cryopreserved | Population Doubling Level (PDL), doubling time, cryopreservation vials, mycoplasma & biosafety level |
| **Mycology** | Fungi | a fungal culture / grow | liquid culture, bulk substrate, colonizing, fruiting | colonization %, contaminant typing, fruiting/flush yield records |

> **Terminology note.** Some shared building blocks keep a plant-tissue-culture name even
> in other profiles — most visibly, every growth event is stored as a **passage/subculture**
> record (in cell culture it *is* a passage; in mycology it doubles as a colonization
> reading). The UI relabels discipline-specific fields per profile, but the underlying record
> type is shared by design.
>
> Switching profiles is **audit-logged** and does **not** rewrite existing specimens — a
> record keeps the vocabulary values it was created with. Run one discipline per database, or
> add further profiles with **plugin vocabulary packs** (see §24).

### Specimens

A specimen is an individual culture in your lab. Each specimen has:

- A unique **accession number** (stable identifier for the physical culture lineage)
- A **species** (the cryptographic root)
- Optional **strain** binding (with version recorded at creation time)
- Current **stage**, **health**, and **location**
- A complete history of **passages** (subcultures)
- Links to its **parent** (if created via split) and **siblings** (other cultures created in the same split)

### Species

Species are the foundation of the system. When you create a new species, it starts its own hash chain. Every specimen or strain created from that species inherits the species hash as its starting point. This creates a permanent cryptographic link between the species definition and every culture derived from it.

Because of this role, species become **very protected** once they have been used to create any specimens or strains.

### Strains & Cultivars (Shipped — Phase TX-1, v1.16.0–v1.17.0)

A **strain** (cultivar, variety, clone, etc.) is a named genetic variant of a species. Strains provide a precise layer of identity between the species and individual specimens.

Key design principle: **Accession numbers and strain identity are permanently separate.** The accession number identifies the culture lineage and never encodes the strain. Strain identity is recorded as supplemental metadata and version-bound at the moment of specimen creation.

This separation ensures accession numbers remain stable and unambiguous even if strain classifications are later corrected or upgraded.

### Passage vs. Split

These are the two primary ways you advance cultures in SteloPTC.

**Passage (Subculture)**
- The same specimen record continues.
- Its chain sequence (`chain_seq`) increments.
- The accession number never changes.
- Used for routine maintenance and growth.

**Split**
- The parent specimen is archived.
- One or more new child specimens are created.
- Each child starts a fresh lineage (chain sequence resets to 1).
- New suffixed accession numbers are generated (e.g. `001A`, `001B`, `001BA`).
- You can configure each child independently.
- The parent’s full history remains visible and linked.

> **Important:** Before confirming a split, you will always see a verification warning reminding you to physically check that your labels match the software-generated accessions.

### Quick Reference: Passage vs. Split

| Aspect                    | Passage (Subculture)                  | Split                                      |
|---------------------------|---------------------------------------|--------------------------------------------|
| Parent record             | Continues                             | Archived                                   |
| New specimens created     | No                                    | Yes                                        |
| Accession number          | Unchanged                             | New suffixed numbers                       |
| Lineage chain             | Continues on same specimen            | Each child starts fresh                    |
| Per-child configuration   | N/A                                   | Yes                                        |
| Typical use case          | Routine maintenance                   | Creating independent lines                 |

Use **Passage** for normal upkeep. Use **Split** when you want to divide a culture into multiple separate lines.

### Lineage & Hash Chain

Every significant action is recorded in the Audit Log as part of a SHA-256 hash chain. This creates an append-only, tamper-evident record. Changing any historical entry breaks all subsequent hashes.

When you split a specimen, each new child starts its own independent lineage while maintaining a cryptographic link back to the parent.

### Genealogy & Provenance

SteloPTC tracks:

- **Generation number**
- **Root specimen**
- **Siblings** (created in the same split)

This information appears in the specimen detail view and helps you understand relationships even many passages later.

---

## 3. Getting Started

### First Launch

On a fresh database you will see the First-Run guidance panel. It walks you through configuring your species registry and accessioning your first specimen. You can also load demo data to explore the interface.

### UI Navigation Tips

- **Sidebar**: Main navigation (Dashboard, Specimens, Species, Media, Vessels, Reminders, Audit Log, Admin).
- **Specimen list**: Click any row to open detail view. Use filters and search.
- **Specimen detail**: Sections for status, history, siblings, reminders, and audit entries.

### Creating Your First Species and Specimen

1. Go to **Species** → **+ New Species**.
2. Enter Species Code and scientific name, then create.
3. Go to **Specimens** → **+ New Specimen**.
4. Select the species, fill in initial details, and create.

The specimen is immediately linked to the species’ cryptographic root.

---

## 4. Managing Species

Species are the cryptographic foundation of your collection.

### Creating, Editing, and Archiving

- New species start their own hash chain.
- Editing a **never-used** species is straightforward.
- Editing a species that has been used triggers stronger protections (warnings and confirmations).
- **Archive** is recommended for used species. Hard delete is only allowed for unused species.

Archived species remain visible in historical views but cannot be used for new specimens.

### Why Species Protection Matters

Every specimen inherits its species’ hash at creation time. Protecting the species record protects the integrity of every derived culture.

---

## 5. Managing Strains & Cultivars (Phase TX-1 — shipped v1.16.0–v1.17.0)

> **Note:** Strain management shipped in Phase TX-1. The backend data model landed in v1.16.0 (WP-28); the full UI — Strain Manager, Hybrid Wizard, and Taxonomy Navigator — shipped in v1.17.0 (WP-29). All features described below are available in the current shipping version.

Strains give you a precise layer of genetic identity between species and individual specimens.

### Key Design Decisions

- Accession numbers identify the **culture lineage** and are stable.
- Strain identity is recorded as **supplemental metadata** and version-bound at specimen creation time (`strain_chain_seq`).
- This separation ensures accession numbers remain unambiguous even if strain classifications change later.

### Strain Status Model

SteloPTC uses a four-level status model:

- `Unverified` (default)
- `Claimed` (low-friction assertion)
- `Confirmed — Manual` (high friction, documented basis + acknowledgment)
- `Confirmed — Genomic` (gold standard, requires fingerprint data)

Downgrades from Confirmed statuses are not permitted. Incorrect identities should be archived and replaced.

### Hybridization

Hybridization is modeled as a distinct event (not a passage or split). It creates a `hybridization_events` record that captures both parents and their exact chain versions at the time of crossing, writing bidirectional audit entries.

---

## 6. Taxonomy Navigator (Phase TX fully shipped — v1.17.0–v1.22.0)

> **Note:** The basic Species → Strains → Specimens navigator shipped in v1.17.0 (WP-29). The taxonomy backbone (`taxa` table for Kingdom → Genus hierarchy, `get_taxon_descendants`) shipped in v1.18.0 (WP-35). The advanced multi-column Kingdom → Species → Strains browser with global search, keyboard navigation, and localStorage path persistence shipped in v1.22.0 (WP-39). Phase TX-2 is complete.

The Taxonomy Navigator lets you browse your collection hierarchically instead of using a flat specimen list. It is especially useful as your collection grows.

---

## 7. Working with Specimens

### Creating, Viewing, and Updating

- Create via **Specimens → + New Specimen**.
- Open any specimen from the list to see status, generation, siblings, full history, reminders, and linked audit entries.
- Update location, health, stage, etc. at any time — all changes are logged.

---

## 8. Splitting Cultures (Detailed)

Splitting is one of the most important and carefully designed workflows in SteloPTC.

### How Splitting Works

When you split a specimen:

- The **parent is archived**.
- New **child specimens** are created.
- Each child starts its own independent lineage (chain resets to 1).
- New suffixed accession numbers are generated.
- You can configure each child independently (health, stage, media, vessel, notes, reminders).
- The parent’s complete history remains visible and linked.
- A contamination flag, if used, applies to the **parent**.

### Performing a Split — Realistic Example

**Example:** Split `2026-06-18-CAN-SAT-001` into two children.

1. Open the specimen and click **Passage / Split**.
2. Check “Split culture into multiple child specimens” and choose 2 children.
3. Review/edit the suggested accessions (`...001A` and `...001B`).
4. Configure each child (health, media, location, reminders).
5. Review the live summary.
6. Click **Confirm Split**.
7. A verification warning appears — physically verify labels before proceeding.

After confirmation the parent is archived and two new active specimens (Gen 1, siblings) are created with fresh lineages.

### Accession Numbers

Children receive suffixed accessions (e.g. `001A`, `001B`). Further splits continue recursively (`001B` → `001BA`). You can manually edit generated accessions during the split.

### Draft Media & Vessels

“Add new” creates a draft/placeholder record (`is_draft = true`). Complete the details later in the Media or Vessel management area before using the record in other actions.

### Best Practices for Splitting

- Use per-child fields when quality or timing differs between children.
- Always respect the verification warning.
- After splitting, check the sibling list and root lineage on the new children.

---

## 9. Recording Passages / Subcultures

A passage continues the same specimen (no archiving, no new children). The chain sequence increments and the accession number stays the same.

Record via **Passage / Split** with the split checkbox **unchecked**.

### Dead Specimen / Archive Workflow (v1.11.0)

When a specimen has died, slide the **health slider to 0 (Dead)**. The form responds immediately:

- The primary action button changes to **☠ Record Death & Archive**.
- A red warning banner confirms this is a **terminal, irreversible** action.

Clicking the button:
1. Archives the specimen (`is_archived = true`, health permanently at 0).
2. Inserts a terminal subculture row with `event_type = 'death'` (does **not** increment the passage count).
3. Writes a `"death"` audit entry to the hash chain.

After archiving, the specimen:
- Shows a red **Dead / Archived** badge instead of the generic grey archived badge.
- Displays a distinct red death event card with skull icon in the passage timeline.
- Blocks all further passage recording.
- Excludes the death event from the displayed passage count.

> **Note:** Dead specimens are permanently archived. If a specimen was incorrectly marked dead, contact an admin — there is no automated un-archive path.

---

## 10. The Audit Log & Cryptographic Hash Chain

The Audit Log records nearly every meaningful action and protects history with SHA-256 hashing. Each entry links to the previous one, forming a continuous, verifiable chain.

You can filter, view hashes, and verify individual rows or entire lineages from the Audit Log view. Verification failures clearly indicate the first broken link.

The Audit Log view also hosts the advanced Trust Layer panels: **Merkle checkpoints** and portable proofs, **On-Chain Anchoring** (Dogecoin), the **Signed Event Ledger** (per-user signatures), and **Specimen Passports** (see below).

### Specimen Passports — inter-lab transfer (v1.45.0)

When you send tissue-culture material to another lab, you can issue a **specimen passport**: a signed file that carries the specimen's identity and its full, tamper-evident provenance. Use the **Issue Passport** button on a specimen's detail page (or the Specimen Passports panel in the Audit Log) to download the passport as JSON, then send it to the receiving lab through your usual channel.

The receiving lab opens the **Audit Log → Specimen Passports** panel, pastes or loads the file, and clicks **Verify** — they can confirm, using only your published public key and the data in the file, that the passport is genuine and its provenance is intact, *without any access to your database*. **Verify & Import** then folds the passport into the receiving lab's own audit chain, creating a permanent, hashed record that they received and accepted it. Each lab sets its own name and shares its public key under **This lab's issuer identity** in the same panel. See [`docs/specimen-passport.md`](docs/specimen-passport.md) for the full format and a standalone verifier.

---

## 11. Provenance & Genealogy Tracking

SteloPTC tracks generation, root specimen, and siblings. This information appears in specimen headers and detail views, making it easy to understand relationships even many passages later.

---

## 12. Reminders & Follow-ups

Create reminders manually or automatically during passages and splits (per-child reminders supported). Manage them from the Reminders sidebar or specimen detail pages. Use them consistently for time-sensitive work.

---

## 13. Importing & Exporting Data

Export specimens, media, audit logs, and full backups in CSV or JSON. Import via Excel with dry-run preview and validation before commit. Imported records affecting the audit log are clearly marked.

---

## 14. Printing Reports

Generate professional Specimens Summary Reports (grouped views, executive summaries, individual details) and QR specimen labels. Use Print Summary from the Specimens list and review the preview before printing.

---

## 15. Understanding the Hash Chain & Data Integrity (Advanced)

Traditional lab software allows silent edits or deletions. SteloPTC’s hash chain makes any change to historical records detectable. Every important action creates a linked audit entry. You can trust that passage and split history has not been secretly altered.

The chain protects against *undetected* changes. It does not prevent authorized changes — it only makes them visible and verifiable.

---

## 16. Troubleshooting & Common Issues

**“I can’t delete a species”** — Hard delete is only allowed for unused species. Archive used species instead.

**Accession numbers look wrong after splitting** — You can manually edit generated suffixed accessions during the split confirmation step.

**Verification fails in the Audit Log** — Indicates out-of-band editing or corruption. The system pinpoints the first broken link.

**“Add New” media/vessel issues** — Draft records must be completed in the Media/Vessel area before they can be used in other actions.

**Strain-related issues (Phase TX)** — See specific notes in Section 5 for status upgrade requirements and version binding behavior.

---

## 17. Best Practices for Tissue Culture Tracking

- Be consistent with accession suffix conventions.
- Record passages promptly.
- Use per-child configuration during splits.
- Double-check physical labeling before confirming splits.
- Archive instead of deleting when possible.
- Review the Audit Log periodically.
- Use reminders actively.
- Check lineage (siblings + root) after every split.

---

## 18. Future Features & Roadmap

Everything below was still "planned" the last time this section was rewritten; nearly all of it has since shipped. It's kept here as a landmark, corrected against the current state — see `ROADMAP.md` for full detail on every work packet (`WP-xx`) and `CHANGELOG.md` for release-by-release history.

### Already shipped (was "planned" here previously)
- Strain/Cultivar registry, hash chain version binding, four-level status model, Hybrid Wizard, Taxonomy Navigator — Phase TX-1/TX-2/TX-3, v1.16.0–v1.37.0
- Cell Culture and Mycology lab profiles — Phase D/E, v1.23.0–v1.32.0
- Local AI note/passage-comment assistance (Ollama, human-approval-gated) — WP-56, v1.40.0
- Environmental sensor logging (manual entry; hardware transport still not wired — see below) — WP-54, v1.39.0
- iOS build scaffold (still unverified end-to-end — see below) — WP-53, v1.39.0
- Interactive lab map, analytics dashboards, encrypted cloud backup, regulatory compliance exports, plugin system, installable PWA, taxon chain re-anchoring — Phase F WP-57–65, v1.40.0
- **On-chain anchoring** (Dogecoin `OP_RETURN`) — Trust Layer Phase 2, WP-66, v1.42.0 (prepares and independently verifies a checkpoint's Merkle root on-chain; you broadcast with your own external wallet) — see [§27](#27-on-chain-anchoring--the-signed-event-ledger)
- **Signed-event ledger** (specimen events as Ed25519-signed, hash-chained ledger transactions) — Trust Layer Phase 3, WP-67, v1.43.0, extended across passages, splits, death and archival by WP-75 (v1.50.0, completed in v1.53.2) — see [§27](#27-on-chain-anchoring--the-signed-event-ledger)
- **Automated regulatory submission pipeline** (monitors compliance state and auto-generates signed, ready-to-submit packages) — WP-68, v1.44.0 — see [§30](#30-the-regulatory-submission-pipeline)
- **Federated inter-lab exchange** — specimen passports (WP-70, v1.45.0), the shared taxonomy registry (WP-71, v1.46.0), and cross-lab breeding coordination (WP-72, v1.47.0). **Phase G complete** — see [§28](#28-working-with-partner-labs--passports-taxonomy-registry--breeding-coordination)
- **Profile-aware compliance rules and flag waivers** — WP-74 (v1.49.0) and WP-77 (v1.52.0). A rule now declares which lab profiles it applies to, so the citrus HLB rule no longer fires in a mycology or cell-culture lab — see [§29](#29-compliance-flags-rules--waivers)
- **Data-integrity self-check** — WP-76, v1.51.0 — see [§31](#31-data-integrity-self-check)
- **Environmental out-of-range monitoring** — WP-78, v1.53.0, delivered as a rule inside the WP-74 engine — see [§29](#29-compliance-flags-rules--waivers)
- **Fruiting overview (Mycology)** — WP-73, v1.48.0 — see [§32](#32-mycology-the-fruiting-overview)

### Genuinely still planned / incomplete
- **Automatic on-chain broadcast** — the anchor payload is prepared and verified (WP-66), but sending the transaction still requires your own funded external wallet; no funded-wallet transport is bundled
- **PostgreSQL as a live backend** — connector compiles and unit-tests but has never been run against a real PostgreSQL server; SQLite remains the only backend a lab can actually use
- **LAN sync transport** — change-detection and conflict-recording exist, but there is no network transport or automatic merge yet
- **iOS end-to-end verification** — the build workflow has never completed a real device/simulator build (no Apple Developer access in CI)
- **Sensor hardware transport** (USB/BLE/MQTT) — only manual entry is wired up today
- **Cloud backup to S3/SFTP** — configurable today but not connected; only `local_nas`/`smb` targets work
- **Plugin WASM rule execution** — plugin manifests are validated and recorded, but a plugin's compliance rules are not yet executed by a sandboxed runtime
- **A network transport for the federated exchanges** — passports, registries, and coordination bundles are files you send through your own channel; there is no built-in lab-to-lab connection
- **Per-lab configurable compliance thresholds** — the environmental ranges and re-test intervals are sensible defaults, not yet editable in the UI
- **Signed events on every mutation** — the whole specimen lifecycle is signed today (creation, passages, splits, death, archival); mutations outside it (media, inventory, compliance records) are incremental follow-up

For the latest status, refer to `ROADMAP.md` in the repository.

---

## 19. Local AI Assistant (Ollama & LocalAI)

SteloPTC includes an **optional, fully on-device AI assistant**. It never sends your data to a cloud service — it talks only to a local model runtime you control.

### What it does

- **Summarize Notes** — condenses a specimen's notes into 2–3 sentences, preserving measurements, dates, and contamination observations.
- **Suggest Passage Comment** — drafts a factual observation for the next passage from the specimen's recent history.
- **Analyze Photo for Contamination** — examines an attached photo for visible microbial growth, discoloration, or turbidity (needs a vision-capable model).

The **Summarize Notes** and **Suggest Passage Comment** buttons appear in the notes area of the Specimen Detail view; **Analyze Photo** appears in the photo lightbox.

### Draft-and-approve — the AI never edits a record on its own

Every AI result is a **pending draft**. You review it and either **Approve** or **Reject** it. On approval, the text is *appended* (never overwrites) to the record's notes, tagged `[AI-assisted, approved by <you>]`, through the normal audit-logged edit path — so the change is attributed to you, with the model name and prompt preserved for traceability.

### Setting it up

1. Install [Ollama](https://ollama.com) and pull a text model and a vision model:
   ```
   ollama pull llama3.1
   ollama pull llava
   ```
2. Open **Settings → AI Assistant** (admin/supervisor). Choose the **Ollama** runtime, then click **Test Connection**. A green result confirms the runtime is reachable and lists your installed models, with a ✓ next to your configured text and vision models.

Prefer an existing OpenAI-compatible server? Choose the **LocalAI** runtime instead and point the base URL at it. Full instructions, model recommendations, and troubleshooting are in **`docs/local-ai.md`**.

> If no runtime is configured or running, the AI buttons simply report that the model is unreachable — every manual workflow is unaffected.

---

## 20. Interactive Lab Map

The lab map gives you a visual floor plan of your lab with a pin for each location.

- Upload a floor-plan image and drop a pin for each **location** (this is purely additive — your existing free-text Room / Rack / Shelf / Tray fields keep working exactly as before).
- Toggle a **heat map** to shade pins by specimen **density**, **contamination risk**, or **age**, so hotspots are obvious at a glance.
- Click a pin to manage the specimens at that location.
- A compact lab-map overview also appears as a **Dashboard** widget.

The map is optional; if you don't configure a floor plan, nothing changes about how you enter locations.

---

## 21. Analytics Dashboard

A dedicated **Analytics** view surfaces lab performance over time:

- A **KPI strip**: active specimens, passages this week, contamination rate, throughput, and growth trend.
- A **time-range selector** (30 days / 90 days / 1 year / all).
- **Trend charts** for growth rate, subculture frequency, contamination rate, passage success, and media efficiency.
- A sortable **Strain Performance** comparison.
- A **Technician Activity** report (supervisor/admin only, framed as workload visibility).
- A one-click **multi-sheet Excel export** of the analytics.

Charts are drawn inline — no external charting service is contacted.

---

## 22. Encrypted Cloud Backup & Multi-Device Sync

Beyond the on-demand local backup (Dashboard), SteloPTC can back up to an **encrypted offsite target**, configured in **Settings → Cloud Backup** (admin only).

- **Zero-knowledge encryption.** Backups are encrypted client-side with Argon2id + AES-256-GCM before they ever leave the machine. **Your passphrase is never stored** — you re-enter it for every backup, restore, or sync.
- **Target types.** **Local / Network Share (NAS)** and **SMB** are fully live for backup, restore, and multi-device sync. **S3** and **SFTP** can be configured today but return a clear "not yet connected" message (no network client was added yet).
- **Restore** uses the same two-step, type-`RESTORE`-to-confirm destructive flow as the local restore; the app restarts on success.
- **Sync** (NAS/SMB) reconciles changes between devices using the audit hash chain; genuine conflicts are recorded durably for review rather than silently merged.

---

## 23. Regulatory Compliance Exports (FDA / USDA / CITES)

From **Compliance → Regulatory Export** (supervisor/admin), you can generate agency-ready bundles:

- **FDA 21 CFR Part 11** — a signed attestation bundle (Ed25519 digital signature) suitable for electronic-records compliance.
- **USDA APHIS PPQ Form 526** — a pre-filled permit export.
- **CITES Species Provenance Dossier** — a chain-of-custody dossier combined with the Darwin Core taxonomy export.

Each bundle is generated from your existing records; see **`docs/regulatory-exports.md`** for the exact contents and formats.

---

## 24. Plugin Manager

SteloPTC can be extended with **plugin vocabulary packs** (`.steloplugin` files — JSON manifests) that add a new lab profile with its own seeded vocabulary (stages, methods, categories, etc.).

- Install and remove plugins from **Settings → Plugin Manager** (admin only).
- Vocabulary seeding is **idempotent and profile-isolated** — installing a plugin never disturbs existing profiles, and uninstalling never rolls back seeded vocabulary.
- **Current limitation:** a plugin's compliance *rules* (WASM) are validated and recorded but **not yet executed** — only the vocabulary is active in this release.

The manifest format and a worked example are documented in **`docs/plugin-authoring.md`**.

---

## 25. Notifications & Environmental Monitoring

### Notifications

SteloPTC can raise **desktop notifications** and send **email digests** for due/overdue work, driven by a background scheduler. Configure email delivery under **Settings → Email (SMTP) Configuration**.

> **Security note:** the SMTP password is currently stored **unencrypted** in the local database (there is no OS-keychain integration yet). It is redacted from all backups, and you should use a dedicated, least-privilege mail account. This caveat is shown directly in the Settings panel.

### Environmental monitoring

You can log **environmental sensor readings** (temperature, humidity, etc.) manually. Readings display as **sparklines** with **threshold alerts** when a value goes out of range. **Current limitation:** automatic hardware ingestion (USB/BLE/MQTT) is not yet wired — entry is manual for now.

---

## 26. Installable Web App (PWA)

The web build of SteloPTC is **installable** as a Progressive Web App (via your browser's "Add to Home Screen"/install prompt) with an offline-capable shell.

**What works in the PWA today:** all read views (Dashboard, Specimen list/detail, Analytics, Audit Log, etc.) once cached, and installability.

**What still requires the desktop app:** any **data mutation**, QR camera scanning, native file access (attachments, local backup/restore), OS print, and desktop notifications. SteloPTC's command layer is desktop-native (Tauri IPC); a browser-only install is a **read-only shell** until a remote API server exists.

The service worker is deliberately gated so it **never** activates inside the desktop app — installing or using the PWA cannot affect the desktop experience.

---

## 27. On-Chain Anchoring & the Signed Event Ledger

Sections 10 and 15 cover the hash chain that makes your history tamper-**evident**. Two further
panels, both in the **Audit Log** view, strengthen that guarantee in different directions.

### On-chain anchoring — proving *when* (Trust Layer Phase 2)

A Merkle checkpoint summarises a whole range of audit history in a single 32-byte root. **On-Chain
Anchoring** publishes that root to the public Dogecoin blockchain in an `OP_RETURN` output, so
anyone — including a regulator or a party who has never seen your database — can confirm the
checkpoint existed at a particular point in time.

1. In **Audit Log → On-Chain Anchoring**, pick a checkpoint and click **Prepare**. SteloPTC builds
   the exact bytes to broadcast and shows you the payload and script.
2. **Broadcast the transaction yourself**, using your own external wallet. *Current limitation:* no
   funded wallet is bundled, so SteloPTC never spends anything on your behalf.
3. Paste the resulting **txid** back into the panel and click **Verify**. SteloPTC fetches the
   on-chain data and checks it against the checkpoint root independently — it trusts the block
   explorer for the raw bytes and nothing else.

The full byte format is documented in [`docs/on-chain-anchoring.md`](docs/on-chain-anchoring.md).

### The signed event ledger — proving *who* (Trust Layer Phase 3)

The hash chain proves history wasn't altered. The **signed event ledger** additionally proves *who
performed* each lifecycle event: every entry is signed with the acting user's own Ed25519 key, so an
entry's authorship cannot be forged by someone who can write to the database but doesn't hold that
key.

- Open **Audit Log → Signed Event Ledger** to browse events with their event type, entity, signer,
  and event hash.
- **Show My Signing Key** displays your own public key so a partner can verify events you signed.
- Events are signed automatically across the specimen lifecycle: **creation, passages, splits,
  recording a death, and archiving** (individually or in bulk). A death appends two events — the
  specimen died, and it was archived as a consequence — because those are two separate facts a
  partner or auditor may need to verify independently.
- *Current limitation:* mutations outside the specimen lifecycle (media batches, inventory,
  compliance records) are not yet signed. The ledger is a strict addition to the audit chain,
  which covers **every** mutation, and never a replacement for it.

See [`docs/signed-event-ledger.md`](docs/signed-event-ledger.md) for the exact format.

---

## 28. Working with Partner Labs — Passports, Taxonomy Registry & Breeding Coordination

SteloPTC extends its Trust Layer **across labs**, with no central authority and no account to sign
up for. Three exchanges are supported. All three share the same shape:

- What you send is a **signed, self-contained JSON file**. You send it however you already send
  files — email, shared drive, a USB stick. *Current limitation:* there is no built-in lab-to-lab
  network connection.
- The receiving lab **verifies it independently**, using only your published public key and the data
  inside the file. They need no access to your database.
- On import, the receiving lab folds the content into **its own audit chain**, so their records show
  permanently and verifiably what they accepted, from whom, and when.
- Imports are **additive and non-destructive**, and each runs as a single transaction — a rejected
  record can never leave a half-finished import behind.

Set your lab's name and view your public key under **This lab's issuer identity**, present at the
top of each panel.

| Exchange | Panel | What travels | Receiver's choice per record |
|---|---|---|---|
| **Specimen passport** (v1.45.0) | Audit Log → Specimen Passports | One specimen's identity + full provenance | Verify, then **Verify & Import** |
| **Taxonomy registry** (v1.46.0) | Audit Log → Taxonomy Registry | Your taxa, species and strains | **Accept / Override / Fork** |
| **Breeding coordination** (v1.47.0) | Audit Log → Breeding Coordination | One breeding program's selection records | **Accept / Skip** |

### Specimen passports

Covered in [§10](#10-the-audit-log--cryptographic-hash-chain) — issue one from a specimen's detail
page when you ship material to a partner.

### Shared taxonomy registry

**Export this lab's registry** produces a signed snapshot of your reference taxonomy. A partner
loads it under **Preview or import a received registry**, sees every record next to its **local
status**, and chooses a **disposition** for each: *accept* a record they don't have, *override*
their own version with yours, or *fork* it to keep both. Imported strains always arrive as
`unverified` — a partner's verification claim never silently becomes yours. Details in
[`docs/taxonomy-registry.md`](docs/taxonomy-registry.md).

### Cross-lab breeding coordination

When two labs run the same breeding program, **Export a breeding program's selection records**
produces a signed coordination bundle. The receiving lab previews it and accepts or skips each
record; the merge is a **set union**, so neither lab loses work.

- If the partner doesn't have the program yet, it is created as a **coordinated-copy shell**.
- A record whose strain isn't present locally is marked **blocked** until that strain is shared via
  the taxonomy registry.
- Merged records keep their **`origin_lab`**, so you can always tell which lab contributed what.

Details in [`docs/breeding-coordination.md`](docs/breeding-coordination.md).

---

## 29. Compliance Flags, Rules & Waivers

The **Compliance** view auto-flags specimens that need attention — expired permits, quarantine
status, overdue mycoplasma or mycology QC testing, citrus HLB screening, and environmental readings
that fall outside their acceptable range.

### Rules only fire in the labs they belong to

Each rule declares which **lab profiles** it applies to. A citrus HLB rule is a plant-tissue-culture
concern, so it no longer raises flags in a mycology or cell-culture lab; a mycoplasma rule is a
cell-culture concern. You don't configure this — switching your lab profile changes which rules are
live.

*Current limitation:* rule thresholds and re-test intervals are sensible built-in defaults and are
not yet editable in the UI.

### Environmental out-of-range monitoring

Readings logged under environmental monitoring ([§25](#25-notifications--environmental-monitoring))
are evaluated against per-type acceptable ranges and surface as ordinary compliance flags, so an
out-of-range incubator shows up in the same place as an expired permit.

### Waiving a flag

Some flags are legitimately not applicable to a particular specimen. Rather than editing a rule,
click **Waive** on the flag and record:

- a **reason** (required — this is the documented justification an auditor will read), and
- an optional **expiry date**, after which the flag reappears on its own.

Waived flags drop out of the active flag list. Existing waivers are listed with their specimen,
reason and expiry, and can be **revoked** at any time. Creating and revoking a waiver are both
written to the audit log and attributed to you — a waiver is a documented decision, never a way to
make a problem disappear quietly.

---

## 30. The Regulatory Submission Pipeline

Section 23 covers exports you generate on demand. The **submission pipeline** (Compliance →
Submission Pipeline, supervisor/admin) goes a step further: a background monitor re-evaluates your
compliance state on every scheduler tick and, once all preconditions for a submission type are met,
generates a **signed, ready-to-submit package** automatically.

- **Check Readiness** shows, for a chosen submission type, exactly which preconditions pass and
  which are still outstanding — so you can see what's blocking a submission before you attempt it.
- **Create Submission** builds the package now, using fields such as date range, lab name, specimen
  IDs, authorized scientist, root specimen ID and — for CITES — the relevant appendix.
- **Run Monitor** triggers the readiness sweep immediately instead of waiting for the next tick.
- The register lists every submission with its **status**, title and package reference; **Re-check**
  re-evaluates one.

*Current limitation:* SteloPTC prepares and signs the package. **Submitting it to the agency's
electronic portal is still a manual step** — nothing is transmitted to any authority on your behalf.

---

## 31. Data Integrity Self-Check

**Audit Log → 🩺 Data Integrity Self-Check** (administrator only) runs a read-only battery of checks
for corruption that a database can't retroactively catch on its own:

- **orphaned rows** — specimens, passages or strains whose parent record no longer exists,
- **broken lineage links** — a passage pointing at a deleted parent, a strain without a species,
- **duplicate accession numbers**, and
- **audit-lineage sequence gaps** — a missing history row.

Click **Run Integrity Check**. A pass reports `✓ All N checks passed`. A failure lists each issue
with its **severity**, a **count**, and **example** records so you know exactly where to look.

The check never modifies anything — it is a diagnostic, and repairs remain a deliberate,
audit-logged action you take yourself. Run it after a restore, after an import, or any time the
database has been handled outside SteloPTC.

---

## 32. Mycology: The Fruiting Overview

In a mycology lab, the sidebar gains a top-level **Fruiting** view (it is hidden in the plant
tissue-culture and cell-culture profiles — see [§2, Lab Profiles](#lab-profiles-plant-tissue-culture--cell-culture--mycology)).

It collects every fruiting record across the whole lab into one sortable table — **harvest date**,
**accession**, **species**, **flush** number and **notes** — so you can review yield across all
active cultures without opening them one at a time. Individual records are still created and edited
from each specimen's detail page.

---

*This manual is a living document and will be updated as features ship.*