use crate::core::channels::Channel;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, fs::File, io::Write};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub channels: HashMap<Uuid, Channel>,
}

impl Config {
    pub fn new(state: AppState) -> Self {
        Self {
            channels: state.channels.clone(),
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let config_path = dirs::config_dir()
            .unwrap()
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
            .unwrap()
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
