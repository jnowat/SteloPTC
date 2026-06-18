# SteloPTC User Manual

**Version:** Aligns with split/passage refinements (June 2026)

SteloPTC is a desktop application for managing plant tissue culture laboratories with a strong focus on **provenance, traceability, and cryptographic data integrity**.

It combines traditional lab record-keeping with an immutable, hash-chained audit trail so you can confidently trace the complete history of any culture — even many generations and splits later.

---

## Table of Contents

1. Introduction & Overview
2. Core Concepts
   - Specimens
   - Species
   - Passage vs. Split
   - Quick Reference: Passage vs. Split
   - Lineage & Hash Chain
   - Genealogy & Provenance
3. Getting Started
4. Managing Species
5. Working with Specimens
6. Splitting Cultures (Detailed)
7. Recording Passages / Subcultures
8. The Audit Log & Cryptographic Hash Chain
9. Provenance & Genealogy Tracking
10. Reminders & Follow-ups
11. Importing & Exporting Data
12. Printing Reports
13. Understanding the Hash Chain & Data Integrity (Advanced)
14. Troubleshooting & Common Issues
15. Best Practices for Tissue Culture Tracking
16. Future Features

---

## 1. Introduction & Overview

SteloPTC helps labs maintain rigorous, auditable records of their tissue culture work. Every meaningful action — creating a specimen, recording a passage, splitting a culture, or editing a species — is logged in a cryptographic hash chain. This makes the history tamper-evident and gives you strong confidence in the provenance of your cultures.

The application is designed to be practical for day-to-day lab use while providing long-term value for research traceability, compliance, quality systems, and knowledge transfer.

**Core philosophy**
- Local-first and offline-capable
- Data integrity is non-negotiable
- Species act as protected cryptographic roots
- Splits create clear lineage branches while preserving full parent history
- The system should make correct work easy and incorrect work visible

---

## 2. Core Concepts

### Specimens

A specimen is an individual culture in your lab. Each specimen has:

- A unique **accession number**
- A **species** (the cryptographic root)
- Current **stage**, **health**, and **location**
- A complete history of **passages** (subcultures)
- Links to its **parent** (if created via split) and **siblings** (other cultures created in the same split)

### Species

Species are the foundation of the system. When you create a new species, it starts its own hash chain. Every specimen created from that species inherits the species hash as its starting point. This creates a permanent cryptographic link between the species definition and every culture derived from it.

Because of this role, species become **very protected** once they have been used to create any specimens.

### Passage vs. Split

These are the two primary ways you advance cultures in SteloPTC. Understanding the difference is essential:

**Passage (Subculture)**
- The same specimen record continues.
- Its chain sequence (`chain_seq`) increments.
- The accession number never changes.
- You record new media, location, health, stage, and observations.
- Used for routine maintenance and growth.

**Split**
- The parent specimen is archived.
- One or more new child specimens are created.
- Each child starts a fresh lineage (its own chain sequence begins at 1).
- New suffixed accession numbers are generated (e.g. `001A`, `001B`, `001BA`).
- You can configure each child independently (health, stage, media, vessel, notes, reminders).
- The parent’s full history remains visible and linked, but the parent is no longer active.
- Used when you want to divide a culture into multiple independent lines.

> **Important:** Before confirming a split, you will always see a verification warning reminding you to physically check that your labels match the software-generated accessions.

### Quick Reference: Passage vs. Split

| Aspect                    | Passage (Subculture)                  | Split                                      |
|---------------------------|---------------------------------------|--------------------------------------------|
| Parent record             | Continues                             | Archived                                   |
| New specimens created     | No                                    | Yes (one or more children)                 |
| Accession number          | Unchanged                             | New suffixed numbers (001A, 001B, etc.)    |
| Lineage chain             | Continues on same specimen            | Each child starts fresh (chain resets)     |
| Per-child configuration   | N/A                                   | Yes (health, stage, media, reminders, etc.)|
| Typical use case          | Routine maintenance & growth          | Dividing into independent lines            |
| Physical labeling         | Same label                            | New labels required for each child         |

Use **Passage** for normal upkeep. Use **Split** when you intentionally want to create multiple separate cultures from one.

### Lineage & Hash Chain

Every significant action is recorded in the Audit Log as part of a SHA-256 hash chain. Each entry contains:

- A sequence number within its lineage
- The hash of the previous entry
- Its own entry hash (calculated from the data + previous hash)

This creates an append-only chain. Changing any historical record would break all subsequent hashes, making tampering detectable.

When you split a specimen, each new child starts its own independent lineage while maintaining a cryptographic link back to the parent through the split event.

### Genealogy & Provenance

SteloPTC actively tracks family relationships:

- **Generation number** — how many times the culture has been split from the original
- **Root specimen** — the original culture everything ultimately traces back to
- **Siblings** — other specimens created in the same split event

This information appears in the specimen detail view and helps you understand relationships even many passages later.

---

## 3. Getting Started

### First Launch

When you open SteloPTC on a fresh database, you will see the First-Run guidance panel. It walks you through the two essential first steps:

1. Configure your species registry
2. Accession your first specimen

You can also load demo data (a small set of realistic specimens with passage history) to explore the interface before entering your own work.

### UI Navigation Tips

- **Sidebar**: Primary navigation between Dashboard, Specimens, Species, Media, Vessels, Reminders, Audit Log, and Admin tools.
- **Specimen list**: Click any row to open the detail view. Use filters and search at the top.
- **Specimen detail**: Tabs and sections for status, history, siblings, reminders, and audit entries.
- **Keyboard**: Many actions support standard shortcuts (check tooltips). Tab navigation works throughout forms.

### Creating Your First Species

1. Go to **Species** in the sidebar.
2. Click **+ New Species**.
3. Enter a short **Species Code** (used in accession numbers, e.g. `CAN-SAT`).
4. Enter Genus + Species name (e.g. *Cannabis sativa*).
5. Add any optional description or notes.
6. Click **Create**.

The species now exists and has started its own hash chain.

### Creating Your First Specimen

1. Go to **Specimens** → **+ New Specimen**.
2. Select the species.
3. Fill in initiation date, starting stage, health, location, and employee ID.
4. Add initial notes if desired.
5. Click **Create**.

The specimen receives an accession number and is immediately linked to the species’ cryptographic root.

---

## 4. Managing Species

Species are the cryptographic foundation of your entire collection. Treat them with care.

### Creating a New Species

See “Getting Started” above.

### Editing a Species

You can edit a species after creation. However, because species serve as the cryptographic root for all derived specimens, edits are handled carefully:

- Editing a **never-used** species is straightforward.
- Editing a species that has already been used to create specimens triggers stronger protections (warning banners and confirmation dialogs).
- Existing specimens continue to reference the species state that existed at the time they were created.

**Best practice:** Only make edits that are genuine corrections or clarifications. Major redefinitions should be avoided once a species is in active use.

### Archiving vs. Hard Deletion

- **Archive** (soft delete): The species can no longer be used to create new specimens but remains visible in historical views. This is the recommended action for any species that has been used.
- **Hard delete**: Only allowed for species that have never been used to create any specimens. This is a permanent, irreversible action.

Archived species appear greyed out in the species list.

### Why Species Protection Matters

Every specimen inherits its species’ hash at creation time. This hash becomes part of the permanent provenance chain. Protecting the species record protects the integrity of every culture derived from it — now and in the future.

---

## 5. Working with Specimens

### Creating a New Specimen

See “Getting Started”.

### Viewing Specimen Details

Click any specimen in the list to open its detail view. Here you can see:

- Current status (stage, health, location)
- Generation number and root lineage badge
- Siblings (other cultures created in the same split)
- Full passage/subculture history
- Linked audit log entries
- Active reminders

### Updating a Specimen

You can update location, health, stage, and other fields at any time. All changes are logged in the audit trail.

---

## 6. Splitting Cultures (Detailed)

Splitting is one of the most important and carefully designed workflows in SteloPTC. It allows you to divide a culture into multiple independent lines while preserving full traceability.

### How Splitting Works

When you split a specimen:

- The **parent is archived**.
- New **child specimens** are created.
- Each child starts its own independent lineage (chain sequence resets to 1).
- New suffixed accession numbers are generated.
- You can configure each child independently.
- The parent’s complete passage history remains visible and linked.
- A contamination flag, if set during the split, applies to the parent.

### Performing a Split — Step by Step Example

**Scenario:** You have specimen `2026-06-18-CAN-SAT-001` (Gen 0) and want to split it into two children.

1. Open `2026-06-18-CAN-SAT-001` and click **Passage / Split**.
2. Check **“Split culture into multiple child specimens”** and select **2 children**.
3. The system suggests `2026-06-18-CAN-SAT-001A` and `2026-06-18-CAN-SAT-001B`. You can edit either if needed (e.g., to match existing physical labels).
4. For Child A: Set health to “Excellent”, choose a media batch, set a 7-day reminder.
5. For Child B: Set health to “Good”, choose a different media batch, set a 10-day reminder.
6. Review the live summary box at the bottom (shows what will be archived and what will be created).
7. Click **Confirm Split**.
8. A verification warning appears — physically check your labels and specimens before proceeding.
9. After confirmation:
   - Parent `...001` is archived.
   - Two new active specimens `...001A` and `...001B` (both Gen 1, siblings) are created.
   - Each has its own fresh lineage chain.
   - Reminders are scheduled for each child.

### Accession Numbers After Splitting

New children receive suffixed accession numbers based on the parent:

- Parent: `2026-06-18-CAN-SAT-001`
- After split into two: `2026-06-18-CAN-SAT-001A` and `2026-06-18-CAN-SAT-001B`

Further splits continue the pattern recursively (splitting `001B` later produces `001BA` and `001BB`).

You can **manually edit** the generated accession numbers during the split if needed (for example, to match existing physical labels).

### “Add New” Media and Vessel (Draft Records)

When you choose **Add new** for media or vessel during a split or passage, SteloPTC creates a **draft/placeholder** record marked as incomplete (`is_draft = true`).

- Draft records appear in lists but are clearly flagged.
- You cannot use an incomplete draft for certain downstream actions until it is completed.
- Go to the **Media** or **Vessel** management area later to fill in the missing details (name, composition, lot number, etc.).
- Once completed, the draft flag is removed and the record becomes fully usable.

This allows you to keep working during a split without stopping to fully define new media batches.

### Best Practices for Splitting

- Use the per-child health, notes, and reminder fields when some splits look stronger or weaker than others.
- Take advantage of per-child configuration — this information becomes very valuable weeks or months later when reviewing performance.
- Always double-check physical labeling before confirming the split. The verification warning exists for this reason.
- Set different reminder intervals for children that will need attention on different schedules.
- After splitting, immediately check the sibling list and root lineage in the new children’s detail views to confirm relationships are correctly recorded.

---

## 7. Recording Passages / Subcultures

A passage (subculture) is the routine transfer of part of a culture into new media to continue its growth.

### How Passages Work

When you record a passage:

- The specimen continues (it is not archived).
- Its chain sequence increments.
- The accession number stays the same.
- You can update location, health, stage, media, and observations.
- The event is recorded in the Audit Log.

Unlike a split, a normal passage does **not** create new specimens or archive the parent.

### Recording a Passage

1. Open the specimen.
2. Click **Passage / Split**.
3. Make sure the “Split culture into multiple child specimens” box is **unchecked**.
4. Fill in the passage details (date, media, vessel, location, health, employee ID, observations).
5. Optionally mark contamination if applicable.
6. Click **Record Passage**.

The system automatically increments the passage count and adds the event to the specimen’s hash chain.

### Important Notes

- The accession number never changes during a normal passage.
- Health status is updated on the specimen record itself.
- Each passage extends the specimen’s own lineage chain.

---

## 8. The Audit Log & Cryptographic Hash Chain

The Audit Log is one of SteloPTC’s most important features. It records nearly every meaningful action and protects that history using cryptography.

### What Gets Logged

- Creating, editing, or archiving species
- Creating, updating, or archiving specimens
- Splitting cultures
- Recording passages
- User logins and role changes
- Imports, exports, and database resets (admin)

### How the Hash Chain Works

Every audit entry contains a sequence number, the previous entry’s hash, and its own hash. This forms a continuous, verifiable chain. Any change to a past entry breaks all subsequent links.

### Viewing and Verifying

Go to **Audit Log** in the sidebar. You can:

- Filter by entity, action, or date
- See chain sequence numbers and hashes
- Verify individual rows or entire lineages

If verification fails, the system clearly indicates the first broken link.

### Why This Matters

The hash chain gives you strong assurance that culture histories have not been tampered with — valuable for long-term research, regulatory needs, publishing, and internal quality control.

---

## 9. Provenance & Genealogy Tracking

SteloPTC actively tracks family relationships between cultures so you can understand lineage at a glance.

**Key concepts**

- **Generation (Gen N)** — how many splits separate the culture from the original
- **Root specimen** — the original culture everything ultimately traces back to
- **Siblings** — other specimens created in the same split event

These appear in the specimen header and detail view. Clicking a sibling or root link takes you directly to that specimen.

Example: You split `001` into `001A` and `001B` (both Gen 1, siblings of each other). Later splitting `001A` into `001AA` and `001AB` creates Gen 2 siblings that still trace back to the original `001`.

---

## 10. Reminders & Follow-ups

SteloPTC includes a built-in reminder system to help you stay on top of cultures that need attention.

### Creating Reminders

You can create reminders:

- Manually from the Reminders sidebar
- Automatically when recording a passage or split (per-child reminders are supported during split)
- From the specimen detail view

Set due date, description, related specimen (recommended), and whether it should repeat.

### Managing Reminders

Go to **Reminders** in the sidebar to see active, overdue, and completed reminders. You can mark them done, edit, or delete. Reminders also appear on the relevant specimen detail page.

### Best Practices

- Use per-child reminders during splits when children will need attention on different schedules.
- Review active reminders regularly — many users check this first when opening the app.

---

## 11. Importing & Exporting Data

### Exporting

You can export specimens, media batches, audit logs, and full database backups in CSV or JSON. JSON exports are more complete and include historical data.

### Importing

Excel (.xlsx) import supports creating/updating specimens and importing passage history. The importer shows a dry-run preview with counts and errors before committing. Always review the preview carefully.

Imported records that affect the audit log or hash chain are clearly marked.

---

## 12. Printing Reports

SteloPTC generates professional print/PDF output for lab notebooks, audits, and sharing.

- **Specimens Summary Report** — executive summary, grouped views, individual specimen details, age of cultures
- **Individual specimen labels** via QR code
- Culture reports from the specimen detail view

Use the Print Summary button on the Specimens list. Choose grouping (Stage, Health, or None) and review the preview before printing.

---

## 13. Understanding the Hash Chain & Data Integrity (Advanced)

Traditional lab software allows records to be edited or deleted without a clear trace. SteloPTC uses a cryptographic hash chain so that any change to historical records becomes detectable.

Every important action creates an audit entry containing what happened, when, who did it, and a cryptographic link to the previous entry. This makes the history of your cultures verifiable.

You can trust that passage and split history has not been secretly altered. If someone tries to change old records, the verification tools will show exactly where the chain is broken.

The hash chain protects against *undetected* changes. It does not prevent authorized users from making changes — it only makes those changes visible and verifiable later.

---

## 14. Troubleshooting & Common Issues

**“I can’t delete a species”**
Species can only be hard-deleted if they have never been used to create specimens. If a species has been used, archive it instead. Archived species remain visible in historical views.

**Accession numbers look wrong after splitting**
The system generates suffixed accessions (e.g. `001A`, `001B`). You can manually edit them during the split confirmation step before saving. The original date prefix is intentionally preserved.

**Verification fails in the Audit Log**
This usually means one or more audit entries were altered outside the application or data corruption occurred. Contact an administrator. The system will pinpoint the first broken link.

**Reminders are not appearing**
Make sure reminders have a future due date. Reminders set for “today” may not appear until the next day depending on system time. Check the specimen detail page as well.

**“Add New” media or vessel isn’t working as expected**
“Add new” during split or passage creates a draft/placeholder record marked as incomplete. You must complete the real details later in the Media or Vessel management area before certain actions are allowed on that record.

**Split form looks stretched or messy**
Try collapsing the Notes fields or temporarily reducing the number of visible children. A layout improvement is planned.

---

## 15. Best Practices for Tissue Culture Tracking

**Be consistent with accession numbers**
Follow a consistent suffix convention for splits. This makes relationships between cultures much easier to understand later.

**Record passages promptly**
Record passages as close as possible to when you physically perform them. This keeps the hash chain and passage counts accurate.

**Use per-child configuration during splits**
Set different health, notes, and reminders per child when splits vary in quality or expected timing. This data becomes extremely valuable for later review.

**Double-check physical labeling before confirming a split**
The verification warning exists for a reason. Take it seriously.

**Archive, don’t delete**
Archive species and specimens instead of deleting them whenever possible. This preserves historical provenance.

**Review the Audit Log periodically**
Even if you’re not concerned about tampering, occasional review helps catch mistakes (missed passages, accidental edits).

**Use reminders actively**
Set reminders during splits and passages for cultures that need follow-up attention.

**Check lineage after splitting**
After a split, open one of the new children and verify the sibling list and root lineage are correct. This confirms the provenance relationships were recorded properly.

---

## 16. Future Features (Planned)

SteloPTC is under active development. Planned or under consideration:

- Improved species versioning and history timeline
- Interactive lineage tree visualization
- Hybridization / merging support
- Better experimental metadata and custom fields
- Mobile companion app (read-only)
- Enhanced reporting and analytics
- Multi-vertical support (Cell Culture and Mycology profiles on the shared engine)

If you need specific features not listed here, please request them.

---

*This manual documents the behavior after the June 2026 split/passage workflow refinements.*