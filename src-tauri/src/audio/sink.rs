use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sink {
    pub id: Uuid,
    pub external_id: String,
}

impl Sink {
    pub fn new(external_id: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            external_id,
        }
    }
}
