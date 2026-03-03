use crate::core::channels::Channel;
use crate::core::state::AppState;
use std::sync::Mutex;

#[tauri::command]
pub fn add_channel(
    state: tauri::State<'_, Mutex<AppState>>,
    name: String,
) -> Result<Channel, String> {
    println!("new channel: {:?}", name.clone());
    let mut state = state.lock().unwrap();

    let sends = state.default_sends.clone();
    let new_channel = Channel::new(name, sends);

    state.add_channel(new_channel.clone());

    Ok(new_channel)
}

pub fn get_channels(state: tauri::State<'_, Mutex<AppState>>) -> Vec<Channel> {
    let state = state.lock().unwrap();
    state.channels.values().cloned().collect()
}

