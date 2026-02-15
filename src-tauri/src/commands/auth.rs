use crate::auth as auth_service;
use crate::db::queries;
use crate::models::user::*;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn login(state: State<AppState>, username: String, password: String) -> Result<LoginResponse, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::authenticate(&db, &username, &password)?;
    let token = auth_service::create_session(&db, &user.id)?;

    queries::log_audit(&db.conn, Some(&user.id), "login", "user", Some(&user.id), None, None, None)
        .ok();

    Ok(LoginResponse {
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
    let user = auth_service::validate_session(&db, &token)?;
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

    let id = uuid::Uuid::new_v4().to_string();
    let hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)
        .map_err(|e| format!("Password hashing failed: {}", e))?;

    db.conn.execute(
        "INSERT INTO users (id, username, password_hash, display_name, email, role)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
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

#[tauri::command]
pub fn update_user_role(state: State<AppState>, token: String, user_id: String, new_role: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let caller = auth_service::validate_session(&db, &token)?;
    if !caller.role.is_admin() {
        return Err("Only admins can change roles".to_string());
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
