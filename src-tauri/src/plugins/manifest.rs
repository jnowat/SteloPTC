// WP-61: plugin manifest format + validation. A manifest is plain JSON (the
// ROADMAP also mentions TOML; JSON was chosen since serde_json is already a
// dependency and every other config surface in this codebase — app_settings
// values, field_permissions seeds, audit `details` payloads — is JSON, so a
// plugin author's manifest looks like everything else in the system).
use serde::{Deserialize, Serialize};

/// Vocabulary tables a plugin is allowed to seed. Deliberately a closed
/// whitelist (not "any table name from the manifest") so a malicious or
/// buggy manifest can never target an arbitrary table via string
/// interpolation — see `loader::apply_vocabulary_seed`.
pub const SEEDABLE_VOCAB_TABLES: &[&str] =
    &["stages", "propagation_methods", "hormone_types", "compliance_record_types", "compliance_agencies", "inventory_categories"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabSeedRow {
    pub table: String,
    pub code: String,
    pub label: String,
    #[serde(default)]
    pub sort_order: i64,
    /// Only meaningful for the `stages` table; ignored elsewhere.
    #[serde(default)]
    pub is_terminal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPanelDescriptor {
    pub id: String,
    pub title: String,
    /// Name of a frontend component this panel renders as. The loader does
    /// not dynamically load arbitrary Svelte code (that would require a
    /// runtime bundler); the frontend's plugin panel registry maps this name
    /// to a small fixed set of built-in generic panel renderers (e.g. a
    /// table-of-vocabulary-rows view, a KPI-number view). Fully custom panel
    /// UI is a documented follow-on — see ROADMAP.md WP-61 "As built".
    pub component: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRuleDescriptor {
    pub id: String,
    pub description: String,
    /// Path (relative to the plugin's install directory) to a compiled WASM
    /// module implementing the rule. Recorded and validated at install time;
    /// **not executed** — see the module doc comment in `loader.rs` for why
    /// sandboxed execution is out of scope for this packet.
    pub wasm_module: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTemplateDescriptor {
    pub id: String,
    pub title: String,
    /// Path (relative to the plugin's install directory) to a Handlebars
    /// HTML template used for print/PDF output.
    pub template_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    /// New profile identifier this plugin introduces, if any (e.g.
    /// `"algae_culture"`). `None` for a plugin that only adds panels/reports
    /// to an existing profile.
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub vocabulary_seed: Vec<VocabSeedRow>,
    #[serde(default)]
    pub dashboard_panels: Vec<DashboardPanelDescriptor>,
    #[serde(default)]
    pub compliance_rules: Vec<ComplianceRuleDescriptor>,
    #[serde(default)]
    pub report_templates: Vec<ReportTemplateDescriptor>,
}

/// Validates a manifest's structural and semantic requirements: non-empty
/// name/version, every `vocabulary_seed` row targets a whitelisted table,
/// and no duplicate `(table, code)` pairs within the manifest itself (a
/// plugin author's own typo, not a cross-plugin isolation concern — that's
/// handled separately by `INSERT OR IGNORE` at seed time).
pub fn validate_manifest(json: &str) -> Result<PluginManifest, String> {
    let manifest: PluginManifest = serde_json::from_str(json).map_err(|e| format!("Invalid manifest JSON: {}", e))?;

    if manifest.name.trim().is_empty() {
        return Err("Manifest 'name' is required".to_string());
    }
    if manifest.version.trim().is_empty() {
        return Err("Manifest 'version' is required".to_string());
    }

    let mut seen = std::collections::HashSet::new();
    for row in &manifest.vocabulary_seed {
        if !SEEDABLE_VOCAB_TABLES.contains(&row.table.as_str()) {
            return Err(format!(
                "Manifest references unknown vocabulary table '{}'. Allowed: {}",
                row.table,
                SEEDABLE_VOCAB_TABLES.join(", ")
            ));
        }
        if row.code.trim().is_empty() || row.label.trim().is_empty() {
            return Err(format!("Vocabulary seed row for table '{}' is missing code/label", row.table));
        }
        if !seen.insert((row.table.clone(), row.code.clone())) {
            return Err(format!("Duplicate vocabulary seed row: {}.{}", row.table, row.code));
        }
    }

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest_json() -> String {
        serde_json::json!({
            "name": "Algae Culture",
            "version": "1.0.0",
            "profile": "algae_culture",
            "vocabulary_seed": [
                { "table": "stages", "code": "inoculum", "label": "Inoculum", "sort_order": 1 },
                { "table": "stages", "code": "bloom", "label": "Bloom", "sort_order": 2, "is_terminal": true }
            ],
            "dashboard_panels": [{ "id": "algae_density", "title": "Culture Density", "component": "generic_kpi" }],
            "compliance_rules": [],
            "report_templates": []
        })
        .to_string()
    }

    #[test]
    fn valid_manifest_parses_successfully() {
        let manifest = validate_manifest(&sample_manifest_json()).unwrap();
        assert_eq!(manifest.name, "Algae Culture");
        assert_eq!(manifest.vocabulary_seed.len(), 2);
    }

    #[test]
    fn manifest_missing_name_is_rejected() {
        let json = serde_json::json!({ "name": "", "version": "1.0.0" }).to_string();
        assert!(validate_manifest(&json).is_err());
    }

    #[test]
    fn manifest_referencing_unknown_table_is_rejected() {
        let json = serde_json::json!({
            "name": "Bad Plugin", "version": "1.0.0",
            "vocabulary_seed": [{ "table": "users", "code": "x", "label": "X" }]
        })
        .to_string();
        let err = validate_manifest(&json).unwrap_err();
        assert!(err.contains("unknown vocabulary table"));
    }

    #[test]
    fn manifest_with_duplicate_seed_rows_is_rejected() {
        let json = serde_json::json!({
            "name": "Dup Plugin", "version": "1.0.0",
            "vocabulary_seed": [
                { "table": "stages", "code": "x", "label": "X" },
                { "table": "stages", "code": "x", "label": "X duplicate" }
            ]
        })
        .to_string();
        assert!(validate_manifest(&json).is_err());
    }

    #[test]
    fn malformed_json_is_rejected() {
        assert!(validate_manifest("not json").is_err());
    }
}
