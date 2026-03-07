use crate::audio::node::NodeInfo;
use crate::audio::{AudioBackend, BackendEvent, Link};
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
// Internal name suffixes that map engine Bus UUIDs to PW bus sink nodes.
// Must match the BUS_NODES constant in backend/pipewire.rs.
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
    /// Maps each Bus UUID to the PW global ID of its corresponding sink node.
    /// Populated reactively when the PW backend reports the bus sink as NodeAdded.
    bus_sink_ids: HashMap<Uuid, u64>,
    /// Maps each Channel UUID to the channel→bus links it owns (one per bus).
    channel_links: HashMap<Uuid, Vec<Link>>,
    /// Maps each Channel UUID to its current input link (physical source → channel sink).
    /// At most one active input link per channel; replacing it destroys the previous one.
    channel_input_links: HashMap<Uuid, Link>,
    /// Maps each Bus UUID to its current output link (bus sink monitor → physical output).
    /// At most one active output link per bus; replacing it destroys the previous one.
    bus_output_links: HashMap<Uuid, Link>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let backend = create_backend();

        let monitor_bus = Bus::new(BUS_SUFFIX_MONITOR.to_string());
        let stream_bus = Bus::new(BUS_SUFFIX_STREAM.to_string());

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
            bus_sink_ids: HashMap::new(),
            channel_links: HashMap::new(),
            channel_input_links: HashMap::new(),
            bus_output_links: HashMap::new(),
        };

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
            bus_sink_ids: HashMap::new(),
            channel_links: HashMap::new(),
            channel_input_links: HashMap::new(),
            bus_output_links: HashMap::new(),
        };

        engine.recreate_virtual_sinks();
        engine.ensure_mic_channel();

        engine
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    /// Recreate the PipeWire virtual sink for every channel loaded from config.
    ///
    /// Channels restored from disk have `virtual_sink.external_id == 0` because
    /// the field is skipped during serialisation. This method creates a fresh PW
    /// node for each channel and writes the new runtime ID back, so that
    /// `wire_all_channels_to_bus` can create valid links once the bus sinks
    /// are discovered.
    fn recreate_virtual_sinks(&mut self) {
        let names: Vec<(Uuid, String)> = self
            .channels
            .values()
            .map(|ch| (ch.id, ch.name.clone()))
            .collect();

        for (id, name) in names {
            match self.backend.create_virtual_sink(&name) {
                Ok(sink) => {
                    if let Some(ch) = self.channels.get_mut(&id) {
                        ch.virtual_sink = sink;
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[engine] failed to recreate sink for channel \
                         '{name}': {e}"
                    );
                    // external_id stays 0; wire_all_channels_to_bus
                    // already skips channels with external_id == 0.
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
                    crate::audio::Sink::new(0)
                });

            let sink_node_id = sink.external_id;
            let mic = Channel::new(
                "mic".to_string(),
                self.default_sends.clone(),
                sink,
            );
            let mic_id = mic.id;
            self.channels.insert(mic_id, mic);
            self.channel_order.push(mic_id);

            if sink_node_id != 0 {
                self.wire_channel_to_buses(mic_id, sink_node_id);
            }
        }
    }

    /// Create PipeWire links from a channel's virtual sink to every bus sink
    /// that already has a known PW node ID.
    fn wire_channel_to_buses(&mut self, channel_id: Uuid, sink_node_id: u64) {
        let bus_ids: Vec<(Uuid, u64)> = self
            .bus_sink_ids
            .iter()
            .map(|(bus_uuid, pw_id)| (*bus_uuid, *pw_id))
            .collect();

        for (bus_uuid, bus_pw_id) in bus_ids {
            match self.backend.create_link(sink_node_id, bus_pw_id) {
                Ok(link_id) => {
                    let link = Link::new(link_id, sink_node_id, bus_pw_id);
                    self.channel_links
                        .entry(channel_id)
                        .or_default()
                        .push(link);
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

    /// Create missing links from every existing channel to a newly discovered
    /// bus sink node.
    fn wire_all_channels_to_bus(&mut self, bus_uuid: Uuid, bus_pw_id: u64) {
        let channel_ids: Vec<(Uuid, u64)> = self
            .channels
            .values()
            .map(|ch| (ch.id, ch.virtual_sink.external_id))
            .collect();

        for (channel_id, sink_node_id) in channel_ids {
            // Skip stub sinks (external_id == 0).
            if sink_node_id == 0 {
                continue;
            }
            // Skip if a link to this bus already exists for this channel.
            let already_linked = self
                .channel_links
                .get(&channel_id)
                .map(|links| links.iter().any(|l| l.input_node_id == bus_pw_id))
                .unwrap_or(false);

            if already_linked {
                continue;
            }

            match self.backend.create_link(sink_node_id, bus_pw_id) {
                Ok(link_id) => {
                    let link = Link::new(link_id, sink_node_id, bus_pw_id);
                    self.channel_links
                        .entry(channel_id)
                        .or_default()
                        .push(link);
                }
                Err(e) => {
                    eprintln!(
                        "[engine] failed to link channel {channel_id} \
                         to new bus {bus_uuid}: {e}"
                    );
                }
            }
        }
    }

    /// Resolve an `amplitude-*` node name to its engine Bus UUID, if any.
    fn bus_uuid_for_node_name(&self, node_name: &str) -> Option<Uuid> {
        let suffix = node_name.strip_prefix("amplitude-")?;
        self.buses.values().find(|b| b.name == suffix).map(|b| b.id)
    }

    // -------------------------------------------------------------------------
    // Public API
    // -------------------------------------------------------------------------

    /// Route a physical input node (microphone, line-in, etc.) into the given
    /// channel's virtual sink. Replaces any previously set input link.
    ///
    /// `input_node_id` is the PipeWire global ID of the source node.
    pub fn set_channel_input(
        &mut self,
        channel_id: Uuid,
        input_node_id: u64,
    ) -> Result<(), String> {
        let sink_node_id = self
            .channels
            .get(&channel_id)
            .map(|ch| ch.virtual_sink.external_id)
            .ok_or_else(|| format!("channel {channel_id} not found"))?;

        // Destroy the previous input link for this channel, if any.
        if let Some(old) = self.channel_input_links.remove(&channel_id) {
            if let Err(e) = self.backend.destroy_link(old.id) {
                eprintln!(
                    "[engine] failed to destroy old input link {}: {e}",
                    old.id
                );
            }
        }

        // A physical source (Audio/Source) connects as:
        //   output_node = source node  →  input_node = channel sink
        let link_id = self.backend.create_link(input_node_id, sink_node_id)?;
        self.channel_input_links.insert(
            channel_id,
            Link::new(link_id, input_node_id, sink_node_id),
        );

        Ok(())
    }

    /// Route the monitor output of a bus's virtual sink to a physical output
    /// device. Replaces any previously set output link for this bus.
    ///
    /// `output_node_id` is the PipeWire global ID of the physical sink node.
    pub fn set_bus_output(
        &mut self,
        bus_id: Uuid,
        output_node_id: u64,
    ) -> Result<(), String> {
        let bus_sink_pw_id =
            *self.bus_sink_ids.get(&bus_id).ok_or_else(|| {
                format!("bus {bus_id} has no known PipeWire sink node yet")
            })?;

        // Destroy the previous output link for this bus, if any.
        if let Some(old) = self.bus_output_links.remove(&bus_id) {
            if let Err(e) = self.backend.destroy_link(old.id) {
                eprintln!(
                    "[engine] failed to destroy old output link {}: {e}",
                    old.id
                );
            }
        }

        // Bus sink monitor → physical output:
        //   output_node = bus sink  →  input_node = physical output device
        let link_id =
            self.backend.create_link(bus_sink_pw_id, output_node_id)?;
        self.bus_output_links
            .insert(bus_id, Link::new(link_id, bus_sink_pw_id, output_node_id));

        Ok(())
    }

    /// Create a virtual sink via the backend, then build and register a channel.
    pub fn add_channel(&mut self, name: String) -> Result<Channel, String> {
        let sink = self.backend.create_virtual_sink(&name)?;
        let sink_node_id = sink.external_id;
        let channel = Channel::new(name, self.default_sends.clone(), sink);
        let id = channel.id;
        self.channels.insert(id, channel.clone());
        if !self.channel_order.contains(&id) {
            self.channel_order.push(id);
        }

        if sink_node_id != 0 {
            self.wire_channel_to_buses(id, sink_node_id);
        }

        Ok(channel)
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

        // Destroy input link first.
        if let Some(link) = self.channel_input_links.remove(&id) {
            if let Err(e) = self.backend.destroy_link(link.id) {
                eprintln!(
                    "[engine] failed to destroy input link {}: {e}",
                    link.id
                );
            }
        }

        // Destroy channel→bus links.
        if let Some(links) = self.channel_links.remove(&id) {
            for link in links {
                if let Err(e) = self.backend.destroy_link(link.id) {
                    eprintln!(
                        "[engine] failed to destroy link {}: {e}",
                        link.id
                    );
                }
            }
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

    /// Drain pending backend events, update caches, and return the events.
    /// When a bus sink node is seen for the first time its ID is stored and
    /// all existing channels are immediately linked to it.
    pub fn poll_events(&mut self) -> Vec<BackendEvent> {
        let events = self.backend.poll_events();

        // Collect bus discoveries first so we can call &mut self methods after.
        let mut new_bus_nodes: Vec<(Uuid, u64)> = Vec::new();

        for event in &events {
            match event {
                BackendEvent::NodeAdded(info) => {
                    self.nodes.insert(info.id, info.clone());

                    // If this is one of our bus sink nodes and we haven't
                    // recorded its PW ID yet, note it for wiring below.
                    if info.is_amplitude_virtual {
                        if let Some(bus_uuid) =
                            self.bus_uuid_for_node_name(&info.name)
                        {
                            if let std::collections::hash_map::Entry::Vacant(
                                e,
                            ) = self.bus_sink_ids.entry(bus_uuid)
                            {
                                e.insert(info.id as u64);
                                new_bus_nodes.push((bus_uuid, info.id as u64));
                            }
                        }
                    }
                }
                BackendEvent::NodeRemoved(id) => {
                    self.nodes.remove(id);
                }
            }
        }

        // Wire all existing channels to any newly discovered bus sinks.
        for (bus_uuid, bus_pw_id) in new_bus_nodes {
            self.wire_all_channels_to_bus(bus_uuid, bus_pw_id);
        }

        events
    }

    /// Returns all currently known nodes sorted by PW global ID.
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
