use crate::db::Database;
use crate::models::user::{User, UserRole};
use rusqlite::params;

/// Simple token-based session management for local desktop app.
/// For a local-only app, we use a lightweight approach: generate a random
/// token on login, store in sessions table, validate on each request.

pub fn authenticate(db: &Database, username: &str, password: &str) -> Result<User, String> {
    let user = db.conn.query_row(
        "SELECT id, username, password_hash, display_name, email, role, is_active, created_at, updated_at
         FROM users WHERE username = ?1 AND is_active = 1",
        params![username],
        |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                password_hash: row.get(2)?,
                display_name: row.get(3)?,
                email: row.get(4)?,
                role: UserRole::from_str(&row.get::<_, String>(5)?),
                is_active: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    ).map_err(|_| "Invalid username or password".to_string())?;

    if !bcrypt::verify(password, &user.password_hash).unwrap_or(false) {
        return Err("Invalid username or password".to_string());
    }

    Ok(user)
}

pub fn create_session(db: &Database, user_id: &str) -> Result<String, String> {
    let token = generate_token();
    let id = uuid::Uuid::new_v4().to_string();
    let expires = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    db.conn.execute(
        "INSERT INTO sessions (id, user_id, token, expires_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, user_id, token, expires],
    ).map_err(|e| format!("Failed to create session: {}", e))?;

    Ok(token)
}

pub fn validate_session(db: &Database, token: &str) -> Result<User, String> {
    let user = db.conn.query_row(
        "SELECT u.id, u.username, u.password_hash, u.display_name, u.email, u.role, u.is_active, u.created_at, u.updated_at
         FROM sessions s JOIN users u ON s.user_id = u.id
         WHERE s.token = ?1 AND s.expires_at > datetime('now') AND u.is_active = 1",
        params![token],
        |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                password_hash: row.get(2)?,
                display_name: row.get(3)?,
                email: row.get(4)?,
                role: UserRole::from_str(&row.get::<_, String>(5)?),
                is_active: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    ).map_err(|_| "Session expired or invalid".to_string())?;

    Ok(user)
}

pub fn invalidate_session(db: &Database, token: &str) -> Result<(), String> {
    db.conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])
        .map_err(|e| format!("Failed to invalidate session: {}", e))?;
    Ok(())
}

fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, &bytes)
}
