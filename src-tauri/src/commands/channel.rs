use crate::core::{
    channels::Channel,
    config::Config,
    engine::{AppStatePayload, AudioEngine},
};
use std::sync::Mutex;
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
pub fn add_channel(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AudioEngine>>,
    name: String,
) -> Result<Channel, String> {
    let mut engine = state
        .lock()
        .map_err(|_| "engine lock poisoned".to_string())?;
    let new_channel = engine.add_channel(name)?;
    let payload = engine.to_payload();
    emit_and_save(&app, payload, &engine)?;
    Ok(new_channel)
}

#[tauri::command]
pub fn get_channels(
    state: tauri::State<'_, Mutex<AudioEngine>>,
) -> Result<Vec<Channel>, String> {
    let engine = state
        .lock()
        .map_err(|_| "engine lock poisoned".to_string())?;
    Ok(engine.ordered_channels())
}

#[tauri::command]
pub fn delete_channel(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AudioEngine>>,
    id: Uuid,
) -> Result<(), String> {
    let mut engine = state
        .lock()
        .map_err(|_| "engine lock poisoned".to_string())?;
    engine.delete_channel(id)?;
    let payload = engine.to_payload();
    emit_and_save(&app, payload, &engine)
}

#[tauri::command]
pub fn reorder_channels(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AudioEngine>>,
    order: Vec<Uuid>,
) -> Result<(), String> {
    let mut engine = state
        .lock()
        .map_err(|_| "engine lock poisoned".to_string())?;
    engine.reorder_channels(order);
    let payload = engine.to_payload();
    emit_and_save(&app, payload, &engine)
}

#[tauri::command]
pub fn update_channel_send(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AudioEngine>>,
    channel_id: Uuid,
    bus_id: Uuid,
    volume: Option<f32>,
    muted: Option<bool>,
) -> Result<(), String> {
    let mut engine = state
        .lock()
        .map_err(|_| "engine lock poisoned".to_string())?;
    engine.update_channel_send(channel_id, bus_id, volume, muted)?;
    let payload = engine.to_payload();
    emit_and_save(&app, payload, &engine)
}

#[tauri::command]
pub fn update_channel_connections(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AudioEngine>>,
    channel_id: Uuid,
    process_names: Vec<String>,
) -> Result<(), String> {
    let mut engine = state
        .lock()
        .map_err(|_| "engine lock poisoned".to_string())?;
    engine.update_channel_connections(channel_id, process_names)?;
    let payload = engine.to_payload();
    emit_and_save(&app, payload, &engine)
}
