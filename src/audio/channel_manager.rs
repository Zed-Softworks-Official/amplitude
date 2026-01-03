use uuid::Uuid;
use std::collections::HashMap;

use crate::audio::channel::Channel;
use crate::audio::audio_manager::ChannelBus;
use lucide_icons::iced::icon_speaker;

#[derive(Default, Debug, Clone)]
pub struct ChannelManager {
    channels: HashMap<Uuid, Channel>
}

impl ChannelManager {
    pub fn new() -> Self {
        ChannelManager {
            channels: HashMap::new()
        }
    }

    pub fn add_channel(&mut self, name: &str) -> Uuid {
        let channel = Channel::new(name.to_string(), icon_speaker);
        self.channels.insert(channel.id, channel.clone());

        channel.id
    }

    pub fn toggle_mute(
        &mut self,
        uuid: Uuid,
        bus: ChannelBus
    ) {
        match bus {
            ChannelBus::Monitor => {
                if let Some(channel) = self.channels.get_mut(&uuid) {
                    channel.monitor_bus_options.muted = !channel.monitor_bus_options.muted;
                }
            },
            ChannelBus::Stream => {
                if let Some(channel) = self.channels.get_mut(&uuid) {
                    channel.stream_bus_options.muted = !channel.stream_bus_options.muted;
                }
            }
        }
    }

    pub fn update_volume(
        &mut self,
        uuid: Uuid,
        volume: f32,
        bus: ChannelBus
    ) {
        match bus {
            ChannelBus::Monitor => {
                if let Some(channel) = self.channels.get_mut(&uuid) {
                    channel.monitor_bus_options.volume = volume;
                }
            },
            ChannelBus::Stream => {
                if let Some(channel) = self.channels.get_mut(&uuid) {
                    channel.stream_bus_options.volume = volume;
                }
            }
        };
    }

    pub fn get_channels(&self) -> &HashMap<Uuid, Channel> {
        &self.channels
    }
}
