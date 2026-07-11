pub mod ai;
pub mod anchoring;
pub mod auth;
pub mod cloud;
pub mod compliance_export;
pub mod db;
pub mod models;
pub mod passport;
pub mod plugins;
pub mod reg_submission;
pub mod registry;
pub mod signed_ledger;

#[cfg(feature = "tauri-commands")]
pub mod commands;

use db::Database;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<Database>,
    // WP-63: in-memory materialized dashboard cache (never persisted — see
    // db::dashboard for the TTL/invalidation logic).
    pub dashboard_cache: Mutex<Option<db::dashboard::DashboardCacheEntry>>,
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
        dashboard_cache: Mutex::new(None),
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
            // WP-63: cursor-based per-lineage audit pagination
            commands::audit::list_audit_entries_cursor,
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
            // WP-63: configurable pedigree depth cap
            commands::strains::get_pedigree_max_depth,
            commands::strains::set_pedigree_max_depth,
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
            // Taxon chain re-anchoring (WP-64)
            commands::taxa::reanchor_taxon_chain_dry_run,
            commands::taxa::reanchor_taxon_chain,
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
            // Analytics & reporting dashboards (WP-58)
            commands::analytics::get_specimen_growth_rate,
            commands::analytics::get_subculture_frequency_trend,
            commands::analytics::get_contamination_rate_trend,
            commands::analytics::get_passage_success_rate,
            commands::analytics::get_media_batch_efficiency,
            commands::analytics::get_strain_performance,
            commands::analytics::get_cryo_utilization,
            commands::analytics::get_technician_activity,
            commands::analytics::get_analytics_kpi_summary,
            commands::analytics::get_analytics_panel_config,
            commands::analytics::set_analytics_panel_config,
            // Interactive lab map (WP-57)
            commands::locations::list_locations,
            commands::locations::get_location,
            commands::locations::create_location,
            commands::locations::update_location,
            commands::locations::delete_location,
            commands::locations::set_specimen_location_pin,
            commands::locations::get_location_map_data,
            // Local AI analysis (WP-56, WP-56b)
            commands::ai::get_ai_config,
            commands::ai::set_ai_config,
            commands::ai::get_ai_status,
            commands::ai::summarize_notes,
            commands::ai::suggest_passage_comment,
            commands::ai::analyze_photo_for_contamination,
            commands::ai::list_ai_suggestions,
            commands::ai::approve_ai_suggestion,
            commands::ai::reject_ai_suggestion,
            // Cloud backup & multi-device sync (WP-59)
            commands::cloud_backup::list_backup_targets,
            commands::cloud_backup::create_backup_target,
            commands::cloud_backup::delete_backup_target,
            commands::cloud_backup::cloud_backup,
            commands::cloud_backup::restore_from_cloud,
            commands::cloud_backup::reconcile_cloud_sync,
            // Regulatory compliance export modules (WP-60)
            commands::compliance_export::get_signing_public_key,
            commands::compliance_export::export_fda_part11_bundle,
            commands::compliance_export::export_usda_permit,
            commands::compliance_export::export_cites_dossier,
            // Plugin / extension system (WP-61)
            commands::plugins::list_installed_plugins,
            commands::plugins::validate_plugin_manifest,
            commands::plugins::install_plugin,
            commands::plugins::install_plugin_from_zip,
            commands::plugins::uninstall_plugin,
            // On-chain anchoring — Trust Layer Phase 2 (WP-66)
            commands::anchoring::preview_checkpoint_anchor_payload,
            commands::anchoring::prepare_checkpoint_anchor,
            commands::anchoring::record_checkpoint_anchor,
            commands::anchoring::verify_checkpoint_anchor,
            commands::anchoring::list_checkpoint_anchors,
            // Signed-event ledger — Trust Layer Phase 3 (WP-67)
            commands::signed_events::get_user_signing_public_key,
            commands::signed_events::record_signed_event,
            commands::signed_events::list_signed_events,
            commands::signed_events::verify_signed_event_ledger,
            // Regulatory submission pipeline (WP-68)
            commands::reg_submission::evaluate_submission_readiness,
            commands::reg_submission::create_submission,
            commands::reg_submission::reevaluate_submission,
            commands::reg_submission::generate_submission_package,
            commands::reg_submission::mark_submission_submitted,
            commands::reg_submission::list_submissions,
            commands::reg_submission::run_submission_monitor,
            // Specimen passports — federated inter-lab transfer (WP-70)
            commands::passport::get_lab_identity,
            commands::passport::set_lab_name,
            commands::passport::issue_specimen_passport,
            commands::passport::verify_specimen_passport,
            commands::passport::import_specimen_passport,
            commands::passport::list_specimen_passports,
            commands::passport::get_specimen_passport_json,
            // Shared taxonomy registry — federated reference-data exchange (WP-71)
            commands::registry::export_taxonomy_registry,
            commands::registry::verify_taxonomy_registry,
            commands::registry::preview_taxonomy_registry_import,
            commands::registry::import_taxonomy_registry,
            commands::registry::list_taxonomy_registries,
            commands::registry::get_taxonomy_registry_json,
            commands::registry::list_registry_dispositions,
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

                    // WP-68: on the same tick, re-evaluate regulatory submissions
                    // against current compliance state and auto-generate any that
                    // became ready. Best-effort — a poisoned lock or error here
                    // must never stop the scheduler loop.
                    match state.db.lock() {
                        Ok(db) => match commands::reg_submission::monitor(&db.conn) {
                            Ok(r) if r.auto_generated > 0 => {
                                eprintln!("Submission monitor: auto-generated {} package(s).", r.auto_generated);
                            }
                            Ok(_) => {}
                            Err(e) => eprintln!("Submission monitor failed: {}", e),
                        },
                        Err(_) => eprintln!("Submission monitor: db mutex poisoned; skipping this tick."),
                    };
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
