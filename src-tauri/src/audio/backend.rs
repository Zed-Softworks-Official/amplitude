use crate::audio::node::NodeInfo;
use crate::audio::Sink;

/// Trait implemented by each platform audio backend.
/// All methods are synchronous from the caller's perspective; the backend
/// implementation is free to use internal threads and channels to communicate
/// with the underlying audio subsystem.
pub trait AudioBackend: Send {
    /// Create a virtual sink (null-sink) with the given human-readable name.
    /// Blocks until the platform confirms the node exists and returns a handle.
    fn create_virtual_sink(&mut self, name: &str) -> Result<Sink, String>;

    /// Destroy a previously created virtual sink.
    fn destroy_virtual_sink(&mut self, sink: &Sink) -> Result<(), String>;

    /// Drain any backend events that have accumulated since the last call.
    /// Non-blocking — returns an empty vec if nothing is pending.
    fn poll_events(&mut self) -> Vec<BackendEvent>;
}

// ---------------------------------------------------------------------------
// Events emitted by the backend to the engine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum BackendEvent {
    /// A new node appeared on the PipeWire graph.
    NodeAdded(NodeInfo),
    /// A node was removed from the PipeWire graph.
    NodeRemoved(u32),
}

// ---------------------------------------------------------------------------
// Commands sent from the engine to the PipeWire thread (internal use only;
// not part of the public trait — kept here for discoverability).
// ---------------------------------------------------------------------------

/// Reply channel type used inside CreateVirtualSink.
/// The PW thread sends back the assigned global node ID on success.
pub type SinkReply = std::sync::mpsc::SyncSender<Result<u64, String>>;

#[derive(Debug)]
pub enum BackendCommand {
    /// Ask the PW thread to create a null-sink with the given node name.
    /// The reply channel receives the PipeWire global node ID on success.
    CreateVirtualSink { name: String, reply: SinkReply },
    /// Ask the PW thread to destroy the sink identified by its PW node ID.
    DestroyVirtualSink { external_id: u64 },
}
