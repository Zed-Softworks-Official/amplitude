use crate::audio::{AudioBackend, Sink};
use crate::core::channels::{Channel, Send};

use uuid::Uuid;

pub struct PipewireBackend {
    pub default_sends: Vec<Send>,
}

impl PipewireBackend {
    pub fn new() -> Self {
        let default_sends = vec![
            Send::new(Uuid::new_v4(), 0.0, false),
            Send::new(Uuid::new_v4(), 0.0, false),
        ];

        Self { default_sends }
    }
}

impl AudioBackend for PipewireBackend {
    fn create_channel(&mut self, name: String) -> Result<Channel, String> {
        // Create A New Virtual Sink using pipewire

        Ok(Channel::new(
            name,
            self.default_sends.clone(),
            Sink::new("TODO".to_string()),
        ))
    }
}

pub fn create_backend() -> Box<dyn AudioBackend> {
    Box::new(PipewireBackend::new())
}
