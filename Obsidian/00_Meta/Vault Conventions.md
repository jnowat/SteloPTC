---
title: Vault Conventions
aliases:
  - Authoring Contract
  - Style Guide
  - How to write a note here
tags:
  - meta
  - vault
  - conventions
  - honesty
type: meta
status: binding
version: 0.54.0
created: 2026-07-29
updated: 2026-07-29
cssclasses:
  - wide-tables
---

> [!abstract] In one sentence
> The binding authoring contract for this vault: every note carries the same frontmatter block,
> opens with an `[!abstract]` callout, closes with `**Back to [[Home]]**`, wikilinks only to notes
> that exist, backticks everything outside the vault — and never describes a dormant capability as
> if it were live.

This vault is modelled on the companion vault at
https://github.com/jnowat/gruper/tree/main/Obsidian. Its purpose is specific: **an LLM or a human
should be able to understand how the SteloPTC backend and frontend work by reading it**, without
opening `src-tauri/` or `src/` first. Every rule below serves that purpose. Rules that only serve
tidiness are not here.

---

## The source of truth is the repository

> [!danger] The code outranks every document, including this one
> This vault lives inside the repository it documents (`Obsidian/` at the repo root). It is a
> *derived* artefact. When a note and the code disagree, the code is right and the note is a bug.
> When a note and another repo document (`README.md`, `ROADMAP.md`, `UserManual.md`, `docs/*.md`)
> disagree, **say so in the note, name the file, and believe the code** — [[Trust Layer]] and
> [[Shipped vs Dormant]] both do this in several places.
>
> Never invent a file path, a command name, a column name, a line number, or a count. If you did
> not read it, do not write it. A plausible number is worse than no number, because the next reader
> cannot tell the difference.

The repo's own honesty norms are inherited wholesale. `SKILLS.md` §10 lists the documents that
carry hard numbers and drift silently; `CHANGELOG.md` is append-only and its shipped entries are
never corrected in place. This vault takes the opposite stance to the changelog: **it describes the
present**, is rewritten freely, and carries `updated:` in frontmatter so staleness is visible.

---

## Frontmatter — required on every note

```yaml
---
title: Note Title
aliases: []        # where useful — the names someone would actually search for
tags: []           # one domain tag first, then topic tags — see [[Tag Index]]
type: architecture | concept | workflow | reference | moc | meta
status: shipped | pre-v1 | binding | living | dormant
version: 0.54.0    # only where a component version is load-bearing
created: 2026-07-29
updated: 2026-07-29
cssclasses: [wide-tables]   # only on notes with wide tables
---
```

| Field | Rule |
|---|---|
| `title` | Matches the filename exactly. Obsidian resolves `[[Wikilinks]]` by filename, so a mismatch silently breaks links. |
| `aliases` | Include the identifier a reader would actually type: `api.ts`, `taxon_path`, `can_write`, `WP-45`. Omit the key rather than write `aliases: []` on a note with no useful alias. |
| `tags` | **First tag is the domain tag** (`architecture`, `trust`, `lab-ops`, `taxonomy`, `security`, `compliance`, `reference`, `moc`, `meta`). The rest are topic tags. Draw only from [[Tag Index]]; adding a tag means adding a row there in the same edit. |
| `type` | The six values above, and only those. `type` is authoritative for what kind of note this is; the tag list is for search. |
| `status` | See the table below. |
| `version` | Present only when the note's content is pinned to a release — e.g. counts measured at `v0.54.0`. A timeless concept note (`Lab Profiles`, `Specimens Strains and Species`) omits it. |
| `created` / `updated` | ISO `YYYY-MM-DD`. Both mandatory. |
| `cssclasses` | `[wide-tables]` on any note with a table wider than about six columns. Nothing else uses it. |

### `status` values

| Value | Means | Used by |
|---|---|---|
| `shipped` | Describes machinery that is live in `v0.54.0` and verified against the code | Most notes |
| `pre-v1` | Shipped, but the note's framing depends on the project being pre-1.0 | [[SteloPTC at a Glance]] |
| `binding` | Contains invariants a change must not break | [[The IPC Seam]], [[Hash-Chained Provenance]], [[Migrations]], this note |
| `living` | Expected to go stale between releases; re-measure before trusting | [[Build and Test Commands]], the four MOCs, [[Home]] |
| `dormant` | The note's subject exists as code but has no live path | Reserved — no note currently carries it; dormant *capabilities* are tabulated in [[Shipped vs Dormant]] instead |

---

## Structure of a note

1. Frontmatter.
2. `> [!abstract] In one sentence` — one sentence, and it must carry a fact, not a restatement of
   the title. Compare: *"Explains the taxonomy backbone"* (useless) against *"`taxa` holds kingdom
   through genus and nothing below; species reach the tree through one JSON column"* (the actual
   opening of [[Taxonomy Backbone]]).
3. Body. `##` for major sections, `###` beneath. No `#` H1 — the title comes from frontmatter.
   [[Home]] is the exception; it is a landing page.
4. A `## Related`, `## See also` or `## Where to look` section on domain notes, pointing at the
   sibling notes and at the code paths.
5. Footer: `**Back to [[Home]]**` followed by a blank line and two to four inline `#tags` that
   echo the frontmatter tags.

```markdown
**Back to [[Home]]**

#architecture #ipc #tauri #api
```

---

## Linking

> [!important] Wikilinks are for the vault. Nothing else.
> `[[Double Brackets]]` resolve inside the vault and **every one must resolve to a note that
> exists**. A wikilink to a note that was never written renders as a dead link in Obsidian and, far
> worse, tells an agent that a document exists which does not.

| Thing | How to write it | Example |
|---|---|---|
| Another note | `[[Wikilink]]`, exact filename | `[[Trust Layer]]` |
| A file in the repo | Backticks, path relative to repo root | `` `src-tauri/src/db/queries.rs` `` |
| A specific line | Backticks, `path:line` | `` `src/commands/sync.rs:90` `` |
| A Rust item | Backticks | `` `validate_session` ``, `` `BackendKind::Postgres` `` |
| A column / table | Backticks | `` `species.taxon_path` `` |
| A version | Backticks | `` `v0.54.0` `` |
| An external URL | Bare or markdown link — **never** a wikilink | https://github.com/jnowat/gruper |

The complete set of link targets is the note inventory below. It is a **closed set**: do not
wikilink a note name that is not on it, and do not add a note without adding it to [[Home]] and to
its domain MOC in the same edit.

---

## The callout palette

Obsidian callouts are the vault's semantic layer — an agent skimming for constraints reads
`[!danger]` blocks and stops. Use them for what they mean, not for emphasis.

| Callout | Means | Typical use here |
|---|---|---|
| `[!abstract]` | The one-sentence summary | Opens every note |
| `[!danger]` | A binding invariant — break this and something is silently wrong | Frozen canonical byte layouts in [[Trust Layer]]; the append-only migration rule in [[Migrations]]; the camelCase/snake_case boundary in [[Command Reference]] |
| `[!warning]` | A caveat or an honest limit | The `## Honest limits` section closing every note in `Workflows/` |
| `[!caution]` | Shipped but dormant — real code, no live path | The dormant block in [[SteloPTC at a Glance]]; rows in [[Shipped vs Dormant]] |
| `[!important]` | A load-bearing distinction two things get confused for | Passage vs. split in [[Specimens Strains and Species]]; "lab-scoped" in [[Database Schema]] |
| `[!tip]` | Operator guidance — what to actually do | Recipes in [[Build and Test Commands]] |
| `[!info]` | Context that is not a rule | Provenance of the numbers in [[Home]] |
| `[!success]` | What genuinely works, offline and tested | The working-set summary in [[SteloPTC at a Glance]] |
| `[!example]` | A worked example with real values | Address grammar in [[Lab Layout Model]] |
| `[!note]` | A side note | Sparingly |

`[!quote]` appears once, on [[Home]]'s North Star. It is not part of the general palette.

---

## Style

- **Tables for structured facts.** Anything with three or more parallel attributes is a table, not
  prose. Add `cssclasses: [wide-tables]` when it gets wide.
- **Mermaid for topology and flow.** `flowchart`, `erDiagram`, `sequenceDiagram`. Quote every node
  label containing punctuation — `A["invoke(cmd, args)"]`, not `A[invoke(cmd, args)]` — because an
  unquoted parenthesis or slash breaks the parser and renders a blank block.
- **Fenced code blocks always carry a language hint**: `rust`, `ts`, `sql`, `bash`, `yaml`, `json`,
  `mermaid`, `svelte`.
- **Escape pipes inside table cells** as `\|` — a bare `|` in a closure like `|e| format!(…)` splits
  the row and eats the rest of the line.
- **ISO dates** everywhere: `2026-07-29`.
- **Backticked versions**: `` `v0.54.0` ``. Historical versions stay as they shipped — releases
  through `v1.53.2` used a 1.x series and are never retro-renumbered (see `CHANGELOG.md`).
- **Density over length.** Every paragraph should carry a fact the reader could not have guessed
  from the heading. No filler, no marketing, no restating the note title, no "in this section we
  will".
- **Name the file.** A claim about behaviour is worth roughly nothing without the path that proves
  it. Prefer `` `src/commands/sync.rs:90` `` to "the sync command".

---

## The honesty rule

> [!danger] Never describe a stub as if it were live
> This is the one rule that would make the vault actively harmful if broken, because its readers
> are agents deciding what to build. The repository ships several capabilities **deliberately
> incomplete** and says so — a PostgreSQL connector behind an off-by-default cargo feature, LAN
> sync with no transport, S3/SFTP backup targets that refuse, WASM plugin rules with no runtime,
> an iOS scaffold never built on a Mac, and on-chain anchoring with no broadcast step.
>
> Three obligations follow:
> 1. **Verify before repeating.** Do not copy a status claim out of `README.md`, `ROADMAP.md` or
>    a previous note. Open the code. [[Shipped vs Dormant]] carries `file:line` evidence for
>    exactly this reason.
> 2. **Say "dormant", and say what dormant means here.** "Cloud backup" is not a yes/no — the
>    crypto is real, `local_nas`/`smb` work, `s3`/`sftp` return a refusal string, and
>    `schedule_cron` is validated on write and read by nothing.
> 3. **Record disagreements rather than resolving them silently.** When `docs/plugin-authoring.md`
>    claims a plugin-declared profile becomes selectable and `src-tauri/src/commands/admin.rs:51`
>    hard-codes three allowed values, the note says both and names the file.

---

## Vault mechanics

| Thing | Detail |
|---|---|
| Location | `Obsidian/` at the repository root, committed alongside the code |
| Entry point | `Obsidian/Home.md` — the directory of every note, grouped by folder, plus the "if you only read three notes" shortlist |
| Vault marker | `Obsidian/.obsidian/` — `app.json` (empty `{}`, so Obsidian's defaults apply) and a `.gitignore` excluding `workspace.json` and `workspace-mobile.json`, which are per-machine UI state |
| Folders | `00_Meta/`, `Architecture/`, `Core_Concepts/`, `Workflows/`, `Reference/` — folder names are structure only; `type` in frontmatter is what a query should filter on |
| Rendering | Plain Markdown with Obsidian callout and wikilink syntax. No plugins, no Dataview, no templater — a note must read correctly as raw Markdown in a diff, because that is how it will usually be reviewed |

### Opening it

Open the `Obsidian/` folder as a vault in Obsidian (*Open folder as vault*). Nothing needs
installing. Reading the files directly in an editor loses callout rendering and mermaid, and
nothing else.

### Adding a note

1. Decide the folder from `type`, and confirm the note does not already exist under another name
   (check the `aliases` on nearby notes — `taxon_path` is an alias of [[Taxonomy Backbone]], not a
   missing note).
2. Create the file. Filename **is** the link target; use spaces, not hyphens, and match `title`.
3. Write the frontmatter block, the `[!abstract]`, the body, the `**Back to [[Home]]**` footer.
4. Verify every wikilink resolves. Obsidian shows unresolved links in a different colour; from a
   shell, the crude check is that each `[[Target]]` has a matching `Target.md`.
5. Add it to **three** places in the same commit: [[Home]]'s folder list, its domain MOC's table,
   and — if it introduces a tag — [[Tag Index]].
6. If the note asserts a count, a line number or a status, re-measure it. Do not carry it forward.

### Editing an existing note

Bump `updated:`. Leave `created:` alone. If the edit changes a status claim, mirror it into
[[Shipped vs Dormant]] — that note is the one an agent reads before proposing work, and a
correction that lands only in a domain note will not be seen in time.

---

## The note inventory

The complete, closed set of link targets. 24 notes.

| Folder | Notes |
|---|---|
| root | [[Home]] |
| `00_Meta/` | [[Vault Conventions]] · [[Tag Index]] · [[Shipped vs Dormant]] · [[MOC - Architecture]] · [[MOC - Core Concepts]] · [[MOC - Workflows]] · [[MOC - Reference]] |
| `Architecture/` | [[SteloPTC at a Glance]] · [[Rust Backend]] · [[Svelte Frontend]] · [[The IPC Seam]] · [[Data Model]] · [[Trust Layer]] |
| `Core_Concepts/` | [[Hash-Chained Provenance]] · [[Lab Profiles]] · [[Taxonomy Backbone]] · [[Specimens Strains and Species]] · [[Roles and Permissions]] · [[Lab Layout Model]] |
| `Workflows/` | [[Daily Bench Work]] · [[Drawing the Lab]] · [[Importing NCBI Taxonomy]] · [[Compliance and Export]] · [[Federated Exchange]] |
| `Reference/` | [[Command Reference]] · [[Database Schema]] · [[Migrations]] · [[Build and Test Commands]] · [[Failure Reference]] |

---

**Back to [[Home]]**

#meta #vault #conventions #honesty
