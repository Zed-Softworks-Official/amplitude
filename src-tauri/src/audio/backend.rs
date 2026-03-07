use crate::audio::Sink;

pub trait AudioBackend: Send {
    fn create_virtual_sink(&mut self, name: &str) -> Result<Sink, String>;
    fn destroy_virtual_sink(&mut self, sink: &Sink) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct NodeInfo {

}

#[derive(Debug, Clone)]
pub enum BackendEvent {
    NodeAdded(NodeInfo),
    NodeRemoved(u32)
}

#[derive(Debug, Clone)]
pub enum BackendCommand {
    CreateVirtualSink(String)
}
