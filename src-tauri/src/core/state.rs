use crate::core::{
    bus::Bus,
    channels::{Channel, Send},
    config::Config,
};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AppState {
    pub channels: HashMap<Uuid, Channel>,
    pub buses: HashMap<Uuid, Bus>,
    pub default_sends: Vec<Send>,
}

impl AppState {
    pub fn default() -> Self {
        let monitor_bus = Bus::new("monitor".to_string());
        let stream_bus = Bus::new("stream".to_string());

        let default_sends = vec![
            Send::new(monitor_bus.id, monitor_bus.volume, monitor_bus.muted),
            Send::new(stream_bus.id, stream_bus.volume, stream_bus.muted),
        ];

        let mic_channel =
            Channel::new("mic".to_string(), default_sends.clone());

        Self {
            channels: HashMap::from([(mic_channel.id, mic_channel)]),
            buses: HashMap::from([
                (monitor_bus.id, monitor_bus),
                (stream_bus.id, stream_bus),
            ]),
            default_sends,
        }
    }

    pub fn from_config(config: Config) -> Self {
        let mut state = Self::default();
        state.channels.clear();
        state.default_sends.clear();

        // Restore persisted buses first, replacing defaults
        state.buses.clear();
        for (_id, bus) in config.buses {
            state.buses.insert(bus.id, bus);
        }

        // Rebuild default sends from the restored buses
        let mut default_sends = Vec::new();
        for bus in state.buses.values() {
            default_sends.push(Send::new(bus.id, bus.volume, bus.muted));
        }
        state.default_sends = default_sends;

        // Add channels from config
        for (_id, channel) in config.channels {
            state.add_channel(channel);
        }

        if state.channels.is_empty() {
            state.add_channel(Channel::new(
                "mic".to_string(),
                state.default_sends.clone(),
            ));
        }

        state
    }

    pub fn add_channel(&mut self, channel: Channel) {
        self.channels.insert(channel.id, channel);
    }
}
