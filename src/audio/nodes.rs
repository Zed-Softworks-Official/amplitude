use std::collections::HashMap;

use super::events::{AudioEvent, NodeInfo, NodeType, PortDirection, PortInfo};

/// Thread-safe node manager for tracking discovered audio nodes
#[derive(Debug, Default)]
pub struct NodeManager {
    nodes: HashMap<u32, NodeInfo>,
    ports: HashMap<u32, PortInfo>,
    node_ports: HashMap<u32, Vec<u32>>,
}

impl NodeManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process an event and update state
    pub fn handle_event(&mut self, event: &AudioEvent) {
        match event {
            AudioEvent::NodeAdded(info) => {
                self.nodes.insert(info.id, info.clone());
                self.node_ports.entry(info.id).or_default();
            }

            AudioEvent::NodeRemoved { id } => {
                self.nodes.remove(id);
                if let Some(port_ids) = self.node_ports.remove(id) {
                    for port_id in port_ids {
                        self.ports.remove(&port_id);
                    }
                }
            }

            AudioEvent::NodeChanged(info) => {
                self.nodes.insert(info.id, info.clone());
            }

            AudioEvent::PortAdded(info) => {
                self.ports.insert(info.id, info.clone());
                self.node_ports
                    .entry(info.node_id)
                    .or_default()
                    .push(info.id);
            }

            AudioEvent::PortRemoved { id, node_id } => {
                self.ports.remove(id);
                if let Some(ports) = self.node_ports.get_mut(node_id) {
                    ports.retain(|p| p != id);
                }
            }

            _ => {}
        }
    }

    /// Get applications outputting audio
    pub fn get_playback_applications(&self) -> Vec<&NodeInfo> {
        self.nodes
            .values()
            .filter(|n| matches!(n.node_type, NodeType::ApplicationOutput))
            .collect()
    }

    /// Get applications recording audio
    pub fn get_recording_applications(&self) -> Vec<&NodeInfo> {
        self.nodes
            .values()
            .filter(|n| matches!(n.node_type, NodeType::ApplicationInput))
            .collect()
    }

    /// Get output devices (speakers, headphones)
    pub fn get_output_devices(&self) -> Vec<&NodeInfo> {
        self.nodes
            .values()
            .filter(|n| matches!(n.node_type, NodeType::SinkDevice))
            .collect()
    }

    /// Get input devices (microphones)
    pub fn get_input_devices(&self) -> Vec<&NodeInfo> {
        self.nodes
            .values()
            .filter(|n| matches!(n.node_type, NodeType::SourceDevice))
            .collect()
    }

    /// Get ports for a node
    pub fn get_node_ports(&self, node_id: u32) -> Vec<&PortInfo> {
        self.node_ports
            .get(&node_id)
            .map(|ids| ids.iter().filter_map(|id| self.ports.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get output ports for a node
    pub fn get_output_ports(&self, node_id: u32) -> Vec<&PortInfo> {
        self.get_node_ports(node_id)
            .into_iter()
            .filter(|p| p.direction == PortDirection::Output)
            .collect()
    }

    /// Get input ports for a node
    pub fn get_input_ports(&self, node_id: u32) -> Vec<&PortInfo> {
        self.get_node_ports(node_id)
            .into_iter()
            .filter(|p| p.direction == PortDirection::Input)
            .collect()
    }

    /// Get a node by ID
    pub fn get_node(&self, id: u32) -> Option<&NodeInfo> {
        self.nodes.get(&id)
    }

    /// Get display name for a node
    pub fn get_display_name(&self, id: u32) -> String {
        self.nodes
            .get(&id)
            .map(|n| {
                n.application_name
                    .clone()
                    .or_else(|| n.description.clone())
                    .unwrap_or_else(|| n.name.clone())
            })
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Get all nodes
    pub fn all_nodes(&self) -> impl Iterator<Item = &NodeInfo> {
        self.nodes.values()
    }
}
