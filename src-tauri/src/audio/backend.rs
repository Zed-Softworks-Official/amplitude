use crate::audio::Sink;
use std::collections::HashMap;
use uuid::Uuid;

pub trait AudioBackend {
    fn new() -> Self;

    fn get_sinks(&self) -> &HashMap<Uuid, Sink>;
    fn get_sink(&self, id: Uuid) -> Option<&Sink>;

    fn create_sink(&mut self) -> Result<Sink, String>;
    fn delete_sink(&mut self, id: Uuid) -> Result<(), String>;
    fn update_sink(&mut self, id: Uuid, sink: Sink) -> Result<(), String>;
}
