# Plugin Authoring Guide (WP-61)

SteloPTC plugins are validated JSON manifests that extend the app **additively**: they can introduce a new lab profile, seed profile-scoped vocabulary, register dashboard panel metadata, and (in a future release) execute compliance rules. Installing a plugin never modifies or removes anything from an existing profile.

## Manifest format

```json
{
  "name": "Algae Culture",
  "version": "1.0.0",
  "profile": "algae_culture",
  "vocabulary_seed": [
    { "table": "stages", "code": "inoculum", "label": "Inoculum", "sort_order": 1 },
    { "table": "stages", "code": "log_phase", "label": "Log Phase", "sort_order": 2 },
    { "table": "stages", "code": "bloom", "label": "Bloom (harvest-ready)", "sort_order": 3 },
    { "table": "stages", "code": "crashed", "label": "Crashed", "sort_order": 4, "is_terminal": true },
    { "table": "propagation_methods", "code": "flask_split", "label": "Flask Split", "sort_order": 1 },
    { "table": "inventory_categories", "code": "algae_media", "label": "Algae Media", "sort_order": 1 }
  ],
  "dashboard_panels": [
    { "id": "algae_density", "title": "Culture Density Trend", "component": "generic_kpi" }
  ],
  "compliance_rules": [],
  "report_templates": []
}
```

### Fields

| Field | Required | Notes |
|---|---|---|
| `name` | yes | Display name; also the unique key in `installed_plugins` (installing the same `name` again updates it rather than duplicating). |
| `version` | yes | Free-text version string, shown in the Plugin Manager. |
| `profile` | no | A new profile identifier. Vocabulary is only seeded when this is set — a plugin with no `profile` seeds nothing (there's no profile to scope the rows to). |
| `vocabulary_seed` | no | Array of `{ table, code, label, sort_order?, is_terminal? }`. `table` **must** be one of the whitelisted tables below — anything else is rejected at validation time, before install. `is_terminal` only applies to `stages`; it's ignored for every other table. |
| `dashboard_panels` | no | `{ id, title, component }`. `component` names one of a small fixed set of generic panel renderers the frontend already knows how to render (e.g. `generic_kpi`) — plugins cannot ship arbitrary Svelte code in this release; see **Limitations** below. |
| `compliance_rules` | no | `{ id, description, wasm_module }`. Recorded as metadata only — see **Limitations**. |
| `report_templates` | no | `{ id, title, template_path }`. Recorded as metadata only; not yet rendered by any print/PDF pipeline. |

### Whitelisted vocabulary tables

`stages`, `propagation_methods`, `hormone_types`, `compliance_record_types`, `compliance_agencies`, `inventory_categories` — the same six profile-scoped lookup tables every built-in profile (`plant_tissue_culture`, `cell_culture`, `mycology`) uses. A manifest referencing any other table name is rejected by `validate_manifest` with a clear error before anything is written.

## Installing

Two ways, both admin-only:

1. **Paste/upload manifest JSON** — call `validate_plugin_manifest(manifestJson)` first to preview (returns the parsed manifest so you can show the operator what's about to be seeded), then `install_plugin(manifestJson)` to actually apply it.
2. **`.steloplugin` zip file** — a zip archive with a top-level `manifest.json`. Call `install_plugin_from_zip(zipBase64)`; the manifest is extracted and validated the same way. Any other files in the zip (e.g. WASM modules, report templates referenced by relative path) are not yet extracted anywhere durable — see **Limitations**.

Installing is idempotent by `INSERT OR IGNORE`: re-installing the same plugin re-applies its vocabulary seed harmlessly (no duplicate rows) and updates its `installed_plugins` row (version, manifest JSON).

## Uninstalling

`uninstall_plugin(pluginId)` removes the plugin's `installed_plugins` row. **Seeded vocabulary is never rolled back** — this is deliberate. Vocabulary rows may already be referenced by real specimen/subculture data by the time someone uninstalls; silently deleting them (or the data pointing at them) would be destructive. If you need to fully remove a vocabulary code, do it by hand through the database, understanding the referential consequences.

## Limitations in this release (v1.40.0)

- **No dynamic Svelte panel loading.** `dashboard_panels[].component` must name one of a small, fixed set of generic renderers the frontend ships with. Fully custom panel UI would require a runtime bundler/loader, which is out of scope for this release.
- **No WASM compliance rule execution.** `compliance_rules` are validated and stored as metadata (id, description, module path) but **no `.wasm` module is ever loaded or executed**. Building a sandboxed WASM runtime (wasmtime/wasmer) with proper fuel-metering and a stable ABI is a substantial project in its own right; it was judged disproportionate scope to add safely alongside nine other work packets in the same session. This is a natural next step once a WASM runtime dependency is deliberately chosen.
- **No report template rendering.** `report_templates` are recorded but not yet wired into any print/PDF pipeline.

None of these limitations affect the vocabulary-seeding mechanism itself, which is fully live, tested, and isolated per profile.

## Worked example: "Algae Culture" profile

1. Author writes the manifest above (`algae-culture.steloplugin.json`), covering the algae life cycle: inoculum → log phase → bloom (harvest-ready) → crashed (terminal).
2. Lab admin opens **Settings → Plugin Manager**, uploads the manifest, reviews the preview (4 stages, 1 propagation method, 1 inventory category, 1 dashboard panel, 0 compliance rules), and clicks **Install**.
3. `algae_culture` now appears as a selectable lab profile (via the existing `set_lab_profile` command, same as switching between `plant_tissue_culture`/`cell_culture`/`mycology`); its stages, propagation method, and inventory category are available immediately.
4. If the admin later uninstalls the plugin, the `algae_culture` profile's seeded vocabulary rows remain in the database (any specimens already created under that profile keep working), but the plugin no longer appears as "installed" and its dashboard panel registration is removed.
