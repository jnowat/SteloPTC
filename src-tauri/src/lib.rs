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
        .plugin(tauri_plugin_notification::init())
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
            commands::compliance::get_mycoplasma_status,
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
            // Advanced taxonomy navigator (WP-39)
            commands::taxa::get_taxon_column,
            commands::taxa::list_species_for_taxon,
            commands::taxa::search_taxonomy,
            // Provisional taxa & Darwin Core export (WP-49)
            commands::taxa::create_provisional_taxon,
            commands::taxa::list_provisional_taxa,
            commands::taxa::map_provisional_taxon,
            commands::taxa::list_taxon_mappings,
            commands::taxa::export_darwin_core,
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
            // Cryopreservation (WP-32)
            commands::cryo::create_frozen_vial,
            commands::cryo::list_frozen_vials,
            commands::cryo::get_frozen_vial,
            commands::cryo::thaw_vial,
            commands::cryo::discard_frozen_vial,
            commands::cryo::get_vial_summary_by_line,
            // Cell-culture dashboard (WP-34)
            commands::subcultures::get_culture_maintenance_alerts,
            // WP-41: colonization history
            commands::subcultures::get_colonization_history,
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
            // Fruiting records (WP-43)
            commands::fruiting::create_fruiting_record,
            commands::fruiting::list_fruiting_records,
            // Breeding programs (WP-47)
            commands::breeding::create_breeding_program,
            commands::breeding::list_breeding_programs,
            commands::breeding::get_breeding_program,
            commands::breeding::add_breeding_record,
            commands::breeding::list_breeding_records_for_program,
            commands::breeding::list_breeding_records_for_strain,
            commands::breeding::get_generational_summary,
            // Backend configuration (WP-50)
            commands::backend_config::get_backend_config,
            commands::backend_config::set_backend_type,
            commands::backend_config::test_postgres_connection,
            commands::backend_config::bootstrap_postgres_schema,
            // LAN sync foundation (WP-51)
            commands::sync::get_sync_status,
            commands::sync::get_changes_since_cursor,
            commands::sync::apply_incoming_changes,
            commands::sync::list_sync_conflicts,
            commands::sync::resolve_sync_conflict,
            commands::sync::register_sync_peer,
            commands::sync::list_sync_peers,
            // Field-level permissions (WP-55)
            commands::permissions::list_field_permissions,
            commands::permissions::set_field_permission,
            // Environmental sensor integration (WP-54)
            commands::sensors::create_environmental_reading,
            commands::sensors::ingest_sensor_payload,
            commands::sensors::list_environmental_readings,
            commands::sensors::get_environmental_alerts,
            // Notifications (WP-52)
            commands::notifications::get_notification_preferences,
            commands::notifications::set_notification_preference,
            commands::notifications::get_smtp_config,
            commands::notifications::set_smtp_config,
            commands::notifications::send_test_desktop_notification,
            commands::notifications::send_test_email,
            commands::notifications::list_recent_notifications,
            commands::notifications::dispatch_due_notifications_now,
        ])
        .setup(|app| {
            let state = app.state::<AppState>();
            let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;
            db.run_migrations().map_err(|e| format!("Migration error: {}", e))?;
            db.seed_defaults().map_err(|e| format!("Seed error: {}", e))?;
            drop(db);

            // WP-52: background scheduler. Sleeps for the configured interval
            // (default 15 minutes, `notification_check_interval_minutes` in
            // app_settings) before each check, so restarting the app during
            // development never immediately fires a notification burst.
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let interval_minutes: i64 = {
                        let state = app_handle.state::<AppState>();
                        // A panic anywhere else in the app while holding this lock poisons
                        // it permanently. Previously that silently killed this loop forever
                        // (`let Ok(db) = ... else { break }`) — the scheduler would just stop
                        // and nothing would ever indicate why. A poisoned rusqlite Connection
                        // is still structurally valid (the panic that poisoned it happened in
                        // unrelated Rust logic, not mid-write to this struct), so recovering
                        // the guard and continuing is safe; we log so the underlying panic
                        // still gets investigated.
                        let db = match state.db.lock() {
                            Ok(db) => db,
                            Err(poisoned) => {
                                eprintln!(
                                    "Notification scheduler: database mutex was poisoned by a \
                                     panic elsewhere while holding the lock. Recovering and \
                                     continuing the scheduler loop, but the underlying panic \
                                     should be investigated."
                                );
                                poisoned.into_inner()
                            }
                        };
                        db::queries::read_setting(&db.conn, "notification_check_interval_minutes", "15")
                            .parse()
                            .unwrap_or(15)
                    };
                    tokio::time::sleep(std::time::Duration::from_secs((interval_minutes.max(1) as u64) * 60)).await;

                    // `dispatch_due_notifications` already maps a poisoned lock to an `Err`
                    // (see `commands::notifications`) rather than panicking, so this arm alone
                    // is sufficient to keep the loop itself alive on that path too — it just
                    // skips this cycle's dispatch and logs, rather than recovering the guard.
                    let state = app_handle.state::<AppState>();
                    if let Err(e) = commands::notifications::dispatch_due_notifications(&app_handle, &state) {
                        eprintln!("Notification dispatch failed: {}", e);
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
