pub mod core;
pub mod commands;

use std::{collections::HashMap, sync::Mutex};
use tauri::Manager;
use uuid::Uuid;

use core::{
    bus::Bus,
    channels::{Channel, Send},
};

pub struct AppState {
    pub channels: HashMap<Uuid, Channel>,
    pub busses: HashMap<Uuid, Bus>,
}

impl AppState {
    pub fn default() -> Self {
        let monitor_bus = Bus::new("Master".to_string());
        let stream_bus = Bus::new("Stream".to_string());

        let default_sends = vec![
            Send::new(monitor_bus.id, monitor_bus.volume, monitor_bus.muted),
            Send::new(stream_bus.id, stream_bus.volume, stream_bus.muted),
        ];

        let mic_channel =
            Channel::new("Mic".to_string(), default_sends.clone());

        Self {
            channels: HashMap::from([(mic_channel.id, mic_channel)]),
            busses: HashMap::from([
                (monitor_bus.id, monitor_bus),
                (stream_bus.id, stream_bus),
            ]),
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(Mutex::new(AppState::default()));

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::channel::add_channel])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
