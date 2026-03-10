use crate::audio::{AudioBackend, BackendEvent, Sink};

pub struct CoreAudioBackend {
    /// Monotonically increasing stub ID used until CoreAudio is implemented.
    next_id: u64,
}

impl CoreAudioBackend {
    pub fn new() -> Self {
        Self { next_id: 1 }
    }
}

impl Default for CoreAudioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioBackend for CoreAudioBackend {
    fn create_bus_sink(&mut self, _name: &str) -> Result<Sink, String> {
        // TODO: CoreAudio bus routing (aggregate device or AU graph output node)
        let id = self.next_id;
        self.next_id += 1;
        Ok(Sink::new(id))
    }

    fn create_virtual_sink(&mut self, _name: &str) -> Result<Sink, String> {
        // TODO: Create a real CoreAudio virtual sink (e.g. via AudioHardware)
        let id = self.next_id;
        self.next_id += 1;
        Ok(Sink::new(id))
    }

    fn destroy_virtual_sink(&mut self, _sink: &Sink) -> Result<(), String> {
        // TODO: Tear down the CoreAudio virtual sink
        Ok(())
    }

    fn create_link(
        &mut self,
        _output_node_id: u64,
        _input_node_id: u64,
    ) -> Result<u64, String> {
        // TODO: CoreAudio routing
        let id = self.next_id;
        self.next_id += 1;
        Ok(id)
    }

    fn destroy_link(&mut self, _link_id: u64) -> Result<(), String> {
        // TODO: CoreAudio routing
        Ok(())
    }

    fn poll_events(&mut self) -> Vec<BackendEvent> {
        // TODO: surface CoreAudio device add/remove notifications
        Vec::new()
    }
}

pub fn create_backend() -> Box<dyn AudioBackend> {
    Box::new(CoreAudioBackend::new())
}
