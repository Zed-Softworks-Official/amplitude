use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: Uuid,
    pub name: String,
    pub sends: Vec<Send>,
    pub connections: Vec<Connection>,
}

impl Channel {
    pub fn new(name: String, sends: Vec<Send>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            sends,
            connections: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub process_id: u32,
    pub process_name: String,
}

impl Connection {
    pub fn new(process_id: u32, process_name: String) -> Self {
        Self {
            process_id,
            process_name,
        }
    }
}
