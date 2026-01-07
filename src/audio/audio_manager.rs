use uuid::Uuid;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::audio::{
    channel_manager::ChannelManager,
    bus::Bus,
    channel::Channel
};

use crate::audio::backend::{
    AudioBackend,
    AudioEvent,
    AudioNode
};
use crate::core::config::Config;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum ChannelBus {
    Monitor,
    Stream
}

pub struct AudioManager {
    audio_backend: Box<dyn AudioBackend>,
    channel_manager: ChannelManager,
    buses: HashMap<ChannelBus, Bus>
}

impl AudioManager {
    pub fn new(config: Config, audio_backend: Box<dyn AudioBackend>) -> Self {
        let buses = HashMap::from(
            [
                (ChannelBus::Stream, Bus::new("Stream".to_string())),
                (ChannelBus::Monitor, Bus::new("Monitor".to_string())),
            ]);

        let channel_manager = ChannelManager::new(config);

        Self {
            channel_manager,
            buses,
            audio_backend
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

    pub fn get_nodes(&self) -> Arc<Mutex<HashMap<u32, AudioNode>>> {
        self.audio_backend.get_nodes()
    }

    pub fn get_channels(&self) -> &HashMap<Uuid, Channel> {
        self.channel_manager.get_channels()
    }

    pub fn get_busses(&self) -> &HashMap<ChannelBus, Bus> {
        &self.buses
    }

    pub fn get_event_receiver(&self) -> Arc<Mutex<mpsc::Receiver<AudioEvent>>> {
        self.audio_backend.get_event_receiver()
    }

    pub fn process_events(&self) {
        self.audio_backend.process_events();
    }
}
