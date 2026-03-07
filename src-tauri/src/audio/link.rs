use serde::{Deserialize, Serialize};

/// A live PipeWire link between the monitor ports of one node and the
/// input ports of another. The `id` is the PipeWire global object ID —
/// needed to destroy the link later via `core.destroy_object`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// PipeWire global object ID for this link.
    pub id: u64,
    /// Global ID of the node whose output/monitor ports are the source.
    pub output_node_id: u64,
    /// Global ID of the node whose input ports receive the audio.
    pub input_node_id: u64,
}

impl Link {
    pub fn new(id: u64, output_node_id: u64, input_node_id: u64) -> Self {
        Self {
            id,
            output_node_id,
            input_node_id,
        }
    }
}
