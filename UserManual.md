Table of Contents (Draft)

Introduction & Overview
Core Concepts (Specimens, Species, Lineage, Hash Chain)
Getting Started
Managing Species
Working with Specimens
Splitting Cultures (Detailed)
Recording Passages / Subcultures
The Audit Log & Cryptographic Hash Chain
Provenance & Genealogy Tracking
Reminders & Follow-ups
Importing & Exporting Data
Printing Reports
Advanced Topics (Future: Hybridization, Versioning)
Troubleshooting & Best Practices


I’ll start writing the manual section by section. Here’s the beginning:

1. Introduction & Overview
SteloPTC is a desktop application designed for tracking plant tissue culture work with strong emphasis on provenance, traceability, and data integrity.
It combines traditional lab record-keeping (specimens, media, passages, locations) with a cryptographic hash chain that makes the history of your cultures tamper-evident. This is especially useful for long-term research, compliance, or when you need to confidently trace the origin of cultures many generations later.
Key Ideas

Every specimen belongs to a species.
Species act as the cryptographic root for all specimens created from them.
When you split a culture, the parent is archived and new child specimens are created with their own independent lineages, while still clearly showing they came from the same parent.
The Audit Log records nearly everything and uses SHA-256 hashing to create a verifiable chain of events.
Genealogy tracking (generation numbers, sibling relationships, root lineage) helps you understand the full history of a culture at a glance.


2. Core Concepts
Specimens
A specimen is an individual culture in your lab. Each specimen has:

An accession number (unique identifier)
A species
Current stage, health, and location
A history of passages (subcultures)
Links to its parent (if it was created via split)

Species
Species are the foundation. When you create a new species, it starts its own hash chain. All specimens created from that species inherit its hash as their starting point. This creates a cryptographic link between the species definition and every culture derived from it.
Lineage & Hash Chain
Every action that matters (creating a specimen, splitting, passaging, etc.) is recorded in the Audit Log as part of a hash chain. This means you can later verify that the history hasn’t been altered.
When you split a specimen:

The parent is archived.
New children are created.
Each child starts its own lineage but shares a cryptographic link back to the parent via the split event.

Genealogy / Provenance
SteloPTC tracks:

Generation number (how many times the culture has been split from the original)
Root specimen (the original culture everything traces back to)
Siblings (other specimens created in the same split)

This makes it much easier to understand relationships between cultures, even many passages later.

3. Getting Started
(I’ll expand this section later with installation, first setup, creating your first species and specimen, etc.)

4. Managing Species
Species are the foundation of your tissue culture tracking. Every specimen must belong to a species, and the species record serves as the cryptographic root for all specimens created from it.
Creating a New Species

Go to the Species section in the sidebar.
Click + New Species.
Fill in the required fields:
Species Code (short code used in accession numbers, e.g. CAN-SAT)
Genus + Species Name (e.g. Cannabis sativa)
Optional fields: Description, Notes, etc.

Click Create.

Once created, the species starts its own hash chain at sequence 0. Any new specimens created from this species will inherit that hash as their starting point.
Editing a Species
Species can be edited after creation. However, because species act as the cryptographic root for specimens, edits are treated carefully:

Editing a species that has never been used to create specimens is straightforward.
Editing a species that has already been used will eventually create a new version (this feature is being implemented). Existing specimens will continue to reference the species state that existed when they were created.

Best Practice: Be thoughtful when editing species that are already in active use. Changes should generally be for corrections or clarifications rather than major redefinitions.
Archiving Species
You can archive (soft delete) a species. Archived species:

Can no longer be used to create new specimens.
Remain visible in historical views (so you can still see which species old specimens came from).
Appear greyed out or marked as archived in the species list.

Hard deletion is only allowed for species that have never been used to create any specimens.
Why Species Matter for Provenance
Because every specimen inherits its species’ hash at creation time, the species record is part of the permanent provenance chain. This allows you (and others) to later verify the origin of cultures, even many generations and splits later.

5. Working with Specimens
Creating a New Specimen

Go to Specimens → + New Specimen.
Select the Species.
Fill in the initial information:
Initiation date
Starting stage
Starting health
Initial location
Employee ID (who initiated the culture)

Add any initial notes or observations.
Click Create.

The new specimen will receive an accession number and will be cryptographically linked to the species it was created from.
Viewing Specimen Details
Click on any specimen to open its detail view. Here you can see:

Current status (stage, health, location)
Generation number and root lineage
Siblings (other specimens created in the same split)
Full passage/subculture history
Linked audit log entries

Updating a Specimen
You can update a specimen’s current status (location, health, stage, etc.) at any time. These updates are logged in the audit trail.

6. Splitting Cultures
Splitting is one of the most important actions in tissue culture work. SteloPTC treats splitting as a significant event that affects both the parent and the new children.
How Splitting Works
When you split a specimen:

The parent specimen is archived.
New child specimens are created.
Each child starts its own independent lineage (chain sequence resets to 1).
All children share a cryptographic link back to the parent through the split event.
The parent’s existing passage history remains fully linked and visible.

Performing a Split

Open the specimen you want to split.
Click Passage / Split.
Check the box “Split culture into multiple child specimens”.
Choose the number of children.
For each child row, configure:
Location (Room, Rack, Shelf, Tray)
Media Batch
Vessel Type
Health Status (you can set different health per child)
Stage (pre-filled from parent, editable per child)
Notes (optional, per child)

Optionally set follow-up reminders for each child (default is 7 days).
Review the summary at the bottom showing what will happen.
Confirm the action.

Important: Before confirming, you will see a warning reminding you to verify that the physical specimens are in the correct order and properly labeled.
Accession Numbers After Splitting
New child specimens receive suffixed accession numbers based on the parent. For example:

Parent: 2026-06-18-CAN-SAT-001
After splitting into two: 2026-06-18-CAN-SAT-001A and 2026-06-18-CAN-SAT-001B

Further splits of a child will continue the letter suffix (e.g. splitting 001B later creates 001BA and 001BB).
You can manually edit the generated accession numbers during the split if needed.
Best Practices for Splitting

Use the per-child health and notes fields when some splits look stronger or weaker than others.
Take advantage of the “Add new” option for media or vessels when working with new batches or containers.
Set reminders per child if the new cultures will need attention on different schedules.
Always double-check physical labeling before confirming the split.

7. Recording Passages / Subcultures
A passage (also called a subculture) is when you transfer part of a culture into new media to continue its growth. This is one of the most common actions you’ll perform.
How Passages Work in SteloPTC
When you record a passage:

The specimen’s subculture count increases.
A new entry is added to the specimen’s own lineage chain (the chain_seq increments).
You can update the specimen’s current location, health, stage, and media.
The passage is recorded in the Audit Log with full details.

Unlike splitting, a normal passage does not archive the parent or create new specimens. The same specimen continues, just with updated history.
Recording a Passage

Open the specimen you want to passage.
Click Passage / Split.
Make sure the “Split culture into multiple child specimens” box is unchecked.
Fill in the passage details:
Date
Media Batch
Vessel Type + conditions (Temp, pH, Light Cycle)
New location
Updated Health Status
Employee ID
Observations and Notes

Optionally check “Contamination detected in this vessel” if applicable.
Click Record Passage.

The system will automatically increment the passage count and add the event to the specimen’s hash chain.
Important Notes About Passages

The accession number does not change during a normal passage.
Each passage extends the specimen’s own lineage chain. This is different from older versions of the software, where passages created separate subculture records.
Health status is updated on the specimen record itself when you record a passage.

Best Practices

Be consistent with when you record passages (e.g. always when you physically move the culture).
Use the Observations field for what you see and Notes for protocol details, media lots, or other technical information.
If a culture looks significantly different after a passage, update the health status accordingly.


8. The Audit Log & Cryptographic Hash Chain
The Audit Log is one of the most important features in SteloPTC. It records nearly every meaningful action and protects that history using cryptography.
What Gets Logged
Most actions are automatically recorded, including:

Creating or editing species
Creating, updating, or archiving specimens
Splitting cultures
Recording passages
User logins and role changes
Imports and exports
Database resets (in admin mode)

How the Hash Chain Works
Every audit entry contains:

A sequence number within its lineage
A previous hash (linking it to the prior entry)
Its own entry hash (calculated from the data + previous hash)

This creates a chain where changing any past entry would break all subsequent hashes. This makes unauthorized changes detectable.
Viewing and Verifying the Audit Log
Go to Audit Log in the sidebar. Here you can:

Filter by entity type, action, or date
See the chain sequence (# column)
View Previous Hash and Entry Hash
Use the Row button to verify a single entry
Use the Chain button to verify an entire lineage

If verification fails on any entry, the system will clearly indicate where the chain is broken.
Why This Matters
The hash chain gives you strong assurance that the history of your cultures has not been tampered with. This is valuable for:

Long-term research traceability
Regulatory or compliance needs
Publishing or sharing culture histories
Internal quality control


9. Provenance & Genealogy Tracking
SteloPTC goes beyond simple record-keeping by actively tracking the family relationships between cultures.
Key Provenance Concepts


TermMeaningWhere You See ItGenerationHow many times a culture has been split from the originalSpecimen header (Gen N badge)Root SpecimenThe original culture everything ultimately traces back toSpecimen detail viewSiblingsOther specimens created in the same split eventLineage banner in specimen detailLineage ChainThe cryptographic history of a specific specimenAudit Log
Using Genealogy Features
When viewing a specimen, you can see:

Its current generation
A list of siblings (other cultures created when it was split from its parent)
Quick links to its root specimen

This makes it much easier to understand relationships between cultures, even when working with many generations of splits.
Example Use Case
You have an original culture (001). You split it into 001A and 001B. Both show Gen 1 and list each other as siblings. Later, you split 001A into 001AA and 001AB. These will show Gen 2 and list each other as siblings, while still being traceable back to the original 001.

10. Reminders & Follow-ups
SteloPTC includes a built-in reminder system to help you stay on top of cultures that need attention.
Creating Reminders
You can create reminders in several ways:

Manually from the Reminders section in the sidebar
Automatically when recording a passage or split (you can set follow-up reminders per child when splitting)
From the specimen detail view

When creating a reminder, you can set:

Due date
Description / notes
Which specimen it relates to (optional but recommended)
Whether it should repeat

Managing Reminders
Go to Reminders in the sidebar to see:

Active reminders
Overdue reminders (highlighted)
Completed reminders

You can mark reminders as done, edit them, or delete them. Reminders are also visible on the specimen detail page so you can see upcoming tasks for that specific culture.
Best Practices

Use reminders consistently for time-sensitive work (e.g. checking contamination risk, media changes, or planned splits).
When splitting, take advantage of the per-child reminder option — different children often need attention on different schedules.
Review your active reminders regularly (many users check this first thing when opening the program).


11. Importing & Exporting Data
SteloPTC supports importing and exporting data to help with backups, reporting, and moving data between systems.
Exporting Data
You can export:

Specimens (with current status and basic history)
Media batches
Audit logs
Full database backups

Exports are available in both CSV and JSON formats. The JSON export is more complete and includes more historical data.
Importing Data
You can import data using Excel (.xlsx) files. The importer supports:

Creating new specimens
Updating existing specimens
Importing media batches
Importing subculture/passage history

Important: When importing, the system validates data and will show you a preview before committing changes. Always review the preview carefully, especially when importing large amounts of data.
Best Practices

Use exports for regular backups of important data.
Use the Excel import feature when bringing in data from other lab systems or spreadsheets.
Be cautious when importing data that affects the audit log or hash chain — imported records are clearly marked as such.


12. Printing Reports
SteloPTC includes professional print and PDF generation features, particularly for specimen inventories and summaries.
Specimens Summary Report
You can generate a formatted report of your specimens that includes:

Executive summary (total specimens, breakdown by stage/health, contamination alerts)
Grouped views (by stage, by health, or flat list)
Individual specimen details (accession, species, location, health, generation, etc.)
Age of cultures (days since initiation)

This report is designed to be printed or saved as PDF for lab notebooks, audits, or sharing with colleagues.
How to Generate a Report

Go to the Specimens list.
Click Print Summary.
Choose your grouping preference (Stage, Health, or None).
Review the preview.
Print or save as PDF.

The report uses a clean, professional layout with proper headers, page numbers, and high-contrast formatting suitable for printing.
Other Print Features

Individual specimen labels (via QR code)
Audit log entries can be printed when needed
Passage/subculture history can be included in specimen detail views


13. Understanding the Hash Chain & Data Integrity (Advanced)
This section is for users who want to understand how SteloPTC protects your data.
Why the Hash Chain Exists
In traditional lab software, records can be edited or deleted without a clear trace. SteloPTC uses a cryptographic hash chain so that any change to historical records becomes detectable.
Every important action creates an audit entry that includes:

What happened
When it happened
Who did it
A cryptographic link to the previous entry

This makes the history of your cultures verifiable.
What This Means in Practice

You can trust that the passage history of a specimen hasn’t been secretly altered.
If someone tries to change old records, the verification tools in the Audit Log will show that the chain is broken.
The system makes it much harder to falsify culture histories (intentionally or accidentally).

Limitations
The hash chain protects against undetected changes. It does not prevent authorized users from making changes — it only makes those changes visible and verifiable later.

14. Troubleshooting & Common Issues
This section covers some common problems users encounter and how to resolve them.
“I can’t delete a species”
Species can only be hard-deleted if they have never been used to create any specimens. If a species has been used, you can only archive it. Archived species remain visible in historical views but cannot be used for new specimens.
Accession numbers look wrong after splitting
When splitting, the system generates suffixed accession numbers (e.g. 001A, 001B). If the numbers don’t look right, you can manually edit them during the split confirmation step before saving.
Note: The original date in the accession number is intentionally preserved during splits.
Verification fails in the Audit Log
If the Row or Chain verify button shows a failure, it means one or more audit entries have been altered or corrupted. This is rare and usually indicates either:

Manual database editing outside the application, or
Data corruption

Contact an administrator if you see verification failures on records you did not intentionally modify.
Reminders are not appearing
Make sure reminders are created with a future due date. Reminders set for “today” may not appear until the next day depending on your system time. You can also check the specimen detail page — reminders linked to a specimen are shown there as well.
“Add New” media or vessel isn’t working as expected
When using “Add new” during a split or passage, the system creates a placeholder record. These placeholders are clearly marked as incomplete. You must go into the Media or Vessel management area later to fill in the real details. Some actions may be restricted until the placeholder is completed.
The split form looks stretched or messy
This is a known layout issue in some screen sizes. Try collapsing the Observations and Notes fields or reducing the number of visible children temporarily while configuring the split. A layout improvement is planned for a future update.

15. Best Practices for Tissue Culture Tracking
Here are some recommended habits when using SteloPTC:
Be Consistent with Accession Numbers
Try to follow a consistent naming convention for splits (e.g. always using letter suffixes like 001A, 001B). This makes it much easier to understand relationships between cultures later.
Record Passages Promptly
Record passages as close as possible to when you actually perform them. This keeps the hash chain and passage count accurate and makes historical reports more reliable.
Use Per-Child Configuration During Splits
When splitting, take advantage of the ability to set different health, media, and notes per child. This information is very valuable when reviewing cultures weeks or months later.
Review the Audit Log Periodically
Even if you’re not worried about tampering, occasionally looking at the Audit Log helps you catch mistakes (such as accidental edits or missed passages).
Use Reminders Actively
The reminder system is most useful when used consistently. Set reminders during splits and passages for cultures that need follow-up attention.
Archive, Don’t Delete
In most cases, archive species and specimens instead of deleting them. This preserves historical provenance and makes it easier to trace cultures over long periods of time.
Double-Check Physical Labeling
Before confirming a split, always verify that your physical labels match what the software will create. The warning popup exists for this reason — take it seriously.

16. Future Features (Planned)
SteloPTC is under active development. Some features planned or under consideration include:

Hybridization / Merging support (creating new specimens from multiple parent specimens)
Improved Species Versioning and history tracking
Interactive lineage tree visualization
Better support for experimental metadata and custom fields
Mobile companion app (read-only access to specimens and reminders)
Enhanced reporting and analytics

If there are specific features you need that aren’t listed here, feel free to request them.
