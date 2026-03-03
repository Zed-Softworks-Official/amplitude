use crate::core::{
    bus::Bus,
    channels::{Channel, Send},
};
use std::collections::HashMap;
use uuid::Uuid;

pub struct AppState {
    pub channels: HashMap<Uuid, Channel>,
    pub busses: HashMap<Uuid, Bus>,
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
            busses: HashMap::from([
                (monitor_bus.id, monitor_bus),
                (stream_bus.id, stream_bus),
            ]),
            default_sends,
        }
    }

    pub fn add_channel(&mut self, channel: Channel) {
        self.channels.insert(channel.id, channel);
    }
}
