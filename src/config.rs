use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// A channel/submix bus that groups multiple applications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub name: String,
    pub applications: Vec<String>, // App names to match
    pub monitor_volume: f64,
    pub monitor_muted: bool,
    pub stream_volume: f64,
    pub stream_muted: bool,
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
            return Self::default();
        };

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Failed to parse config: {}", e);
                    Self::default()
                }
            },
            Err(e) => {
                eprintln!("Failed to read config: {}", e);
                Self::default()
            }
        }
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
        self.channels.push(Channel::new(name));
        self.channels.last_mut().unwrap()
    }

    /// Remove a channel by name
    pub fn remove_channel(&mut self, name: &str) {
        self.channels.retain(|ch| ch.name != name);
    }
}
