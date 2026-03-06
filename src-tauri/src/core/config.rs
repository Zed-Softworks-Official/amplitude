use crate::core::{bus::Bus, channels::Channel};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, fs::File, io::Write};
use uuid::Uuid;

/// The data persisted to disk. Sinks are included via `Channel.virtual_sink`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavePayload {
    pub channels: HashMap<Uuid, Channel>,
    pub buses: HashMap<Uuid, Bus>,
    pub channel_order: Vec<Uuid>,
}

/// Serializable config file structure — same shape as SavePayload,
/// kept as a distinct type so the on-disk format can diverge later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub channels: HashMap<Uuid, Channel>,
    pub buses: HashMap<Uuid, Bus>,
    pub channel_order: Vec<Uuid>,
}

impl Config {
    pub fn from_payload(payload: SavePayload) -> Self {
        Self {
            channels: payload.channels,
            buses: payload.buses,
            channel_order: payload.channel_order,
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let config_path = dirs::config_dir()
            .ok_or("failed to get config dir")?
            .join("amplitude")
            .join("config.toml");

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut config_file = File::create(config_path.clone())?;
        config_file.write_all(toml::to_string(self)?.as_bytes())?;

        println!("saved config to {:?}", config_path);
        Ok(())
    }

    pub fn load() -> Result<Self, Box<dyn Error>> {
        let config_path = dirs::config_dir()
            .ok_or("failed to get config dir")?
            .join("amplitude")
            .join("config.toml");

        if !config_path.exists() {
            return Err("config not found".into());
        }

        let config_file = File::open(config_path)?;
        let config_str = std::io::read_to_string(config_file)?;
        let config: Self = toml::from_str(&config_str)?;

        Ok(config)
    }
}
