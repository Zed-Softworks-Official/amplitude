pub mod audio;
pub mod backend;
pub mod commands;
pub mod core;

use std::sync::{Arc, Mutex};
use tauri::{tray::TrayIconBuilder, Emitter, Manager};

use commands::{
    bus::{get_buses, set_bus_output, update_bus},
    channel::{
        add_channel, delete_channel, get_channels, reorder_channels,
        set_channel_input, update_channel_connections, update_channel_send,
    },
    nodes::get_nodes,
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

            // Wrap in Arc<Mutex> so the poller task can share ownership with
            // Tauri's managed state.
            let engine = Arc::new(Mutex::new(engine));

            app.manage(Arc::clone(&engine));

            // -----------------------------------------------------------------
            // Background node poller — drains BackendEvents every 100 ms and
            // emits "nodes-changed" when the node list changes.
            // Runs on a dedicated OS thread to avoid a tokio dependency.
            // -----------------------------------------------------------------

            let app_handle = app.handle().clone();
            let engine_poller = Arc::clone(&engine);

            std::thread::Builder::new()
                .name("Amplitude Node Poller".to_string())
                .spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    let events = match engine_poller.lock() {
                        Ok(mut eng) => eng.poll_events(),
                        Err(_) => break,
                    };

                    if !events.is_empty() {
                        let nodes = match engine_poller.lock() {
                            Ok(eng) => eng.get_nodes(),
                            Err(_) => break,
                        };
                        let _ = app_handle.emit("nodes-changed", nodes);
                    }
                })
                .expect("failed to spawn node poller thread");

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
                .on_menu_event(|app, event| {
                    if event.id().as_ref() == "quit" {
                        app.exit(0);
                    }
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
            set_channel_input,
            get_buses,
            update_bus,
            set_bus_output,
            get_nodes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
