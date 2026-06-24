pub mod auth;
pub mod db;
pub mod models;

#[cfg(feature = "tauri-commands")]
pub mod commands;

use db::Database;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<Database>,
}

#[cfg(feature = "tauri-commands")]
use tauri::Manager;

#[cfg(feature = "tauri-commands")]
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
            commands::auth::change_password,
            commands::auth::logout,
            // Specimens
            commands::specimens::list_specimens,
            commands::specimens::get_specimen,
            commands::specimens::create_specimen,
            commands::specimens::update_specimen,
            commands::specimens::delete_specimen,
            commands::specimens::search_specimens,
            commands::specimens::get_specimen_stats,
            commands::specimens::bulk_archive_specimens,
            commands::specimens::bulk_update_location,
            commands::specimens::bulk_update_stage,
            commands::specimens::split_specimen,
            commands::specimens::preview_split_accessions,
            commands::specimens::get_specimen_family,
            // Media
            commands::media::list_media,
            commands::media::get_media_batch,
            commands::media::create_media_batch,
            commands::media::create_draft_media_batch,
            commands::media::update_media_batch,
            commands::media::delete_media_batch,
            // Subcultures
            commands::subcultures::list_subcultures,
            commands::subcultures::list_all_subcultures,
            commands::subcultures::create_subculture,
            commands::subcultures::record_specimen_death,
            commands::subcultures::update_subculture,
            commands::subcultures::get_contamination_stats,
            commands::subcultures::get_subculture_schedule,
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
            commands::species::list_projects,
            // Audit
            commands::audit::get_audit_log,
            commands::audit::verify_audit_entry,
            commands::audit::verify_audit_lineage,
            commands::audit::create_audit_checkpoint,
            commands::audit::verify_against_checkpoint,
            commands::audit::list_audit_checkpoints,
            // WP-21 — proof export, standalone verification, auto-checkpointing
            commands::audit::export_audit_proof,
            commands::audit::verify_exported_proof,
            commands::audit::get_auto_checkpoint_config,
            commands::audit::set_auto_checkpoint_config,
            commands::audit::run_auto_checkpoint,
            // Export/Import
            commands::export::export_specimens_csv,
            commands::export::export_specimens_json,
            commands::import::import_xlsx,
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
            commands::backup::restore_backup,
            // Admin tools
            commands::admin::reset_database,
            commands::admin::load_demo_data,
            commands::admin::get_lab_profile,
            commands::admin::set_lab_profile,
            // Vocabulary lookups (WP-23 / WP-24)
            commands::vocabulary::list_stages,
            commands::vocabulary::list_propagation_methods,
            commands::vocabulary::list_hormone_types,
            commands::vocabulary::list_compliance_record_types,
            commands::vocabulary::list_compliance_agencies,
            commands::vocabulary::list_inventory_categories,
            // Strains (WP-28)
            commands::strains::create_strain,
            commands::strains::get_strain,
            commands::strains::list_strains_by_species,
            commands::strains::update_strain,
            commands::strains::archive_strain,
            commands::strains::update_strain_status,
            commands::strains::create_hybridization_event,
            // Hybridization tools (WP-38)
            commands::strains::suggest_generation_label,
            commands::strains::get_generational_stats,
            // Pedigree (WP-37)
            commands::strains::get_strain_ancestry,
            commands::strains::get_strain_descendants,
            commands::strains::get_strain_specimen_tree,
            commands::strains::export_strain_pedigree,
            // Taxa (WP-35)
            commands::taxa::create_taxon,
            commands::taxa::get_taxon,
            commands::taxa::update_taxon,
            commands::taxa::list_taxa_by_rank,
            commands::taxa::get_taxon_descendants,
            // NCBI Taxonomy (WP-36)
            commands::ncbi::import_ncbi_taxonomy,
            commands::ncbi::resolve_ncbi_conflict,
            commands::ncbi::sync_ncbi_taxon,
            commands::ncbi::list_ncbi_sync_log,
            // Error Logs
            commands::error_logs::log_error,
            commands::error_logs::list_error_logs,
            commands::error_logs::get_unread_error_count,
            commands::error_logs::mark_errors_read,
            commands::error_logs::clear_error_logs,
            // QR Scans
            commands::qr_scans::store_qr_scan,
            commands::qr_scans::list_qr_scans,
            // Attachments
            commands::attachments::list_attachments,
            commands::attachments::upload_attachment,
            commands::attachments::get_attachment_data,
            commands::attachments::delete_attachment,
            // Work Queue
            commands::work_queue::get_work_queue,
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
