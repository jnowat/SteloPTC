//! WP-52 — Email/desktop notification foundation.
//!
//! `compute_due_notifications` builds candidates entirely from
//! `db::work_queue::compute_work_queue_items` (accession, reason,
//! reason_code, urgency) — fields that are never subject to WP-55 field
//! masking. This is the enforcement mechanism for "notifications must
//! respect field-level permissions": the notification templates simply never
//! draw from a maskable entity/field in the first place, so there is no
//! masked value that could leak into a notification body.
//!
//! Direct integration with `get_compliance_flags` (permits, HLB, mycoplasma)
//! is deferred — see the WP-52 "As built" note in ROADMAP.md. Work Queue
//! already covers the two most safety-relevant conditions (quarantine,
//! contamination), so this is a bounded, disclosed scope reduction rather
//! than a silent gap.
//!
//! Sending real email requires a configured SMTP server and is exercised via
//! `commands::notifications::send_test_email`; it cannot be unit-tested
//! without a live mail server, matching the same limitation already
//! documented for WP-50's PostgreSQL connector.

use super::DbResult;
use crate::models::notifications::{
    NotificationCandidate, NotificationPreference, SetSmtpConfigRequest, SmtpConfig,
};
use rusqlite::{params, Connection};

fn severity_rank(severity: &str) -> i32 {
    match severity {
        "critical" => 3,
        "high" => 2,
        _ => 1, // "normal" and any unrecognized value
    }
}

/// True when `candidate_severity` meets or exceeds `min_severity`.
pub fn severity_meets_threshold(candidate_severity: &str, min_severity: &str) -> bool {
    severity_rank(candidate_severity) >= severity_rank(min_severity)
}

/// Builds one notification candidate per open Work Queue item. `urgency` on
/// `WorkQueueItem` already uses the same three-value vocabulary
/// ("critical"/"high"/"normal") as notification severity, so no mapping is needed.
pub fn compute_due_notifications(conn: &Connection) -> Result<Vec<NotificationCandidate>, String> {
    let items = crate::db::work_queue::compute_work_queue_items(conn)?;
    Ok(items
        .into_iter()
        .map(|item| NotificationCandidate {
            severity: item.urgency,
            subject: format!(
                "[SteloPTC] {} — {}",
                item.accession_number,
                item.reason_code.replace('_', " ")
            ),
            body: format!("{}: {}", item.accession_number, item.reason),
            source_reason_code: item.reason_code,
            specimen_accession: Some(item.accession_number),
        })
        .collect())
}

/// Returns (enabled, min_severity) for a user's channel, defaulting to
/// enabled + "normal" when no preference row exists yet.
pub fn effective_preference(conn: &Connection, user_id: &str, channel: &str) -> (bool, String) {
    conn.query_row(
        "SELECT enabled, min_severity FROM notification_preferences WHERE user_id = ?1 AND channel = ?2",
        params![user_id, channel],
        |r| Ok((r.get::<_, i64>(0)? != 0, r.get::<_, String>(1)?)),
    )
    .unwrap_or((true, "normal".to_string()))
}

pub fn list_notification_preferences(conn: &Connection, user_id: &str) -> DbResult<Vec<NotificationPreference>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_id, channel, enabled, min_severity FROM notification_preferences WHERE user_id = ?1",
    )?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(NotificationPreference {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            channel: row.get("channel")?,
            enabled: row.get::<_, i64>("enabled")? != 0,
            min_severity: row.get("min_severity")?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn set_notification_preference(
    conn: &Connection,
    user_id: &str,
    channel: &str,
    enabled: bool,
    min_severity: &str,
) -> DbResult<()> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM notification_preferences WHERE user_id = ?1 AND channel = ?2",
            params![user_id, channel],
            |r| r.get(0),
        )
        .ok();
    if let Some(id) = existing {
        conn.execute(
            "UPDATE notification_preferences SET enabled = ?1, min_severity = ?2 WHERE id = ?3",
            params![enabled as i64, min_severity, id],
        )?;
    } else {
        conn.execute(
            "INSERT INTO notification_preferences (id, user_id, channel, enabled, min_severity) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![uuid::Uuid::new_v4().to_string(), user_id, channel, enabled as i64, min_severity],
        )?;
    }
    Ok(())
}

/// Reads the single-row SMTP config for display — never returns the raw password.
pub fn get_smtp_config_display(conn: &Connection) -> DbResult<SmtpConfig> {
    conn.query_row(
        "SELECT host, port, username, password, from_address, use_tls FROM smtp_config WHERE id = 1",
        [],
        |row| {
            let password: Option<String> = row.get("password")?;
            Ok(SmtpConfig {
                host: row.get("host")?,
                port: row.get("port")?,
                username: row.get("username")?,
                password_set: password.is_some_and(|p| !p.is_empty()),
                from_address: row.get("from_address")?,
                use_tls: row.get::<_, i64>("use_tls")? != 0,
            })
        },
    )
    .map_err(super::DbError::from)
}

/// Internal, never-serialized SMTP config including the raw password — used
/// only by `send_email`. Never returned from a Tauri command.
struct SmtpConfigInternal {
    host: Option<String>,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    from_address: Option<String>,
    use_tls: bool,
}

fn get_smtp_config_internal(conn: &Connection) -> DbResult<SmtpConfigInternal> {
    conn.query_row(
        "SELECT host, port, username, password, from_address, use_tls FROM smtp_config WHERE id = 1",
        [],
        |row| {
            Ok(SmtpConfigInternal {
                host: row.get("host")?,
                port: row.get::<_, i64>("port")? as u16,
                username: row.get("username")?,
                password: row.get("password")?,
                from_address: row.get("from_address")?,
                use_tls: row.get::<_, i64>("use_tls")? != 0,
            })
        },
    )
    .map_err(super::DbError::from)
}

/// Sends one email via the configured SMTP server. Requires `host`,
/// `from_address`, `username`, and `password` to all be set — returns a clear
/// error naming the missing field otherwise. This cannot be exercised in a
/// unit test without a live SMTP server; see the module doc comment.
pub fn send_email(conn: &Connection, to: &str, subject: &str, body: &str) -> Result<(), String> {
    let config = get_smtp_config_internal(conn).map_err(|e| e.to_string())?;

    let host = config.host.filter(|h| !h.is_empty()).ok_or("SMTP host is not configured")?;
    let from = config.from_address.filter(|f| !f.is_empty()).ok_or("SMTP from address is not configured")?;
    let username = config.username.filter(|u| !u.is_empty()).ok_or("SMTP username is not configured")?;
    let password = config.password.filter(|p| !p.is_empty()).ok_or("SMTP password is not configured")?;

    let email = lettre::Message::builder()
        .from(from.parse().map_err(|e| format!("Invalid from address: {}", e))?)
        .to(to.parse().map_err(|e| format!("Invalid recipient address: {}", e))?)
        .subject(subject)
        .body(body.to_string())
        .map_err(|e| format!("Failed to build email: {}", e))?;

    let creds = lettre::transport::smtp::authentication::Credentials::new(username, password);

    let mailer_builder = if config.use_tls {
        lettre::SmtpTransport::starttls_relay(&host)
            .map_err(|e| format!("Failed to configure SMTP transport: {}", e))?
    } else {
        lettre::SmtpTransport::builder_dangerous(&host)
    };
    let mailer = mailer_builder.port(config.port).credentials(creds).build();

    lettre::Transport::send(&mailer, &email).map_err(|e| format!("Failed to send email: {}", e))?;
    Ok(())
}

/// Updates the SMTP config. `req.password = None` leaves the stored password
/// untouched (so re-saving host/port doesn't force re-entering the password).
pub fn set_smtp_config(conn: &Connection, req: &SetSmtpConfigRequest) -> DbResult<()> {
    match &req.password {
        Some(password) => {
            conn.execute(
                "UPDATE smtp_config SET host = ?1, port = ?2, username = ?3, password = ?4, \
                 from_address = ?5, use_tls = ?6, updated_at = datetime('now') WHERE id = 1",
                params![req.host, req.port, req.username, password, req.from_address, req.use_tls as i64],
            )?;
        }
        None => {
            conn.execute(
                "UPDATE smtp_config SET host = ?1, port = ?2, username = ?3, \
                 from_address = ?4, use_tls = ?5, updated_at = datetime('now') WHERE id = 1",
                params![req.host, req.port, req.username, req.from_address, req.use_tls as i64],
            )?;
        }
    }
    Ok(())
}

/// Records a dispatched notification in the standard audit trail — every
/// sent notification is auditable the same way every other mutation is,
/// rather than a bespoke notification log table.
pub fn record_notification_audit(
    conn: &Connection,
    user_id: Option<&str>,
    candidate: &NotificationCandidate,
    channel: &str,
) -> DbResult<()> {
    crate::db::queries::log_audit(
        conn,
        user_id,
        "notify",
        "notification",
        None,
        None,
        Some(&candidate.body),
        Some(&format!(
            "channel={} severity={} reason_code={}",
            channel, candidate.severity, candidate.source_reason_code
        )),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_all;

    fn migrated_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        run_all(&conn).expect("all migrations must succeed on a fresh in-memory DB");
        conn
    }

    fn insert_user(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, role) VALUES (?1, ?1, 'x', 'Test', 'tech')",
            params![id],
        )
        .unwrap();
    }

    #[test]
    fn severity_ranking_orders_correctly() {
        assert!(severity_meets_threshold("critical", "normal"));
        assert!(severity_meets_threshold("high", "normal"));
        assert!(severity_meets_threshold("normal", "normal"));
        assert!(!severity_meets_threshold("normal", "high"));
        assert!(!severity_meets_threshold("high", "critical"));
        assert!(severity_meets_threshold("critical", "critical"));
    }

    #[test]
    fn effective_preference_defaults_when_no_row() {
        let conn = migrated_db();
        let (enabled, min_severity) = effective_preference(&conn, "user-1", "desktop");
        assert!(enabled);
        assert_eq!(min_severity, "normal");
    }

    #[test]
    fn set_and_get_notification_preference_round_trip() {
        let conn = migrated_db();
        insert_user(&conn, "user-1");
        set_notification_preference(&conn, "user-1", "email", false, "high").unwrap();
        let (enabled, min_severity) = effective_preference(&conn, "user-1", "email");
        assert!(!enabled);
        assert_eq!(min_severity, "high");
    }

    #[test]
    fn set_notification_preference_upserts_without_duplicating() {
        let conn = migrated_db();
        insert_user(&conn, "user-1");
        set_notification_preference(&conn, "user-1", "desktop", true, "normal").unwrap();
        set_notification_preference(&conn, "user-1", "desktop", false, "critical").unwrap();
        let prefs = list_notification_preferences(&conn, "user-1").unwrap();
        assert_eq!(prefs.len(), 1);
        assert!(!prefs[0].enabled);
        assert_eq!(prefs[0].min_severity, "critical");
    }

    #[test]
    fn compute_due_notifications_reflects_work_queue_state() {
        let conn = migrated_db();
        // Empty lab => no candidates (mirrors compute_work_queue_items behavior).
        let candidates = compute_due_notifications(&conn).unwrap();
        assert!(candidates.is_empty());
    }

    #[test]
    fn smtp_config_display_never_exposes_raw_password() {
        let conn = migrated_db();
        set_smtp_config(
            &conn,
            &SetSmtpConfigRequest {
                host: Some("smtp.example.com".to_string()),
                port: 587,
                username: Some("lab@example.com".to_string()),
                password: Some("super-secret".to_string()),
                from_address: Some("lab@example.com".to_string()),
                use_tls: true,
            },
        )
        .unwrap();

        let display = get_smtp_config_display(&conn).unwrap();
        assert!(display.password_set);
        assert_eq!(display.host.as_deref(), Some("smtp.example.com"));
        // SmtpConfig has no password field at all — this is a compile-time
        // guarantee, not just a runtime check, but we assert the shape here too.
        let serialized = serde_json::to_string(&display).unwrap();
        assert!(!serialized.contains("super-secret"));
    }

    #[test]
    fn set_smtp_config_with_none_password_preserves_existing() {
        let conn = migrated_db();
        set_smtp_config(
            &conn,
            &SetSmtpConfigRequest {
                host: Some("smtp.example.com".to_string()),
                port: 587,
                username: None,
                password: Some("super-secret".to_string()),
                from_address: None,
                use_tls: true,
            },
        )
        .unwrap();
        // Re-save without touching the password.
        set_smtp_config(
            &conn,
            &SetSmtpConfigRequest {
                host: Some("smtp2.example.com".to_string()),
                port: 465,
                username: None,
                password: None,
                from_address: None,
                use_tls: true,
            },
        )
        .unwrap();

        let display = get_smtp_config_display(&conn).unwrap();
        assert!(display.password_set, "password must survive a re-save that doesn't touch it");
        assert_eq!(display.host.as_deref(), Some("smtp2.example.com"));
    }

    #[test]
    fn record_notification_audit_writes_full_body_to_audit_log() {
        let conn = migrated_db();
        insert_user(&conn, "user-1");
        let candidate = NotificationCandidate {
            severity: "high".to_string(),
            subject: "[SteloPTC] ACC-001 — subculture due".to_string(),
            body: "ACC-001: Subculture overdue by 3 days".to_string(),
            source_reason_code: "subculture_due".to_string(),
            specimen_accession: Some("ACC-001".to_string()),
        };
        record_notification_audit(&conn, Some("user-1"), &candidate, "desktop").unwrap();

        let stored: String = conn
            .query_row(
                "SELECT new_value FROM audit_log WHERE entity_type = 'notification'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored, candidate.body);
    }
}
