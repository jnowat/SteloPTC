use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorLog {
    pub id: String,
    pub timestamp: String,
    pub title: String,
    pub message: String,
    pub module: Option<String>,
    pub severity: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub form_payload: Option<String>,
    pub stack_trace: Option<String>,
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateErrorLogRequest {
    pub title: String,
    pub message: String,
    pub module: Option<String>,
    pub severity: Option<String>,
    pub form_payload: Option<String>,
    pub stack_trace: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ErrorLogSearchParams {
    pub severity: Option<String>,
    pub module: Option<String>,
    pub unread_only: Option<bool>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}
