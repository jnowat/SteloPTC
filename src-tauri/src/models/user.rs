use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub display_name: String,
    pub email: Option<String>,
    pub role: UserRole,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Supervisor,
    Tech,
    Guest,
}

impl UserRole {
    pub fn as_str(&self) -> &str {
        match self {
            UserRole::Admin => "admin",
            UserRole::Supervisor => "supervisor",
            UserRole::Tech => "tech",
            UserRole::Guest => "guest",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "admin" => UserRole::Admin,
            "supervisor" => UserRole::Supervisor,
            "tech" => UserRole::Tech,
            "guest" => UserRole::Guest,
            _ => UserRole::Guest,
        }
    }

    pub fn can_write(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Supervisor | UserRole::Tech)
    }

    pub fn can_manage(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Supervisor)
    }

    pub fn is_admin(&self) -> bool {
        matches!(self, UserRole::Admin)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPublic {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub role: String,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub email: Option<String>,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserPublic,
}
