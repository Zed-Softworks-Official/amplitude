use crate::audio::{Link, Sink};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bus {
    pub id: Uuid,
    pub name: String,
    pub volume: f32,
    pub muted: bool,
    /// Runtime-only: the live virtual sink node for this bus.
    /// Skipped during serialisation — the node ID is ephemeral and is
    /// recreated fresh on every startup via `AudioBackend::create_bus_sink`.
    #[serde(skip)]
    pub sink: Sink,
    /// Runtime-only: current link from this bus's sink to a physical output
    /// device. At most one active output link per bus; replaced when the user
    /// selects a different output.
    #[serde(skip)]
    pub output_link: Option<Link>,
}

impl Bus {
    pub fn new(name: String, sink: Sink) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            volume: 0.8,
            muted: false,
            sink,
            output_link: None,
        }
    }
}
