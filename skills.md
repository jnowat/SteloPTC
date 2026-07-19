# skills.md — Contributor Playbook for SteloPTC

A single-page operating guide for anyone editing this repository — human or AI (Claude,
Grok, etc.). Read this before touching code. It captures the architecture, the golden
rules, the exact verification gates, and the known traps that have bitten this codebase.

> **North star:** SteloPTC is a released, local-first lab-provenance app (v1.53.1) with a
> fully green test suite. It is **mature, not greenfield.** Prefer surgical, verified changes
> over sweeping refactors. Every change must keep the audit hash chain, the test suite, and
> clippy green.

---

## 1. What this is

A desktop + Android app (Tauri 2) for tracking lab cultures through their lifecycle on a
tamper-evident SHA-256 hash chain. One engine serves **three lab profiles**:
Plant Tissue Culture (Plantae), Cell Culture (Animalia), and Mycology (Fungi).

- **Backend:** Rust — `src-tauri/src`
- **Frontend:** Svelte 5 + TypeScript — `src`
- **DB:** SQLite (bundled, WAL), single `Connection` behind a `Mutex` in `AppState`
- **Docs:** `README.md` (overview), `ROADMAP.md` (per-work-packet status), `UserManual.md`,
  `CHANGELOG.md`, `docs/*.md`

## 2. Architecture map (where things live)

| Area | Path | Notes |
|---|---|---|
| Tauri command registry | `src-tauri/src/lib.rs` | Every `#[tauri::command]` is registered here in `invoke_handler![]`. Add new commands here. |
| Commands (API layer) | `src-tauri/src/commands/*.rs` | One file per domain area. Pattern: lock DB → `validate_session` → permission check → do work → `log_audit`. |
| Queries (SQL) | `src-tauri/src/db/queries.rs` | Large shared query module (~6.8k lines). Most raw SQL lives here. |
| Migrations | `src-tauri/src/db/migrations.rs` | Append-only, numbered. **52 migrations today; next is 053.** Never edit a shipped migration — add a new one. |
| Models | `src-tauri/src/models/*.rs` | serde structs. **Field names here are the API contract** the frontend receives (no `#[serde(rename)]` in use). |
| Profiles / vocabulary | `src-tauri/src/db/vocabulary.rs` + `src/lib/profile.ts` | The domain-separation machinery. See §4. |
| Frontend API bridge | `src/lib/api.ts` | Single `call()` wrapper around Tauri `invoke` — catches/normalizes/rethrows as `Error`. All UI calls go through it. |
| Stores | `src/lib/stores/{app,auth}.ts` | `currentView`, `darkMode`, auth token, `labProfile`. |
| Components | `src/lib/components/*.svelte` | Rendered by a `{#if $currentView === …}` switch in `src/App.svelte`. A component not referenced there is dead. |
| Pure utils (tested) | `src/lib/{exportUtils,importUtils,offlineQueue,profile,utils,printUtils}.ts` | Logic extracted so it's unit-testable without a webview. Add tests here. |

## 3. Verification gates (run before every commit — these are the CI gates)

```bash
# Frontend
npm test            # Vitest  (unit tests)
npm run check       # svelte-check + TypeScript — must be 0 errors / 0 warnings

# Backend (from src-tauri/)
cargo test --lib --no-default-features        # pure-logic tests (no GTK/WebKit needed)
cargo clippy --lib --no-default-features -- -D warnings   # warnings are HARD errors in CI
```

- CI (`.github/workflows/test.yml`) runs `cargo test --lib` **with** default features
  (the `tauri-commands` feature adds the command-layer tests on top). That full build needs
  `webkit2gtk`/GTK and usually **can't run in a headless sandbox** — so verify locally with
  `--no-default-features` + clippy, and lean on unit tests for command logic you can't
  exercise here.
- Current baseline: **640 Rust tests, 113 TS tests, clippy clean, svelte-check clean.**
- `cargo test`/`clippy` compile from scratch is slow (~40–60s). Compile once, batch your edits.

## 4. THE GOLDEN RULE: domain separation

The three biological domains are kept apart by **data, not code branches**. Respect this or
you corrupt the whole design.

- **Vocabularies are data.** Stages, propagation/culture methods, hormone/additive types,
  compliance record types & agencies, and inventory categories live in **profile-scoped
  lookup tables** (`UNIQUE(profile, code)`), seeded per profile in migrations. Adding a value
  = a data insert, **not** a schema migration or Rust recompile. See `docs/vocabulary-system.md`.
- **Never hardcode a domain term** (`explant`, `callus`, `grain spawn`, `passage`, `cell_line`, …)
  in Rust or Svelte where it should come from the vocabulary/profile. If you're typing a
  biology word into a `match`, an `<option>`, or a `CHECK` constraint, stop.
- **Always validate user-supplied vocabulary against the active profile** before writing it.
  Use `vocabulary::require_selectable_stage(conn, &profile, &code)` (and friends). The
  create-specimen path was silently accepting cross-profile stages until this was added —
  don't reintroduce that gap on any new write path.
- **Frontend:** the active profile is the `labProfile` store; `PROFILE_DOMAIN` and
  `DOMAIN_MANIFESTS` in `src/lib/profile.ts` give per-domain rank order, strain-type labels,
  and confirmation-method labels. Gate discipline-specific UI with `{#if $labProfile === …}`.
- **Shared tables use nullable, documented domain-specific columns** (e.g. `subcultures`
  carries PTC pH/media, cell-culture PDL, and mycology `colonization_pct`). That's the
  intended single-table pattern — a `subculture`/`passage`/`colonization` record is one
  shared type. Don't "fix" it by forking tables; relabel in the UI instead.

**To add a whole new profile:** add its rows to each vocabulary table via a new migration,
add it to `LabProfile`/`PROFILE_DOMAIN`/`LAB_PROFILE_LABELS`/`DOMAIN_MANIFESTS` in
`profile.ts`, and register the domain in `app_config.domain`. Prefer a `.steloplugin`
vocabulary pack (see `docs/plugin-authoring.md`) when you don't need new columns.

## 5. Non-negotiables (things that must not regress)

- **Audit hash chain.** Every create/update/delete goes through `queries::log_audit`. The
  per-lineage SHA-256 chain (species → strain → specimen; splits fork from the parent hash)
  is the product. Don't write entities without their audit entry, and don't change how
  `compute_entry_hash` / canonical bytes are formed without a migration + verification story.
- **Multi-step writes are transactional.** `split_specimen`, `create_subculture`,
  `create_media_batch`, `thaw_vial`, `reanchor_taxon_chain`, `import_xlsx`, etc. wrap their
  writes in a transaction and roll back on error. Keep new multi-step writes atomic.
- **Never hold the DB `Mutex` across a panic-prone or network call.** A panic while the guard
  is held **poisons the mutex** and kills DB access app-wide. Parse external bytes defensively
  (see the `dechunk` UTF-8 lesson in §7).
- **Permissions & auth.** Commands call `validate_session` then check `role.can_write()` /
  role gates. Field-level masking (`MASKABLE_FIELDS`) has a tripwire test — don't add a read
  path to a maskable field without a matching mask.
- **CSP is locked down** (`script-src 'self'`). No remote scripts, no `unsafe-eval`.

## 6. Conventions

- **Work packets.** History is `WP-xx` packets, one per release. A change should update
  `CHANGELOG.md`, bump the version in `package.json` **and** `src-tauri/tauri.conf.json`
  (keep them in sync), and reflect status in `ROADMAP.md`.
- **Docs must match reality.** When you ship a feature, move it from "planned" to "shipped" in
  ROADMAP/UserManual and update counts. When you change the test count, update the README
  badge **and** the prose in the Testing section (they drift apart — grep for the old number).
- **Be honest about foundation-only features.** Several capabilities ship deliberately
  incomplete (PostgreSQL connector, LAN sync transport, S3/SFTP targets, plugin WASM
  execution, iOS). They are disclosed in-app and in ROADMAP's "Foundation-only" table. Keep
  that disclosure accurate — don't imply more than is wired.
- **Tests are part of the change.** New logic gets a unit test. For exporters/importers,
  **test with the real serialized entity shape** (the model's field names), not a hand-authored
  object — fabricated-shape tests hid a whole class of blank-column export bugs (§7).

## 7. Known traps (learned the hard way)

- **Exporter field names must match the model.** `src/lib/exportUtils.ts` reads plain `any`
  objects; a typo (`min_stock` vs `minimum_stock`, `batch_code` vs `batch_id`, `authority`
  vs `agency`) silently ships a blank column. Cross-check every read against
  `src-tauri/src/models/*.rs`.
- **Byte boundaries, not char boundaries.** HTTP chunk sizes and `Content-Length` are byte
  counts; slicing a `&str` by them panics on multi-byte UTF-8 (`°`, `µ`, `×`). Decode over
  `&[u8]` and convert to `String` once, at the end (`src-tauri/src/ai/ollama.rs`).
- **Genesis rows start at `chain_seq = 0`.** Don't compute counts as `chain_seq - 1` (it
  underflows `usize`). Use the loop index.
- **Pagination params come from the request.** Clamp `per_page` (0 → `LIMIT 0`) and use
  `saturating_mul` for the offset (`PaginationParams` in `queries.rs`).
- **`$lib/` alias exists only for type-checking**, not for the Vite build — importing from it
  in a real component breaks the build. Use relative imports.

## 8. Open follow-ups (audit-flagged — verify with the full command suite before shipping)

**Fixed in v1.48.0 (WP-73)** — kept here as a record so the fixes aren't undone:

- ~~**Signed-ledger key substitution (MEDIUM, security).**~~ Fixed: `signed_ledger::verify_ledger`
  now treats a missing registered `user_signing_keys` row as a verification *failure* rather than
  silently skipping the cross-check (test `deleted_registered_key_forgery_is_detected`).
- ~~**Server-side forced-password-change (MEDIUM, security).**~~ Fixed: `validate_session` now
  rejects any session whose user has `must_change_password = 1`; `change_password` and
  `get_current_user` use the `validate_session_allow_password_change` carve-out. Default-deny across
  all command modules — don't route a normal command through the allow-variant.
- ~~**Domain UI wiring gaps (MEDIUM/LOW).**~~ Fixed: `StrainManager.svelte` derives strain types from
  the domain manifest; `origin_type`/`contaminant_type` are single-sourced in `profile.ts`
  (`ORIGIN_TYPE_META`/`CONTAMINANT_TYPE_LABELS`); Cell Count/PDL fields are cell-culture-gated; nav
  has a `profiles` filter (Media → PTC, new Fruiting view → Mycology).

**Fixed in v1.49.0 (WP-74)** — kept here as a record so the fix isn't undone:

- ~~**Compliance rule engine is still PTC-only.**~~ Fixed: the auto-flag rules in
  `commands/compliance.rs` are now gated through the pure `compliance_rules` catalogue
  (`is_rule_active(flag_type, &profile)`), so the citrus HLB rule (`CIT-%`) no longer fires in
  mycology/cell-culture labs. Adding a rule = one `RuleDef` entry + its SQL block gated by
  `is_rule_active`. Per-specimen exceptions are handled by WP-77 flag waivers, not by editing rules.

**Still open:**

- **Compliance-rule thresholds are hardcoded defaults.** The WP-78 environmental ranges (and the
  WP-33/WP-44 interval settings) are sensible defaults, not per-lab-configurable in the UI. A
  user-facing threshold editor is a disclosed follow-up.
- **Signed lifecycle events cover creation, passage, and split only** (WP-75). Extending
  `try_append_signed_event` to the remaining ~25 mutation commands is the same incremental
  one-line-per-site work disclosed in WP-67.
- **Foundation-only features remain foundation-only** (PostgreSQL connector, LAN sync transport,
  S3/SFTP targets, plugin WASM execution, iOS) — disclosed in ROADMAP; keep the disclosure honest.

## 9. Quick recipe: adding a command

1. Add the `#[tauri::command] pub fn …` in `src-tauri/src/commands/<area>.rs`
   (lock DB → `validate_session` → permission check → work → `log_audit`).
2. Put SQL in `db/queries.rs` (parameterized — never string-format runtime values).
3. If it needs schema, add migration `052…` in `migrations.rs` (+ a `migration_052_*` test).
4. Register it in `lib.rs` `invoke_handler![]`.
5. Add a wrapper in `src/lib/api.ts`; call it from the component through that wrapper.
6. Add tests. Run the four §3 gates. Update CHANGELOG/version/ROADMAP.
