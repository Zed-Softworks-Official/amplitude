use crate::core::channels::Channel;
use crate::core::state::AppState;
use std::sync::Mutex;

#[tauri::command]
pub fn add_channel(
    state: tauri::State<'_, Mutex<AppState>>,
    name: String,
) -> Result<Channel, String> {
    println!("new channel: {:?}", name.clone());
    let mut state =
        state.lock().map_err(|_| "state lock poisend".to_string())?;

    let sends = state.default_sends.clone();
    let new_channel = Channel::new(name, sends);

    state.add_channel(new_channel.clone());

    Ok(new_channel)
}

#[tauri::command]
pub fn get_channels(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Vec<Channel>, String> {
    let state = state.lock().map_err(|_| "state lock poisend".to_string())?;
    Ok(state.channels.values().cloned().collect())
}
