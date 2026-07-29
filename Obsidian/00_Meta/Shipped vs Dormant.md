---
title: Shipped vs Dormant
aliases:
  - Status Ledger
  - What actually works
  - Stubs
  - Do not re-plan
tags:
  - meta
  - reference
  - stub
  - dormant
  - roadmap
type: reference
status: living
version: 0.54.0
created: 2026-07-29
updated: 2026-07-29
cssclasses:
  - wide-tables
---

> [!abstract] In one sentence
> The honest capability ledger at `v0.54.0` — every subsystem rated shipped, partial, dormant or
> stub, with the `file:line` that proves it — written because SteloPTC deliberately ships several
> foundations without their transport layers, and reading `README.md` or `ROADMAP.md` alone will
> lead an agent to plan work that already exists or to promise a capability that refuses at
> runtime.

> [!danger] Read this before proposing work
> A capability being *described* anywhere in this repository — including in `docs/`, which contains
> specifications written ahead of the code — is not evidence that it is live. Every row below was
> re-verified against the source at `v0.54.0`, not copied from a document. Where a repo document
> and the code disagree, the disagreement is recorded in [§ Where the docs overstate](#where-the-docs-overstate).

---

## The four ratings

| Rating | Means | The concrete test |
|---|---|---|
| **shipped** | Live, reachable from the UI, covered by tests | An operator can do it today and it works |
| **partial** | Live, but a named part of it does not do what its name implies | Works, with a caveat you must state to a user |
| **dormant** | Real, tested code with **no live path** — nothing calls it, or the only caller is disabled | The code compiles and is correct; using it requires new wiring, not new logic |
| **stub** | The entry point exists and **refuses** | Calling it returns an error string or a hardcoded zero |

> [!important] "Dormant" is not "broken" and not "nearly done"
> The dormant subsystems here are deliberate foundations: the hard, testable part was built and
> the transport was not, because the transport needs credentials, a network stack, or hardware the
> project does not have. Re-implementing the foundation is wasted work; **wiring the transport is
> the actual remaining task**, and it is usually the smaller half.

---

## The ledger

### Trust and provenance

| Capability | Status | Evidence | What the limit actually is |
|---|---|---|---|
| Hash chain over `audit_log` (WP-18) | **shipped** | `src-tauri/src/db/queries.rs` `log_audit`; canonical form frozen — see [[Hash-Chained Provenance]] | Tamper-*evident*, not tamper-proof. Rows written before `v1.5.0` have `NULL` chain columns and verify as "no chain data" |
| Merkle checkpoints + portable proofs (WP-20/21) | **shipped** | `create_audit_checkpoint`, `export_audit_proof`, `verify_exported_proof` in `src-tauri/src/commands/audit.rs`; auto-checkpointing via `queries::auto_checkpoint_lineages` | `build_merkle_root`'s duplicate-last rule is permanently frozen — changing it invalidates every proof ever exported |
| Ed25519 signed event ledger (WP-67/75) | **partial** | `try_append_signed_event` wired at `src-tauri/src/commands/specimens.rs:280` (created), `:393`/`:642` (archive, single + bulk), `:992` (split), `src-tauri/src/commands/subcultures.rs:93` (passage) and `:96` (death) | **5 of the 6 declared lifecycle events are emitted.** `SPECIMEN_STATUS_CHANGED` is declared at `src-tauri/src/signed_ledger/lifecycle.rs:25` and listed in `ALL` at `:34`, but **nothing ever emits it** — the ledger advertises a vocabulary term it cannot produce. Non-lifecycle mutations (edits, media, compliance) are audit-chained but unsigned |
| Data-integrity self-check (WP-76) | **shipped** | `src-tauri/src/integrity/mod.rs` — 8 orphan checks, a chain-gap check, an FTS index check | **Read-only by construction.** There is no `UPDATE` or `DELETE` anywhere in `integrity/mod.rs` outside its test module. It reports damage; a human repairs it |
| On-chain anchoring (WP-66) | **partial** | `src-tauri/src/anchoring/mod.rs:12-18` states it outright; `record_anchor_txid` at `src-tauri/src/anchoring/store.rs:144` | **No automatic broadcast, deliberately.** The app builds the canonical `OP_RETURN` payload and verifies a txid trustlessly, but there is no wallet, no node client and no explorer API. The operator broadcasts by hand and pastes the txid back, which is validated as 64 hex chars before it is stored |
| Specimen passports (WP-70) | **shipped** | `src-tauri/src/passport/`, `src-tauri/src/commands/passport.rs` | Sign, verify and import all work. **No transport** — issuing downloads a file, importing reads one, and the issuer's public key must be exchanged out of band |
| Shared taxonomy registry (WP-71) | **shipped** | `src-tauri/src/registry/`, `src-tauri/src/commands/registry.rs` | Additive import with per-record dispositions. **No subscription server.** `gather_records` selects **all** `taxa` rows with no rank filter, despite `docs/taxonomy-registry.md` describing it as genus-and-above |
| Breeding coordination bundles (WP-72) | **shipped** | `src-tauri/src/coordination/`, `src-tauri/src/commands/coordination.rs` | Accept/skip set-union merge. **No coordination server** — same file-passing model as passports |

### Storage and sync

| Capability | Status | Evidence | What the limit actually is |
|---|---|---|---|
| SQLite backend | **shipped** | `rusqlite 0.32` bundled; `src-tauri/src/db/` | The **only** live backend. WAL, `foreign_keys=ON`, `busy_timeout=5000` |
| **PostgreSQL backend (WP-50)** | **dormant** | Feature declared `postgres = ["dep:sqlx"]` in `src-tauri/Cargo.toml`, **off by default**; without it every entry point returns the string at `src-tauri/src/db/postgres.rs:151`: *"This build was not compiled with PostgreSQL support. Rebuild with `--features postgres`…"* | Even **with** the feature on, this is not a second backend. `BOOTSTRAP_SCHEMA_SQL` (`src-tauri/src/db/postgres.rs:24`) defines **five** tables — `specimens`, `subcultures`, `audit_log`, `taxa`, `strains` — against 61 in SQLite and 59 migrations. It is a connectivity test plus a schema stub reachable only from admin-only commands in `src-tauri/src/commands/backend_config.rs`. **No read or write path in the app ever goes near it.** The Cargo.toml comment says so verbatim |
| **LAN sync transport (WP-51)** | **dormant** | `apply_incoming_changes` returns `applied: 0` — hardcoded at `src-tauri/src/commands/sync.rs:90`, documented at `:56-63` | Change detection, conflict detection and durable conflict recording (`sync_conflicts`) all work. **Nothing transports anything**: no discovery, no server, no client, no socket. Incoming changes are counted and conflicts recorded; **not one row is ever written into a domain table.** `get_changes_since_cursor` and `register_sync_peer` exist and are correct — they have no peer to talk to |
| Cloud backup crypto (WP-59) | **shipped** | `src-tauri/src/cloud/crypto.rs` — Argon2id (128 MiB / 3 iterations / 4 lanes) + AES-256-GCM, `STEL` magic header | Wrong passphrase and tampering return the **same** message, deliberately |
| Cloud backup to `local_nas` / `smb` | **shipped** | `src-tauri/src/commands/cloud_backup.rs` — auto-checkpoints, `wal_checkpoint(TRUNCATE)`, redacts `smtp_config.password` in the copy, encrypts, writes `{path}/{backup_id}.stelobak` | Both target types are just a filesystem path — an `smb` target is a mounted share |
| **S3 / SFTP backup targets** | **stub** | Gated at `src-tauri/src/commands/cloud_backup.rs:152` (backup), `:246` (restore), `:310` (sync) by `matches!(target_type, "local_nas" \| "smb")` | You can create the target and its credentials **are** encrypted at rest. Every operation then refuses: *"Target type '{}' is configured but not yet connected — only local_nas/smb targets … can complete a live backup today."* Disclosed in the module header at `:1-12` |
| **`backup_targets.schedule_cron`** | **dormant** | Validated on write at `src-tauri/src/commands/cloud_backup.rs:85`, stored at `:96-98`. Grep the whole crate: the only other hit is the DDL at `src-tauri/src/db/migrations.rs:1111` | **Nothing reads it.** `is_valid_cron` is a syntactic 5-field validator, not a scheduler. The only background loop in the app (`src-tauri/src/lib.rs:436`) handles notification dispatch and the submission monitor. A scheduled backup silently never runs |
| Delta sync segments | **partial** | `reconcile_cloud_sync` in `src-tauri/src/commands/cloud_backup.rs` | Publishes this device's WAL segments and reads peers' — then **detects and records** conflicts without merging. `new_changes` in the result means "found, not applied" |
| Local backup / restore | **shipped** | `src-tauri/src/commands/backup.rs` | `restore_backup` is admin-only, validates the SQLite magic bytes, and restarts the app |

### Extensibility and platforms

| Capability | Status | Evidence | What the limit actually is |
|---|---|---|---|
| Plugin vocabulary seeding (WP-61) | **shipped** | `src-tauri/src/plugins/loader.rs` — `INSERT OR IGNORE`, whitelist re-checked before interpolating a table name | Six seedable tables only. A manifest with no `profile` seeds **nothing** and returns `Ok(0)`. Uninstall never rolls back seeded rows |
| **Plugin WASM rule execution** | **dormant** | `wasm_module: String` is parsed and stored at `src-tauri/src/plugins/manifest.rs:48`; the rationale is at `src-tauri/src/plugins/loader.rs:6-13`. **`grep -i wasm src-tauri/Cargo.toml` returns nothing** — there is no `wasmtime`, no `wasmer`, no runtime dependency of any kind | The manifest field is **metadata only**. No `.wasm` file is extracted, loaded or executed, ever. The same is true of `dashboard_panels` and `report_templates` — recorded, never rendered |
| **Plugin-declared lab profiles** | **stub** | `allowed = ["plant_tissue_culture", "cell_culture", "mycology"]` is hard-coded at `src-tauri/src/commands/admin.rs:51`, and `app_config` carries `CHECK (lab_profile IN (…those three…))` from migration 015 at `src-tauri/src/db/migrations.rs:2099` — never dropped | A plugin can *seed vocabulary under* a new profile name; that profile can **never be activated**. `docs/plugin-authoring.md` says otherwise — see below. `integrity::ORPHAN_CHECKS` would additionally flag any such specimen as `specimen_unknown_lab_profile`, severity critical |
| Local AI (WP-56/56b) | **shipped** | `src-tauri/src/ai/ollama.rs` — a hand-rolled blocking HTTP/1.1 client over `std::net::TcpStream`, **no HTTP crate** | Requires the operator to run Ollama or a LocalAI-compatible server themselves. **No cloud provider path exists anywhere.** Every generation is written as a *pending* `ai_suggestions` row; approval appends to notes and is audited to the approving human |
| Windows / Linux / macOS desktop | **shipped** | Tauri v2, `src-tauri/` | — |
| Android | **shipped** | `src-tauri/gen/android/` present and built in CI; `versionCode` 29 at `v0.54.0` | — |
| **iOS** | **stub** | **There is no `src-tauri/gen/apple/` directory.** `README.md:132` — *"CI scaffold only, **never verified end-to-end**, not distributed"*; `ROADMAP.md:1017` (WP-53) states it plainly | With neither Apple secret configured — the actual current state — CI runs `cargo check --target aarch64-apple-ios-sim` and nothing more. That validates Rust-level compilation only; the Swift/Xcode side has never been exercised. No IPA has ever been produced. The blocker is a Mac and an Apple Developer account, not code |
| Frontend offline queue | **dormant** | `src/lib/offlineQueue.ts` is imported by exactly one file: `src/lib/offlineQueue.test.ts` | Written, tested, IndexedDB-shaped — and **not wired into `src/lib/api.ts`**. No UI path enqueues anything |

### Compliance and export

| Capability | Status | Evidence | What the limit actually is |
|---|---|---|---|
| Compliance rule catalogue + waivers (WP-74/77) | **partial** | `src-tauri/src/compliance_rules/mod.rs` | Catalogue and waiver logic are correct and unit-tested. **Profile gating is broken** — see the bug below |
| FDA Part 11 bundle | **shipped** | `src-tauri/src/commands/compliance_export.rs:39-43` — signs each file and includes `signing_public_key.b64` | Genuinely signed and independently verifiable |
| USDA PPQ 526 pre-fill | **partial** | `export_usda_permit` writes bare JSON | **Unsigned, no zip.** Refuses cross-lab specimens |
| CITES dossier | **partial** | `src-tauri/src/commands/compliance_export.rs:127` — `build_zip(&[("cites_dossier.json", …)])`, a **one-entry** archive | **No signature and no public key in the bundle**, contradicting `docs/regulatory-exports.md`. The CITES appendix is operator-asserted, not looked up |
| Submission pipeline (WP-68) | **partial** | `src-tauri/src/reg_submission/`, background monitor in `src-tauri/src/lib.rs` | Readiness evaluation and background auto-generation ship. **Nothing is submitted to any agency** — `mark_submitted` records the reference a human types in. The `acknowledged` status is in the CHECK constraint at `src-tauri/src/db/migrations.rs:771` and is dead: the comment at `:760` says it is *"reserved for a later"* release |
| Environmental monitoring (WP-78) | **partial** | `src-tauri/src/monitoring/mod.rs` — `default_threshold(reading_type)` | Manual readings only. **Thresholds are hardcoded** in Rust, not configurable, and unknown reading types return `None` (no evaluation at all). The flag query has no lab-profile filter |
| CSV / JSON / Excel export, Excel import, printing | **shipped** | `src/lib/exportUtils.ts`, `src/lib/importUtils.ts`, `src/lib/printUtils.ts` | Exports are lab-scoped and exclude archived specimens. Neither export nor import writes an audit entry |
| Regulatory transmission of any kind | **does not exist** | No HTTP client in the backend; no `http:*` permission in `src-tauri/capabilities/default.json` | Every "export" writes a file to local disk and returns its path. There is no portal client, no credential store, no mail step for any regulatory artefact |

### Networking, in one row

| Capability | Status | Evidence |
|---|---|---|
| **Any outbound network call in normal operation** | **does not exist** | No `reqwest`, `ureq` or `tauri-plugin-http` in `src-tauri/Cargo.toml`; no `http:*` in `src-tauri/capabilities/default.json`; CSP `connect-src 'self' ipc: http://ipc.localhost`; no `@tauri-apps/plugin-http` in `package.json`. The two exceptions are both to `127.0.0.1`-class hosts the operator configures: SMTP (`lettre`) and the local AI runtime (`src-tauri/src/ai/ollama.rs`, raw `TcpStream`) |

This is the property that makes [[Importing NCBI Taxonomy]] and [[Federated Exchange]] work the way
they do: the NCBI panel *builds* an E-utilities URL and hands it to the operator with a **Copy URL**
button, and federation moves files a human carries.

---

## Known-wrong and disclosed

> [!danger] The compliance profile-gating bug
> Three call sites read the active lab profile from the **wrong table**:
>
> ```rust
> // src-tauri/src/commands/compliance.rs:235  and  :644
> // src-tauri/src/reg_submission/mod.rs:168
> let profile = queries::read_setting(&db.conn, "lab_profile", "plant_tissue_culture");
> ```
>
> `read_setting` queries `app_settings`. The lab profile lives in **`app_config.lab_profile`** and
> is read correctly everywhere else by `active_profile` at `src-tauri/src/db/vocabulary.rs:5`.
> **Nothing in the repository ever writes an `app_settings` row keyed `lab_profile`** — grep
> confirms the only four hits on that string are the three reads above and a `row.get` on the
> `specimens` column.
>
> Consequence: every installation is gated as `plant_tissue_culture`. The citrus-HLB rule fires in
> mycology and cell-culture labs; the mycology and mycoplasma rules never fire anywhere. The
> row-level SQL filters *inside* those same functions are correct — it is only the rule gating that
> is wrong, which is why the symptom is "wrong rules, right specimens".

Two smaller dead spots, both harmless but worth not re-discovering:

- `SPECIMEN_STATUS_CHANGED` — declared and exported, never emitted (`signed_ledger/lifecycle.rs:25`).
- `regulatory_submissions.status = 'acknowledged'` — permitted by the CHECK constraint, reachable
  by no code path (`db/migrations.rs:760,771`).

### Where the docs overstate

`docs/` holds specifications, several written before or ahead of the code. Believe the code.

| Document | Claim | Reality |
|---|---|---|
| `docs/plugin-authoring.md` | A plugin-declared profile "now appears as a **selectable lab profile**" | False — hard-coded allowlist at `src-tauri/src/commands/admin.rs:51` plus an undropped CHECK constraint |
| `docs/regulatory-exports.md` | Verify "a **Part 11 or CITES** export's signatures … given the bundled `signing_public_key.b64`" | CITES ships neither a signature nor a key (`commands/compliance_export.rs:127`). USDA is bare unsigned JSON |
| `docs/merkle-checkpoints.md` | "No automatic checkpointing yet", "No proof export yet", "`anchored_txid` is always NULL" | All three stale — auto-checkpointing, proof export and `record_anchor_txid` all ship |
| `docs/signed-event-ledger.md` §5 | "**One** wired demonstrating integration" | Stale (WP-67 state). WP-75 wired passage, death, archive and split as well |
| `docs/taxonomy-registry.md` | The registry carries "genus-and-above" taxa | `registry::store::gather_records` selects **all** `taxa` rows, no rank filter |
| `docs/*` on federated import | Writes a `passport_imported` / `registry_imported` audit action | The action string is literally `"import"`; `entity_type` is what distinguishes them |
| `SKILLS.md` §2 | "**52 migrations** today; next is 053" | **59** migrations; next is **060**. See [[Migrations]] |
| `SKILLS.md` §3 | Baseline "642 Rust tests / 679 … 113 TS tests … 418 files" | Stale by a wide margin. Re-measure with [[Build and Test Commands]] rather than trusting either number |

`docs/on-chain-anchoring.md`, `docs/local-ai.md`, `docs/vocabulary-system.md`,
`docs/merkle-proofs.md` and `docs/breeding-coordination.md` were checked against the code and are
**accurate**.

---

## Do not re-plan these — they shipped in `v0.54.0`

> [!success] Already done
> These were the obvious gaps three weeks ago. They are closed. Proposing them again is the most
> likely way to waste a work packet.

| Shipped in `v0.54.0` | Evidence |
|---|---|
| **Species are filed under their genus on create.** The "full Species Registry, empty Taxonomy Navigator" symptom is fixed at the write path, not patched in the UI | [[Taxonomy Backbone]]; `create_species` now links, and the genus half of an update relinks |
| **`migration_058` repairs already-drifted databases**, idempotently, case-insensitively on genus, and never flattens a deliberately deep hand-built backbone | `src-tauri/src/db/migrations.rs:339` `migration_058_relink_orphan_species` |
| **`rebuild_species_taxonomy`** exposes that same repair as a supervisor action from the Species Registry *and* from the navigator's empty state | `src-tauri/src/db/queries.rs:1170`, `src-tauri/src/commands/species.rs:197` |
| **`locate_species`** resolves a species into the shape the navigator walks, so Registry → Taxonomy is one navigation path rather than two | `src-tauri/src/db/queries.rs:1320`, `src-tauri/src/commands/taxa.rs:319` |
| **The lab-map room designer** — draw furniture on a grid with a shelf/tray breakdown, occupancy shading from live counts | [[Drawing the Lab]], [[Lab Layout Model]]; `src-tauri/src/commands/locations.rs:266` `get_location_occupancy` |
| **`migration_059` adds `locations.layout_json`**, one nullable column holding the plan as a document | `src-tauri/src/db/migrations.rs:356` `migration_059_location_layout` |
| **Add Specimen's location dropdowns come from the drawing**, replacing the invented hardcoded Room 1–5 / Rack A–D list. Labs that never open the designer keep the old lists | [[Drawing the Lab]] |
| **NCBI import reports what it dropped.** Out-of-backbone records return in `skipped_records` with a reason instead of a bare `continue` | `src-tauri/src/models/taxon.rs:135`; [[Importing NCBI Taxonomy]] |
| **NCBI import builds a real tree.** `parent_ncbi_id` is resolved against the batch and against existing taxa, and `taxon_path` is recomputed cycle-safely over affected subtrees | [[Importing NCBI Taxonomy]] |
| **The import box takes what you actually have** — `esummary` JSON, `efetch` XML (expanding `LineageEx`), `taxdump` rows, CSV/TSV, plain JSON — with a per-row preview and a dry run | [[Importing NCBI Taxonomy]] |
| **`get_location_map_data` is lab-scoped.** It previously counted other profiles' specimens; both it and `get_location_occupancy` now go through `vocabulary::active_lab_sql` | [[Lab Profiles]] |
| **Add Specimen can register a strain inline**, and links to the Species Registry when a lab has no species yet | [[Specimens Strains and Species]] |
| **The version series was renumbered to pre-1.0** across all six manifests. Android `versionCode` still increases (28 → 29) | `CHANGELOG.md` `[0.54.0]`; [[Build and Test Commands]] |
| **Dark-mode token pass**, the dead Google Fonts `@import` removed, and `--legacy-peer-deps` dropped from all five CI workflows | `CHANGELOG.md` `[0.54.0]` |

---

## Using this note

> [!tip] Three questions to ask before writing a plan
> 1. **Is it already in the "do not re-plan" table?** If so, the work is reading the existing
>    implementation, not rebuilding it.
> 2. **Is it rated dormant?** Then the foundation exists and is tested. The task is the transport —
>    wire `applied` in `commands/sync.rs`, add an S3 client behind the existing target gate, add a
>    scheduler that reads `schedule_cron`, import `offlineQueue` into `api.ts`. Do not restart from
>    the model layer.
> 3. **Does a `docs/` file say it works?** Check the code. Six specifications currently overstate.

When a status changes, edit **this note in the same commit** as the code. A correction that lands
only in a domain note will not be read in time.

---

**Back to [[Home]]**

#meta #reference #stub #dormant
