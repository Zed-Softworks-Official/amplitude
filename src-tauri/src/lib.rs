pub mod commands;
pub mod core;

use std::sync::Mutex;
use tauri::Manager;

use core::{AppState, Config};

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let mut state = AppState::default();

            match Config::load() {
                Ok(config) => state = AppState::from_config(config),
                Err(err) if err.to_string() == "config not found" => {
                    println!("no config found, creating new one");
                    Config::new(state.clone()).save().map_err(|e| {
                        format!("failed to save initial config: {e}")
                    })?;
                }
                Err(err) => {
                    return Err(format!("failed to load config: {err}").into())
                }
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
