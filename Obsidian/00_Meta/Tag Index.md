---
title: Tag Index
aliases:
  - Tags
  - Controlled Vocabulary
  - Tag Vocabulary
tags:
  - meta
  - vault
  - tags
type: meta
status: binding
created: 2026-07-29
updated: 2026-07-29
cssclasses:
  - wide-tables
---

> [!abstract] In one sentence
> The complete controlled tag vocabulary of this vault — **77 flat, un-nested tags** across five
> classes (domain, type, topic, property, vault-mechanics) — where a note's **first** frontmatter
> tag is always its domain, and adding a tag anywhere means adding a row here in the same edit.

---

## How tagging works here

> [!important] Flat tags, not nested ones
> This vault uses **flat** tags — `#architecture`, `#taxonomy` — not the nested
> `#steloptc/architecture` form some Obsidian vaults adopt. That was a deliberate choice at
> creation and it is now load-bearing: every existing note's frontmatter uses the flat form, so
> introducing a nested variant would split the tag pane in two and make `#taxonomy` and
> `#steloptc/taxonomy` look like different subjects. If the vault ever migrates, it migrates all at
> once.

Three rules, all enforced by hand:

1. **First tag is the domain tag.** `tags: [architecture, database, sqlite, schema, taxonomy]` on
   [[Data Model]] means: this is an Architecture-domain note about the database. The remaining tags
   are topics, in no particular order.
2. **`type:` is authoritative, not the tag.** Whether a note is a concept or a workflow is decided
   by the `type` frontmatter field ([[Vault Conventions]]). Some type values happen to double as
   tags (`architecture`, `workflow`, `reference`, `moc`, `meta`); `concept` does not — no note
   carries a `#concept` tag, so **query `type: concept`, not `#concept`**, to find the
   `Core_Concepts/` notes.
3. **Footer tags are a subset of frontmatter tags.** The two-to-four inline `#tags` closing each
   note echo its frontmatter; they exist so the tag survives when the note is read as raw text with
   the frontmatter stripped. Two exceptions are deliberate: `#steloptc` and `#local-first` appear
   only as footer tags on some notes, not in their frontmatter.

> [!warning] Two strings that look like tags and are not
> `#ef4444` in [[Drawing the Lab]] and `#app` in [[Failure Reference]] are a CSS hex colour and an
> HTML element id inside code blocks. Obsidian will not index a tag beginning with a digit, but a
> naive `grep '#[a-z]'` over the vault will report both. Ignore them.

---

## Domain tags

Nine values. Exactly one appears, first, in every note's tag list.

| Tag | Meaning | Notes that carry it |
|---|---|---|
| `#architecture` | What the system *is* — components, seams, module maps | [[SteloPTC at a Glance]] · [[Rust Backend]] · [[Svelte Frontend]] · [[The IPC Seam]] · [[Data Model]] · [[Trust Layer]] · [[MOC - Architecture]] |
| `#trust` | Tamper evidence and cross-lab verification — the product's reason to exist | [[Hash-Chained Provenance]] · [[Federated Exchange]] · [[MOC - Core Concepts]] |
| `#lab-ops` | The bench: specimens, passages, rooms, the daily loop | [[Lab Profiles]] · [[Specimens Strains and Species]] · [[Lab Layout Model]] · [[Daily Bench Work]] · [[Drawing the Lab]] · [[MOC - Core Concepts]] · [[MOC - Workflows]] |
| `#taxonomy` | Biological classification — `taxa`, `species`, NCBI ingest | [[Taxonomy Backbone]] · [[Importing NCBI Taxonomy]] · [[Data Model]] · [[MOC - Core Concepts]] |
| `#security` | Authentication, roles, authorisation predicates | [[Roles and Permissions]] · [[MOC - Core Concepts]] |
| `#compliance` | Regulatory flags, waivers, export bundles, submissions | [[Compliance and Export]] · [[MOC - Workflows]] |
| `#reference` | Lookup material — exact names, numbers, knobs | [[Command Reference]] · [[Database Schema]] · [[Migrations]] · [[Build and Test Commands]] · [[Failure Reference]] · [[Shipped vs Dormant]] · [[MOC - Reference]] |
| `#moc` | A map of content — a note whose job is to point at other notes | [[Home]] · [[MOC - Architecture]] · [[MOC - Core Concepts]] · [[MOC - Workflows]] · [[MOC - Reference]] |
| `#meta` | About the vault itself, not about SteloPTC | Everything in `00_Meta/` |

---

## Type tags

These mirror the `type:` frontmatter field. They are redundant by design — the tag pane is how a
human browses, `type:` is how a query filters.

| Tag | Mirrors | Notes |
|---|---|---|
| `#architecture` | `type: architecture` | The six `Architecture/` notes (also the domain tag — the two coincide for this folder) |
| `#workflow` | `type: workflow` | [[Daily Bench Work]] · [[Drawing the Lab]] · [[Importing NCBI Taxonomy]] · [[Compliance and Export]] · [[Federated Exchange]] · [[MOC - Workflows]] |
| `#reference` | `type: reference` | The five `Reference/` notes · [[Shipped vs Dormant]] · [[MOC - Reference]] |
| `#moc` | `type: moc` | [[Home]] and the four MOCs |
| `#meta` | `type: meta` | [[Vault Conventions]] · [[Tag Index]] |
| *(none)* | `type: concept` | The six `Core_Concepts/` notes carry **no** type tag — filter on `type: concept` |

---

## Topic tags

Everything else. Grouped by what they are about rather than alphabetically, because the useful
question is "what else covers this".

### Stack and platform

| Tag | Meaning | Notes |
|---|---|---|
| `#rust` | The Rust crate, `src-tauri/` | [[Rust Backend]] · [[SteloPTC at a Glance]] · [[The IPC Seam]] · [[Command Reference]] · [[Migrations]] · [[Build and Test Commands]] |
| `#tauri` | Tauri v2 specifics — commands, capabilities, the WebView host | [[Rust Backend]] · [[SteloPTC at a Glance]] · [[The IPC Seam]] · [[Command Reference]] |
| `#backend` | The backend as a whole | [[Rust Backend]] |
| `#svelte` | Svelte 5 runes mode | [[SteloPTC at a Glance]] · [[Svelte Frontend]] |
| `#frontend` | `src/` — components, stores, the browser side of a seam | [[Svelte Frontend]] · [[The IPC Seam]] · [[Lab Layout Model]] · [[Drawing the Lab]] · [[Command Reference]] · [[Build and Test Commands]] |
| `#typescript` | TypeScript typing discipline | [[Svelte Frontend]] |
| `#accessibility` | a11y — and the fact that its lint warnings are suppressed at build time | [[Svelte Frontend]] |
| `#overview` | The whole-system view | [[SteloPTC at a Glance]] |

### Data

| Tag | Meaning | Notes |
|---|---|---|
| `#database` | The database layer generally | [[Data Model]] · [[Database Schema]] · [[Migrations]] |
| `#sqlite` | SQLite specifics — WAL, PRAGMAs, the bundled amalgamation | [[Data Model]] · [[Rust Backend]] · [[SteloPTC at a Glance]] · [[Database Schema]] · [[Migrations]] |
| `#schema` | Tables, columns, constraints | [[Data Model]] · [[Database Schema]] |
| `#data-model` | Entity relationships and their sharp edges | [[Taxonomy Backbone]] · [[Database Schema]] |
| `#migrations` | The numbered, append-only migration chain | [[Migrations]] |
| `#taxa` | The `taxa` table — kingdom through genus, never species | [[Taxonomy Backbone]] |
| `#species` | The `species` registry and its `taxon_path` | [[Taxonomy Backbone]] · [[Specimens Strains and Species]] |
| `#strains` | Named lines within a species, and their pedigree | [[Specimens Strains and Species]] |
| `#specimens` | Individual physical cultures | [[Specimens Strains and Species]] · [[Daily Bench Work]] |
| `#navigator` | The Taxonomy Navigator UI and what it derives | [[Taxonomy Backbone]] |

### Seam and API

| Tag | Meaning | Notes |
|---|---|---|
| `#ipc` | The Tauri IPC boundary | [[The IPC Seam]] · [[Roles and Permissions]] · [[Command Reference]] |
| `#api` | The `src/lib/api.ts` wrapper and command signatures | [[The IPC Seam]] · [[Command Reference]] |

### Trust and provenance

| Tag | Meaning | Notes |
|---|---|---|
| `#provenance` | The chain of custody a record carries | [[Trust Layer]] · [[Hash-Chained Provenance]] · [[Home]] |
| `#hash-chain` | The SHA-256 per-lineage chain (WP-18) | [[Hash-Chained Provenance]] |
| `#audit-log` | The `audit_log` table itself | [[Hash-Chained Provenance]] |
| `#audit` | Auditing as an activity, incl. regulatory audit | [[Trust Layer]] · [[Compliance and Export]] |
| `#merkle` | Merkle checkpoints and portable proofs (WP-20/21) | [[Trust Layer]] · [[Hash-Chained Provenance]] |
| `#cryptography` | Primitives — SHA-256, Ed25519, AES-GCM, Argon2id | [[Trust Layer]] |
| `#ed25519` | Signing specifically — `ed25519-dalek`, never RSA | [[Trust Layer]] |
| `#integrity` | The read-only self-check and its verdicts | [[Failure Reference]] |
| `#federation` | Cross-lab exchange without a server | [[Federated Exchange]] |
| `#passport` | Signed per-specimen provenance documents (WP-70) | [[Federated Exchange]] |
| `#registry` | The shared taxonomy registry document (WP-71) | [[Federated Exchange]] |
| `#coordination` | Cross-lab breeding bundles (WP-72) | [[Federated Exchange]] |

### Lab operations

| Tag | Meaning | Notes |
|---|---|---|
| `#lifecycle` | Accession → passage → split → death → archive | [[Specimens Strains and Species]] · [[Daily Bench Work]] |
| `#work-queue` | The five-rule due-work computation | [[Daily Bench Work]] |
| `#reminders` | The `reminders` table and dismissal | [[Daily Bench Work]] |
| `#profiles` | The three lab profiles and the switch between them | [[Lab Profiles]] |
| `#domain` | `LabDomain` — Plantae / Animalia / Fungi and its manifests | [[Lab Profiles]] |
| `#vocabulary` | Vocabulary-as-data: stages, methods, hormones, agencies | [[Lab Profiles]] |
| `#isolation` | Lab-profile scoping of queries, and where it leaks | [[Lab Profiles]] |
| `#lab-map` | The physical map of the lab | [[Lab Layout Model]] · [[Drawing the Lab]] |
| `#locations` | The `locations` table and `specimens.location` | [[Lab Layout Model]] · [[Drawing the Lab]] |
| `#layout` | `locations.layout_json` — geometry as a document | [[Lab Layout Model]] · [[Drawing the Lab]] |

### Access control

| Tag | Meaning | Notes |
|---|---|---|
| `#roles` | The four roles: admin, supervisor, tech, guest | [[Roles and Permissions]] |
| `#permissions` | The three predicates and per-field visibility (WP-55) | [[Roles and Permissions]] · [[Command Reference]] · [[Failure Reference]] |
| `#auth` | Sessions, tokens, login, expiry | [[Roles and Permissions]] · [[Failure Reference]] |
| `#admin` | Admin-only operations | [[Importing NCBI Taxonomy]] |

### Data in and out

| Tag | Meaning | Notes |
|---|---|---|
| `#import` | Reading external data in | [[Importing NCBI Taxonomy]] |
| `#export` | Writing files out — CSV, JSON, Excel, bundles | [[Compliance and Export]] |
| `#ncbi` | NCBI E-utilities and `taxdump` formats | [[Importing NCBI Taxonomy]] |
| `#regulatory` | FDA Part 11, USDA PPQ 526, CITES, submissions | [[Compliance and Export]] |

### Engineering practice

| Tag | Meaning | Notes |
|---|---|---|
| `#build` | Building and packaging | [[Build and Test Commands]] |
| `#testing` | The test suites and what each one covers | [[Build and Test Commands]] |
| `#ci` | What the merge-gating workflows actually run | [[Build and Test Commands]] |
| `#tooling` | npm, cargo, and the version-bump lockstep | [[Build and Test Commands]] |
| `#errors` | The error strings a user can see | [[Failure Reference]] |
| `#troubleshooting` | Diagnosing a failure from its message | [[Failure Reference]] |
| `#validation` | Input validation and its refusal messages | [[Failure Reference]] |

---

## Property tags

Cross-cutting claims about the *state* of a thing rather than its subject. These are the tags an
agent should search first.

| Tag | Meaning | Notes that carry it |
|---|---|---|
| `#stub` | The note documents something that ships deliberately incomplete | [[Compliance and Export]] · [[Shipped vs Dormant]] |
| `#dormant` | Real code with no live path — the honest status of several subsystems | [[Shipped vs Dormant]] |
| `#offline` | Works with the network cable pulled, because there is no network code | [[Federated Exchange]] · [[Importing NCBI Taxonomy]] |
| `#local-first` | The architectural stance: the operator's machine owns the data | [[Home]] · [[Importing NCBI Taxonomy]] (footer only) |
| `#roadmap` | Relates to planned-versus-shipped status | [[Shipped vs Dormant]] |
| `#honesty` | The rule that a stub is never described as live | [[Vault Conventions]] |

---

## Vault-mechanics tags

| Tag | Meaning | Notes |
|---|---|---|
| `#vault` | About how this vault is built and maintained | [[Vault Conventions]] · [[Tag Index]] |
| `#conventions` | The authoring contract | [[Vault Conventions]] |
| `#tags` | This note | [[Tag Index]] |
| `#steloptc` | The project as a whole — used as a footer tag on the `Reference/` notes and in [[Home]]'s frontmatter | [[Home]] · the five `Reference/` notes (footer only) |

---

## Adding a tag

> [!tip] Prefer an existing tag to a precise new one
> Seventy-seven tags across twenty-four notes is already past the point where a tag reliably
> narrows anything. Before adding one, check whether an existing topic tag covers it — `#lab-map` and
> `#layout` and `#locations` already coexist on the same two notes and arguably did not all need to
> exist.
>
> If you do add one: pick the flat form, add the row to the right table above, and put it in the
> note's frontmatter *and* its footer. A tag that appears in exactly one note and is not in this
> index is a typo until proven otherwise.

---

**Back to [[Home]]**

#meta #vault #tags
