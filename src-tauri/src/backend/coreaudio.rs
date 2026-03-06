use crate::audio::{AudioBackend, Sink};
use crate::core::channels::{Channel, Send};

pub struct CoreAudioBackend {}

impl CoreAudioBackend {
    pub fn new() -> Self {
        Self {}
    }
}

impl AudioBackend for CoreAudioBackend {
    fn create_channel(
        &mut self,
        name: String,
        default_sends: Vec<Send>,
    ) -> Result<Channel, String> {
        Ok(Channel::new(
            name,
            default_sends,
            Sink::new("TODO".to_string()),
        ))
    }
}

pub fn create_backend() -> Box<dyn AudioBackend> {
    Box::new(CoreAudioBackend::new())
}
