use crate::core::{bus::Bus, AppState};
use std::sync::Mutex;

#[tauri::command]
pub fn get_buses(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Vec<Bus>, String> {
    let state = state.lock().map_err(|_| "state lock poisend".to_string())?;
    Ok(state.buses.values().cloned().collect())
}
