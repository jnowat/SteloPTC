# SteloPTC User Manual

**Current as of:** June 2026 · **v1.30.0** (Trust Layer Phase 1 complete · Phase C fully shipped · Phase TX-1 fully shipped · Phase TX-2 fully shipped (WP-35–39, v1.18.0–v1.22.0: taxonomy backbone, NCBI sync, multi-generational pedigree, advanced hybridization tools, advanced Taxonomy Navigator) · Phase D Cell Culture fully shipped (WP-30–34, v1.23.0–v1.27.0: vocabulary, PDL tracking, cryostorage, mycoplasma compliance, cell culture dashboard) · Phase E Mycology WP-40–42 shipped (v1.28.0–v1.30.0: mycology vocabulary, colonization & contamination tracking, genetic lineage & strain isolation markers))

> **Scope note:** This manual documents both shipping features and planned functionality. Sections marked "Phase E WP-43+" describe upcoming work in the Mycology vertical. Phase TX-2 (WP-35–39: taxonomy backbone through advanced Taxonomy Navigator) is fully shipped as of v1.22.0. Phase D (Cell Culture: WP-30–34) is fully shipped as of v1.27.0. Phase E (Mycology: WP-40–42) is shipped as of v1.30.0 — WP-43 (fruiting conditions & yield) and WP-44 (mycology QC rules) remain planned. Core features such as the split/passage workflow, hash chain, dead specimen archiving, provenance tracking, and reminders are fully implemented and stable.

SteloPTC is a desktop application for managing plant tissue culture laboratories with a strong focus on **provenance, traceability, and cryptographic data integrity**.

It combines traditional lab record-keeping with an immutable, hash-chained audit trail so you can confidently trace the complete history of any culture — even many generations and splits later.

---

## Table of Contents

1. Introduction & Overview
2. Core Concepts
3. Getting Started
4. Managing Species
5. Managing Strains & Cultivars (Phase TX-1)
6. Taxonomy Navigator (Phase TX-1 / TX-2)
7. Working with Specimens
8. Splitting Cultures (Detailed)
9. Recording Passages / Subcultures (incl. Dead Specimen / Archive Workflow)
10. The Audit Log & Cryptographic Hash Chain
11. Provenance & Genealogy Tracking
12. Reminders & Follow-ups
13. Importing & Exporting Data
14. Printing Reports
15. Understanding the Hash Chain & Data Integrity (Advanced)
16. Troubleshooting & Common Issues
17. Best Practices for Tissue Culture Tracking
18. Future Features & Roadmap

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

### Strains & Cultivars (Planned — Phase TX-1)

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

## 6. Taxonomy Navigator (Phase TX-1 shipped v1.17.0 / TX-2 WP-35 backbone v1.18.0 / TX-2 UI planned)

> **Note:** The basic Species → Strains → Specimens navigator shipped in v1.17.0 (WP-29). The taxonomy backbone (`taxa` table for Kingdom → Genus hierarchy, `get_taxon_descendants` tree command, automatic genus backfill) shipped in v1.18.0 (WP-35). The full multi-rank column browser UI (WP-39) arrives in a later Phase TX-2 packet.

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

SteloPTC development continues along two main tracks: the **Phase C de-hardening** (making vocabulary data-driven for multi-vertical support) and the **Taxonomic & Provenance Module (Phase TX)**. The Trust Layer (WP-18–21) shipped completely in v1.9.0–v1.10.0; Phase C began with WP-22 (lab profile + dead specimen workflow) in v1.11.0.

### Phase TX-1 (v2.0.0 target)
- Strain/Cultivar registry with hash chain and version binding
- Four-level strain status model
- Hybridization as a distinct event with bidirectional audit entries
- Basic Taxonomy Navigator (Species → Strains → Specimens)

### Phase TX-2 (v2.x target)
- Full multi-rank taxonomy backbone
- Multi-generational pedigree queries and visualization
- Advanced Taxonomy Navigator with filtering and keyboard support

### Other Planned Work
- Cell Culture (SteloCC) and Mycology (SteloMyco) verticals
- On-chain anchoring (Dogecoin)
- Local AI assistance
- Environmental sensor integration
- iOS support

For the latest status, refer to `ROADMAP.md` in the repository.

---

*This manual is a living document and will be updated as features ship.*