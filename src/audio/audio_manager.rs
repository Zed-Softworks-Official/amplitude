use uuid::Uuid;
use std::collections::HashMap;

use crate::audio::{
    channel_manager::ChannelManager,
    bus::Bus,
    channel::Channel
};

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum ChannelBus {
    Monitor,
    Stream
}

#[derive(Default, Debug, Clone)]
pub struct AudioManager {
    channel_manager: ChannelManager,
    buses: HashMap<ChannelBus, Bus>
}

impl AudioManager {
    pub fn new() -> Self {
        let buses = HashMap::from(
            [
                (ChannelBus::Monitor, Bus::new("Monitor".to_string())),
                (ChannelBus::Stream, Bus::new("Stream".to_string()))
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

    pub fn get_busses(&self) -> &HashMap<ChannelBus, Bus> {
        &self.buses
    }
}
