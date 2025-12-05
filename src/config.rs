use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Name of the built-in system channel
pub const SYSTEM_CHANNEL_NAME: &str = "System";

/// A channel/submix bus that groups multiple applications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub name: String,
    pub applications: Vec<String>, // App names to match
    pub monitor_volume: f64,
    pub monitor_muted: bool,
    pub stream_volume: f64,
    pub stream_muted: bool,
    /// Order for display sorting (lower = earlier)
    #[serde(default)]
    pub order: usize,
    /// Whether this is a built-in channel that cannot be deleted
    #[serde(default)]
    pub is_builtin: bool,
}

impl Channel {
    pub fn new(name: String) -> Self {
        Self {
            name,
            applications: Vec::new(),
            monitor_volume: 0.8,
            monitor_muted: false,
            stream_volume: 0.7,
            stream_muted: false,
            order: usize::MAX, // New channels go to the end
            is_builtin: false,
        }
    }

    /// Create the built-in System channel
    pub fn system() -> Self {
        Self {
            name: SYSTEM_CHANNEL_NAME.to_string(),
            applications: Vec::new(),
            monitor_volume: 0.8,
            monitor_muted: false,
            stream_volume: 0.7,
            stream_muted: false,
            order: 0, // System channel is always first
            is_builtin: true,
        }
    }

    /// Check if an application name matches this channel
    pub fn matches_application(&self, app_name: &str) -> bool {
        self.applications
            .iter()
            .any(|name| name.eq_ignore_ascii_case(app_name))
    }

    /// Add an application to this channel
    pub fn add_application(&mut self, app_name: String) {
        if !self.matches_application(&app_name) {
            self.applications.push(app_name);
        }
    }

    /// Remove an application from this channel
    pub fn remove_application(&mut self, app_name: &str) {
        self.applications
            .retain(|name| !name.eq_ignore_ascii_case(app_name));
    }
}

/// Application configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub channels: Vec<Channel>,
}

impl Config {
    /// Get the config file path
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("amplitude").join("config.toml"))
    }

    /// Load config from disk, or create default if not exists
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default_with_system();
        };

        if !path.exists() {
            return Self::default_with_system();
        }

        match fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(mut config) => {
                    // Ensure System channel exists
                    config.ensure_system_channel();
                    config
                }
                Err(e) => {
                    eprintln!("Failed to parse config: {}", e);
                    Self::default_with_system()
                }
            },
            Err(e) => {
                eprintln!("Failed to read config: {}", e);
                Self::default_with_system()
            }
        }
    }

    /// Create a default config with the System channel
    fn default_with_system() -> Self {
        let mut config = Self::default();
        config.ensure_system_channel();
        config
    }

    /// Ensure the System channel exists
    fn ensure_system_channel(&mut self) {
        if !self.channels.iter().any(|c| c.name == SYSTEM_CHANNEL_NAME) {
            self.channels.insert(0, Channel::system());
        }
        // Sort channels by order
        self.channels.sort_by_key(|c| c.order);
    }

    /// Save config to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let Some(path) = Self::config_path() else {
            anyhow::bail!("Could not determine config directory");
        };

        // Create config directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;

        Ok(())
    }

    /// Find which channel an application belongs to
    pub fn find_channel_for_app(&self, app_name: &str) -> Option<&Channel> {
        self.channels
            .iter()
            .find(|ch| ch.matches_application(app_name))
    }

    /// Find which channel an application belongs to (mutable)
    pub fn find_channel_for_app_mut(&mut self, app_name: &str) -> Option<&mut Channel> {
        self.channels
            .iter_mut()
            .find(|ch| ch.matches_application(app_name))
    }

    /// Get a channel by name
    pub fn get_channel(&self, name: &str) -> Option<&Channel> {
        self.channels.iter().find(|ch| ch.name == name)
    }

    /// Get a channel by name (mutable)
    pub fn get_channel_mut(&mut self, name: &str) -> Option<&mut Channel> {
        self.channels.iter_mut().find(|ch| ch.name == name)
    }

    /// Add a new channel
    pub fn add_channel(&mut self, name: String) -> &mut Channel {
        // Find the highest order value and add 1
        let max_order = self.channels.iter().map(|c| c.order).max().unwrap_or(0);
        let mut channel = Channel::new(name);
        channel.order = max_order + 1;
        self.channels.push(channel);
        self.channels.last_mut().unwrap()
    }

    /// Remove a channel by name (cannot remove built-in channels)
    pub fn remove_channel(&mut self, name: &str) -> bool {
        let is_builtin = self
            .channels
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.is_builtin)
            .unwrap_or(false);

        if is_builtin {
            return false;
        }

        self.channels.retain(|ch| ch.name != name);
        true
    }

    /// Reorder channels by providing new order indices
    pub fn reorder_channels(&mut self, channel_orders: &[(String, usize)]) {
        for (name, order) in channel_orders {
            if let Some(channel) = self.get_channel_mut(name) {
                // Don't allow reordering the System channel to not be first
                if !channel.is_builtin {
                    channel.order = *order;
                }
            }
        }
        self.channels.sort_by_key(|c| c.order);
    }

    /// Move a channel to a new position
    pub fn move_channel(&mut self, name: &str, new_position: usize) {
        // Don't move builtin channels
        if let Some(channel) = self.get_channel(name) {
            if channel.is_builtin {
                return;
            }
        }

        // Get current positions
        let current_pos = self.channels.iter().position(|c| c.name == name);
        if let Some(current) = current_pos {
            if current == new_position {
                return;
            }

            // Remove and reinsert at new position
            let channel = self.channels.remove(current);
            let insert_pos = new_position.min(self.channels.len());
            self.channels.insert(insert_pos, channel);

            // Update order values to reflect new positions
            for (i, ch) in self.channels.iter_mut().enumerate() {
                ch.order = i;
            }
        }
    }
}

