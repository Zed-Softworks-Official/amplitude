use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bus {
    pub id: Uuid,
    pub name: String,
    pub volume: f32,
    pub muted: bool,
}

impl Bus {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            volume: 0.8,
            muted: false,
        }
    }
}
