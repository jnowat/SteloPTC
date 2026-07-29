---
title: Rust Backend
aliases: [src-tauri, Backend]
tags: [architecture, rust, tauri, backend, sqlite]
type: architecture
status: shipped
version: 0.54.0
created: 2026-07-29
updated: 2026-07-29
cssclasses: [wide-tables]
---

> [!abstract] In one sentence
> `src-tauri/src/` is a single Rust crate (`stelo-ptc`, lib name `stelo_ptc_lib`) that owns one
> `rusqlite::Connection` behind one global mutex, exposes exactly 263 synchronous
> `#[tauri::command]` functions that all take a session token and all return
> `Result<T, String>`, and keeps its business logic in DB-free or connection-only modules so the
> whole thing tests headlessly.

## Crate shape

| Item | Value |
|---|---|
| package / lib | `stelo-ptc` / `stelo_ptc_lib` — the name every test and benchmark imports |
| crate-type | `["lib", "cdylib", "staticlib"]` (the last two for mobile) |
| edition / license | 2021 / `LicenseRef-Proprietary` |
| default feature | `tauri-commands` — pulls in `tauri` + the dialog/fs/shell/notification plugins and gates `pub mod commands` and `pub fn run()` |
| optional feature | `postgres` — compiles the real `db::postgres` connector instead of the erroring stubs; **never wired into the live query path** |
| headless build | `cargo test --lib --no-default-features` — the canonical build for a sandbox with no GTK/WebKit |

> [!danger] `commands/` only compiles under the default feature
> `cargo test --lib --no-default-features` and `cargo clippy --no-default-features` never see
> `src-tauri/src/commands/`. A type error there passes every headless gate and fails CI. This is
> not hypothetical: `v1.53.2` exists because an `i32`/`i64` mismatch in `create_subculture` kept
> `master` red for six days. `SKILLS.md` §3 carries the standing rule to run the full-feature
> build before pushing anything under `commands/`. See [[Build and Test Commands]].

## Module map

| Module | Shape | What it is |
|---|---|---|
| `lib.rs` | 481 lines | `AppState`, the `generate_handler!` list of 263 commands, `.setup()`, the background scheduler |
| `main.rs` | 9 lines | Calls `run()` under the feature; prints "desktop UI unavailable" without it |
| `commands/` | 42 modules, ~14 000 lines | The IPC surface. Session + role gating, then delegation. Biggest: `specimens.rs` (1273), `audit.rs` (1152), `compliance.rs` (1028) |
| `db/` | 14 files | `mod.rs` (the whole `Database` abstraction, 83 lines), `queries.rs` (7432 — audit chain, Merkle, taxa, strains, cryo), `migrations.rs` (5775), `dashboard.rs`, `vocabulary.rs`, `permissions.rs`, `analytics.rs`, `work_queue.rs`, `notifications.rs`, `sensors.rs`, `sync.rs`, `backend.rs`, `postgres.rs`, `fixtures.rs` |
| `models/` | 23 files, no tests | Plain serde structs. `User.password_hash` is `#[serde(skip_serializing)]` so it can never cross IPC |
| `auth/mod.rs` | 602 lines | Opaque bearer tokens in a `sessions` table — no JWT, no OAuth. Password policy, session TTL, `LoginThrottle` |
| `signed_ledger/` | `mod.rs`, `lifecycle.rs` | Per-user Ed25519-signed, hash-chained event ledger (WP-67/75) |
| `anchoring/` | `mod.rs`, `store.rs` | Dogecoin `OP_RETURN` payload build + trustless verify. Does not broadcast |
| `integrity/mod.rs` | 518 lines | Read-only 10-check lab health report (WP-76) |
| `passport/`, `registry/`, `coordination/` | pure `mod.rs` + `store.rs` each | The three signed cross-lab document formats (WP-70/71/72) |
| `compliance_export/` | `bundle.rs`, `signing.rs`, `zip_writer.rs` | FDA Part 11 / USDA PPQ 526 / CITES bundles; Ed25519 primitives shared by every other subsystem |
| `compliance_rules/mod.rs` | 308 lines | Pure rule catalogue + profile scoping + waiver decisions |
| `reg_submission/mod.rs` | 512 lines | Readiness evaluation and submission lifecycle over export bundles |
| `monitoring/mod.rs` | 146 lines | Pure environmental range evaluation |
| `plugins/` | `manifest.rs`, `loader.rs` | `.steloplugin` manifest validation + additive vocabulary seeding. No runtime |
| `ai/ollama.rs` | 553 lines | Hand-rolled HTTP/1.1 client to a **local** Ollama / OpenAI-compatible endpoint |
| `cloud/` | `crypto.rs`, `targets.rs`, `sync.rs` | Argon2id + AES-256-GCM zero-knowledge backup; WAL segment naming |

The house pattern for every subsystem is three layers: **pure core (no DB) → `store.rs`
(connection-only, unit-testable) → `commands/*.rs` (session and role gating only)**. Everything
except `commands/` compiles without the Tauri feature, which is what makes 700-odd tests runnable
in a headless container.

## `AppState` and the shared DB handle

```rust
pub struct AppState {
    pub db: Mutex<Database>,
    pub dashboard_cache: Mutex<Option<db::dashboard::DashboardCacheEntry>>,
    pub login_throttle: auth::LoginThrottle,
    pub degraded_reason: Option<String>,
}
```

`Database` is a newtype over one `rusqlite::Connection`. There is no pool, no trait, no async.
`Database::new()` opens `<dirs>/stelo_ptc.db` and sets `journal_mode=WAL`, `foreign_keys=ON`,
`busy_timeout=5000`; the directory is `%APPDATA%/SteloPTC` on Windows and `$HOME/.steloptc`
elsewhere, falling back to the **current working directory** if neither env var is set.

`AppState::db()` is the only way a command takes the lock, and it recovers from mutex poisoning:

```rust
pub fn db(&self) -> std::sync::MutexGuard<'_, Database> {
    self.db.lock().unwrap_or_else(|poisoned| {
        eprintln!("WARN: the database mutex was poisoned by an earlier panic; recovering. \
                   The operation that panicked was rolled back.");
        poisoned.into_inner()
    })
}
```

> [!important] Why recovering from poisoning is correct here
> Poisoning is permanent. Propagating it turns one unanticipated panic into an app that keeps
> running but fails every command with an opaque string. Recovery is sound because a panic cannot
> leave the connection invalid — rusqlite rolls back any in-flight transaction when the
> `Transaction` guard drops during unwind, and SQLite's own state is transactional.

`dashboard_cache` (60-second TTL, `db::dashboard`) and `login_throttle` are **in-memory only** and
never persisted; both are recomputed from scratch on every launch.

## Boot sequence — `run()`

1. `Database::new()`; on failure fall back to `Database::new_in_memory()` and record a
   `degraded_reason` (below).
2. Register the four Tauri plugins (`dialog`, `fs`, `shell`, `notification`).
3. `.manage(state)` and `.invoke_handler(tauri::generate_handler![ …263 paths… ])`.
4. `.setup()`: `run_migrations()` → `"Migration error: {e}"`, `seed_defaults()` → `"Seed error: {e}"`,
   then `drop(db)` before spawning anything.
5. Spawn **one** `tauri::async_runtime::spawn` loop. Per iteration it reads
   `app_settings.notification_check_interval_minutes` (default `"15"`), **sleeps first** —
   deliberately, so a dev restart never fires an immediate notification burst — then calls
   `commands::notifications::dispatch_due_notifications` and `commands::reg_submission::monitor`.
   Neither error breaks the loop; both `eprintln!`.

`seed_defaults` returns early if `SELECT COUNT(*) FROM users > 0`. On a truly empty database it
creates the `admin` / `admin` account with `must_change_password = 1`, six species
(`ASP-OFF`, `NAN-DOM`, `CIT-SIN`, `CIT-LIM`, `CIT-PAR`, `CIT-RET`), and a two-level tag tree.

## The command convention

Every one of the 263 commands follows this shape. It is a convention, not an abstraction — there is
no middleware, no macro beyond `#[tauri::command]`, and no way to enforce it except review.

```rust
#[tauri::command]
pub fn my_command(state: State<AppState>, token: String, /* args */) -> Result<T, String> {
    let db = state.db();
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }
    // ... rusqlite work, .map_err(|e| e.to_string())
    queries::log_audit(&db.conn, Some(&user.id), "create", "thing", Some(&id), None, None, None).ok();
    Ok(value)
}
```

> [!danger] Four invariants
> 1. **`State<AppState>` first, `token: String` second.** Only two commands skip
>    `validate_session`: `commands::auth::login`, and `commands::admin::get_degraded_reason`,
>    which is unauthenticated on purpose.
> 2. **Synchronous `fn`, never `async fn`** — universally. Where async is unavoidable (the
>    PostgreSQL connector) the command bridges with `tauri::async_runtime::block_on` rather than
>    introduce the first async command and its `State<'_, T>` lifetime rules.
> 3. **`Result<T, String>` always.** The `String` *is* the UI message; there is no error code and
>    no i18n layer.
> 4. **Registration is manual.** `generate_handler!` is a compile-time macro over a literal list.
>    A command not in that list simply does not exist to the WebView. See [[The IPC Seam]] for the
>    full add-a-command recipe.

Role gates come from `models/user.rs`: `can_write()` = Admin | Supervisor | Tech (53 call sites),
`can_manage()` = Admin | Supervisor (52), `is_admin()` (25). Row mapping uses
`role.parse().unwrap_or(UserRole::Guest)` everywhere, so an unknown role string degrades to the
least privilege rather than erroring. `validate_session` additionally **rejects any session whose
user has `must_change_password = 1`**; only `change_password` and `get_current_user` use the
`validate_session_allow_password_change` carve-out. Full matrix in [[Roles and Permissions]].

## Error handling, and what reaches the UI

| Layer | Type | Convention |
|---|---|---|
| `db::*` pure helpers | `DbResult<T>` = `Result<T, DbError>` | `?` on rusqlite errors via `#[from]` |
| `db::{dashboard, work_queue, sensors, notifications}` | `Result<T, String>` | already user-facing text |
| `#[tauri::command]` | **always** `Result<T, String>` | `.map_err(|e| e.to_string())` or `format!("Failed to …: {}", e)` |

`DbError` has four variants — `Sqlite`, `NotFound`, `Migration`, `Constraint`. In practice only
`Sqlite` and hand-constructed `Constraint` values appear.

Rules the code actually holds itself to:

- **The error string is a contract with the frontend.** `src/lib/api.ts` substring-matches on
  `"Session expired or invalid"` to clear auth. Change that text and auto-logout breaks silently.
- **Multi-statement writes use `conn.unchecked_transaction()`**, not `conn.transaction()`, because
  the `Connection` sits behind a `MutexGuard`. `Transaction` derefs to `Connection`, so `log_audit`
  can be handed `&tx`.
- **Audit writes are fire-and-forget by default** — of the 104 `queries::log_audit*` calls in
  `commands/`, 83 end in `.ok();`, so a failed audit write does not fail the command. The rest are
  `?`-propagated inside a transaction where atomicity matters.
  `create_specimen` wraps INSERT + audit in one transaction precisely so a specimen without an
  audit entry can never be committed.
- **Validation is factored into pure functions** so it tests without a DB:
  `queries::check_profile_change_allowed`, `queries::validate_strain_status_transition`,
  `db::backend::validate_backend_switch`, `db::permissions::validate_admin_role`,
  `db::sensors::validate_source`, `auth::validate_password`.
- **Pagination is clamped, not trusted.** `PaginationParams` clamps `per_page` to `[1, 1000]` and
  uses `saturating_sub`/`saturating_mul` for the offset; `configured_pedigree_max_depth` clamps to
  `[1, 20]`.

> [!warning] Row-mapping strictness is inconsistent
> `list_specimens` collects into `rusqlite::Result<Vec<_>>` and fails the whole call with
> `"Failed to read specimen rows: {e}"` — the comment argues that showing 19 of 20 cultures and
> calling it 20 is worse than showing an error. Older sites (`list_users`,
> `list_audit_entries_by_cursor`, `passport::store::gather_provenance`) still use
> `.filter_map(|r| r.ok())` and silently drop rows. The verification paths in `signed_ledger` and
> the audit chain are deliberately strict so a mapping bug can never masquerade as tamper
> evidence.

## Degraded / in-memory fallback

Triggered **only** when `Database::new()` fails. The app still opens — with an in-memory database
that runs migrations and `seed_defaults` like any other, so it looks like a working empty lab. The
only thing preventing a day of lost work is the banner:

```
SteloPTC could not open its database ({e}).

It is running in TEMPORARY mode: nothing you enter will be saved. Close the app and resolve the
problem before recording any work.
```

Surfaced by `#[tauri::command] get_degraded_reason(state) -> Option<String>`, which is
**deliberately unauthenticated** — the warning has to arrive before the user trusts the app with
data, and on a fresh in-memory database the seeded account may not be the one they expect.

> [!warning] The banner cannot currently fire before login
> `src/lib/api.ts` routes `getDegradedReason()` through `call()`, whose `getToken()` throws
> `Error('Not authenticated')` when no token exists — so the unauthenticated command is never
> reached on the login screen. The banner appears only once a session exists. Recorded in
> [[Failure Reference]].

Two further honest edges: `new_in_memory()` sets **only** `PRAGMA foreign_keys=ON` — no WAL, no
`busy_timeout` — so every test and benchmark runs under weaker settings than production; and
`create_backup` refuses with `"Database file not found (using in-memory database)"`. If even the
in-memory open fails, the app panics with `"Failed to create even an in-memory database"`.

## Tests

There are no separate test crates for the bulk of the suite: tests live in `#[cfg(test)] mod tests`
blocks at the bottom of the file they cover, using an in-memory database.

```rust
fn migrated_db() -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory DB");
    run_all(&conn).expect("all migrations must succeed on a fresh in-memory DB");
    conn
}
```

| Location | `#[test]` count (grep) |
|---|---|
| `src/db/` | 464 — of which `queries.rs` 184, `migrations.rs` 131, `dashboard.rs` 31 |
| `src/commands/` | 58 |
| `src/coordination/`, `registry/`, `ai/`, `anchoring/` | 28 · 26 · 24 · 23 |
| `src/auth/`, `passport/`, `signed_ledger/`, `cloud/` | 21 · 21 · 18 · 18 |
| `src/compliance_rules/`, `integrity/`, `compliance_export/`, `plugins/`, `reg_submission/`, `monitoring/` | 13 · 12 · 10 · 10 · 10 · 5 |
| `src/models/` | 0 — plain serde structs |
| `src-tauri/tests/` | 10, across `db_tests.rs` (migrations + the death workflow + lab profile) and `wp63_fixtures.rs` |

`cargo test --lib` reported **758 passing** at the `v0.54.0` release under the full
`tauri-commands` feature; the raw grep above sums slightly higher because a handful of tests are
`cfg`-gated on the `postgres` feature. Benchmarks live in `src-tauri/benches/performance.rs`
(Criterion, `harness = false`) and talk to `db::` directly, never the command layer, so they build
without GTK/WebKit. `.github/workflows/benchmarks.yml` uploads Criterion artifacts but does **not**
automate regression comparison — a maintainer compares by hand.

## See also

- [[The IPC Seam]] · [[Data Model]] · [[Trust Layer]] · [[Migrations]]
- [[Command Reference]] · [[Build and Test Commands]] · [[Roles and Permissions]]

**Back to [[Home]]**

#architecture #rust #tauri #backend
