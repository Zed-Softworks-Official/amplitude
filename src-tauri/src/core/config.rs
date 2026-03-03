use serde::{Deserialize, Serialize};
use crate::core::channels::Channel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub channels: Vec<Channel>
}

impl Config {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
        }
    }
}
