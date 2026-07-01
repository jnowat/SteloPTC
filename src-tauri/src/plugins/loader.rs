// WP-61: plugin loader — applies a validated manifest's vocabulary seed and
// registers the plugin in `installed_plugins`.
//
// Scope note on the WASM compliance-rule ABI: the ROADMAP calls for a
// sandboxed WASM execution environment for `compliance_rules`. Adding a WASM
// runtime (wasmtime or wasmer) is a substantial new dependency with its own
// sandboxing/fuel-metering/ABI-versioning surface — building that safely
// alongside nine other work packets in one session was judged disproportionate
// scope for this combined effort (the same trade-off already made for WP-50's
// PostgreSQL connector and WP-52's SMTP transport: ship the validated,
// tested foundation now; wire up live execution once the runtime dependency
// is deliberately chosen, not squeezed in). `ComplianceRuleDescriptor`
// records and validates the rule's metadata and module path; no `.wasm`
// module is ever loaded or executed by this packet.
use rusqlite::{params, Connection};

use super::manifest::PluginManifest;
use crate::db::DbResult;

/// Applies every vocabulary_seed row in `manifest`, scoped to
/// `manifest.profile` when set (falling back to the row's own table with no
/// profile scoping only if the manifest declares no new profile — i.e. it's
/// adding vocabulary to an *existing* profile, which must be named
/// explicitly per row in a future extension; for now a profile-less
/// manifest seeds no vocabulary, since there is no profile to scope it to).
/// `INSERT OR IGNORE` makes this idempotent and additive — re-installing, or
/// installing two plugins that happen to seed the same code, never
/// clobbers existing rows or duplicates data. Only tables in
/// `manifest::SEEDABLE_VOCAB_TABLES` are ever touched (enforced by
/// `validate_manifest` already having rejected anything else).
pub fn apply_vocabulary_seed(conn: &Connection, manifest: &PluginManifest) -> DbResult<usize> {
    let Some(profile) = &manifest.profile else {
        return Ok(0);
    };
    let mut applied = 0;
    for row in &manifest.vocabulary_seed {
        // Defense-in-depth: `row.table` is interpolated into the SQL string
        // below, so it MUST be a whitelisted vocabulary table name. Callers
        // are expected to have run `validate_manifest` first (which rejects
        // any other table), but this function is `pub` — re-checking here
        // guarantees a non-whitelisted table can never reach the interpolation
        // even if some future caller skips validation. This closes the only
        // path by which a table name could be attacker-influenced.
        if !super::manifest::SEEDABLE_VOCAB_TABLES.contains(&row.table.as_str()) {
            return Err(crate::db::DbError::Constraint(format!(
                "Refusing to seed unknown vocabulary table '{}'",
                row.table
            )));
        }
        let sql = format!(
            "INSERT OR IGNORE INTO {} (profile, code, label, sort_order, is_terminal) VALUES (?1, ?2, ?3, ?4, ?5)",
            row.table
        );
        // Tables other than `stages` don't have an `is_terminal` column;
        // build the statement without it for those instead of erroring.
        let result = if row.table == "stages" {
            conn.execute(&sql, params![profile, row.code, row.label, row.sort_order, row.is_terminal as i64])
        } else {
            conn.execute(
                &format!(
                    "INSERT OR IGNORE INTO {} (profile, code, label, sort_order) VALUES (?1, ?2, ?3, ?4)",
                    row.table
                ),
                params![profile, row.code, row.label, row.sort_order],
            )
        };
        applied += result?;
    }
    Ok(applied)
}

/// Registers a validated plugin in `installed_plugins`. Idempotent by
/// `plugin_name` (the table's `UNIQUE` constraint) — installing the same
/// plugin twice updates its stored manifest/version rather than erroring.
pub fn register_installed_plugin(conn: &Connection, manifest: &PluginManifest, manifest_json: &str) -> DbResult<String> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO installed_plugins (id, plugin_name, version, profile, manifest_json, vocabulary_seeded) \
         VALUES (?1, ?2, ?3, ?4, ?5, 1) \
         ON CONFLICT(plugin_name) DO UPDATE SET version = excluded.version, manifest_json = excluded.manifest_json, \
             profile = excluded.profile, vocabulary_seeded = 1",
        params![id, manifest.name, manifest.version, manifest.profile, manifest_json],
    )?;
    Ok(conn.query_row("SELECT id FROM installed_plugins WHERE plugin_name = ?1", [&manifest.name], |r| r.get(0))?)
}

/// Uninstalls a plugin: removes its `installed_plugins` row (which drops its
/// dashboard panel / report template registrations, since those live only in
/// the manifest JSON on that row) but never touches vocabulary rows already
/// seeded — vocabulary is additive and data-preserving by design, per
/// ROADMAP.md WP-61.
pub fn uninstall_plugin(conn: &Connection, plugin_id: &str) -> DbResult<()> {
    conn.execute("DELETE FROM installed_plugins WHERE id = ?1", [plugin_id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_all;
    use super::super::manifest::validate_manifest;

    fn plugin_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_all(&conn).unwrap();
        conn
    }

    fn algae_manifest() -> (PluginManifest, String) {
        let json = serde_json::json!({
            "name": "Algae Culture",
            "version": "1.0.0",
            "profile": "algae_culture",
            "vocabulary_seed": [
                { "table": "stages", "code": "inoculum", "label": "Inoculum", "sort_order": 1 },
                { "table": "stages", "code": "bloom", "label": "Bloom", "sort_order": 2, "is_terminal": true }
            ]
        })
        .to_string();
        (validate_manifest(&json).unwrap(), json)
    }

    #[test]
    fn apply_vocabulary_seed_inserts_rows_scoped_to_the_new_profile() {
        let conn = plugin_test_db();
        let (manifest, _json) = algae_manifest();
        let applied = apply_vocabulary_seed(&conn, &manifest).unwrap();
        assert_eq!(applied, 2);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM stages WHERE profile = 'algae_culture'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn apply_vocabulary_seed_rejects_a_non_whitelisted_table_even_without_validate_manifest() {
        // Defense-in-depth: construct a manifest DIRECTLY (bypassing
        // `validate_manifest`, which would normally reject this) with a table
        // name that is not in SEEDABLE_VOCAB_TABLES, and confirm
        // `apply_vocabulary_seed` refuses it rather than interpolating the
        // name into a SQL string. This protects the one path by which a table
        // identifier could be attacker-influenced.
        let conn = plugin_test_db();
        let malicious = super::PluginManifest {
            name: "Evil".to_string(),
            version: "1.0.0".to_string(),
            profile: Some("evil_profile".to_string()),
            vocabulary_seed: vec![super::super::manifest::VocabSeedRow {
                table: "users".to_string(), // not a whitelisted vocabulary table
                code: "x".to_string(),
                label: "X".to_string(),
                sort_order: 0,
                is_terminal: false,
            }],
            dashboard_panels: vec![],
            compliance_rules: vec![],
            report_templates: vec![],
        };
        let result = apply_vocabulary_seed(&conn, &malicious);
        assert!(result.is_err(), "seeding a non-whitelisted table must be refused");
        let msg = format!("{:?}", result.unwrap_err());
        assert!(msg.contains("users"), "error should name the rejected table");
    }

    #[test]
    fn vocabulary_seed_is_isolated_from_existing_profiles() {
        let conn = plugin_test_db();
        crate::db::migrations::seed_defaults(&conn).unwrap();
        let ptc_stage_count_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM stages WHERE profile = 'plant_tissue_culture'", [], |r| r.get(0))
            .unwrap();

        let (manifest, _) = algae_manifest();
        apply_vocabulary_seed(&conn, &manifest).unwrap();

        let ptc_stage_count_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM stages WHERE profile = 'plant_tissue_culture'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ptc_stage_count_before, ptc_stage_count_after, "seeding a new profile must not touch existing profiles' vocabulary");
    }

    #[test]
    fn seeding_twice_is_idempotent() {
        let conn = plugin_test_db();
        let (manifest, _) = algae_manifest();
        apply_vocabulary_seed(&conn, &manifest).unwrap();
        apply_vocabulary_seed(&conn, &manifest).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM stages WHERE profile = 'algae_culture'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2, "re-applying the same seed must not duplicate rows");
    }

    #[test]
    fn uninstall_removes_plugin_row_but_leaves_vocabulary_intact() {
        let conn = plugin_test_db();
        let (manifest, json) = algae_manifest();
        apply_vocabulary_seed(&conn, &manifest).unwrap();
        let id = register_installed_plugin(&conn, &manifest, &json).unwrap();

        uninstall_plugin(&conn, &id).unwrap();

        let plugin_count: i64 = conn.query_row("SELECT COUNT(*) FROM installed_plugins", [], |r| r.get(0)).unwrap();
        assert_eq!(plugin_count, 0);
        let vocab_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM stages WHERE profile = 'algae_culture'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(vocab_count, 2, "uninstalling must not roll back seeded vocabulary");
    }
}
