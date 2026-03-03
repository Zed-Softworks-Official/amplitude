pub mod commands;
pub mod core;

use std::sync::Mutex;
use tauri::Manager;

use core::{AppState, Config};

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config = Config::load();
            let mut state = AppState::default();

            if let Ok(config) = config {
                state = AppState::from_config(config);
            } else {
                println!("no config found, creating new one");
                let config = Config::new(state.clone());
                let _ = config.save();
            }

            app.manage(Mutex::new(state));

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::channel::add_channel
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
