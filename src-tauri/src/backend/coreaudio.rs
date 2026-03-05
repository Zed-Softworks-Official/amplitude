use crate::audio::{AudioBackend, Sink};
use std::collections::HashMap;
use uuid::Uuid;

pub struct CoreAudioBackend {
    sinks: HashMap<Uuid, Sink>,
}

impl AudioBackend for CoreAudioBackend {
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

    fn create_sink(&mut self) -> Result<Sink, String> {
        // TODO: Implement
        let sink = Sink::new("TODO".to_string());
        self.sinks.insert(sink.id, sink.clone());
        Ok(sink)
    }

    fn delete_sink(&mut self, id: Uuid) -> Result<(), String> {
        self.sinks.remove(&id);
        Ok(())
    }

    fn update_sink(&mut self, id: Uuid, sink: Sink) -> Result<(), String> {
        self.sinks.insert(id, sink);
        Ok(())
    }
}
