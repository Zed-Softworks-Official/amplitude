use spdlog::prelude::*;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use super::client::{AudioCommandSender, PipeWireClient, CHANNEL_SINK_PREFIX, MONITOR_SINK_NAME, STREAM_SINK_NAME};
use super::events::{AudioEvent, NodeInfo};
use super::nodes::NodeManager;

/// Main audio manager that coordinates PipeWire and node tracking
pub struct AudioManager {
    client: PipeWireClient,
    node_manager: Arc<RwLock<NodeManager>>,
    event_broadcaster: broadcast::Sender<AudioEvent>,
    /// Node ID for the monitor virtual sink
    monitor_sink_id: Option<u32>,
    /// Node ID for the stream virtual sink
    stream_sink_id: Option<u32>,
    /// Maps channel names to their virtual sink node IDs
    channel_sink_ids: HashMap<String, u32>,
    /// Channels that are pending creation (waiting for sink to be discovered)
    pending_channel_sinks: Vec<String>,
    /// Channel sink IDs that need to be connected to buses once ports are available
    pending_channel_connections: Vec<u32>,
    /// Channel names to create on startup (set before run() is called)
    startup_channels: Vec<String>,
}

impl AudioManager {
    pub async fn new() -> anyhow::Result<Self> {
        let client = PipeWireClient::new().await?;
        let node_manager = Arc::new(RwLock::new(NodeManager::new()));

        // Broadcast channel for multiple subscribers (UI, etc.)
        let (event_broadcaster, _) = broadcast::channel(256);

        Ok(Self {
            client,
            node_manager,
            event_broadcaster,
            monitor_sink_id: None,
            stream_sink_id: None,
            channel_sink_ids: HashMap::new(),
            pending_channel_sinks: Vec::new(),
            pending_channel_connections: Vec::new(),
            startup_channels: Vec::new(),
        })
    }

    /// Ensure virtual sinks exist, creating them if necessary
    /// This should be called after the PipeWire connection is ready
    pub fn ensure_virtual_sinks(&self) -> anyhow::Result<()> {
        // Create monitor sink if it doesn't exist
        self.client.create_virtual_sink(
            MONITOR_SINK_NAME.to_string(),
            "Amplitude Monitor Output".to_string(),
        )?;

        // Create stream sink if it doesn't exist
        self.client.create_virtual_sink(
            STREAM_SINK_NAME.to_string(),
            "Amplitude Stream Output".to_string(),
        )?;

        Ok(())
    }

    /// Get the monitor sink node ID
    pub fn monitor_sink_id(&self) -> Option<u32> {
        self.monitor_sink_id
    }

    /// Get the stream sink node ID
    pub fn stream_sink_id(&self) -> Option<u32> {
        self.stream_sink_id
    }

    /// Get the sink node ID for a channel
    pub fn get_channel_sink_id(&self, channel_name: &str) -> Option<u32> {
        self.channel_sink_ids.get(channel_name).copied()
    }

    /// Create a virtual sink for a channel
    pub fn create_channel_sink(&mut self, channel_name: &str) -> anyhow::Result<()> {
        let sink_name = format!("{}{}", CHANNEL_SINK_PREFIX, channel_name);
        let description = format!("Amplitude Channel: {}", channel_name);

        // Check if we already have this channel sink
        if self.channel_sink_ids.contains_key(channel_name) {
            info!("Channel sink for '{}' already exists", channel_name);
            return Ok(());
        }

        // Mark as pending so we can connect it when discovered
        if !self.pending_channel_sinks.contains(&channel_name.to_string()) {
            self.pending_channel_sinks.push(channel_name.to_string());
        }

        info!("Creating channel sink: {}", sink_name);
        self.client.create_virtual_sink(sink_name, description)
    }

    /// Create channel sinks for multiple channels
    pub fn create_channel_sinks(&mut self, channel_names: &[String]) -> anyhow::Result<()> {
        for name in channel_names {
            if let Err(e) = self.create_channel_sink(name) {
                error!("Failed to create channel sink for '{}': {}", name, e);
            }
        }
        Ok(())
    }

    /// Set the channels to create on startup (call before run())
    /// These will be created when PipeWire connection is ready
    pub fn set_startup_channels(&mut self, channels: Vec<String>) {
        self.startup_channels = channels;
    }

    /// Request creation of a channel sink (non-blocking, uses only the client)
    /// The sink will be discovered and connected via events
    /// This is safe to call even while the run() loop is active
    pub fn request_channel_sink(&self, channel_name: &str) -> anyhow::Result<()> {
        let sink_name = format!("{}{}", CHANNEL_SINK_PREFIX, channel_name);
        let description = format!("Amplitude Channel: {}", channel_name);
        info!("Requesting channel sink creation: {}", sink_name);
        self.client.create_virtual_sink(sink_name, description)
    }

    /// Request routing an app to a channel sink by name (non-blocking)
    /// This is safe to call even while the run() loop is active
    pub fn request_route_to_channel(&self, app_node_id: u32, channel_name: &str) -> anyhow::Result<()> {
        info!("Requesting route: app {} -> channel '{}'", app_node_id, channel_name);
        self.client.route_app_to_channel_by_name(app_node_id, channel_name.to_string())
    }

    /// Request routing an app to both monitor and stream sinks (non-blocking)
    /// This is safe to call even while the run() loop is active
    pub fn request_route_to_virtual_sinks(&self, app_node_id: u32) -> anyhow::Result<()> {
        info!("Requesting route: app {} -> monitor & stream sinks", app_node_id);
        self.client.route_app_to_virtual_sinks(app_node_id)
    }

    /// Get a command sender that can be used to send commands without needing a lock
    /// This should be called BEFORE the run() loop starts
    pub fn command_sender(&self) -> AudioCommandSender {
        self.client.command_sender()
    }

    /// Subscribe to audio events
    pub fn subscribe(&self) -> broadcast::Receiver<AudioEvent> {
        self.event_broadcaster.subscribe()
    }

    /// Get a clone of the node manager for read access
    pub fn node_manager(&self) -> Arc<RwLock<NodeManager>> {
        self.node_manager.clone()
    }

    /// Start the event processing loop
    /// This should be spawned as a tokio task
    pub async fn run(&mut self) -> anyhow::Result<()> {
        info!("Starting audio manager event loop");

        loop {
            let event = match self.client.recv_event().await {
                Some(e) => e,
                None => {
                    info!("PipeWire event channel closed");
                    break;
                }
            };

            // Update node manager
            {
                let mut nm = self.node_manager.write().await;
                nm.handle_event(&event);
            }

            // Broadcast to subscribers
            // Ignore errors (no subscribers is fine)
            let _ = self.event_broadcaster.send(event.clone());

            // Log significant events and handle special cases
            match &event {
                AudioEvent::Ready => {
                    info!("PipeWire connection ready");
                    // Ensure virtual sinks are created after PipeWire is ready
                    if let Err(e) = self.ensure_virtual_sinks() {
                        error!("Failed to ensure virtual sinks: {}", e);
                    }
                    // Find default output device
                    if let Err(e) = self.client.find_default_output() {
                        error!("Failed to find default output: {}", e);
                    }
                    // Create channel sinks for startup channels
                    let channels = std::mem::take(&mut self.startup_channels);
                    if !channels.is_empty() {
                        info!("Creating channel sinks for startup channels: {:?}", channels);
                        if let Err(e) = self.create_channel_sinks(&channels) {
                            error!("Failed to create startup channel sinks: {}", e);
                        }
                    }
                }
                AudioEvent::NodeAdded(info) => {
                    info!("Node added: {} ({:?})", info.name, info.node_type);
                }
                AudioEvent::NodeRemoved { id } => {
                    info!("Node removed: {}", id);
                }
                AudioEvent::Error(msg) => {
                    error!("PipeWire error: {}", msg);
                }
                AudioEvent::VirtualSinkDiscovered(sink_info) => {
                    info!(
                        "Virtual sink discovered: {} (id={})",
                        sink_info.name, sink_info.id
                    );
                    // Track our virtual sink IDs
                    if sink_info.name == MONITOR_SINK_NAME {
                        self.monitor_sink_id = Some(sink_info.id);
                        // Connect monitor sink to default output
                        if let Err(e) = self.client.connect_sink_to_output(sink_info.id) {
                            error!("Failed to connect monitor sink to output: {}", e);
                        }
                        // Connect any pending channel sinks to this monitor
                        self.connect_pending_channel_sinks();
                    } else if sink_info.name == STREAM_SINK_NAME {
                        self.stream_sink_id = Some(sink_info.id);
                        // Note: Stream sink typically goes to OBS/streaming software, not speakers
                        // So we don't connect it to the default output
                        // Connect any pending channel sinks to this stream
                        self.connect_pending_channel_sinks();
                    } else if sink_info.name.starts_with(CHANNEL_SINK_PREFIX) {
                        // This is a channel sink
                        let channel_name = sink_info.name
                            .strip_prefix(CHANNEL_SINK_PREFIX)
                            .unwrap_or(&sink_info.name)
                            .to_string();
                        
                        info!("Channel sink discovered: {} -> channel '{}'", sink_info.id, channel_name);
                        self.channel_sink_ids.insert(channel_name.clone(), sink_info.id);
                        
                        // Remove from pending creation list
                        self.pending_channel_sinks.retain(|n| n != &channel_name);
                        
                        // Add to pending connections - ports may not be available yet
                        // We'll retry when ports are added
                        if !self.pending_channel_connections.contains(&sink_info.id) {
                            self.pending_channel_connections.push(sink_info.id);
                        }
                        
                        // Try to connect immediately (may succeed if ports are already available)
                        self.try_connect_pending_channels();
                        
                        // Send event for UI
                        let _ = self.event_broadcaster.send(AudioEvent::ChannelSinkDiscovered {
                            name: channel_name,
                            id: sink_info.id,
                        });
                    }
                }
                AudioEvent::VirtualSinksStatus {
                    monitor_exists,
                    stream_exists,
                    monitor_id,
                    stream_id,
                } => {
                    info!(
                        "Virtual sinks status - Monitor: {} ({:?}), Stream: {} ({:?})",
                        monitor_exists, monitor_id, stream_exists, stream_id
                    );
                    if let Some(id) = monitor_id {
                        self.monitor_sink_id = Some(*id);
                    }
                    if let Some(id) = stream_id {
                        self.stream_sink_id = Some(*id);
                    }
                }
                AudioEvent::PortAdded(port_info) => {
                    // When a new port is added, check if it belongs to a pending channel sink
                    // and try to connect if we now have all the ports we need
                    if self.pending_channel_connections.contains(&port_info.node_id) {
                        info!("Port added for pending channel sink {}, retrying connections", port_info.node_id);
                        self.try_connect_pending_channels();
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Get current playback applications
    pub async fn get_playback_applications(&self) -> Vec<NodeInfo> {
        let nm = self.node_manager.read().await;
        nm.get_playback_applications().into_iter().cloned().collect()
    }

    /// Get current microphones
    pub async fn get_microphones(&self) -> Vec<NodeInfo> {
        let nm = self.node_manager.read().await;
        nm.get_input_devices().into_iter().cloned().collect()
    }

    /// Get current output devices
    pub async fn get_output_devices(&self) -> Vec<NodeInfo> {
        let nm = self.node_manager.read().await;
        nm.get_output_devices().into_iter().cloned().collect()
    }

    /// Route an application to a sink
    pub async fn route_to_sink(
        &self,
        app_node_id: u32,
        sink_node_id: u32,
    ) -> anyhow::Result<()> {
        let nm = self.node_manager.read().await;

        let app_ports = nm.get_output_ports(app_node_id);
        let sink_ports = nm.get_input_ports(sink_node_id);

        for app_port in &app_ports {
            for sink_port in &sink_ports {
                // Match by channel (FL->FL, FR->FR)
                if app_port.channel == sink_port.channel {
                    self.client.create_link(app_port.id, sink_port.id)?;
                    info!(
                        "Created link: {}:{} -> {}:{}",
                        app_node_id, app_port.name, sink_node_id, sink_port.name
                    );
                }
            }
        }

        Ok(())
    }

    /// Route an application to both monitor and stream virtual sinks
    pub fn route_to_virtual_sinks(&self, app_node_id: u32) -> anyhow::Result<()> {
        // Route to monitor sink if available
        if let Some(monitor_id) = self.monitor_sink_id {
            if let Err(e) = self.client.route_app_to_sink(app_node_id, monitor_id) {
                warn!("Failed to route to monitor sink: {}", e);
            }
        } else {
            warn!("Monitor sink not available for routing");
        }

        // Route to stream sink if available
        if let Some(stream_id) = self.stream_sink_id {
            if let Err(e) = self.client.route_app_to_sink(app_node_id, stream_id) {
                warn!("Failed to route to stream sink: {}", e);
            }
        } else {
            warn!("Stream sink not available for routing");
        }

        Ok(())
    }

    /// Route an application to the monitor sink only
    pub fn route_to_monitor(&self, app_node_id: u32) -> anyhow::Result<()> {
        if let Some(monitor_id) = self.monitor_sink_id {
            self.client.route_app_to_sink(app_node_id, monitor_id)
        } else {
            anyhow::bail!("Monitor sink not available")
        }
    }

    /// Route an application to the stream sink only
    pub fn route_to_stream(&self, app_node_id: u32) -> anyhow::Result<()> {
        if let Some(stream_id) = self.stream_sink_id {
            self.client.route_app_to_sink(app_node_id, stream_id)
        } else {
            anyhow::bail!("Stream sink not available")
        }
    }

    /// Get the PipeWire client for direct operations
    pub fn client(&self) -> &PipeWireClient {
        &self.client
    }

    /// Set volume for a channel (0.0 to 1.0)
    pub fn set_channel_volume(&self, channel_name: &str, volume: f32) -> anyhow::Result<()> {
        self.client.set_channel_volume(channel_name.to_string(), volume)
    }

    /// Mute/unmute a channel
    pub fn set_channel_muted(&self, channel_name: &str, muted: bool) -> anyhow::Result<()> {
        self.client.set_channel_muted(channel_name.to_string(), muted)
    }

    /// Route an application to a specific channel sink
    /// This disconnects from any existing outputs and connects to the channel sink
    pub fn route_app_to_channel(&self, app_node_id: u32, channel_name: &str) -> anyhow::Result<()> {
        // First, disconnect from any existing links
        info!("Disconnecting app {} from existing links", app_node_id);
        if let Err(e) = self.client.destroy_links_from_node(app_node_id) {
            warn!("Failed to destroy existing links: {}", e);
        }

        // Then connect to the channel sink
        if let Some(channel_sink_id) = self.channel_sink_ids.get(channel_name) {
            info!("Routing app {} to channel sink {} ('{}')", app_node_id, channel_sink_id, channel_name);
            self.client.route_app_to_sink(app_node_id, *channel_sink_id)
        } else {
            anyhow::bail!("Channel sink for '{}' not found", channel_name)
        }
    }

    /// Connect a channel sink to both monitor and stream sinks
    fn connect_channel_sink_to_outputs(&self, channel_sink_id: u32) {
        let monitor_id = self.monitor_sink_id;
        let stream_id = self.stream_sink_id;

        match (monitor_id, stream_id) {
            (Some(monitor), Some(stream)) => {
                info!(
                    "Connecting channel sink {} to monitor {} and stream {}",
                    channel_sink_id, monitor, stream
                );
                if let Err(e) = self.client.connect_channel_to_outputs(channel_sink_id, monitor, stream) {
                    error!("Failed to connect channel sink to outputs: {}", e);
                }
            }
            (Some(monitor), None) => {
                info!("Connecting channel sink {} to monitor {} (stream not ready)", channel_sink_id, monitor);
                // Connect to monitor only for now
                if let Err(e) = self.client.connect_channel_to_outputs(channel_sink_id, monitor, monitor) {
                    error!("Failed to connect channel sink to monitor: {}", e);
                }
            }
            (None, Some(stream)) => {
                info!("Connecting channel sink {} to stream {} (monitor not ready)", channel_sink_id, stream);
                // Connect to stream only for now
                if let Err(e) = self.client.connect_channel_to_outputs(channel_sink_id, stream, stream) {
                    error!("Failed to connect channel sink to stream: {}", e);
                }
            }
            (None, None) => {
                warn!("Cannot connect channel sink {} - monitor and stream sinks not ready", channel_sink_id);
            }
        }
    }

    /// Try to connect pending channel sinks to monitor/stream buses
    /// This is called when a channel sink is discovered and when ports are added
    fn try_connect_pending_channels(&mut self) {
        // Need both monitor and stream sinks ready
        let (Some(monitor_id), Some(stream_id)) = (self.monitor_sink_id, self.stream_sink_id) else {
            info!("Monitor or stream sink not ready, deferring channel connections");
            return;
        };

        // Keep track of which channels were successfully connected
        let mut connected_ids: Vec<u32> = Vec::new();

        for &channel_sink_id in &self.pending_channel_connections {
            info!("Attempting to connect channel sink {} to buses", channel_sink_id);
            
            // Try to connect - the command will check for available ports
            if let Err(e) = self.client.connect_channel_to_outputs(channel_sink_id, monitor_id, stream_id) {
                error!("Failed to connect channel sink {} to outputs: {}", channel_sink_id, e);
            } else {
                info!("Successfully sent connection command for channel sink {}", channel_sink_id);
                connected_ids.push(channel_sink_id);
            }
        }

        // Remove successfully connected channels from pending list
        for id in connected_ids {
            self.pending_channel_connections.retain(|&x| x != id);
        }
    }

    /// Connect any channel sinks that were created before monitor/stream sinks were ready
    fn connect_pending_channel_sinks(&mut self) {
        // Only proceed if both monitor and stream sinks are available
        if self.monitor_sink_id.is_none() || self.stream_sink_id.is_none() {
            return;
        }

        // Add all existing channel sinks to pending connections if not already there
        for (channel_name, &channel_sink_id) in &self.channel_sink_ids {
            if !self.pending_channel_connections.contains(&channel_sink_id) {
                info!("Adding channel sink for '{}' to pending connections", channel_name);
                self.pending_channel_connections.push(channel_sink_id);
            }
        }

        // Try to connect them
        self.try_connect_pending_channels();
    }

    /// Shutdown the audio manager
    pub async fn shutdown(&mut self) {
        info!("Shutting down audio manager");
        self.client.shutdown().await;
    }
}
