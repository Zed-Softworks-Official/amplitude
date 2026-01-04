use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::collections::HashMap;

use std::fs;
use crate::audio::Channel;
use crate::core::icon::Icon;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub channels: Vec<Channel>
}

impl Config {
    fn new() -> Self {
        Self {
            channels: vec![
                Channel::new(
                    "Microphone".to_string(),
                    Icon::Microphone
                )
            ]
        }
    }

    fn config_exists() -> bool {
        config_path().exists()
    }

    pub fn load() -> Self {
        if !Self::config_exists() {
            let mut config = Self::new();
            config.save(None);

            return config;
        }

        let config_str = fs::read_to_string(config_path()).unwrap();
        toml::from_str(&config_str).unwrap()
    }

    pub fn save(&mut self, channels: Option<HashMap<Uuid, Channel>>) {
        if let Some(channels) = channels {
            self.channels = channels.iter()
                .map(|(_id, channel)| channel.clone())
                .collect();
        }

        let config_str = toml::to_string(&self).unwrap();
        fs::write(config_path(), config_str).unwrap();
    }
}

fn config_path() -> std::path::PathBuf {
    dirs::config_dir().unwrap().join("amplitude").join("config.toml")
}
