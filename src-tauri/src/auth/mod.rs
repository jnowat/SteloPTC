use crate::db::Database;
use crate::models::user::{User, UserRole};
use rusqlite::params;

// Simple token-based session management for local desktop app.
// For a local-only app, we use a lightweight approach: generate a random
// token on login, store in sessions table, validate on each request.

pub fn authenticate(db: &Database, username: &str, password: &str) -> Result<User, String> {
    let user = db.conn.query_row(
        "SELECT id, username, password_hash, display_name, email, role, is_active, must_change_password, created_at, updated_at
         FROM users WHERE username = ?1 AND is_active = 1",
        params![username],
        |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                password_hash: row.get(2)?,
                display_name: row.get(3)?,
                email: row.get(4)?,
                role: row.get::<_, String>(5)?.parse().unwrap_or(UserRole::Guest),
                is_active: row.get::<_, i32>(6)? != 0,
                must_change_password: row.get::<_, i32>(7)? != 0,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
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
        .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    db.conn.execute(
        "INSERT INTO sessions (id, user_id, token, expires_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, user_id, token, expires],
    ).map_err(|e| format!("Failed to create session: {}", e))?;

    Ok(token)
}

/// Validate a session token for a **normal** command.
///
/// In addition to the token/expiry/active checks, this rejects any user who
/// still has `must_change_password` set. Until they complete the mandated
/// change, their (otherwise full 24h) token must be usable only for the
/// password-change and current-user endpoints — not for reading or mutating lab
/// data. Enforcing it here (rather than only in the UI) means every command that
/// calls `validate_session` is protected with no per-command change: the block
/// is default-deny.
pub fn validate_session(db: &Database, token: &str) -> Result<User, String> {
    let user = validate_session_allow_password_change(db, token)?;
    if user.must_change_password {
        return Err("A password change is required before continuing.".to_string());
    }
    Ok(user)
}

/// Raw session lookup: returns the user for a valid, unexpired token belonging
/// to an active account, **regardless of the `must_change_password` flag**.
///
/// Only the two endpoints a locked-out user still needs may use this:
/// `change_password` (to clear the flag) and `get_current_user` (so the forced-
/// change screen can render who is logged in). Everything else must go through
/// `validate_session`.
pub fn validate_session_allow_password_change(db: &Database, token: &str) -> Result<User, String> {
    let user = db.conn.query_row(
        "SELECT u.id, u.username, u.password_hash, u.display_name, u.email, u.role, u.is_active, u.must_change_password, u.created_at, u.updated_at
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
                role: row.get::<_, String>(5)?.parse().unwrap_or(UserRole::Guest),
                is_active: row.get::<_, i32>(6)? != 0,
                must_change_password: row.get::<_, i32>(7)? != 0,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    /// Build an in-memory DB with one user (with the given forced-change flag)
    /// and a live session token for them. Returns (db, token).
    fn db_with_session(must_change: bool) -> (Database, String) {
        let db = Database::new_in_memory().unwrap();
        db.run_migrations().unwrap();
        db.conn
            .execute(
                "INSERT INTO users (id, username, password_hash, display_name, role, is_active, must_change_password) \
                 VALUES ('u1', 'tech1', 'x', 'Tech One', 'tech', 1, ?1)",
                params![if must_change { 1 } else { 0 }],
            )
            .unwrap();
        let token = create_session(&db, "u1").unwrap();
        (db, token)
    }

    #[test]
    fn normal_session_passes_when_no_forced_change() {
        let (db, token) = db_with_session(false);
        let user = validate_session(&db, &token).expect("clean user should validate");
        assert_eq!(user.id, "u1");
        assert!(!user.must_change_password);
    }

    #[test]
    fn forced_change_blocks_normal_commands() {
        // The core of the fix: a user who still owes a forced password change
        // must NOT be able to authorize an ordinary command server-side, even
        // though their token is otherwise valid and unexpired.
        let (db, token) = db_with_session(true);
        let err = validate_session(&db, &token).expect_err("must be rejected");
        assert!(err.to_lowercase().contains("password change"), "message was: {}", err);
    }

    #[test]
    fn forced_change_still_allows_password_change_endpoint() {
        // The carve-out: change_password / get_current_user use the allow variant
        // so a locked-out user can actually clear the flag.
        let (db, token) = db_with_session(true);
        let user = validate_session_allow_password_change(&db, &token)
            .expect("allow variant must return the user so they can change their password");
        assert_eq!(user.id, "u1");
        assert!(user.must_change_password);
    }

    #[test]
    fn invalid_token_is_rejected_by_both_variants() {
        let (db, _token) = db_with_session(false);
        assert!(validate_session(&db, "not-a-real-token").is_err());
        assert!(validate_session_allow_password_change(&db, "not-a-real-token").is_err());
    }
}
