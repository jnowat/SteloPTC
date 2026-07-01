// WP-61: Plugin / extension system — Tauri command layer. Install/uninstall
// is admin-only (a plugin can introduce a whole new lab profile and
// vocabulary, a system-level change); listing is available to any
// authenticated user so the Settings panel can show installed plugins to
// everyone even if only an admin can change them.
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use tauri::State;

use crate::auth as auth_service;
use crate::plugins::{loader, manifest};
use crate::AppState;

#[derive(serde::Serialize)]
pub struct InstalledPlugin {
    pub id: String,
    pub plugin_name: String,
    pub version: String,
    pub profile: Option<String>,
    pub vocabulary_seeded: bool,
    pub installed_at: String,
}

#[tauri::command]
pub fn list_installed_plugins(state: State<AppState>, token: String) -> Result<Vec<InstalledPlugin>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    let mut stmt = db
        .conn
        .prepare("SELECT id, plugin_name, version, profile, vocabulary_seeded, installed_at FROM installed_plugins ORDER BY plugin_name ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(InstalledPlugin {
                id: r.get(0)?,
                plugin_name: r.get(1)?,
                version: r.get(2)?,
                profile: r.get(3)?,
                vocabulary_seeded: r.get::<_, i64>(4)? != 0,
                installed_at: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

/// Validates a manifest without installing anything — used by the Plugin
/// Manager UI to preview a `.steloplugin` file's contents before the admin
/// confirms installation.
#[tauri::command]
pub fn validate_plugin_manifest(state: State<AppState>, token: String, manifest_json: String) -> Result<manifest::PluginManifest, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    manifest::validate_manifest(&manifest_json)
}

fn install_from_manifest(db: &crate::db::Database, user: &crate::models::user::User, manifest_json: &str) -> Result<InstalledPlugin, String> {
    let manifest = manifest::validate_manifest(manifest_json)?;
    loader::apply_vocabulary_seed(&db.conn, &manifest).map_err(|e| e.to_string())?;
    let id = loader::register_installed_plugin(&db.conn, &manifest, manifest_json).map_err(|e| e.to_string())?;

    crate::db::queries::log_audit(
        &db.conn, Some(&user.id), "create", "plugin", Some(&id),
        None, Some(&manifest.name), Some("Plugin installed"),
    ).ok();

    db.conn
        .query_row(
            "SELECT id, plugin_name, version, profile, vocabulary_seeded, installed_at FROM installed_plugins WHERE id = ?1",
            [&id],
            |r| {
                Ok(InstalledPlugin {
                    id: r.get(0)?,
                    plugin_name: r.get(1)?,
                    version: r.get(2)?,
                    profile: r.get(3)?,
                    vocabulary_seeded: r.get::<_, i64>(4)? != 0,
                    installed_at: r.get(5)?,
                })
            },
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn install_plugin(state: State<AppState>, token: String, manifest_json: String) -> Result<InstalledPlugin, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can install plugins".to_string());
    }
    install_from_manifest(&db, &user, &manifest_json)
}

/// Installs from a `.steloplugin` zip archive (base64-encoded, matching the
/// existing attachment-upload calling convention). The archive must contain
/// a top-level `manifest.json`; any WASM modules or report templates it
/// references are extracted alongside it for future use but are not
/// executed — see `plugins::loader`'s module doc comment.
#[tauri::command]
pub fn install_plugin_from_zip(state: State<AppState>, token: String, zip_b64: String) -> Result<InstalledPlugin, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can install plugins".to_string());
    }

    let zip_bytes = B64.decode(&zip_b64).map_err(|e| format!("Invalid .steloplugin file: {}", e))?;
    let reader = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("Not a valid .steloplugin archive: {}", e))?;
    let mut manifest_file = archive
        .by_name("manifest.json")
        .map_err(|_| "'.steloplugin' archive must contain a top-level manifest.json".to_string())?;
    let mut manifest_json = String::new();
    std::io::Read::read_to_string(&mut manifest_file, &mut manifest_json)
        .map_err(|e| format!("Failed to read manifest.json: {}", e))?;
    drop(manifest_file);

    install_from_manifest(&db, &user, &manifest_json)
}

#[tauri::command]
pub fn uninstall_plugin(state: State<AppState>, token: String, plugin_id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can uninstall plugins".to_string());
    }
    let plugin_name: Option<String> = db.conn.query_row("SELECT plugin_name FROM installed_plugins WHERE id = ?1", [&plugin_id], |r| r.get(0)).ok();
    loader::uninstall_plugin(&db.conn, &plugin_id).map_err(|e| e.to_string())?;

    crate::db::queries::log_audit(
        &db.conn, Some(&user.id), "delete", "plugin", Some(&plugin_id),
        None, plugin_name.as_deref(), Some("Plugin uninstalled (seeded vocabulary retained)"),
    ).ok();
    Ok(())
}
