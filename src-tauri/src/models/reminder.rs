use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub specimen_id: Option<String>,
    pub specimen_accession: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub reminder_type: String,
    pub due_date: String,
    pub is_recurring: bool,
    pub recurrence_days: Option<i32>,
    pub recurrence_rule: Option<String>,
    pub status: String,
    pub snooze_count: i32,
    pub urgency: String,
    pub assigned_to: Option<String>,
    pub assigned_name: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateReminderRequest {
    pub specimen_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub reminder_type: String,
    pub due_date: String,
    pub is_recurring: Option<bool>,
    pub recurrence_days: Option<i32>,
    pub recurrence_rule: Option<String>,
    pub urgency: Option<String>,
    pub assigned_to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateReminderRequest {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub due_date: Option<String>,
    pub urgency: Option<String>,
    pub status: Option<String>,
    pub assigned_to: Option<String>,
}
