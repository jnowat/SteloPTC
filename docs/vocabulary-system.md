# Vocabulary System (WP-23 / WP-24)

Starting with schema version 16–17, all controlled vocabularies that were
previously enforced as `CHECK` constraints are stored in lookup tables.
Adding or renaming a vocabulary value now requires only a data insert — no
schema migration, no Rust recompile.

## Lookup tables

| Table | Governs |
|---|---|
| `stages` | `specimens.stage`; includes `is_terminal` flag |
| `propagation_methods` | `specimens.propagation_method` |
| `hormone_types` | `media_hormones.hormone_type` |
| `compliance_record_types` | `compliance_records.record_type` |
| `compliance_agencies` | `compliance_records.agency` |
| `inventory_categories` | `inventory_items.category` |

Every table has the shape:

```sql
(id, profile TEXT, code TEXT, label TEXT, sort_order INTEGER,
 UNIQUE(profile, code))
```

`stages` also has `is_terminal INTEGER` (0/1). Stages with `is_terminal = 1`
(currently only `archived`) must not be offered as choices in the New Specimen
or bulk-update stage dropdowns.

## Profile scoping

`app_config.lab_profile` determines which rows are active. All vocabulary
queries filter by profile. The helper `crate::db::vocabulary::active_profile`
reads this value and falls back to `'plant_tissue_culture'` if the row is
missing.

## Adding a new vocabulary value

```sql
INSERT INTO stages (profile, code, label, sort_order, is_terminal)
VALUES ('plant_tissue_culture', 'microshoot', 'Microshoot', 8, 0);
```

The frontend dropdown picks up the new entry on its next load. No code change
is needed.

## Rust helpers (`src/db/vocabulary.rs`)

- `active_profile(conn)` — returns the lab profile string
- `stage_is_selectable(conn, profile, code)` — returns `true` only when the
  code exists in `stages` for the given profile and `is_terminal = 0`

`bulk_update_stage` in `specimens.rs` uses `stage_is_selectable` to validate
the target stage against the database instead of maintaining a hardcoded list.
