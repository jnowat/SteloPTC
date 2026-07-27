use crate::auth as auth_service;
use crate::db::queries;
use crate::models::user::*;
use crate::AppState;
use tauri::State;

/// The roles the system recognises. Module-level so `create_user` and
/// `update_user_role` validate against the same list — they previously
/// disagreed, with create_user passing an unchecked string straight to the DB.
const VALID_ROLES: &[&str] = &["admin", "supervisor", "tech", "guest"];

#[tauri::command]
pub fn login(state: State<AppState>, username: String, password: String) -> Result<LoginResponse, String> {
    // Check the lockout BEFORE taking the DB lock and before hashing, so a
    // guessing loop cannot hold the global mutex or burn CPU on bcrypt.
    if let Err(e) = state.login_throttle.check(&username) {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let remaining = state
            .login_throttle
            .lock_remaining(&username)
            .map(|d| d.as_secs() / 60)
            .unwrap_or(0);
        queries::log_audit(
            &db.conn, None, "login_blocked", "user", None, None, Some(&username),
            Some(&format!("Locked out after repeated failures; ~{} minute(s) remaining", remaining)),
        ).ok();
        return Err(e);
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::authenticate(&db, &username, &password).inspect_err(|e| {
        state.login_throttle.record_failure(&username);
        queries::log_audit(&db.conn, None, "login_failed", "user", None, None, Some(&username), Some(e.as_str())).ok();
    })?;
    state.login_throttle.clear(&username);
    let token = auth_service::create_session(&db, &user.id)?;

    queries::log_audit(&db.conn, Some(&user.id), "login", "user", Some(&user.id), None, None, None)
        .ok();

    Ok(LoginResponse {
        must_change_password: user.must_change_password,
        token,
        user: UserPublic {
            id: user.id,
            username: user.username,
            display_name: user.display_name,
            email: user.email,
            role: user.role.as_str().to_string(),
            is_active: user.is_active,
        },
    })
}

#[tauri::command]
pub fn get_current_user(state: State<AppState>, token: String) -> Result<UserPublic, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // Allow-password-change variant: a user who still owes a forced change must
    // be able to fetch their own identity so the forced-change screen can render.
    let user = auth_service::validate_session_allow_password_change(&db, &token)?;
    Ok(UserPublic {
        id: user.id,
        username: user.username,
        display_name: user.display_name,
        email: user.email,
        role: user.role.as_str().to_string(),
        is_active: user.is_active,
    })
}

#[tauri::command]
pub fn logout(state: State<AppState>, token: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    auth_service::invalidate_session(&db, &token)
}

#[tauri::command]
pub fn list_users(state: State<AppState>, token: String) -> Result<Vec<UserPublic>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let caller = auth_service::validate_session(&db, &token)?;
    if !caller.role.can_manage() {
        return Err("Insufficient permissions".to_string());
    }

    let mut stmt = db.conn.prepare(
        "SELECT id, username, display_name, email, role, is_active FROM users ORDER BY username"
    ).map_err(|e| e.to_string())?;

    let users = stmt.query_map([], |row| {
        Ok(UserPublic {
            id: row.get(0)?,
            username: row.get(1)?,
            display_name: row.get(2)?,
            email: row.get(3)?,
            role: row.get(4)?,
            is_active: row.get::<_, i32>(5)? != 0,
        })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    Ok(users)
}

#[tauri::command]
pub fn create_user(state: State<AppState>, token: String, request: CreateUserRequest) -> Result<UserPublic, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let caller = auth_service::validate_session(&db, &token)?;
    if !caller.role.is_admin() {
        return Err("Only admins can create users".to_string());
    }

    // Same policy as change_password — enforced here rather than left to the DB
    // CHECK constraint so the caller gets a readable message instead of a raw
    // SQL error, and so a weak provisioned password is impossible rather than
    // merely discouraged.
    auth_service::validate_password(&request.password)?;
    if !VALID_ROLES.contains(&request.role.as_str()) {
        return Err(format!(
            "Invalid role '{}'. Must be one of: admin, supervisor, tech, guest",
            request.role
        ));
    }
    if request.username.trim().is_empty() {
        return Err("Username is required".to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)
        .map_err(|e| format!("Password hashing failed: {}", e))?;

    // New accounts start under a forced password change: the admin who typed
    // the initial password knows it, so it is a shared secret until the user
    // replaces it.
    db.conn.execute(
        "INSERT INTO users (id, username, password_hash, display_name, email, role, must_change_password)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
        rusqlite::params![id, request.username, hash, request.display_name, request.email, request.role],
    ).map_err(|e| format!("Failed to create user: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&caller.id), "create", "user", Some(&id),
        None, Some(&request.username), Some("User created"),
    ).ok();

    Ok(UserPublic {
        id,
        username: request.username,
        display_name: request.display_name,
        email: request.email,
        role: request.role,
        is_active: true,
    })
}

/// Change the calling user's own password.
///
/// `current_password` is required for a **voluntary** change and re-authenticates
/// the caller: a session token proves only that someone holds the token, which
/// is a weaker claim than knowing the password. Without this check, a token
/// obtained for a moment (an unlocked workstation, a copy of the app data
/// directory) converts into permanent ownership of the account.
///
/// The **forced**-change flow is exempt, and deliberately so — a user under
/// `must_change_password` is by definition holding a credential they were told
/// to replace, often one an admin issued and they never chose. Requiring them to
/// retype it would block the only endpoint they can still reach.
///
/// On success every *other* session for the user is revoked. A password change
/// is frequently a response to a suspected compromise, and leaving the
/// attacker's token live for the remainder of its 24h window would defeat the
/// point of changing it.
#[tauri::command]
pub fn change_password(
    state: State<AppState>,
    token: String,
    new_password: String,
    current_password: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // Allow-password-change variant: this is the one endpoint a user under a
    // forced change must reach in order to clear the flag.
    let user = auth_service::validate_session_allow_password_change(&db, &token)?;

    if !user.must_change_password {
        let current = current_password
            .filter(|p| !p.is_empty())
            .ok_or_else(|| "Your current password is required to change it.".to_string())?;
        if !bcrypt::verify(&current, &user.password_hash).unwrap_or(false) {
            queries::log_audit(
                &db.conn, Some(&user.id), "change_password_denied", "user", Some(&user.id),
                None, None, Some("Current password did not match"),
            ).ok();
            return Err("Your current password is incorrect.".to_string());
        }
    }

    auth_service::validate_password(&new_password)?;

    // Reject a no-op change outright rather than logging a rotation that did
    // not happen — an audit entry claiming the password changed when it did not
    // is worse than no entry.
    if bcrypt::verify(&new_password, &user.password_hash).unwrap_or(false) {
        return Err("Your new password must be different from your current one.".to_string());
    }

    let hash = bcrypt::hash(&new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| format!("Password hashing failed: {}", e))?;

    db.conn.execute(
        "UPDATE users SET password_hash = ?1, must_change_password = 0, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![hash, user.id],
    ).map_err(|e| format!("Failed to update password: {}", e))?;

    // A password change is a revocation event. The caller's own token survives
    // so they are not logged out by their own action.
    let revoked = db.conn.execute(
        "DELETE FROM sessions WHERE user_id = ?1 AND token <> ?2",
        rusqlite::params![user.id, token],
    ).unwrap_or(0);

    let detail = if user.must_change_password {
        format!("Password changed via forced change flow; {} other session(s) revoked", revoked)
    } else {
        format!("Password changed after re-authentication; {} other session(s) revoked", revoked)
    };
    queries::log_audit(
        &db.conn, Some(&user.id), "change_password", "user", Some(&user.id),
        None, None, Some(&detail),
    ).ok();

    Ok(())
}

#[tauri::command]
pub fn update_user_role(state: State<AppState>, token: String, user_id: String, new_role: String) -> Result<(), String> {
    if !VALID_ROLES.contains(&new_role.as_str()) {
        return Err(format!("Invalid role '{}'. Must be one of: admin, supervisor, tech, guest", new_role));
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let caller = auth_service::validate_session(&db, &token)?;
    if !caller.role.is_admin() {
        return Err("Only admins can change roles".to_string());
    }

    // Refuse to remove the last administrator. Nothing else in the system can
    // restore one: changing roles, setting the lab profile and resetting the
    // database are all admin-only, so a lab that demotes its final admin is
    // locked out until someone hand-edits SQLite.
    if new_role != "admin" {
        let other_admins: i64 = db.conn.query_row(
            "SELECT COUNT(*) FROM users WHERE role = 'admin' AND is_active = 1 AND id <> ?1",
            rusqlite::params![user_id],
            |r| r.get(0),
        ).unwrap_or(0);
        if other_admins == 0 {
            return Err(
                "This is the last active administrator. Promote another user to admin first."
                    .to_string(),
            );
        }
    }

    db.conn.execute(
        "UPDATE users SET role = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![new_role, user_id],
    ).map_err(|e| format!("Failed to update role: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&caller.id), "update_role", "user", Some(&user_id),
        None, Some(&new_role), None,
    ).ok();

    Ok(())
}
