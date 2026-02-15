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
    let db = Database::new().expect("Failed to initialize database");

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
        ])
        .setup(|app| {
            let state = app.state::<AppState>();
            let db = state.db.lock().unwrap();
            db.run_migrations().expect("Failed to run migrations");
            db.seed_defaults().expect("Failed to seed defaults");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
