use crate::audio::{Link, Sink};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: Uuid,
    pub name: String,
    pub sends: Vec<Send>,
    pub connections: Vec<Connection>,
    /// Runtime-only: the live PipeWire sink node for this channel.
    /// Skipped during serialisation — the node ID is ephemeral and is
    /// recreated fresh on every startup.
    #[serde(skip)]
    pub virtual_sink: Sink,
    /// Runtime-only: current link from a physical source into this channel's
    /// virtual sink. At most one active input link per channel; replaced when
    /// the user selects a different input device.
    #[serde(skip)]
    pub input_link: Option<Link>,
    /// Runtime-only: one link per bus this channel feeds into.
    /// Populated by `wire_channel_to_buses` after the channel sink is live.
    #[serde(skip)]
    pub bus_links: Vec<Link>,
}

impl Channel {
    pub fn new(name: String, sends: Vec<Send>, virtual_sink: Sink) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            sends,
            connections: Vec::new(),
            virtual_sink,
            input_link: None,
            bus_links: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
