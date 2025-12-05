use std::collections::{HashMap, HashSet};

use crate::config::{Channel, Config, SYSTEM_CHANNEL_NAME};

/// Runtime channel state that tracks which nodes are assigned to which channels
#[derive(Debug)]
pub struct ChannelManager {
    config: Config,
    /// Maps node IDs to channel names
    node_to_channel: HashMap<u32, String>,
    /// Maps channel names to active node IDs
    channel_to_nodes: HashMap<String, HashSet<u32>>,
    /// Unassigned node IDs (apps without a channel)
    unassigned_nodes: HashSet<u32>,
}

impl ChannelManager {
    pub fn new() -> Self {
        let config = Config::load();
        Self {
            config,
            node_to_channel: HashMap::new(),
            channel_to_nodes: HashMap::new(),
            unassigned_nodes: HashSet::new(),
        }
    }

    /// Get the current config
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get mutable config reference
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Save the current config to disk
    pub fn save_config(&self) -> anyhow::Result<()> {
        self.config.save()
    }

    /// Get all channels
    pub fn channels(&self) -> &[Channel] {
        &self.config.channels
    }

    /// Create a new channel
    pub fn create_channel(&mut self, name: String) -> anyhow::Result<()> {
        if self.config.get_channel(&name).is_some() {
            anyhow::bail!("Channel '{}' already exists", name);
        }
        self.config.add_channel(name.clone());
        self.channel_to_nodes.insert(name, HashSet::new());
        self.save_config()?;
        Ok(())
    }

    /// Delete a channel (cannot delete built-in channels like System)
    pub fn delete_channel(&mut self, name: &str) -> anyhow::Result<()> {
        // Check if this is a built-in channel
        if let Some(channel) = self.config.get_channel(name) {
            if channel.is_builtin {
                anyhow::bail!("Cannot delete built-in channel '{}'", name);
            }
        }

        // Move all nodes from this channel to System channel
        if let Some(nodes) = self.channel_to_nodes.remove(name) {
            for node_id in nodes {
                self.node_to_channel.remove(&node_id);
                // Move to System channel instead of unassigned
                self.node_to_channel
                    .insert(node_id, SYSTEM_CHANNEL_NAME.to_string());
                self.channel_to_nodes
                    .entry(SYSTEM_CHANNEL_NAME.to_string())
                    .or_default()
                    .insert(node_id);
            }
        }

        if !self.config.remove_channel(name) {
            anyhow::bail!("Failed to remove channel '{}'", name);
        }
        self.save_config()?;
        Ok(())
    }

    /// Assign an application to a channel by name
    /// This updates the config so the app will auto-assign in the future
    pub fn assign_app_to_channel(
        &mut self,
        app_name: &str,
        channel_name: &str,
    ) -> anyhow::Result<()> {
        // Remove from any existing channel first
        for channel in &mut self.config.channels {
            channel.remove_application(app_name);
        }

        // Add to the new channel
        if let Some(channel) = self.config.get_channel_mut(channel_name) {
            channel.add_application(app_name.to_string());
            self.save_config()?;
            Ok(())
        } else {
            anyhow::bail!("Channel '{}' not found", channel_name)
        }
    }

    /// Remove an application from its channel
    pub fn unassign_app(&mut self, app_name: &str) -> anyhow::Result<()> {
        for channel in &mut self.config.channels {
            channel.remove_application(app_name);
        }
        self.save_config()?;
        Ok(())
    }

    /// Called when a new audio node is discovered
    /// Returns Some(channel_name) if the app is assigned to a channel, None if unassigned
    pub fn on_node_added(&mut self, node_id: u32, app_name: &str) -> Option<String> {
        // Check if this app has a channel assignment in config
        if let Some(channel) = self.config.find_channel_for_app(app_name) {
            let channel_name = channel.name.clone();
            self.node_to_channel.insert(node_id, channel_name.clone());
            self.channel_to_nodes
                .entry(channel_name.clone())
                .or_default()
                .insert(node_id);
            Some(channel_name)
        } else {
            // App is unassigned - add to unassigned set
            self.unassigned_nodes.insert(node_id);
            None
        }
    }

    /// Called when an audio node is removed
    pub fn on_node_removed(&mut self, node_id: u32) {
        if let Some(channel_name) = self.node_to_channel.remove(&node_id) {
            if let Some(nodes) = self.channel_to_nodes.get_mut(&channel_name) {
                nodes.remove(&node_id);
            }
        }
        self.unassigned_nodes.remove(&node_id);
    }

    /// Manually assign a running node to a channel
    /// Also updates config so it persists
    pub fn assign_node_to_channel(
        &mut self,
        node_id: u32,
        app_name: &str,
        channel_name: &str,
    ) -> anyhow::Result<()> {
        // Remove from current location
        self.on_node_removed(node_id);

        // Add to config for future auto-assignment
        self.assign_app_to_channel(app_name, channel_name)?;

        // Add to runtime state
        self.node_to_channel
            .insert(node_id, channel_name.to_string());
        self.channel_to_nodes
            .entry(channel_name.to_string())
            .or_default()
            .insert(node_id);

        Ok(())
    }

    /// Get the channel name for a node
    pub fn get_node_channel(&self, node_id: u32) -> Option<&str> {
        self.node_to_channel.get(&node_id).map(|s| s.as_str())
    }

    /// Get all node IDs in a channel
    pub fn get_channel_nodes(&self, channel_name: &str) -> Vec<u32> {
        self.channel_to_nodes
            .get(channel_name)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get all unassigned node IDs
    pub fn get_unassigned_nodes(&self) -> Vec<u32> {
        self.unassigned_nodes.iter().copied().collect()
    }

    /// Update channel volume settings
    pub fn set_channel_monitor_volume(
        &mut self,
        channel_name: &str,
        volume: f64,
    ) -> anyhow::Result<()> {
        if let Some(channel) = self.config.get_channel_mut(channel_name) {
            channel.monitor_volume = volume;
            self.save_config()?;
        }
        Ok(())
    }

    /// Update channel mute settings
    pub fn set_channel_monitor_muted(
        &mut self,
        channel_name: &str,
        muted: bool,
    ) -> anyhow::Result<()> {
        if let Some(channel) = self.config.get_channel_mut(channel_name) {
            channel.monitor_muted = muted;
            self.save_config()?;
        }
        Ok(())
    }

    /// Update channel stream volume
    pub fn set_channel_stream_volume(
        &mut self,
        channel_name: &str,
        volume: f64,
    ) -> anyhow::Result<()> {
        if let Some(channel) = self.config.get_channel_mut(channel_name) {
            channel.stream_volume = volume;
            self.save_config()?;
        }
        Ok(())
    }

    /// Update channel stream mute
    pub fn set_channel_stream_muted(
        &mut self,
        channel_name: &str,
        muted: bool,
    ) -> anyhow::Result<()> {
        if let Some(channel) = self.config.get_channel_mut(channel_name) {
            channel.stream_muted = muted;
            self.save_config()?;
        }
        Ok(())
    }

    /// Move a channel to a new position
    pub fn move_channel(&mut self, name: &str, new_position: usize) -> anyhow::Result<()> {
        self.config.move_channel(name, new_position);
        self.save_config()?;
        Ok(())
    }

    /// Check if a channel is built-in (cannot be deleted)
    pub fn is_builtin_channel(&self, name: &str) -> bool {
        self.config
            .get_channel(name)
            .map(|c| c.is_builtin)
            .unwrap_or(false)
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}


