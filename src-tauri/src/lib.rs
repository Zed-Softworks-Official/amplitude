pub mod audio;
pub mod backend;
pub mod commands;
pub mod core;

use std::sync::Mutex;
use tauri::{tray::TrayIconBuilder, Manager};

use commands::{
    bus::{get_buses, update_bus},
    channel::{
        add_channel, delete_channel, get_channels, reorder_channels,
        update_channel_connections, update_channel_send,
    },
};
use core::{config::Config, engine::AudioEngine};

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let engine = match Config::load() {
                Ok(config) => {
                    println!("loaded config");
                    AudioEngine::from_config(config)
                }
                Err(err) if err.to_string() == "config not found" => {
                    println!("no config found, creating new one");
                    let engine = AudioEngine::new();
                    Config::from_payload(engine.to_save_payload())
                        .save()
                        .map_err(|e| {
                            format!("failed to save initial config: {e}")
                        })?;
                    engine
                }
                Err(err) => {
                    return Err(format!("failed to load config: {err}").into())
                }
            };

            app.manage(Mutex::new(engine));

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
            add_channel,
            get_channels,
            delete_channel,
            reorder_channels,
            update_channel_send,
            update_channel_connections,
            get_buses,
            update_bus,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
