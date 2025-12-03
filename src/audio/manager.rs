use spdlog::prelude::*;

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use super::client::PipeWireClient;
use super::events::{AudioEvent, NodeInfo, PortDirection};
use super::nodes::NodeManager;

/// Main audio manager that coordinates PipeWire and node tracking
pub struct AudioManager {
    client: PipeWireClient,
    node_manager: Arc<RwLock<NodeManager>>,
    event_broadcaster: broadcast::Sender<AudioEvent>,
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
        })
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

            // Log significant events
            match &event {
                AudioEvent::Ready => {
                    info!("PipeWire connection ready");
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
                        app_node_id,
                        app_port.name,
                        sink_node_id,
                        sink_port.name
                    );
                }
            }
        }

        Ok(())
    }

    /// Shutdown the audio manager
    pub async fn shutdown(&mut self) {
        info!("Shutting down audio manager");
        self.client.shutdown().await;
    }
}
