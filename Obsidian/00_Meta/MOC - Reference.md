---
title: MOC - Reference
aliases:
  - Reference Map
  - Reference MOC
tags:
  - moc
  - meta
  - reference
type: moc
status: living
version: 0.54.0
created: 2026-07-29
updated: 2026-07-29
cssclasses:
  - wide-tables
---

> [!abstract] In one sentence
> The `Reference/` folder is the lookup layer — every `#[tauri::command]` with its role gate, every
> table and column, every numbered migration, every gate you must run before pushing, and every
> error string a user can see — five notes written to be searched rather than read, and the only
> notes in this vault whose numbers go stale.

---

## Which note answers which question

| You have | You want | Go to |
|---|---|---|
| A command name | Its parameters, return type and role gate | [[Command Reference]] |
| A table name | Its columns, constraints and indexes | [[Database Schema]] |
| A schema change to make | The next number and the exact recipe | [[Migrations]] |
| Code to push | The gates, in order, and what each one misses | [[Build and Test Commands]] |
| An error message on screen | Where it came from and what it means | [[Failure Reference]] |

---

## The notes

| Note | Status | What it actually tells you |
|---|---|---|
| [[Command Reference]] | `shipped` | All **263** commands registered in `src-tauri/src/lib.rs`, grouped by the 42 modules under `src-tauri/src/commands/`, each with its `api.ts` wrapper, its parameters exactly as Rust declares them, its `Ok` type and its role gate. Distribution: **129** any-session · **54** `tech`+ · **52** `supervisor`+ · **26** admin · **2** unauthenticated. Leads with the camelCase/snake_case boundary — top-level parameters are `snake_case` in Rust and **camelCase** in TypeScript, and Tauri converts them; nested struct fields are not converted |
| [[Database Schema]] | `shipped` | The schema at migration head **059**: **61 application tables**, one FTS5 virtual table with five shadow tables and three triggers, roughly a hundred named indexes, one table per section. Also the ground rules that are easy to get wrong — UUIDv4 `TEXT PRIMARY KEY` except six vocabulary tables and three single-row config tables; **two coexisting timestamp formats**; and the fact that **there is no `lab_id` column anywhere**, so "lab-scoped" means `specimens.lab_profile` and inheritance through a specimen |
| [[Migrations]] | `binding` | `src-tauri/src/db/migrations.rs` as a 5,775-line append-only file: `run_all`'s flat gate list, the `schema_version` stamp, and the split between `apply` (atomic) and `apply_untransacted` (not atomic, used by exactly **seven** transaction-hostile legacy migrations). Lists all 59, calls out three worth reading in full, and gives the step-by-step recipe for migration **060** including the tests it must come with |
| [[Build and Test Commands]] | `living` | The five gates in the order to run them, each with what it actually verifies and its measured result at `v0.54.0`. The load-bearing distinction: **gates 3 and 4 are not the same gate** — `cargo test --lib --no-default-features` never compiles `src-tauri/src/commands/`, so a type error across the entire 263-command layer is invisible to it. Also the Linux system packages, what CI runs, and the complete list of manifests a version bump must touch together |
| [[Failure Reference]] | `shipped` | Every error string a user can actually see, grouped by origin — auth, permissions, lab isolation, validation, DB constraints, import/export, federated import, integrity verdicts, backup, local AI — plus the three full-screen or banner failure states that are not ordinary errors. Opens with the fact that governs all of it: **no error code and no i18n layer**, so the `String` a Rust command returns *is* the text on screen |

---

## How to read this domain

Do not read it. **Search it.** These notes are indexed by name — command names, table names,
migration numbers, error substrings — and are meant to be jumped into.

The one exception is [[Build and Test Commands]], which is worth reading end to end **once**, before
your first push. It is short, and the two-gate distinction it documents has cost this project real
time: `v1.53.1` shipped with `master` red for six days because `commands/subcultures.rs` passed an
`i32` where an `i64` was wanted — invisible to every gate a headless sandbox can run.

> [!warning] This is the domain that goes stale
> Four of these five notes carry hard numbers: 263 commands, 61 tables, 59 migrations, test counts,
> file counts. Nothing enforces them. `SKILLS.md` §10 lists the same hazard for the repo's own docs,
> and its own §2 and §3 are currently stale (52 migrations, 642/679/113 tests) — proof that the
> failure mode is real. **Measure, then write; never carry a number forward from a previous
> document.** The `updated:` field in frontmatter is how you tell how much to trust one.

### The binding rules that constrain this domain

> [!danger] Four rules a change here must respect
> 1. **Migrations are append-only and numbered.** Never edit a shipped migration; add the next one.
>    The gate goes at the end of `run_all`, the body immediately after it. [[Migrations]]
> 2. **A new command lands in three places at once** — the `#[tauri::command]` function, the
>    `invoke_handler![]` registry in `src-tauri/src/lib.rs`, and the `api.ts` wrapper. Miss the
>    registry and it fails at runtime with no compile error. [[Command Reference]] · [[The IPC Seam]]
> 3. **Every command returns `Result<T, String>`** and that `String` is user-facing text. Write it
>    for the operator, not for the log. [[Failure Reference]]
> 4. **A version bump touches six manifests together** — `package.json`, `package-lock.json`,
>    `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, and the Android
>    `build.gradle.kts` (where `versionCode` must **increase**, independently of the version name).
>    [[Build and Test Commands]]

### Where this domain hands off

| Question this domain raises | Answered in |
|---|---|
| Why is the schema shaped this way? | [[Data Model]] |
| What do these three role predicates mean? | [[Roles and Permissions]] |
| Why does the seam convert argument names at all? | [[The IPC Seam]] |
| What is `taxon_path` for? | [[Taxonomy Backbone]] |
| Which of these commands is a stub? | [[Shipped vs Dormant]] |
| How do I write a note like these? | [[Vault Conventions]] |

---

**Back to [[Home]]** · Sibling maps: [[MOC - Architecture]] · [[MOC - Core Concepts]] · [[MOC - Workflows]]

#moc #meta #reference
