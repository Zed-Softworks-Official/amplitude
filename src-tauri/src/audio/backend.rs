use crate::audio::node::NodeInfo;
use crate::audio::Sink;

/// Trait implemented by each platform audio backend.
/// All methods are synchronous from the caller's perspective; the backend
/// implementation is free to use internal threads and channels to communicate
/// with the underlying audio subsystem.
pub trait AudioBackend: Send {
    /// Create a virtual sink node that acts as a mix bus.
    ///
    /// Semantically distinct from a channel sink — on future platforms
    /// (CoreAudio, WASAPI) these may map to different constructs such as
    /// aggregate devices or AU graph output nodes. Blocks until the platform
    /// confirms the node exists and returns a handle.
    fn create_bus_sink(&mut self, name: &str) -> Result<Sink, String>;

    /// Create a virtual sink (null-sink) with the given human-readable name.
    /// Blocks until the platform confirms the node exists and returns a handle.
    fn create_virtual_sink(&mut self, name: &str) -> Result<Sink, String>;

    /// Destroy a previously created virtual sink.
    fn destroy_virtual_sink(&mut self, sink: &Sink) -> Result<(), String>;

    /// Create a PipeWire link from the monitor/output ports of `output_node_id`
    /// to the input ports of `input_node_id`.
    /// Returns the PipeWire global link ID on success.
    fn create_link(
        &mut self,
        output_node_id: u64,
        input_node_id: u64,
    ) -> Result<u64, String>;

    /// Destroy a previously created link by its PipeWire global ID.
    fn destroy_link(&mut self, link_id: u64) -> Result<(), String>;

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
// Reply channel types used in commands that need a synchronous response
// ---------------------------------------------------------------------------

/// Reply channel for `CreateVirtualSink` — carries the PW node global ID.
pub type SinkReply = std::sync::mpsc::SyncSender<Result<u64, String>>;

/// Reply channel for `CreateLink` — carries the PW link global ID.
pub type LinkReply = std::sync::mpsc::SyncSender<Result<u64, String>>;

// ---------------------------------------------------------------------------
// Commands sent from the engine to the PipeWire thread (internal to the
// PipeWire backend; not part of the public trait).
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum BackendCommand {
    /// Create a null-sink node. The reply channel receives the PW global node
    /// ID once the registry confirms the node is live.
    CreateVirtualSink { name: String, reply: SinkReply },

    /// Destroy the sink identified by its PW global node ID.
    DestroyVirtualSink { external_id: u64 },

    /// Create a link from the monitor/output ports of `output_node_id` to the
    /// input ports of `input_node_id`. Reply carries the PW link global ID.
    CreateLink {
        output_node_id: u64,
        input_node_id: u64,
        reply: LinkReply,
    },

    /// Destroy the link with the given PW global ID.
    DestroyLink { link_id: u64 },
}
