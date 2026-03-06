use crate::audio::{AudioBackend, Sink};

pub struct PipewireBackend {}

impl PipewireBackend {
    pub fn new() -> Self {
        Self {}
    }
}

impl AudioBackend for PipewireBackend {
    fn create_virtual_sink(&mut self, name: &str) -> Result<Sink, String> {
        // TODO: Create a real PipeWire virtual sink node
        Ok(Sink::new(format!("pipewire:{}", name)))
    }

    fn destroy_virtual_sink(&mut self, _sink: &Sink) -> Result<(), String> {
        // TODO: Destroy the PipeWire virtual sink node
        Ok(())
    }
}

pub fn create_backend() -> Box<dyn AudioBackend> {
    Box::new(PipewireBackend::new())
}
