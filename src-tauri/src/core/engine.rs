use crate::audio::AudioBackend;
use crate::core::{
    bus::Bus,
    channels::{Channel, Connection, Send},
    config::{Config, SavePayload},
};
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(target_os = "linux")]
use crate::backend::pipewire::create_backend;

#[cfg(target_os = "macos")]
use crate::backend::coreaudio::create_backend;

/// Payload emitted on the "appstate-changed" Tauri event.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatePayload {
    pub channels: Vec<Channel>,
    pub buses: Vec<Bus>,
}

pub struct AudioEngine {
    backend: Box<dyn AudioBackend>,
    pub channels: HashMap<Uuid, Channel>,
    pub buses: HashMap<Uuid, Bus>,
    pub default_sends: Vec<Send>,
    pub channel_order: Vec<Uuid>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let backend = create_backend();

        let monitor_bus = Bus::new("monitor".to_string());
        let stream_bus = Bus::new("stream".to_string());

        let default_sends = vec![
            Send::new(monitor_bus.id, monitor_bus.volume, monitor_bus.muted),
            Send::new(stream_bus.id, stream_bus.volume, stream_bus.muted),
        ];

        let buses = HashMap::from([
            (monitor_bus.id, monitor_bus),
            (stream_bus.id, stream_bus),
        ]);

        let mut engine = Self {
            backend,
            channels: HashMap::new(),
            buses,
            default_sends,
            channel_order: Vec::new(),
        };

        // Mic channel is always required.
        engine.ensure_mic_channel();

        engine
    }

    /// Load state from a saved config, then guarantee the mic channel exists.
    pub fn from_config(config: Config) -> Self {
        let backend = create_backend();

        let mut buses = HashMap::new();
        for (_id, bus) in config.buses {
            buses.insert(bus.id, bus);
        }

        let default_sends: Vec<Send> = buses
            .values()
            .map(|bus| Send::new(bus.id, bus.volume, bus.muted))
            .collect();

        let mut channels = HashMap::new();
        let mut channel_order: Vec<Uuid> = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for id in &config.channel_order {
            if seen.insert(*id) {
                if let Some(channel) = config.channels.get(id) {
                    channels.insert(channel.id, channel.clone());
                    channel_order.push(*id);
                }
            }
        }

        // Any channels in the map not covered by channel_order
        for (_id, channel) in &config.channels {
            if !channel_order.contains(&channel.id) {
                channel_order.push(channel.id);
                channels.insert(channel.id, channel.clone());
            }
        }

        let mut engine = Self {
            backend,
            channels,
            buses,
            default_sends,
            channel_order,
        };

        engine.ensure_mic_channel();

        engine
    }

    /// Guarantee a mic channel always exists. If one is absent, create it via the backend.
    fn ensure_mic_channel(&mut self) {
        let has_mic = self
            .channels
            .values()
            .any(|ch| ch.name.to_lowercase() == "mic");

        if !has_mic {
            let sink =
                self.backend.create_virtual_sink("mic").unwrap_or_else(|_| {
                    crate::audio::Sink::new("mic:stub".to_string())
                });

            let mic = Channel::new(
                "mic".to_string(),
                self.default_sends.clone(),
                sink,
            );
            let mic_id = mic.id;
            self.channels.insert(mic_id, mic);
            self.channel_order.push(mic_id);
        }
    }

    /// Create a virtual sink via the backend, then build and register a channel.
    pub fn add_channel(&mut self, name: String) -> Result<Channel, String> {
        let sink = self.backend.create_virtual_sink(&name)?;
        let channel = Channel::new(name, self.default_sends.clone(), sink);
        let id = channel.id;
        self.channels.insert(id, channel.clone());
        if !self.channel_order.contains(&id) {
            self.channel_order.push(id);
        }
        Ok(channel)
    }

    /// Destroy the channel's virtual sink via the backend, then remove the channel.
    /// The mic channel cannot be deleted.
    pub fn delete_channel(&mut self, id: Uuid) -> Result<(), String> {
        if let Some(ch) = self.channels.get(&id) {
            if ch.name.to_lowercase() == "mic" {
                return Err("cannot delete the mic channel".to_string());
            }
        } else {
            return Err(format!("channel {id} not found"));
        }

        let channel = self.channels.remove(&id).unwrap();
        self.backend.destroy_virtual_sink(&channel.virtual_sink)?;
        self.channel_order.retain(|oid| *oid != id);

        Ok(())
    }

    pub fn reorder_channels(&mut self, order: Vec<Uuid>) {
        let old_order = self.channel_order.clone();
        let mut seen = std::collections::HashSet::new();

        self.channel_order = order
            .into_iter()
            .filter(|id| self.channels.contains_key(id) && seen.insert(*id))
            .collect();

        let mut extra: Vec<Uuid> = self
            .channels
            .keys()
            .filter(|id| !seen.contains(*id) && !old_order.contains(id))
            .cloned()
            .collect();
        extra.sort();

        for id in old_order.iter().chain(extra.iter()) {
            if self.channels.contains_key(id) && seen.insert(*id) {
                self.channel_order.push(*id);
            }
        }
    }

    pub fn update_channel_send(
        &mut self,
        channel_id: Uuid,
        bus_id: Uuid,
        volume: Option<f32>,
        muted: Option<bool>,
    ) -> Result<(), String> {
        let channel = self
            .channels
            .get_mut(&channel_id)
            .ok_or_else(|| format!("channel {channel_id} not found"))?;

        let send = channel
            .sends
            .iter_mut()
            .find(|s| s.bus_id == bus_id)
            .ok_or_else(|| {
                format!(
                    "send to bus {bus_id} not found on channel {channel_id}"
                )
            })?;

        if let Some(v) = volume {
            send.volume = v.clamp(0.0, 1.0);
        }
        if let Some(m) = muted {
            send.muted = m;
        }

        Ok(())
    }

    pub fn update_channel_connections(
        &mut self,
        channel_id: Uuid,
        process_names: Vec<String>,
    ) -> Result<(), String> {
        let channel = self
            .channels
            .get_mut(&channel_id)
            .ok_or_else(|| format!("channel {channel_id} not found"))?;

        channel.connections = process_names
            .into_iter()
            .enumerate()
            .map(|(i, name)| Connection::new(i as u32, name))
            .collect();

        Ok(())
    }

    pub fn update_bus(
        &mut self,
        bus_id: Uuid,
        volume: Option<f32>,
        muted: Option<bool>,
    ) -> Result<(), String> {
        let bus = self
            .buses
            .get_mut(&bus_id)
            .ok_or_else(|| format!("bus {bus_id} not found"))?;

        if let Some(v) = volume {
            bus.volume = v.clamp(0.0, 1.0);
        }
        if let Some(m) = muted {
            bus.muted = m;
        }

        Ok(())
    }

    /// Returns channels in their persisted order.
    pub fn ordered_channels(&self) -> Vec<Channel> {
        let mut seen = std::collections::HashSet::new();
        self.channel_order
            .iter()
            .filter_map(|id| {
                if seen.insert(*id) {
                    self.channels.get(id).cloned()
                } else {
                    None
                }
            })
            .collect()
    }

    /// Payload for the "appstate-changed" Tauri event.
    pub fn to_payload(&self) -> AppStatePayload {
        AppStatePayload {
            channels: self.ordered_channels(),
            buses: self.buses.values().cloned().collect(),
        }
    }

    /// Minimal payload for persistence. Sinks are included via Channel.
    pub fn to_save_payload(&self) -> SavePayload {
        SavePayload {
            channels: self.channels.clone(),
            buses: self.buses.clone(),
            channel_order: self.channel_order.clone(),
        }
    }
}
