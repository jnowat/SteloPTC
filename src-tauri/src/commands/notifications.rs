//! WP-52 — Email/desktop notification command surface + dispatch logic.
//!
//! Recipients are every active admin/supervisor (Work Queue items have no
//! per-specimen "assigned technician" field to target more narrowly — see
//! ROADMAP.md for this disclosed scope boundary). Each dispatch cycle sends
//! at most one desktop popup and one digest email per recipient, summarizing
//! all due items, rather than one notification per item — real per-item
//! paging would be noisy at any nontrivial Work Queue size.

use crate::auth as auth_service;
use crate::db::notifications as notif_queries;
use crate::models::notifications::{
    DispatchNotificationsResult, NotificationPreference, SetNotificationPreferenceRequest,
    SetSmtpConfigRequest, SmtpConfig,
};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn get_notification_preferences(
    state: State<AppState>,
    token: String,
) -> Result<Vec<NotificationPreference>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    notif_queries::list_notification_preferences(&db.conn, &user.id).map_err(|e| e.to_string())
}

/// Every user configures their own preferences — this is not admin-gated.
#[tauri::command]
pub fn set_notification_preference(
    state: State<AppState>,
    token: String,
    request: SetNotificationPreferenceRequest,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    notif_queries::set_notification_preference(
        &db.conn,
        &user.id,
        &request.channel,
        request.enabled,
        &request.min_severity,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_smtp_config(state: State<AppState>, token: String) -> Result<SmtpConfig, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can view the SMTP configuration".to_string());
    }
    notif_queries::get_smtp_config_display(&db.conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_smtp_config(
    state: State<AppState>,
    token: String,
    request: SetSmtpConfigRequest,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can change the SMTP configuration".to_string());
    }
    notif_queries::set_smtp_config(&db.conn, &request).map_err(|e| e.to_string())?;
    crate::db::queries::log_audit(
        &db.conn, Some(&user.id), "update", "smtp_config", None, None, None,
        Some("SMTP configuration updated"),
    ).ok();
    Ok(())
}

/// Any user: sends a desktop popup to verify the notification plugin works
/// on this platform/build.
#[tauri::command]
pub fn send_test_desktop_notification(
    app: tauri::AppHandle,
    state: State<AppState>,
    token: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    drop(db);

    use tauri_plugin_notification::NotificationExt;
    app.notification()
        .builder()
        .title("SteloPTC")
        .body("This is a test notification. If you can see this, desktop notifications are working.")
        .show()
        .map_err(|e| format!("Failed to show desktop notification: {}", e))
}

/// Admin-only: sends a real test email using the currently-saved SMTP config.
#[tauri::command]
pub fn send_test_email(
    state: State<AppState>,
    token: String,
    to_address: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.is_admin() {
        return Err("Only admins can send a test email".to_string());
    }
    notif_queries::send_email(
        &db.conn,
        &to_address,
        "[SteloPTC] Test email",
        "This is a test email from SteloPTC. If you received this, your SMTP configuration is working.",
    )
}

/// Supervisor+: lists recent dispatched notifications from the audit trail.
#[tauri::command]
pub fn list_recent_notifications(
    state: State<AppState>,
    token: String,
    limit: Option<i64>,
) -> Result<Vec<crate::models::audit::AuditEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_manage() {
        return Err("Insufficient permissions".to_string());
    }
    let limit = limit.unwrap_or(50).clamp(1, 500);
    let mut stmt = db
        .conn
        .prepare(
            "SELECT a.*, u.username FROM audit_log a LEFT JOIN users u ON a.user_id = u.id \
             WHERE a.entity_type = 'notification' ORDER BY a.created_at DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let entries = stmt
        .query_map(rusqlite::params![limit], |row| {
            Ok(crate::models::audit::AuditEntry {
                id: row.get("id")?,
                user_id: row.get("user_id")?,
                username: row.get("username")?,
                action: row.get("action")?,
                entity_type: row.get("entity_type")?,
                entity_id: row.get("entity_id")?,
                old_value: row.get("old_value")?,
                new_value: row.get("new_value")?,
                details: row.get("details")?,
                created_at: row.get("created_at")?,
                lineage_id: row.get("lineage_id")?,
                chain_seq: row.get("chain_seq")?,
                prev_hash: row.get("prev_hash")?,
                entry_hash: row.get("entry_hash")?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(entries)
}

/// Admin/supervisor: manually triggers one dispatch cycle immediately,
/// without waiting for the background scheduler's interval.
#[tauri::command]
pub fn dispatch_due_notifications_now(
    app: tauri::AppHandle,
    state: State<AppState>,
    token: String,
) -> Result<DispatchNotificationsResult, String> {
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let user = auth_service::validate_session(&db, &token)?;
        if !user.role.can_manage() {
            return Err("Insufficient permissions".to_string());
        }
    }
    dispatch_due_notifications(&app, &state)
}

/// Core dispatch logic, called both by the manual command above and by the
/// background scheduler loop in `lib.rs::run`.
pub fn dispatch_due_notifications(
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<DispatchNotificationsResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let candidates = notif_queries::compute_due_notifications(&db.conn)?;

    let mut result = DispatchNotificationsResult {
        candidates_found: candidates.len(),
        desktop_sent: 0,
        email_sent: 0,
        recipients_notified: 0,
    };
    if candidates.is_empty() {
        return Ok(result);
    }

    let highest_severity = candidates
        .iter()
        .map(|c| c.severity.as_str())
        .max_by_key(|s| match *s {
            "critical" => 3,
            "high" => 2,
            _ => 1,
        })
        .unwrap_or("normal");

    let mut stmt = db
        .conn
        .prepare("SELECT id, email FROM users WHERE role IN ('admin','supervisor') AND is_active = 1")
        .map_err(|e| e.to_string())?;
    let recipients: Vec<(String, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    let subject = format!(
        "[SteloPTC] {} item{} need attention",
        candidates.len(),
        if candidates.len() == 1 { "" } else { "s" }
    );
    let digest_body = candidates.iter().map(|c| format!("- {}", c.body)).collect::<Vec<_>>().join("\n");

    for (user_id, email) in &recipients {
        let mut notified_this_user = false;

        let (desktop_enabled, desktop_min) = notif_queries::effective_preference(&db.conn, user_id, "desktop");
        if desktop_enabled && notif_queries::severity_meets_threshold(highest_severity, &desktop_min) {
            use tauri_plugin_notification::NotificationExt;
            if app.notification().builder().title(&subject).body(&digest_body).show().is_ok() {
                result.desktop_sent += 1;
                notified_this_user = true;
                for c in &candidates {
                    notif_queries::record_notification_audit(&db.conn, Some(user_id), c, "desktop").ok();
                }
            }
        }

        let (email_enabled, email_min) = notif_queries::effective_preference(&db.conn, user_id, "email");
        if email_enabled && notif_queries::severity_meets_threshold(highest_severity, &email_min) {
            if let Some(addr) = email {
                if notif_queries::send_email(&db.conn, addr, &subject, &digest_body).is_ok() {
                    result.email_sent += 1;
                    notified_this_user = true;
                    for c in &candidates {
                        notif_queries::record_notification_audit(&db.conn, Some(user_id), c, "email").ok();
                    }
                }
            }
        }

        if notified_this_user {
            result.recipients_notified += 1;
        }
    }

    Ok(result)
}
