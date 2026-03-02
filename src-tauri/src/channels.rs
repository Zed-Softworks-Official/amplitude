use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Channel {
    pub id: Uuid,
    pub name: String,
    pub sends: Vec<Send>,
}

impl Channel {
    pub fn new(name: String, sends: Vec<Send>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            sends,
        }
    }
}

#[tauri::command]
pub fn add_channel() {
    println!("add_channel");
}

#[derive(Debug, Clone)]
pub struct Send {
    pub bus_id: Uuid,
    pub volume: f32,
    pub muted: bool,
}

impl Send {
    pub fn new(bus_id: Uuid, volume: f32, muted: bool) -> Self {
        Self {
            bus_id,
            volume,
            muted,
        }
    }
}
