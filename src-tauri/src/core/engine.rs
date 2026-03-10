use crate::audio::node::NodeInfo;
use crate::audio::{AudioBackend, BackendEvent, Link, Sink};
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

// ---------------------------------------------------------------------------
// Internal name suffixes used when creating bus sink nodes.
// These determine the `node.name` in PipeWire ("amplitude-monitor", etc.)
// and are matched back by `bus_uuid_for_node_name` for observability.
// ---------------------------------------------------------------------------

const BUS_SUFFIX_MONITOR: &str = "monitor";
const BUS_SUFFIX_STREAM: &str = "stream";

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
    /// Live cache of PipeWire nodes keyed by their PW global ID.
    pub nodes: HashMap<u32, NodeInfo>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let mut backend = create_backend();

        // Create bus sinks synchronously — blocks until the platform confirms
        // each node is live. This guarantees bus IDs are known before any
        // channel is created, eliminating the startup race condition.
        let monitor_sink = backend
            .create_bus_sink(BUS_SUFFIX_MONITOR)
            .unwrap_or_else(|e| {
                eprintln!("[engine] failed to create monitor bus sink: {e}");
                Sink::new(0)
            });
        let stream_sink = backend
            .create_bus_sink(BUS_SUFFIX_STREAM)
            .unwrap_or_else(|e| {
                eprintln!("[engine] failed to create stream bus sink: {e}");
                Sink::new(0)
            });

        let monitor_bus =
            Bus::new(BUS_SUFFIX_MONITOR.to_string(), monitor_sink);
        let stream_bus = Bus::new(BUS_SUFFIX_STREAM.to_string(), stream_sink);

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
            nodes: HashMap::new(),
        };

        engine.ensure_mic_channel();

        engine
    }

    /// Load state from a saved config, then guarantee the mic channel exists.
    pub fn from_config(config: Config) -> Self {
        let mut backend = create_backend();

        // Recreate each bus's virtual sink synchronously before touching
        // channels. This mirrors Bus::new taking a Sink: the persisted Bus
        // carries no live node ID, so we create a fresh one here.
        let mut buses = HashMap::new();
        for (_id, mut bus) in config.buses {
            bus.sink = backend.create_bus_sink(&bus.name).unwrap_or_else(|e| {
                eprintln!(
                    "[engine] failed to recreate sink for bus '{}': {e}",
                    bus.name
                );
                Sink::new(0)
            });
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

        for channel in config.channels.values() {
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
            nodes: HashMap::new(),
        };

        engine.recreate_virtual_sinks();
        engine.ensure_mic_channel();

        engine
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    /// Recreate the virtual sink for every channel loaded from config and wire
    /// each channel to all buses whose sinks are live.
    ///
    /// Channels restored from disk have `virtual_sink.external_id == 0` because
    /// the field is skipped during serialisation. This method creates a fresh
    /// node for each channel, writes the new runtime ID back, then wires it to
    /// every bus sink (which are guaranteed live at this point).
    fn recreate_virtual_sinks(&mut self) {
        let ids: Vec<Uuid> = self.channels.keys().copied().collect();

        for id in ids {
            let name = self.channels[&id].name.clone();
            match self.backend.create_virtual_sink(&name) {
                Ok(sink) => {
                    self.channels.get_mut(&id).unwrap().virtual_sink = sink;
                    self.wire_channel_to_buses(id);
                }
                Err(e) => {
                    eprintln!(
                        "[engine] failed to recreate sink for channel \
                         '{name}': {e}"
                    );
                    // external_id stays 0; wire_channel_to_buses skips
                    // channels with external_id == 0.
                }
            }
        }
    }

    /// Guarantee a mic channel always exists.
    fn ensure_mic_channel(&mut self) {
        let has_mic = self
            .channels
            .values()
            .any(|ch| ch.name.to_lowercase() == "mic");

        if !has_mic {
            let sink =
                self.backend.create_virtual_sink("mic").unwrap_or_else(|e| {
                    eprintln!("[engine] failed to create mic sink: {e}");
                    Sink::new(0)
                });

            let mic = Channel::new(
                "mic".to_string(),
                self.default_sends.clone(),
                sink,
            );
            let mic_id = mic.id;
            self.channels.insert(mic_id, mic);
            self.channel_order.push(mic_id);

            self.wire_channel_to_buses(mic_id);
        }
    }

    /// Create PipeWire links from `channel_id`'s virtual sink to every bus
    /// sink that has a live node ID. Stores the resulting links on the channel.
    fn wire_channel_to_buses(&mut self, channel_id: Uuid) {
        let sink_node_id = match self.channels.get(&channel_id) {
            Some(ch) if ch.virtual_sink.external_id != 0 => {
                ch.virtual_sink.external_id
            }
            _ => return,
        };

        // Collect (bus_uuid, bus_pw_id) for all live bus sinks.
        let bus_pairs: Vec<(Uuid, u64)> = self
            .buses
            .values()
            .filter(|b| b.sink.external_id != 0)
            .map(|b| (b.id, b.sink.external_id))
            .collect();

        for (bus_uuid, bus_pw_id) in bus_pairs {
            match self.backend.create_link(sink_node_id, bus_pw_id) {
                Ok(link_id) => {
                    if let Some(ch) = self.channels.get_mut(&channel_id) {
                        ch.bus_links.push(Link::new(
                            link_id,
                            sink_node_id,
                            bus_pw_id,
                        ));
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[engine] failed to link channel {channel_id} \
                         to bus {bus_uuid}: {e}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Public API
    // -------------------------------------------------------------------------

    /// Route a physical input node (microphone, line-in, etc.) into the given
    /// channel's virtual sink. Replaces any previously set input link.
    ///
    /// `input_node_id` is the platform-specific global ID of the source node.
    pub fn set_channel_input(
        &mut self,
        channel_id: Uuid,
        input_node_id: u64,
    ) -> Result<(), String> {
        let channel = self
            .channels
            .get_mut(&channel_id)
            .ok_or_else(|| format!("channel {channel_id} not found"))?;

        let sink_node_id = channel.virtual_sink.external_id;

        // Destroy the previous input link for this channel, if any.
        if let Some(old) = channel.input_link.take() {
            if let Err(e) = self.backend.destroy_link(old.id) {
                eprintln!(
                    "[engine] failed to destroy old input link {}: {e}",
                    old.id
                );
            }
        }

        // Physical source → channel sink.
        let link_id = self.backend.create_link(input_node_id, sink_node_id)?;
        let channel = self.channels.get_mut(&channel_id).unwrap();
        channel.input_link =
            Some(Link::new(link_id, input_node_id, sink_node_id));

        Ok(())
    }

    /// Route the monitor output of a bus's virtual sink to a physical output
    /// device. Replaces any previously set output link for this bus.
    ///
    /// `output_node_id` is the platform-specific global ID of the physical
    /// sink node.
    pub fn set_bus_output(
        &mut self,
        bus_id: Uuid,
        output_node_id: u64,
    ) -> Result<(), String> {
        let bus = self
            .buses
            .get_mut(&bus_id)
            .ok_or_else(|| format!("bus {bus_id} not found"))?;

        let bus_sink_pw_id = bus.sink.external_id;
        if bus_sink_pw_id == 0 {
            return Err(format!("bus {bus_id} has no live sink node"));
        }

        // Destroy the previous output link for this bus, if any.
        if let Some(old) = bus.output_link.take() {
            if let Err(e) = self.backend.destroy_link(old.id) {
                eprintln!(
                    "[engine] failed to destroy old output link {}: {e}",
                    old.id
                );
            }
        }

        // Bus sink monitor → physical output.
        let link_id =
            self.backend.create_link(bus_sink_pw_id, output_node_id)?;
        let bus = self.buses.get_mut(&bus_id).unwrap();
        bus.output_link =
            Some(Link::new(link_id, bus_sink_pw_id, output_node_id));

        Ok(())
    }

    /// Create a virtual sink via the backend, wire it to all buses, then
    /// build and register a new channel.
    pub fn add_channel(&mut self, name: String) -> Result<Channel, String> {
        let sink = self.backend.create_virtual_sink(&name)?;
        let channel = Channel::new(name, self.default_sends.clone(), sink);
        let id = channel.id;
        self.channels.insert(id, channel.clone());
        if !self.channel_order.contains(&id) {
            self.channel_order.push(id);
        }

        self.wire_channel_to_buses(id);

        Ok(self.channels[&id].clone())
    }

    /// Destroy all links for a channel, then destroy its sink, then remove it.
    /// The mic channel cannot be deleted.
    pub fn delete_channel(&mut self, id: Uuid) -> Result<(), String> {
        if let Some(ch) = self.channels.get(&id) {
            if ch.name.to_lowercase() == "mic" {
                return Err("cannot delete the mic channel".to_string());
            }
        } else {
            return Err(format!("channel {id} not found"));
        }

        let mut channel = self.channels.remove(&id).unwrap();

        // Destroy input link.
        if let Some(link) = channel.input_link.take() {
            if let Err(e) = self.backend.destroy_link(link.id) {
                eprintln!(
                    "[engine] failed to destroy input link {}: {e}",
                    link.id
                );
            }
        }

        // Destroy channel→bus links.
        for link in channel.bus_links.drain(..) {
            if let Err(e) = self.backend.destroy_link(link.id) {
                eprintln!(
                    "[engine] failed to destroy bus link {}: {e}",
                    link.id
                );
            }
        }

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

    /// Drain pending backend events and update the node cache.
    pub fn poll_events(&mut self) -> Vec<BackendEvent> {
        let events = self.backend.poll_events();

        for event in &events {
            match event {
                BackendEvent::NodeAdded(info) => {
                    self.nodes.insert(info.id, info.clone());
                }
                BackendEvent::NodeRemoved(id) => {
                    self.nodes.remove(id);
                }
            }
        }

        events
    }

    /// Returns all currently known nodes sorted by platform global ID.
    pub fn get_nodes(&self) -> Vec<NodeInfo> {
        let mut nodes: Vec<NodeInfo> = self.nodes.values().cloned().collect();
        nodes.sort_by_key(|n| n.id);
        nodes
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

    /// Minimal payload for persistence.
    pub fn to_save_payload(&self) -> SavePayload {
        SavePayload {
            channels: self.channels.clone(),
            buses: self.buses.clone(),
            channel_order: self.channel_order.clone(),
        }
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}
