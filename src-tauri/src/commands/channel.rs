#[tauri::command]
pub fn add_channel(name: String) {
    println!("add_channel: {:?}", name);
}
