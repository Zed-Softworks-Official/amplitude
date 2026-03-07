use crate::core::{
    bus::Bus,
    config::Config,
    engine::{AppStatePayload, AudioEngine},
};
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use uuid::Uuid;

fn emit_and_save(
    app: &tauri::AppHandle,
    payload: AppStatePayload,
    engine: &AudioEngine,
) -> Result<(), String> {
    app.emit("appstate-changed", payload)
        .map_err(|e| format!("failed to emit appstate-changed: {e}"))?;
    Config::from_payload(engine.to_save_payload())
        .save()
        .map_err(|e| format!("failed to save config: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn get_buses(
    state: tauri::State<'_, Arc<Mutex<AudioEngine>>>,
) -> Result<Vec<Bus>, String> {
    let engine = state
        .lock()
        .map_err(|_| "engine lock poisoned".to_string())?;
    Ok(engine.buses.values().cloned().collect())
}

#[tauri::command]
pub fn update_bus(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Mutex<AudioEngine>>>,
    bus_id: Uuid,
    volume: Option<f32>,
    muted: Option<bool>,
) -> Result<(), String> {
    let mut engine = state
        .lock()
        .map_err(|_| "engine lock poisoned".to_string())?;
    engine.update_bus(bus_id, volume, muted)?;
    let payload = engine.to_payload();
    emit_and_save(&app, payload, &engine)
}
