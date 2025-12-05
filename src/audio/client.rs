use spdlog::prelude::*;

use pipewire as pw;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use tokio::sync::mpsc;

use super::events::{AudioEvent, LinkInfo, NodeInfo, NodeType, PortDirection, PortInfo, VirtualSinkInfo};

/// Names for our virtual sinks
pub const MONITOR_SINK_NAME: &str = "amplitude-monitor";
pub const STREAM_SINK_NAME: &str = "amplitude-stream";
/// Prefix for channel sink names
pub const CHANNEL_SINK_PREFIX: &str = "amplitude-channel-";

#[derive(Debug, Clone)]
pub enum PipeWireCommand {
    CreateLink { output_port: u32, input_port: u32 },
    DestroyLink { link_id: u32 },
    /// Destroy all links originating from a node's output ports
    DestroyLinksFromNode { node_id: u32 },
    CreateVirtualSink { name: String, description: String },
    CheckVirtualSinks,
    /// Route an application's output to a virtual sink
    RouteAppToSink { app_node_id: u32, sink_node_id: u32 },
    /// Connect virtual sink monitor to the default output device
    ConnectSinkToOutput { sink_node_id: u32 },
    /// Connect a channel sink to both monitor and stream sinks
    ConnectChannelToOutputs {
        channel_sink_id: u32,
        monitor_sink_id: u32,
        stream_sink_id: u32,
    },
    /// Find and store the default output device
    FindDefaultOutput,
    /// Set volume on a channel sink (0.0 to 1.0)
    SetChannelVolume { channel_name: String, volume: f32 },
    /// Mute/unmute a channel sink
    SetChannelMuted { channel_name: String, muted: bool },
    /// Route an application to a channel sink by name
    RouteAppToChannelByName { app_node_id: u32, channel_name: String },
    /// Route an application to both monitor and stream virtual sinks
    RouteAppToVirtualSinks { app_node_id: u32 },
    Shutdown,
}

pub struct PipeWireClient {
    command_tx: std::sync::mpsc::Sender<PipeWireCommand>,
    event_rx: mpsc::UnboundedReceiver<AudioEvent>,
    thread_handle: Option<JoinHandle<()>>
}

/// A handle for sending commands to PipeWire without needing a lock on AudioManager
/// Can be cloned and shared freely
#[derive(Clone)]
pub struct AudioCommandSender {
    command_tx: std::sync::mpsc::Sender<PipeWireCommand>,
}

impl AudioCommandSender {
    /// Route an application to a channel sink by name
    pub fn route_app_to_channel(&self, app_node_id: u32, channel_name: &str) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::RouteAppToChannelByName {
                app_node_id,
                channel_name: channel_name.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("Failed to send route command: {}", e))
    }

    /// Route an application to both monitor and stream sinks
    pub fn route_app_to_virtual_sinks(&self, app_node_id: u32) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::RouteAppToVirtualSinks { app_node_id })
            .map_err(|e| anyhow::anyhow!("Failed to send route command: {}", e))
    }

    /// Create a virtual sink for a channel
    pub fn create_channel_sink(&self, channel_name: &str) -> anyhow::Result<()> {
        let sink_name = format!("{}{}", CHANNEL_SINK_PREFIX, channel_name);
        let description = format!("Amplitude Channel: {}", channel_name);
        self.command_tx
            .send(PipeWireCommand::CreateVirtualSink { name: sink_name, description })
            .map_err(|e| anyhow::anyhow!("Failed to send create sink command: {}", e))
    }
}

impl PipeWireClient {
    pub async fn new() -> anyhow::Result<Self> {
        // Tokio channel for events (Pipewire -> App)
        let (event_tx, event_rx) = mpsc::unbounded_channel::<AudioEvent>();

        // Std channel for commands (App -> Pipewire)
        // We use std here because Pipewire thread isn't async
        let (command_tx, command_rx) = std::sync::mpsc::channel::<PipeWireCommand>();

        // Spawn pipewire in a dedicated OS thread
        // Pipewire has its own event loop that's not compatible with tokio
        let thread_handle = thread::Builder::new()
            .name("pipewire-main".into())
            .spawn(move || {
                if let Err(e) = run_pipewire_thread(command_rx, event_tx) {
                    error!("PipeWire thread error: {}", e);
                }
            })?;

        Ok(Self {
            command_tx,
            event_rx,
            thread_handle: Some(thread_handle)
        })
    }

    /// Get a command sender that can be used independently of the AudioManager lock
    pub fn command_sender(&self) -> AudioCommandSender {
        AudioCommandSender {
            command_tx: self.command_tx.clone(),
        }
    }

    // Recieve the next event (async)
    pub async fn recv_event(&mut self) -> Option<AudioEvent> {
        self.event_rx.recv().await
    }

    // Try to receive an event without waiting
    pub fn try_recv_event(&mut self) -> Option<AudioEvent> {
        self.event_rx.try_recv().ok()
    }

    // Create a link between two ports
    pub fn create_link(&self, output_port: u32, input_port: u32) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::CreateLink {
                output_port,
                input_port
            })
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("")
            })
    }

    // destroy link
    pub fn destroy_link(&self, link_id: u32) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::DestroyLink { link_id })
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("")
            })
    }

    /// Destroy all links originating from a node's output ports
    pub fn destroy_links_from_node(&self, node_id: u32) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::DestroyLinksFromNode { node_id })
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("Failed to send destroy links from node command")
            })
    }

    /// Create a virtual sink with the given name and description
    pub fn create_virtual_sink(&self, name: String, description: String) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::CreateVirtualSink { name, description })
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("Failed to send create virtual sink command")
            })
    }

    /// Request a check for existing virtual sinks
    pub fn check_virtual_sinks(&self) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::CheckVirtualSinks)
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("Failed to send check virtual sinks command")
            })
    }

    /// Route an application's audio output to a virtual sink
    pub fn route_app_to_sink(&self, app_node_id: u32, sink_node_id: u32) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::RouteAppToSink {
                app_node_id,
                sink_node_id,
            })
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("Failed to send route app to sink command")
            })
    }

    /// Connect a virtual sink's monitor output to the default audio output
    pub fn connect_sink_to_output(&self, sink_node_id: u32) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::ConnectSinkToOutput { sink_node_id })
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("Failed to send connect sink to output command")
            })
    }

    /// Connect a channel sink to both monitor and stream sinks
    pub fn connect_channel_to_outputs(
        &self,
        channel_sink_id: u32,
        monitor_sink_id: u32,
        stream_sink_id: u32,
    ) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::ConnectChannelToOutputs {
                channel_sink_id,
                monitor_sink_id,
                stream_sink_id,
            })
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("Failed to send connect channel to outputs command")
            })
    }

    /// Find and store the default output device
    pub fn find_default_output(&self) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::FindDefaultOutput)
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("Failed to send find default output command")
            })
    }

    /// Set volume on a channel sink (0.0 to 1.0)
    pub fn set_channel_volume(&self, channel_name: String, volume: f32) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::SetChannelVolume { channel_name, volume })
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("Failed to send set channel volume command")
            })
    }

    /// Mute/unmute a channel sink
    pub fn set_channel_muted(&self, channel_name: String, muted: bool) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::SetChannelMuted { channel_name, muted })
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("Failed to send set channel muted command")
            })
    }

    /// Route an application to a channel sink by name (non-blocking)
    pub fn route_app_to_channel_by_name(&self, app_node_id: u32, channel_name: String) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::RouteAppToChannelByName { app_node_id, channel_name })
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("Failed to send route app to channel command")
            })
    }

    /// Route an application to both monitor and stream virtual sinks (non-blocking)
    pub fn route_app_to_virtual_sinks(&self, app_node_id: u32) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::RouteAppToVirtualSinks { app_node_id })
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("Failed to send route app to virtual sinks command")
            })
    }

    // Shutdown the pipewire connection
    pub async fn shutdown(&mut self) {
        let _ = self.command_tx.send(PipeWireCommand::Shutdown);

        if let Some(handle) = self.thread_handle.take() {
            tokio::task::spawn_blocking(move || {
                let _ = handle.join();
            })
            .await
            .ok();
        }
    }
}

impl Drop for PipeWireClient {
    fn drop(&mut self) {
        let _ = self.command_tx.send(PipeWireCommand::Shutdown);
    }
}

struct PipeWireState {
    nodes: HashMap<u32, NodeInfo>,
    ports: HashMap<u32, PortInfo>,
    /// Maps node IDs to their port IDs
    node_ports: HashMap<u32, Vec<u32>>,
    /// Track links: link_id -> LinkInfo
    links: HashMap<u32, LinkInfo>,
    /// Maps output port ID to link IDs originating from that port
    port_links: HashMap<u32, Vec<u32>>,
    event_tx: mpsc::UnboundedSender<AudioEvent>,
    /// Track virtual sink names we've found
    found_virtual_sinks: HashSet<String>,
    /// Track virtual sink node IDs by name
    virtual_sink_ids: HashMap<String, u32>,
    /// The default output device node ID
    default_output_id: Option<u32>,
}

impl PipeWireState {
    fn new(event_tx: mpsc::UnboundedSender<AudioEvent>) -> Self {
        Self {
            nodes: HashMap::new(),
            ports: HashMap::new(),
            node_ports: HashMap::new(),
            links: HashMap::new(),
            port_links: HashMap::new(),
            event_tx,
            found_virtual_sinks: HashSet::new(),
            virtual_sink_ids: HashMap::new(),
            default_output_id: None,
        }
    }

    fn send_event(&self, event: AudioEvent) {
        if let Err(e) = self.event_tx.send(event) {
            error!("Failed to send audio event: {}", e);
        }
    }

    fn register_virtual_sink(&mut self, name: &str, node_id: u32) {
        self.found_virtual_sinks.insert(name.to_string());
        self.virtual_sink_ids.insert(name.to_string(), node_id);
    }

    fn has_virtual_sink(&self, name: &str) -> bool {
        self.found_virtual_sinks.contains(name)
    }

    fn add_port(&mut self, port: PortInfo) {
        self.node_ports
            .entry(port.node_id)
            .or_default()
            .push(port.id);
        self.ports.insert(port.id, port);
    }

    fn remove_port(&mut self, port_id: u32) -> Option<PortInfo> {
        if let Some(port) = self.ports.remove(&port_id) {
            if let Some(ports) = self.node_ports.get_mut(&port.node_id) {
                ports.retain(|&id| id != port_id);
            }
            Some(port)
        } else {
            None
        }
    }

    /// Get output ports for a node
    fn get_output_ports(&self, node_id: u32) -> Vec<&PortInfo> {
        self.node_ports
            .get(&node_id)
            .map(|port_ids| {
                port_ids
                    .iter()
                    .filter_map(|id| self.ports.get(id))
                    .filter(|p| p.direction == PortDirection::Output)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get input ports for a node
    fn get_input_ports(&self, node_id: u32) -> Vec<&PortInfo> {
        self.node_ports
            .get(&node_id)
            .map(|port_ids| {
                port_ids
                    .iter()
                    .filter_map(|id| self.ports.get(id))
                    .filter(|p| p.direction == PortDirection::Input)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find a sink device to use as default output
    fn find_default_output(&mut self) {
        // Look for a real hardware output (alsa_output devices are real hardware)
        // Skip virtual sinks and null sinks
        for node in self.nodes.values() {
            if matches!(node.node_type, NodeType::SinkDevice)
                && !self.found_virtual_sinks.contains(&node.name)
                && !node.name.contains("null-sink")
                && node.name.starts_with("alsa_output")
            {
                info!("Found default output device: {} (id={})", node.name, node.id);
                self.default_output_id = Some(node.id);
                return;
            }
        }
        // Fallback: any sink that isn't a null sink or our virtual sinks
        for node in self.nodes.values() {
            if matches!(node.node_type, NodeType::SinkDevice)
                && !self.found_virtual_sinks.contains(&node.name)
                && !node.name.contains("null-sink")
            {
                info!("Found fallback output device: {} (id={})", node.name, node.id);
                self.default_output_id = Some(node.id);
                return;
            }
        }
        warn!("No default output device found");
    }

    /// Add a link to tracking
    fn add_link(&mut self, link: LinkInfo) {
        self.port_links
            .entry(link.output_port)
            .or_default()
            .push(link.id);
        self.links.insert(link.id, link);
    }

    /// Remove a link from tracking
    fn remove_link(&mut self, link_id: u32) -> Option<LinkInfo> {
        if let Some(link) = self.links.remove(&link_id) {
            if let Some(port_links) = self.port_links.get_mut(&link.output_port) {
                port_links.retain(|&id| id != link_id);
            }
            Some(link)
        } else {
            None
        }
    }

    /// Get all link IDs originating from a node's output ports
    fn get_links_from_node(&self, node_id: u32) -> Vec<u32> {
        let output_ports = self.get_output_ports(node_id);
        output_ports
            .iter()
            .flat_map(|port| {
                self.port_links
                    .get(&port.id)
                    .cloned()
                    .unwrap_or_default()
            })
            .collect()
    }
}

/// Pending commands to be processed in the main loop
struct PendingCommands {
    commands: RefCell<Vec<PipeWireCommand>>,
    shutdown_requested: AtomicBool,
}

fn run_pipewire_thread(
    command_rx: std::sync::mpsc::Receiver<PipeWireCommand>,
    event_tx: mpsc::UnboundedSender<AudioEvent>,
) -> anyhow::Result<()> {
    let mainloop = pw::main_loop::MainLoopBox::new(None)?;
    let context = pw::context::ContextBox::new(&mainloop.loop_(), None)?;
    let core = context.connect(None)?;
    let registry = core.get_registry()?;

    let state = Rc::new(RefCell::new(PipeWireState::new(event_tx.clone())));

    // Setup registry listener for node/port discovery
    let _listener = {
        let state_add = state.clone();
        let state_remove = state.clone();

        registry
            .add_listener_local()
            .global(move |global| {
                handle_global_added(&state_add, global);
            })
            .global_remove(move |id| {
                handle_global_removed(&state_remove, id);
            })
            .register()
    };

    // Keep track of created nodes and links to prevent them from being dropped
    let created_nodes: Rc<RefCell<Vec<pw::node::Node>>> = Rc::new(RefCell::new(Vec::new()));
    let created_links: Rc<RefCell<Vec<pw::link::Link>>> = Rc::new(RefCell::new(Vec::new()));

    // Shared pending commands buffer
    let pending = Rc::new(PendingCommands {
        commands: RefCell::new(Vec::new()),
        shutdown_requested: AtomicBool::new(false),
    });
    let pending_clone = pending.clone();

    // Timer just collects commands from the channel
    let timer = mainloop.loop_().add_timer(move |_| {
        while let Ok(cmd) = command_rx.try_recv() {
            if matches!(cmd, PipeWireCommand::Shutdown) {
                pending_clone.shutdown_requested.store(true, Ordering::SeqCst);
            } else {
                pending_clone.commands.borrow_mut().push(cmd);
            }
        }
    });

    timer.update_timer(
        Some(std::time::Duration::from_millis(10)),
        Some(std::time::Duration::from_millis(10)),
    );

    // Signal that we're ready
    event_tx.send(AudioEvent::Ready)?;

    // Main loop with periodic command processing
    loop {
        // Run one iteration of the main loop (process pipewire events)
        mainloop.loop_().iterate(std::time::Duration::from_millis(10));

        // Check for shutdown
        if pending.shutdown_requested.load(Ordering::SeqCst) {
            break;
        }

        // Process pending commands
        let commands: Vec<_> = pending.commands.borrow_mut().drain(..).collect();
        for cmd in commands {
            match cmd {
                PipeWireCommand::Shutdown => {
                    // Already handled above
                }
                PipeWireCommand::CreateLink {
                    output_port,
                    input_port,
                } => {
                    info!("Creating Link: {} -> {}", output_port, input_port);
                    match create_link(&core, output_port, input_port) {
                        Ok(link) => {
                            info!("Link created successfully");
                            created_links.borrow_mut().push(link);
                        }
                        Err(e) => {
                            error!("Failed to create link: {}", e);
                        }
                    }
                }
                PipeWireCommand::DestroyLink { link_id } => {
                    info!("Destroying Link: {}", link_id);
                    let st = state.borrow();
                    if st.links.contains_key(&link_id) {
                        // Remove from our created_links if present
                        created_links.borrow_mut().retain(|_l| {
                            // Note: pw::link::Link doesn't expose ID, so we can't filter
                            // The link will be destroyed when we remove it from PipeWire
                            true
                        });
                    }
                    // Links with object.linger=true will persist until explicitly destroyed
                    // For now, we rely on PipeWire to clean up links when nodes are destroyed
                }
                PipeWireCommand::DestroyLinksFromNode { node_id } => {
                    info!("Destroying all links from node {}", node_id);
                    let st = state.borrow();
                    let link_ids = st.get_links_from_node(node_id);
                    drop(st);

                    // We need to destroy these links in PipeWire
                    // Since we created links with object.linger=true, they persist
                    // We'll remove them from our tracking - PipeWire will notify us via global_remove
                    for link_id in link_ids {
                        info!("Marking link {} for destruction", link_id);
                        // The links will be cleaned up when we stop holding references to them
                        // For now, remove from our created_links
                        created_links.borrow_mut().retain(|_| true);
                    }
                    // Note: Full link destruction requires using pw_registry_destroy
                    // For now, we rely on routing to override the connection
                }
                PipeWireCommand::CreateVirtualSink { name, description } => {
                    let st = state.borrow();
                    if st.has_virtual_sink(&name) {
                        info!("Virtual sink '{}' already exists, skipping creation", name);
                        continue;
                    }
                    drop(st);

                    info!("Creating virtual sink: {} ({})", name, description);
                    match create_null_sink(&core, &name, &description) {
                        Ok(node) => {
                            info!("Virtual sink '{}' created successfully", name);
                            created_nodes.borrow_mut().push(node);
                        }
                        Err(e) => {
                            error!("Failed to create virtual sink '{}': {}", name, e);
                        }
                    }
                }
                PipeWireCommand::CheckVirtualSinks => {
                    let st = state.borrow();
                    let monitor_exists = st.has_virtual_sink(MONITOR_SINK_NAME);
                    let stream_exists = st.has_virtual_sink(STREAM_SINK_NAME);
                    let monitor_id = st.virtual_sink_ids.get(MONITOR_SINK_NAME).copied();
                    let stream_id = st.virtual_sink_ids.get(STREAM_SINK_NAME).copied();

                    let _ = st.event_tx.send(AudioEvent::VirtualSinksStatus {
                        monitor_exists,
                        stream_exists,
                        monitor_id,
                        stream_id,
                    });
                }
                PipeWireCommand::RouteAppToSink {
                    app_node_id,
                    sink_node_id,
                } => {
                    info!("Routing app {} to sink {}", app_node_id, sink_node_id);
                    let st = state.borrow();

                    // Get app's output ports
                    let app_ports = st.get_output_ports(app_node_id);
                    // Get sink's input ports
                    let sink_ports = st.get_input_ports(sink_node_id);

                    // Create links matching channels
                    for app_port in &app_ports {
                        for sink_port in &sink_ports {
                            // Match by channel (FL->FL, FR->FR) or just connect if no channel info
                            let should_connect = match (&app_port.channel, &sink_port.channel) {
                                (Some(ac), Some(sc)) => ac == sc,
                                _ => true, // Connect if no channel info
                            };

                            if should_connect {
                                info!(
                                    "Creating link: port {} -> port {}",
                                    app_port.id, sink_port.id
                                );
                                match create_link(&core, app_port.id, sink_port.id) {
                                    Ok(link) => {
                                        created_links.borrow_mut().push(link);
                                    }
                                    Err(e) => {
                                        error!("Failed to create link: {}", e);
                                    }
                                }
                                break; // Only one link per app port
                            }
                        }
                    }
                }
                PipeWireCommand::ConnectSinkToOutput { sink_node_id } => {
                    let st = state.borrow();
                    let default_output = st.default_output_id;

                    if let Some(output_id) = default_output {
                        info!(
                            "Connecting sink {} monitor to output {}",
                            sink_node_id, output_id
                        );

                        // For null sinks, the monitor ports are OUTPUT ports on the sink
                        // We need to find ports named "monitor_*" or with direction Output
                        let monitor_ports: Vec<_> = st
                            .get_output_ports(sink_node_id)
                            .into_iter()
                            .filter(|p| p.name.contains("monitor") || p.direction == PortDirection::Output)
                            .collect();

                        let output_ports = st.get_input_ports(output_id);

                        for monitor_port in &monitor_ports {
                            for output_port in &output_ports {
                                let should_connect = match (&monitor_port.channel, &output_port.channel) {
                                    (Some(mc), Some(oc)) => mc == oc,
                                    _ => true,
                                };

                                if should_connect {
                                    info!(
                                        "Creating monitor link: port {} -> port {}",
                                        monitor_port.id, output_port.id
                                    );
                                    match create_link(&core, monitor_port.id, output_port.id) {
                                        Ok(link) => {
                                            created_links.borrow_mut().push(link);
                                        }
                                        Err(e) => {
                                            error!("Failed to create monitor link: {}", e);
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    } else {
                        warn!("No default output device found, cannot connect monitor");
                    }
                }
                PipeWireCommand::ConnectChannelToOutputs {
                    channel_sink_id,
                    monitor_sink_id,
                    stream_sink_id,
                } => {
                    info!(
                        "Connecting channel sink {} to monitor {} and stream {}",
                        channel_sink_id, monitor_sink_id, stream_sink_id
                    );
                    let st = state.borrow();

                    // Get channel sink's monitor/output ports
                    let channel_ports: Vec<_> = st
                        .get_output_ports(channel_sink_id)
                        .into_iter()
                        .filter(|p| p.name.contains("monitor") || p.direction == PortDirection::Output)
                        .collect();

                    // Get monitor sink's input ports
                    let monitor_ports = st.get_input_ports(monitor_sink_id);
                    // Get stream sink's input ports
                    let stream_ports = st.get_input_ports(stream_sink_id);

                    // Connect channel to monitor sink
                    for channel_port in &channel_ports {
                        for monitor_port in &monitor_ports {
                            let should_connect = match (&channel_port.channel, &monitor_port.channel) {
                                (Some(cc), Some(mc)) => cc == mc,
                                _ => true,
                            };

                            if should_connect {
                                info!(
                                    "Creating channel->monitor link: port {} -> port {}",
                                    channel_port.id, monitor_port.id
                                );
                                match create_link(&core, channel_port.id, monitor_port.id) {
                                    Ok(link) => {
                                        created_links.borrow_mut().push(link);
                                    }
                                    Err(e) => {
                                        error!("Failed to create channel->monitor link: {}", e);
                                    }
                                }
                                break;
                            }
                        }
                    }

                    // Connect channel to stream sink
                    for channel_port in &channel_ports {
                        for stream_port in &stream_ports {
                            let should_connect = match (&channel_port.channel, &stream_port.channel) {
                                (Some(cc), Some(sc)) => cc == sc,
                                _ => true,
                            };

                            if should_connect {
                                info!(
                                    "Creating channel->stream link: port {} -> port {}",
                                    channel_port.id, stream_port.id
                                );
                                match create_link(&core, channel_port.id, stream_port.id) {
                                    Ok(link) => {
                                        created_links.borrow_mut().push(link);
                                    }
                                    Err(e) => {
                                        error!("Failed to create channel->stream link: {}", e);
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
                PipeWireCommand::FindDefaultOutput => {
                    let mut st = state.borrow_mut();
                    st.find_default_output();
                }
                PipeWireCommand::SetChannelVolume { channel_name, volume } => {
                    // For now, just log the volume change
                    // Full implementation would require setting node params via pw_node_set_param
                    info!("Setting channel '{}' volume to {}", channel_name, volume);
                    let st = state.borrow();
                    let sink_name = format!("{}{}", CHANNEL_SINK_PREFIX, channel_name);
                    if let Some(&sink_id) = st.virtual_sink_ids.get(&sink_name) {
                        info!("Channel sink {} found for volume control", sink_id);
                        // TODO: Implement actual volume control via PipeWire props
                        // This requires using pw_node_set_param with SPA_PARAM_Props
                    } else {
                        warn!("Channel sink for '{}' not found", channel_name);
                    }
                }
                PipeWireCommand::SetChannelMuted { channel_name, muted } => {
                    info!("Setting channel '{}' muted to {}", channel_name, muted);
                    let st = state.borrow();
                    let sink_name = format!("{}{}", CHANNEL_SINK_PREFIX, channel_name);
                    if let Some(&sink_id) = st.virtual_sink_ids.get(&sink_name) {
                        info!("Channel sink {} found for mute control", sink_id);
                        // TODO: Implement actual mute control via PipeWire props
                    } else {
                        warn!("Channel sink for '{}' not found", channel_name);
                    }
                }
                PipeWireCommand::RouteAppToChannelByName { app_node_id, channel_name } => {
                    let sink_name = format!("{}{}", CHANNEL_SINK_PREFIX, channel_name);
                    info!("Routing app {} to channel sink '{}'", app_node_id, sink_name);
                    let st = state.borrow();
                    
                    if let Some(&sink_node_id) = st.virtual_sink_ids.get(&sink_name) {
                        // Get app's output ports
                        let app_ports = st.get_output_ports(app_node_id);
                        // Get sink's input ports
                        let sink_ports = st.get_input_ports(sink_node_id);
                        
                        if app_ports.is_empty() {
                            warn!("No output ports found for app {}", app_node_id);
                        }
                        if sink_ports.is_empty() {
                            warn!("No input ports found for sink {}", sink_node_id);
                        }
                        
                        // Create links matching channels
                        for app_port in &app_ports {
                            for sink_port in &sink_ports {
                                let should_connect = match (&app_port.channel, &sink_port.channel) {
                                    (Some(ac), Some(sc)) => ac == sc,
                                    _ => true,
                                };
                                
                                if should_connect {
                                    info!("Creating link: app port {} -> channel sink port {}", app_port.id, sink_port.id);
                                    match create_link(&core, app_port.id, sink_port.id) {
                                        Ok(link) => {
                                            created_links.borrow_mut().push(link);
                                        }
                                        Err(e) => {
                                            error!("Failed to create link: {}", e);
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    } else {
                        warn!("Channel sink '{}' not found, cannot route app {}", sink_name, app_node_id);
                    }
                }
                PipeWireCommand::RouteAppToVirtualSinks { app_node_id } => {
                    info!("Routing app {} to monitor and stream sinks", app_node_id);
                    let st = state.borrow();
                    
                    // Get app's output ports
                    let app_ports = st.get_output_ports(app_node_id);
                    if app_ports.is_empty() {
                        warn!("No output ports found for app {}", app_node_id);
                    }
                    
                    // Route to monitor sink
                    if let Some(&monitor_id) = st.virtual_sink_ids.get(MONITOR_SINK_NAME) {
                        let sink_ports = st.get_input_ports(monitor_id);
                        for app_port in &app_ports {
                            for sink_port in &sink_ports {
                                let should_connect = match (&app_port.channel, &sink_port.channel) {
                                    (Some(ac), Some(sc)) => ac == sc,
                                    _ => true,
                                };
                                if should_connect {
                                    info!("Creating link: app port {} -> monitor sink port {}", app_port.id, sink_port.id);
                                    match create_link(&core, app_port.id, sink_port.id) {
                                        Ok(link) => created_links.borrow_mut().push(link),
                                        Err(e) => error!("Failed to create monitor link: {}", e),
                                    }
                                    break;
                                }
                            }
                        }
                    } else {
                        warn!("Monitor sink not found");
                    }
                    
                    // Route to stream sink
                    if let Some(&stream_id) = st.virtual_sink_ids.get(STREAM_SINK_NAME) {
                        let sink_ports = st.get_input_ports(stream_id);
                        for app_port in &app_ports {
                            for sink_port in &sink_ports {
                                let should_connect = match (&app_port.channel, &sink_port.channel) {
                                    (Some(ac), Some(sc)) => ac == sc,
                                    _ => true,
                                };
                                if should_connect {
                                    info!("Creating link: app port {} -> stream sink port {}", app_port.id, sink_port.id);
                                    match create_link(&core, app_port.id, sink_port.id) {
                                        Ok(link) => created_links.borrow_mut().push(link),
                                        Err(e) => error!("Failed to create stream link: {}", e),
                                    }
                                    break;
                                }
                            }
                        }
                    } else {
                        warn!("Stream sink not found");
                    }
                }
            }
        }
    }

    // Keep created objects alive until shutdown
    drop(created_nodes);
    drop(created_links);

    Ok(())
}

/// Create a null sink (virtual audio sink) using PipeWire's adapter factory
fn create_null_sink(
    core: &pw::core::CoreBox,
    name: &str,
    description: &str,
) -> anyhow::Result<pw::node::Node> {
    // Create properties for the null sink
    let props = pw::properties::properties! {
        "factory.name" => "support.null-audio-sink",
        "node.name" => name,
        "node.description" => description,
        "media.class" => "Audio/Sink",
        "audio.position" => "FL,FR",
        "monitor.channel-volumes" => "true",
        "monitor.passthrough" => "true",
    };

    // Create the null sink using the adapter factory
    let node = core.create_object::<pw::node::Node>("adapter", &props)?;

    Ok(node)
}

/// Create a link between two ports
fn create_link(
    core: &pw::core::CoreBox,
    output_port: u32,
    input_port: u32,
) -> anyhow::Result<pw::link::Link> {
    let props = pw::properties::properties! {
        "link.output.port" => output_port.to_string(),
        "link.input.port" => input_port.to_string(),
        "object.linger" => "true",
    };

    let link = core.create_object::<pw::link::Link>("link-factory", &props)?;
    Ok(link)
}

fn handle_global_added(
    state: &Rc<RefCell<PipeWireState>>,
    global: &pw::registry::GlobalObject<&pw::spa::utils::dict::DictRef>
) {
    let props = match global.props.as_ref() {
        Some(p) => p,
        None => return
    };

    match global.type_ {
        pw::types::ObjectType::Node => {
            handle_node_added(state, global.id, props);
        }
        pw::types::ObjectType::Port => {
            handle_port_added(state, global.id, props);
        }
        pw::types::ObjectType::Link => {
            handle_link_added(state, global.id, props);
        }

        _ => {}
    }
}

// Handle a node being added
fn handle_node_added(
    state: &Rc<RefCell<PipeWireState>>,
    id: u32,
    props: &pw::spa::utils::dict::DictRef,
) {
    let name = props.get("node.name").unwrap_or("Unknown").to_string();
    let description = props.get("node.description").map(String::from);
    let application_name = props.get("application.name").map(String::from);
    let media_class = props.get("media.class").unwrap_or("").to_string();

    let node_type = classify_node(&media_class);

    // Skip nodes we don't care about
    if matches!(node_type, NodeType::Unknown) {
        return;
    }

    let node_info = NodeInfo {
        id,
        name: name.clone(),
        description,
        application_name,
        media_class,
        node_type: node_type.clone(),
    };

    info!(
        "Node discovered: {} (id={}, type={:?})",
        name, id, node_info.node_type
    );

    let mut state = state.borrow_mut();

    // Check if this is one of our virtual sinks (monitor, stream, or channel)
    if name == MONITOR_SINK_NAME || name == STREAM_SINK_NAME || name.starts_with(CHANNEL_SINK_PREFIX) {
        info!("Found Amplitude virtual sink: {} (id={})", name, id);
        state.register_virtual_sink(&name, id);

        // Send event about virtual sink discovery
        state.send_event(AudioEvent::VirtualSinkDiscovered(VirtualSinkInfo {
            id,
            name: name.clone(),
            description: node_info.description.clone(),
        }));
    }

    state.nodes.insert(id, node_info.clone());
    state.send_event(AudioEvent::NodeAdded(node_info));
}

// Handle a port being added
fn handle_port_added(
    state: &Rc<RefCell<PipeWireState>>,
    id: u32,
    props: &pw::spa::utils::dict::DictRef,
) {
    let name = props.get("port.name").unwrap_or("Unknown").to_string();

    let node_id: u32 = props
        .get("node.id")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let direction = match props.get("port.direction") {
        Some("in") => PortDirection::Input,
        Some("out") => PortDirection::Output,
        _ => return,
    };

    let channel = props.get("audio.channel").map(String::from);

    let port_info = PortInfo {
        id,
        node_id,
        name,
        direction,
        channel,
    };

    let mut state = state.borrow_mut();
    state.add_port(port_info.clone());
    state.send_event(AudioEvent::PortAdded(port_info));
}

/// Handle a link being added
fn handle_link_added(
    state: &Rc<RefCell<PipeWireState>>,
    id: u32,
    props: &pw::spa::utils::dict::DictRef,
) {
    let output_port: u32 = props
        .get("link.output.port")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let input_port: u32 = props
        .get("link.input.port")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let output_node: u32 = props
        .get("link.output.node")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let input_node: u32 = props
        .get("link.input.node")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let link_info = LinkInfo {
        id,
        output_port,
        input_port,
        output_node,
        input_node,
    };

    info!(
        "Link discovered: {} ({}:{} -> {}:{})",
        id, output_node, output_port, input_node, input_port
    );

    let mut state = state.borrow_mut();
    state.add_link(link_info.clone());
    state.send_event(AudioEvent::LinkAdded(link_info));
}

/// Handle an object being removed
fn handle_global_removed(state: &Rc<RefCell<PipeWireState>>, id: u32) {
    let mut state = state.borrow_mut();

    // Check if it's a node
    if let Some(node) = state.nodes.remove(&id) {
        info!("Node removed: {} (id={})", node.name, id);
        // Also remove all ports for this node
        if let Some(port_ids) = state.node_ports.remove(&id) {
            for port_id in port_ids {
                state.ports.remove(&port_id);
            }
        }
        state.send_event(AudioEvent::NodeRemoved { id });
        return;
    }

    // Check if it's a port
    if let Some(port) = state.remove_port(id) {
        state.send_event(AudioEvent::PortRemoved {
            id,
            node_id: port.node_id,
        });
        return;
    }

    // Check if it's a link
    if let Some(link) = state.remove_link(id) {
        info!("Link removed: {} ({}:{} -> {}:{})", 
            id, link.output_node, link.output_port, link.input_node, link.input_port);
        state.send_event(AudioEvent::LinkRemoved { id });
    }
}

/// Classify a node based on media.class
fn classify_node(media_class: &str) -> NodeType {
    match media_class {
        "Stream/Output/Audio" => NodeType::ApplicationOutput,
        "Stream/Input/Audio" => NodeType::ApplicationInput,
        "Audio/Sink" => NodeType::SinkDevice,
        "Audio/Source" => NodeType::SourceDevice,
        "Audio/Sink/Virtual" | "Audio/Source/Virtual" => NodeType::Virtual,
        _ => NodeType::Unknown,
    }
}
