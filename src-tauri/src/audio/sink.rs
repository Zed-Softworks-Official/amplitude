use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sink {
    pub id: Uuid,
    /// Platform-specific node identifier.
    /// On PipeWire this is the global node ID (u32) cast to u64.
    /// On CoreAudio this will be an AudioDeviceID (u32) cast to u64.
    /// Using u64 gives headroom for both platforms.
    pub external_id: u64,
}

impl Sink {
    pub fn new(external_id: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            external_id,
        }
    }
}
