use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreference {
    pub id: String,
    pub user_id: String,
    /// "desktop" | "email" | "mobile_push"
    pub channel: String,
    pub enabled: bool,
    /// "normal" | "high" | "critical" — the minimum severity this channel should surface.
    pub min_severity: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetNotificationPreferenceRequest {
    pub channel: String,
    pub enabled: bool,
    pub min_severity: String,
}

/// SMTP configuration as returned to the frontend — the password itself is
/// never serialized back out; `password_set` tells the UI whether one has
/// been configured, without exposing it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub host: Option<String>,
    pub port: i64,
    pub username: Option<String>,
    pub password_set: bool,
    pub from_address: Option<String>,
    pub use_tls: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetSmtpConfigRequest {
    pub host: Option<String>,
    pub port: i64,
    pub username: Option<String>,
    /// `None` leaves the currently-stored password unchanged; `Some("")` clears it.
    pub password: Option<String>,
    pub from_address: Option<String>,
    pub use_tls: bool,
}

/// A single due notification, built entirely from non-sensitive Work Queue
/// fields (accession, reason, urgency) — never from a WP-55-masked field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationCandidate {
    /// "normal" | "high" | "critical"
    pub severity: String,
    pub subject: String,
    pub body: String,
    pub source_reason_code: String,
    pub specimen_accession: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchNotificationsResult {
    pub candidates_found: usize,
    pub desktop_sent: usize,
    pub email_sent: usize,
    pub recipients_notified: usize,
}
