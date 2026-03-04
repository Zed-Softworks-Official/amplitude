use crate::core::{channels::Channel, config::Config, AppState};
use std::sync::Mutex;
use tauri::Emitter;
use uuid::Uuid;

fn emit_and_save(
    app: &tauri::AppHandle,
    state: &AppState,
) {
    if let Err(e) = app.emit("appstate-changed", state.to_payload()) {
        eprintln!("failed to emit appstate-changed: {e}");
    }
    if let Err(e) = Config::new(state.clone()).save() {
        eprintln!("failed to save config: {e}");
    }
}

#[tauri::command]
pub fn add_channel(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    name: String,
) -> Result<Channel, String> {
    println!("new channel: {:?}", name.clone());
    let mut state = state.lock().map_err(|_| "state lock poisoned".to_string())?;

    let sends = state.default_sends.clone();
    let new_channel = Channel::new(name, sends);

    state.add_channel(new_channel.clone());
    emit_and_save(&app, &state);

    Ok(new_channel)
}

#[tauri::command]
pub fn get_channels(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Vec<Channel>, String> {
    let state = state.lock().map_err(|_| "state lock poisoned".to_string())?;
    Ok(state.ordered_channels())
}

#[tauri::command]
pub fn delete_channel(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    id: Uuid,
) -> Result<(), String> {
    let mut state = state.lock().map_err(|_| "state lock poisoned".to_string())?;

    // Protect mic: it is always the first channel and named "mic"
    if let Some(ch) = state.channels.get(&id) {
        if ch.name.to_lowercase() == "mic" {
            return Err("cannot delete the mic channel".to_string());
        }
    }

    state.channels.remove(&id);
    state.channel_order.retain(|oid| *oid != id);
    emit_and_save(&app, &state);

    Ok(())
}

#[tauri::command]
pub fn reorder_channels(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    order: Vec<Uuid>,
) -> Result<(), String> {
    let mut state = state.lock().map_err(|_| "state lock poisoned".to_string())?;

    // Only keep ids that actually exist
    state.channel_order = order
        .into_iter()
        .filter(|id| state.channels.contains_key(id))
        .collect();

    // Append any that were missing from the supplied order
    for id in state.channels.keys().cloned().collect::<Vec<_>>() {
        if !state.channel_order.contains(&id) {
            state.channel_order.push(id);
        }
    }

    emit_and_save(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn update_channel_send(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    channel_id: Uuid,
    bus_id: Uuid,
    volume: Option<f32>,
    muted: Option<bool>,
) -> Result<(), String> {
    let mut state = state.lock().map_err(|_| "state lock poisoned".to_string())?;

    let channel = state
        .channels
        .get_mut(&channel_id)
        .ok_or_else(|| format!("channel {channel_id} not found"))?;

    let send = channel
        .sends
        .iter_mut()
        .find(|s| s.bus_id == bus_id)
        .ok_or_else(|| format!("send to bus {bus_id} not found on channel {channel_id}"))?;

    if let Some(v) = volume {
        send.volume = v.clamp(0.0, 1.0);
    }
    if let Some(m) = muted {
        send.muted = m;
    }

    emit_and_save(&app, &state);
    Ok(())
}

#[tauri::command]
pub fn update_channel_connections(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    channel_id: Uuid,
    process_names: Vec<String>,
) -> Result<(), String> {
    use crate::core::channels::Connection;

    let mut state = state.lock().map_err(|_| "state lock poisoned".to_string())?;

    let channel = state
        .channels
        .get_mut(&channel_id)
        .ok_or_else(|| format!("channel {channel_id} not found"))?;

    channel.connections = process_names
        .into_iter()
        .enumerate()
        .map(|(i, name)| Connection::new(i as u32, name))
        .collect();

    emit_and_save(&app, &state);
    Ok(())
}
