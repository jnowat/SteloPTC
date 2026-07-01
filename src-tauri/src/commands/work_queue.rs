use crate::auth as auth_service;
use crate::db::work_queue::compute_work_queue_items;
use crate::AppState;
use tauri::State;

pub use crate::db::work_queue::WorkQueueItem;

#[tauri::command]
pub fn get_work_queue(
    state: State<AppState>,
    token: String,
) -> Result<Vec<WorkQueueItem>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _user = auth_service::validate_session(&db, &token)?;
    compute_work_queue_items(&db.conn)
}
