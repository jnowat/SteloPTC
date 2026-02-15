use crate::auth as auth_service;
use crate::db::queries;
use crate::models::reminder::*;
use crate::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn list_reminders(state: State<AppState>, token: String) -> Result<Vec<Reminder>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db.conn.prepare(
        "SELECT r.*, s.accession_number as specimen_accession, u.display_name as assigned_name
         FROM reminders r
         LEFT JOIN specimens s ON r.specimen_id = s.id
         LEFT JOIN users u ON r.assigned_to = u.id
         ORDER BY r.due_date ASC"
    ).map_err(|e| e.to_string())?;

    let reminders = stmt.query_map([], |row| {
        Ok(Reminder {
            id: row.get("id")?,
            specimen_id: row.get("specimen_id")?,
            specimen_accession: row.get("specimen_accession")?,
            title: row.get("title")?,
            description: row.get("description")?,
            reminder_type: row.get("reminder_type")?,
            due_date: row.get("due_date")?,
            is_recurring: row.get::<_, i32>("is_recurring")? != 0,
            recurrence_days: row.get("recurrence_days")?,
            recurrence_rule: row.get("recurrence_rule")?,
            status: row.get("status")?,
            snooze_count: row.get("snooze_count")?,
            urgency: row.get("urgency")?,
            assigned_to: row.get("assigned_to")?,
            assigned_name: row.get("assigned_name")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    Ok(reminders)
}

#[tauri::command]
pub fn get_active_reminders(state: State<AppState>, token: String) -> Result<Vec<Reminder>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;

    let mut stmt = db.conn.prepare(
        "SELECT r.*, s.accession_number as specimen_accession, u.display_name as assigned_name
         FROM reminders r
         LEFT JOIN specimens s ON r.specimen_id = s.id
         LEFT JOIN users u ON r.assigned_to = u.id
         WHERE r.status IN ('active', 'snoozed') AND r.due_date <= date('now', '+7 days')
         ORDER BY r.urgency DESC, r.due_date ASC"
    ).map_err(|e| e.to_string())?;

    let reminders = stmt.query_map([], |row| {
        Ok(Reminder {
            id: row.get("id")?,
            specimen_id: row.get("specimen_id")?,
            specimen_accession: row.get("specimen_accession")?,
            title: row.get("title")?,
            description: row.get("description")?,
            reminder_type: row.get("reminder_type")?,
            due_date: row.get("due_date")?,
            is_recurring: row.get::<_, i32>("is_recurring")? != 0,
            recurrence_days: row.get("recurrence_days")?,
            recurrence_rule: row.get("recurrence_rule")?,
            status: row.get("status")?,
            snooze_count: row.get("snooze_count")?,
            urgency: row.get("urgency")?,
            assigned_to: row.get("assigned_to")?,
            assigned_name: row.get("assigned_name")?,
            created_by: row.get("created_by")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    Ok(reminders)
}

#[tauri::command]
pub fn create_reminder(
    state: State<AppState>,
    token: String,
    request: CreateReminderRequest,
) -> Result<Reminder, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let id = uuid::Uuid::new_v4().to_string();

    db.conn.execute(
        "INSERT INTO reminders (id, specimen_id, title, description, reminder_type, due_date,
         is_recurring, recurrence_days, recurrence_rule, urgency, assigned_to, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            id, request.specimen_id, request.title, request.description, request.reminder_type,
            request.due_date, request.is_recurring.unwrap_or(false) as i32,
            request.recurrence_days, request.recurrence_rule,
            request.urgency.as_deref().unwrap_or("normal"),
            request.assigned_to, user.id,
        ],
    ).map_err(|e| format!("Failed to create reminder: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "create", "reminder", Some(&id),
        None, Some(&request.title), None,
    ).ok();

    db.conn.query_row(
        "SELECT r.*, s.accession_number as specimen_accession, u.display_name as assigned_name
         FROM reminders r
         LEFT JOIN specimens s ON r.specimen_id = s.id
         LEFT JOIN users u ON r.assigned_to = u.id
         WHERE r.id = ?1",
        params![id],
        |row| {
            Ok(Reminder {
                id: row.get("id")?,
                specimen_id: row.get("specimen_id")?,
                specimen_accession: row.get("specimen_accession")?,
                title: row.get("title")?,
                description: row.get("description")?,
                reminder_type: row.get("reminder_type")?,
                due_date: row.get("due_date")?,
                is_recurring: row.get::<_, i32>("is_recurring")? != 0,
                recurrence_days: row.get("recurrence_days")?,
                recurrence_rule: row.get("recurrence_rule")?,
                status: row.get("status")?,
                snooze_count: row.get("snooze_count")?,
                urgency: row.get("urgency")?,
                assigned_to: row.get("assigned_to")?,
                assigned_name: row.get("assigned_name")?,
                created_by: row.get("created_by")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    ).map_err(|e| format!("Failed to fetch reminder: {}", e))
}

#[tauri::command]
pub fn update_reminder(
    state: State<AppState>,
    token: String,
    request: UpdateReminderRequest,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;
    if !user.role.can_write() {
        return Err("Insufficient permissions".to_string());
    }

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref title) = request.title {
        updates.push(format!("title = ?{}", values.len() + 1));
        values.push(Box::new(title.clone()));
    }
    if let Some(ref desc) = request.description {
        updates.push(format!("description = ?{}", values.len() + 1));
        values.push(Box::new(desc.clone()));
    }
    if let Some(ref dd) = request.due_date {
        updates.push(format!("due_date = ?{}", values.len() + 1));
        values.push(Box::new(dd.clone()));
    }
    if let Some(ref urg) = request.urgency {
        updates.push(format!("urgency = ?{}", values.len() + 1));
        values.push(Box::new(urg.clone()));
    }
    if let Some(ref status) = request.status {
        updates.push(format!("status = ?{}", values.len() + 1));
        values.push(Box::new(status.clone()));
    }
    if let Some(ref at) = request.assigned_to {
        updates.push(format!("assigned_to = ?{}", values.len() + 1));
        values.push(Box::new(at.clone()));
    }

    if updates.is_empty() {
        return Err("No fields to update".to_string());
    }

    updates.push("updated_at = datetime('now')".to_string());
    let sql = format!(
        "UPDATE reminders SET {} WHERE id = ?{}",
        updates.join(", "),
        values.len() + 1
    );
    values.push(Box::new(request.id.clone()));

    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    db.conn.execute(&sql, bind_refs.as_slice())
        .map_err(|e| format!("Failed to update reminder: {}", e))?;

    queries::log_audit(
        &db.conn, Some(&user.id), "update", "reminder", Some(&request.id),
        None, None, Some("Reminder updated"),
    ).ok();

    Ok(())
}

#[tauri::command]
pub fn dismiss_reminder(state: State<AppState>, token: String, id: String, snooze: bool) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = auth_service::validate_session(&db, &token)?;

    if snooze {
        db.conn.execute(
            "UPDATE reminders SET status = 'snoozed', snooze_count = snooze_count + 1,
             due_date = date(due_date, '+1 day'), updated_at = datetime('now')
             WHERE id = ?1",
            params![id],
        ).map_err(|e| e.to_string())?;

        // Check if snooze count >= 2 for escalation
        let snooze_count: i32 = db.conn.query_row(
            "SELECT snooze_count FROM reminders WHERE id = ?1",
            params![id],
            |r| r.get(0),
        ).unwrap_or(0);

        if snooze_count >= 2 {
            db.conn.execute(
                "UPDATE reminders SET urgency = 'critical' WHERE id = ?1",
                params![id],
            ).ok();
        }
    } else {
        db.conn.execute(
            "UPDATE reminders SET status = 'dismissed', updated_at = datetime('now') WHERE id = ?1",
            params![id],
        ).map_err(|e| e.to_string())?;
    }

    let action = if snooze { "snooze" } else { "dismiss" };
    queries::log_audit(
        &db.conn, Some(&user.id), action, "reminder", Some(&id), None, None, None,
    ).ok();

    Ok(())
}
