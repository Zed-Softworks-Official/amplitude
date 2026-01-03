use uuid::Uuid;
use std::collections::HashMap;

use crate::audio::{
    channel_manager::ChannelManager,
    bus::Bus,
    channel::Channel
};

pub enum ChannelBus {
    Monitor,
    Stream
}

#[derive(Default, Debug, Clone)]
pub struct AudioManager {
    channel_manager: ChannelManager,
    buses: HashMap<String, Bus>
}

impl AudioManager {
    pub fn new() -> Self {
        let buses = HashMap::from(
            [
                ("Monitor".to_string(), Bus::new("Monitor".to_string())),
                ("Stream".to_string(), Bus::new("Stream".to_string()))
            ]);

        Self {
            channel_manager: ChannelManager::new(),
            buses
        }
    }

    pub fn add_channel(&mut self, name: &str) -> Uuid {
        self.channel_manager.add_channel(name)
    }

    pub fn update_volume(
        &mut self,
        uuid: Uuid,
        volume: f32,
        bus: ChannelBus
    ) {
        self.channel_manager.update_volume(uuid, volume, bus);
    }

    pub fn toggle_mute(
        &mut self,
        uuid: Uuid,
        bus: ChannelBus
    ) {
        self.channel_manager.toggle_mute(uuid, bus);
    }

    pub fn get_channels(&self) -> &HashMap<Uuid, Channel> {
        self.channel_manager.get_channels()
    }
}
