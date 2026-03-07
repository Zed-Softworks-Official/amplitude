/// Runtime handle for a platform virtual sink node.
///
/// This type is intentionally **not** serialised — the PipeWire global node ID
/// (`external_id`) is ephemeral and meaningless across process restarts.
/// The engine recreates the underlying PW node on every startup and fills in
/// the real ID at that point.
#[derive(Debug, Clone)]
pub struct Sink {
    /// Platform-specific node identifier.
    /// On PipeWire this is the PW global node ID cast to u64.
    /// On CoreAudio this will be an AudioDeviceID cast to u64.
    /// Zero means "no live node" (stub / creation failed).
    pub external_id: u64,
}

impl Sink {
    pub fn new(external_id: u64) -> Self {
        Self { external_id }
    }
}

impl Default for Sink {
    /// Returns a stub sink with no live PW node.
    /// Used as the deserialized default when loading channels from config.
    fn default() -> Self {
        Self { external_id: 0 }
    }
}
