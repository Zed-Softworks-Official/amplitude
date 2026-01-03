use crate::audio::{
    channel_manager::ChannelManager,
    bus::Bus,
};

pub enum ChannelBus {
    Monitor,
    Stream
}

struct AudioManager {
    channel_manager: ChannelManager,
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            channel_manager: ChannelManager::new(),
        }
    }
}
