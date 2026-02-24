// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // TODO: Find a fix for this on wayland
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");

    amplitude_lib::run()
}
