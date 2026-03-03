pub mod core;
pub mod commands;

use std::sync::Mutex;
use tauri::Manager;

use core::AppState;

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
