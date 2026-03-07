use crate::audio::node::NodeInfo;
use crate::core::engine::AudioEngine;
use std::sync::{Arc, Mutex};

#[tauri::command]
pub fn get_nodes(
    state: tauri::State<'_, Arc<Mutex<AudioEngine>>>,
) -> Result<Vec<NodeInfo>, String> {
    let engine = state
        .lock()
        .map_err(|_| "engine lock poisoned".to_string())?;
    Ok(engine.get_nodes())
}
