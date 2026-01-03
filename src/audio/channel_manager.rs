use uuid::Uuid;
use std::collections::HashMap;

use crate::audio::channel::Channel;
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

    pub fn get_channels(&self) -> &HashMap<Uuid, Channel> {
        &self.channels
    }
}
