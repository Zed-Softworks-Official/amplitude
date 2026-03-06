use crate::audio::AudioBackend;
use crate::core::{
    bus::Bus,
    channels::{Channel, Send},
};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(target_os = "linux")]
use crate::backend::pipewire::create_backend;

#[cfg(target_os = "macos")]
use crate::backend::coreaudio::create_backend;

pub struct AudioEngine {
    backend: Box<dyn AudioBackend>,
    channels: HashMap<Uuid, Channel>,
    buses: HashMap<Uuid, Bus>,
    default_sends: Vec<Send>,
    channel_order: Vec<Uuid>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let backend = create_backend();

        Self {
            backend,
            channels: HashMap::new(),
            buses: HashMap::new(),
            default_sends: Vec::new(),
            channel_order: Vec::new(),
        }
    }
}
