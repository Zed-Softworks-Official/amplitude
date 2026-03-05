use crate::audio::{AudioBackend, Sinks};
use std::collections::HashMap;

pub struct PipewireBackend {
    sinks: HashMap<Uuid, Sink>,
}

impl AudioBackend for PipewireBackend {
    fn new() -> Self {
        Self {
            sinks: HashMap::new(),
        }
    }

    fn get_sinks(&self) -> &HashMap<Uuid, Sink> {
        &self.sinks
    }

    fn get_sink(&self, id: Uuid) -> Option<&Sink> {
        self.sinks.get(&id)
    }

    fn create_sink(&self) -> Result<Sink, String> {
        // TODO: Implement
        let sink = Sink::new("TODO");
        self.sinks.insert(sink.id, sink);
        Ok(sink)
    }

    fn delete_sink(&self, id: Uuid) -> Result<(), String> {
        self.sinks.remove(&id);
        Ok(())
    }

    fn update_sink(&self, id: Uuid, sink: Sink) -> Result<(), String> {
        self.sinks.insert(id, sink);
        Ok(())
    }
}
