use crate::core::{bus::Bus, config::Config, AppState};
use std::sync::Mutex;
use tauri::Emitter;
use uuid::Uuid;

fn emit_and_save(app: &tauri::AppHandle, state: &AppState) {
    if let Err(e) = app.emit("appstate-changed", state.to_payload()) {
        eprintln!("failed to emit appstate-changed: {e}");
    }
    if let Err(e) = Config::new(state.clone()).save() {
        eprintln!("failed to save config: {e}");
    }
}

#[tauri::command]
pub fn get_buses(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Vec<Bus>, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_string())?;
    Ok(state.buses.values().cloned().collect())
}

#[tauri::command]
pub fn update_bus(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    bus_id: Uuid,
    volume: Option<f32>,
    muted: Option<bool>,
) -> Result<(), String> {
    let mut state = state.lock().map_err(|_| "state lock poisoned".to_string())?;

    let bus = state
        .buses
        .get_mut(&bus_id)
        .ok_or_else(|| format!("bus {bus_id} not found"))?;

    if let Some(v) = volume {
        bus.volume = v.clamp(0.0, 1.0);
    }
    if let Some(m) = muted {
        bus.muted = m;
    }

    emit_and_save(&app, &state);
    Ok(())
}
