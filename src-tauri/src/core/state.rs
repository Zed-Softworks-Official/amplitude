use crate::core::{
    bus::Bus,
    channels::{Channel, Send},
    config::Config,
};
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

/// Payload emitted on the "appstate-changed" Tauri event.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatePayload {
    pub channels: Vec<Channel>,
    pub buses: Vec<Bus>,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub channels: HashMap<Uuid, Channel>,
    pub buses: HashMap<Uuid, Bus>,
    pub default_sends: Vec<Send>,
    pub channel_order: Vec<Uuid>,
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

        let mic_id = mic_channel.id;

        Self {
            channels: HashMap::from([(mic_channel.id, mic_channel)]),
            buses: HashMap::from([
                (monitor_bus.id, monitor_bus),
                (stream_bus.id, stream_bus),
            ]),
            default_sends,
            channel_order: vec![mic_id],
        }
    }

    pub fn from_config(config: Config) -> Self {
        let mut state = Self::default();
        state.channels.clear();
        state.default_sends.clear();
        state.channel_order.clear();

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

        // Add channels from config in persisted order
        for id in &config.channel_order {
            if let Some(channel) = config.channels.get(id) {
                state.channels.insert(channel.id, channel.clone());
                state.channel_order.push(*id);
            }
        }

        // Any channels not in channel_order (shouldn't happen, but be safe)
        for (_id, channel) in &config.channels {
            if !state.channel_order.contains(&channel.id) {
                state.channel_order.push(channel.id);
                state.channels.insert(channel.id, channel.clone());
            }
        }

        if state.channels.is_empty() {
            let mic = Channel::new("mic".to_string(), state.default_sends.clone());
            let mic_id = mic.id;
            state.channels.insert(mic_id, mic);
            state.channel_order.push(mic_id);
        }

        state
    }

    pub fn add_channel(&mut self, channel: Channel) {
        let id = channel.id;
        self.channels.insert(id, channel);
        if !self.channel_order.contains(&id) {
            self.channel_order.push(id);
        }
    }

    /// Returns channels in their persisted order.
    pub fn ordered_channels(&self) -> Vec<Channel> {
        self.channel_order
            .iter()
            .filter_map(|id| self.channels.get(id).cloned())
            .collect()
    }

    /// Builds the event payload representing current state.
    pub fn to_payload(&self) -> AppStatePayload {
        AppStatePayload {
            channels: self.ordered_channels(),
            buses: self.buses.values().cloned().collect(),
        }
    }
}
