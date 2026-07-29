---
title: Roles and Permissions
aliases: [can_write, can_manage, is_admin, UserRole, Field Permissions]
tags: [security, roles, permissions, auth, ipc]
type: concept
status: shipped
version: 0.54.0
created: 2026-07-29
updated: 2026-07-29
cssclasses: [wide-tables]
---

> [!abstract] In one sentence
> Four roles collapse into three predicates — `can_write()`, `can_manage()`, `is_admin()` — checked
> in Rust on every command, mirrored by hand in ~40 separate Svelte expressions, and the gap between
> those two lists is the standing hazard of this codebase.

---

## The four roles and the three predicates

`UserRole` is a plain enum in `src-tauri/src/models/user.rs`, serialised lowercase.

| Predicate | Admin | Supervisor | Tech | Guest |
|---|:---:|:---:|:---:|:---:|
| `can_write()` — record work | ✅ | ✅ | ✅ | ❌ |
| `can_manage()` — curate reference data, delete, checkpoint | ✅ | ✅ | ❌ | ❌ |
| `is_admin()` — change the shape of the installation | ✅ | ❌ | ❌ | ❌ |

Read it as three concentric rings rather than a matrix: `is_admin ⊂ can_manage ⊂ can_write`. Guest
satisfies none of them and can only read.

> [!danger] The predicates are the only authorisation primitive on the backend
> There is no role hierarchy object, no permission string, no policy engine. Every Tauri command
> begins the same way — `auth_service::validate_session(&db, &token)?` — and then, if it writes,
> calls one of these three methods and returns a plain `String` error. That error string **is** the
> UI message; there is no error code and no i18n layer.

### Which predicate guards what

| Predicate | Representative commands |
|---|---|
| `can_write()` | `create_specimen`, `update_specimen`, `create_subculture`, `record_specimen_death`, `bulk_update_location`, `bulk_update_stage`, `split_specimen`, `create_strain`, `update_strain`, `archive_strain`, `update_strain_status`, `create_hybridization_event`, `save_location_layout`, `export_taxonomy_registry`, `import_taxonomy_registry` |
| `can_manage()` | `delete_specimen`, `bulk_archive_specimens`, `create_species`, `update_species`, `rebuild_species_taxonomy`, `create_taxon`, `update_taxon`, `create_provisional_taxon`, `map_provisional_taxon`, `create_audit_checkpoint`, `run_auto_checkpoint`, `reanchor_taxon_chain_dry_run`, `cloud_backup` |
| `is_admin()` | `set_lab_profile`, `reset_database`, `import_ncbi_taxonomy`, `resolve_ncbi_conflict`, `sync_ncbi_taxon`, `set_pedigree_max_depth`, `reanchor_taxon_chain`, `list_field_permissions`, `set_field_permission` |

Two notable asymmetries that are deliberate, not oversights:

- **`create_strain` is `can_write`, but `create_species` is `can_manage`.** A technician may register
  the line they are working with; naming a new organism is curation. See
  [[Specimens Strains and Species]].
- **`reanchor_taxon_chain_dry_run` is `can_manage` while `reanchor_taxon_chain` is `is_admin`.**
  Anyone who can curate may *ask* what a re-anchor would touch; only an admin may do it, and only
  with a reason of at least 20 characters. See [[Hash-Chained Provenance]].

---

## The frontend mirrors this by hand — and that is the hazard

There is **no shared `canWrite` / `canManage` derived pair** in the Svelte layer. Every component
re-derives the predicate inline from the `currentUser` store, and the codebase contains roughly
forty independent copies in at least three different spellings:

```ts
// three ways of saying the same thing, all present today
const canWrite  = $derived(!!$currentUser && $currentUser.role !== 'guest');            // StrainManager
const canManage = $derived($currentUser?.role === 'admin' || $currentUser?.role === 'supervisor');  // SpeciesManager
const canExport = $derived($currentUser?.role === 'admin' || $currentUser?.role === 'supervisor' || $currentUser?.role === 'tech');  // TaxonomyRegistryPanel
```

> [!danger] A frontend gate is a *hint*, never a boundary
> Hiding a button changes nothing about what the IPC layer will accept — the backend check is the
> only real one. But a frontend gate that **disagrees** with its backend gate is a genuine defect in
> both directions:
> - **Too strict** → a role that is authorised sees nothing, cannot do the job, and has no error to
>   report. This is the silent failure and it is much worse, because nobody files a bug about a
>   button they never saw.
> - **Too loose** → a role that is not authorised is offered the action and gets a raw
>   `"Insufficient permissions"` back from the IPC seam, which reads as a broken app.

### The `v0.54.0` cautionary example — both directions, in one feature

Two gates around strain creation were fixed in the same release. `create_strain` requires
`can_write()`, which is **Admin | Supervisor | Tech**.

| Component | Gate before `v0.54.0` | Symptom | Gate now |
|---|---|---|---|
| `SpecimenForm.svelte` "+ New strain" | `role === 'admin' \|\| role === 'supervisor'` | **Too strict.** A tech was shown *"No strains registered for this species yet. Ask a supervisor to add one."* while the backend would have accepted the call. The inline form exists precisely so a technician logging a specimen of a brand-new strain does not have to leave the form — and it was hidden from that exact person | `!!$currentUser && $currentUser.role !== 'guest'` |
| `StrainManager.svelte` "+ New Strain" / "+ New Hybrid Strain" | **no gate at all** | **Too loose.** A guest saw both buttons and got the raw `"Insufficient permissions"` string back from the IPC layer | `!!$currentUser && $currentUser.role !== 'guest'` |

Both fixed sites now carry the reason in a comment above the derived value, naming the backend
predicate they mirror. That convention is the cheapest available mitigation until a shared helper
exists.

> [!warning] The same class of divergence is still live elsewhere
> **`TaxonomyRegistryPanel` is mounted only inside `AuditLog.svelte`.** It has no `View` id, no
> Sidebar entry and no route — and the Audit Log itself is gated `['admin','supervisor']` in the
> sidebar, while `import_taxonomy_registry` and `export_taxonomy_registry` require only
> `can_write()`. A **tech can never reach a feature they are authorised to use**. See
> [[Federated Exchange]] and [[Shipped vs Dormant]].

### View-level gating

`Sidebar.svelte` filters nav entries by role *and* by lab profile. Only three entries carry a role
gate at all:

| View | `roles` |
|---|---|
| NCBI Sync | `['admin']` |
| Users | `['admin']` |
| Settings | `['admin']` |
| Audit Log | `['admin','supervisor']` |

Everything else — Dashboard, Work Queue, Analytics, Lab Map, Specimens, Reminders, Compliance,
Species, Taxonomy, Inventory, Cryostorage, Breeding, Prov. Taxa, Error Log, Export, Import — is
visible to every authenticated role including guest, and the per-action gating happens inside each
view. `Settings.svelte` additionally renders an explicit "admin only" message rather than a blank
screen for non-admins.

---

## Field-level permissions (WP-55)

A second, orthogonal axis: not *may you act*, but *may you see this column*.

`field_permissions(id, role, entity_type, field_name, visible)` is a plain lookup. **Absence of a
row means visible** — a permissive default, so adding a brand-new sensitive field never silently
locks everyone out before an admin has configured it. `migration_036` seeds 4 roles × 3 fields = 12
rows, all `visible = 1`.

| `entity_type` | `field_name` | Masked at |
|---|---|---|
| `strain` | `genomic_fingerprint` | `get_strain`, `list_strains_by_species` |
| `breeding_program` | `goal` | the breeding read path |
| `breeding_program` | `target_traits` | the breeding read path |

That list is the constant `MASKABLE_FIELDS` in `src-tauri/src/db/permissions.rs`, and it is the
single source of truth tying together three things that would otherwise drift: the migration seed,
the mask calls in the command layer, and what the admin editor may configure.

> [!important] Four design decisions worth knowing
> 1. **Masking is applied per call site, not by an interceptor.** Rust has no runtime reflection, and
>    the same convention already governs audit logging — cross-cutting concerns are applied
>    explicitly at each site. The masked surface is intentionally tiny.
> 2. **The placeholder is `"[RESTRICTED]"`, not `null`**, so the frontend can distinguish *no data*
>    from *hidden data*, and masking never has to guess whether an `Option<String>` was already
>    `None` for an unrelated reason. A `None` is never masked — there is nothing to hide.
> 3. **`set_field_permission` refuses a non-maskable field.** Storing a rule the read path never
>    consults would be a silent no-op that looks like protection. `strain.name` is rejected with an
>    error naming the whole allowed set.
> 4. **Masking never reaches the audit log.** `log_audit` always stores the raw value it was given;
>    masking happens only when a read command constructs its response. There is a test asserting
>    exactly this.

A tripwire test, `maskable_fields_registry_matches_migration_seed`, fails the build if the seed and
the constant ever disagree — catching "added a seed row but forgot the call site" and the reverse.

Two performance / correctness details:

- `FieldPermissionSet::load(conn, role)` loads a role's whole rule set once and answers in memory.
  Use it for any list or loop; `mask_optional_field` issues one query per call and is for
  single-record reads only. This was an N+1 fix.
- `reject_if_restricted_marker(value, field_label)` **must** be called on every write that accepts a
  value for a masked field. It is the hard backend guarantee that a masked read cannot be
  round-tripped back into the database as if it were real data, and it holds regardless of what any
  frontend does. See [[Specimens Strains and Species]] for the real bug that motivated it.

`validate_admin_role(role)` — the gate on the permissions editor itself — is kept as a pure function
so the authorisation logic is testable without a Tauri runtime, mirroring
`check_profile_change_allowed`.

---

## What roles do *not* control

> [!warning] Lab isolation is a separate mechanism
> A role says what you may do; `specimens.lab_profile` and `require_active_lab_profile` say which
> lab's data you may touch. An admin of a mycology-configured installation still cannot read a plant
> tissue culture specimen by id without switching profiles. See [[Lab Profiles]].

Field masking also covers only what a *read command returns*. Regulatory export bundles, the audit
log, and the taxonomy registry each have their own rules — the registry exporter, for instance,
never emits `genomic_fingerprint` at all.

---

## Related

[[The IPC Seam]] · [[Trust Layer]] · [[Lab Profiles]] · [[Specimens Strains and Species]] ·
[[Command Reference]] · [[Failure Reference]] · [[Shipped vs Dormant]]

---

**Back to [[Home]]**

#security #permissions #roles
