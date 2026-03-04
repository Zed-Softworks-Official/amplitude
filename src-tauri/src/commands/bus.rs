use crate::core::{config::Config, AppState, AppStatePayload, Bus};
use std::sync::Mutex;
use tauri::Emitter;
use uuid::Uuid;

fn emit_and_save(
    app: &tauri::AppHandle,
    payload: AppStatePayload,
    config: Config,
) -> Result<(), String> {
    app.emit("appstate-changed", payload)
        .map_err(|e| format!("failed to emit appstate-changed: {e}"))?;
    config
        .save()
        .map_err(|e| format!("failed to save config: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn get_buses(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Vec<Bus>, String> {
    let state = state
        .lock()
        .map_err(|_| "state lock poisoned".to_string())?;
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
    let (payload, config) = {
        let mut state = state
            .lock()
            .map_err(|_| "state lock poisoned".to_string())?;

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

        (state.to_payload(), Config::new(state.clone()))
        // mutex guard dropped here
    };

    emit_and_save(&app, payload, config)
}
