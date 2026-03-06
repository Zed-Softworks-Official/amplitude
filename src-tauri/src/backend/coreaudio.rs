use crate::audio::{AudioBackend, Sink};

pub struct CoreAudioBackend {}

impl CoreAudioBackend {
    pub fn new() -> Self {
        Self {}
    }
}

impl AudioBackend for CoreAudioBackend {
    fn create_virtual_sink(&mut self, name: &str) -> Result<Sink, String> {
        // TODO: Create a real CoreAudio virtual sink
        Ok(Sink::new(format!("coreaudio:{}", name)))
    }

    fn destroy_virtual_sink(&mut self, _sink: &Sink) -> Result<(), String> {
        // TODO: Tear down the CoreAudio virtual sink
        Ok(())
    }
}

pub fn create_backend() -> Box<dyn AudioBackend> {
    Box::new(CoreAudioBackend::new())
}
