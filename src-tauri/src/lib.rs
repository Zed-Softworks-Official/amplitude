pub mod commands;
pub mod core;

use std::sync::Mutex;
use tauri::{tray::TrayIconBuilder, Manager};

use core::{AppState, Config};

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let mut state = AppState::default();

            // App State and Config Loading
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

            // Tray icon
            let quit = tauri::menu::MenuItem::with_id(
                app,
                "quit",
                "Quit",
                true,
                None::<&str>,
            )?;

            let menu = tauri::menu::Menu::with_items(app, &[&quit])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // Channels
            commands::channel::add_channel,
            commands::channel::get_channels,
            // Buses
            commands::bus::get_buses,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
