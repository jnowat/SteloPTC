pub mod auth;
pub mod commands;
pub mod db;
pub mod models;

use db::Database;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<Database>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = match Database::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to initialize database: {}", e);
            // Fall back to in-memory database so the app can at least start
            Database::new_in_memory()
                .expect("Failed to create even an in-memory database")
        }
    };

    let state = AppState {
        db: Mutex::new(db),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            // Auth
            commands::auth::login,
            commands::auth::get_current_user,
            commands::auth::list_users,
            commands::auth::create_user,
            commands::auth::update_user_role,
            commands::auth::logout,
            // Specimens
            commands::specimens::list_specimens,
            commands::specimens::get_specimen,
            commands::specimens::create_specimen,
            commands::specimens::update_specimen,
            commands::specimens::delete_specimen,
            commands::specimens::search_specimens,
            commands::specimens::get_specimen_stats,
            // Media
            commands::media::list_media,
            commands::media::get_media_batch,
            commands::media::create_media_batch,
            commands::media::update_media_batch,
            commands::media::delete_media_batch,
            // Subcultures
            commands::subcultures::list_subcultures,
            commands::subcultures::create_subculture,
            commands::subcultures::update_subculture,
            // Reminders
            commands::reminders::list_reminders,
            commands::reminders::create_reminder,
            commands::reminders::update_reminder,
            commands::reminders::dismiss_reminder,
            commands::reminders::get_active_reminders,
            // Compliance
            commands::compliance::list_compliance_records,
            commands::compliance::create_compliance_record,
            commands::compliance::update_compliance_record,
            commands::compliance::get_compliance_flags,
            // Species
            commands::species::list_species,
            commands::species::create_species,
            commands::species::update_species,
            // Audit
            commands::audit::get_audit_log,
            // Export/Import
            commands::export::export_specimens_csv,
            commands::export::export_specimens_json,
            // Inventory
            commands::inventory::list_inventory,
            commands::inventory::create_inventory_item,
            commands::inventory::update_inventory_item,
            commands::inventory::delete_inventory_item,
            commands::inventory::adjust_stock,
            commands::inventory::get_low_stock_alerts,
            // Prepared Solutions
            commands::inventory::list_prepared_solutions,
            commands::inventory::create_prepared_solution,
            commands::inventory::update_prepared_solution,
            commands::inventory::delete_prepared_solution,
            // Backup
            commands::backup::create_backup,
            commands::backup::list_backups,
            // Admin / Dev tools
            commands::admin::reset_database,
        ])
        .setup(|app| {
            let state = app.state::<AppState>();
            let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;
            db.run_migrations().map_err(|e| format!("Migration error: {}", e))?;
            db.seed_defaults().map_err(|e| format!("Seed error: {}", e))?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
