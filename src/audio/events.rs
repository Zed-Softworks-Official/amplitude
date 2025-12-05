#[derive(Debug, Clone)]
pub enum AudioEvent {
    NodeAdded(NodeInfo),
    NodeRemoved { id: u32 },
    NodeChanged(NodeInfo),
    PortAdded(PortInfo),
    PortRemoved { id: u32, node_id: u32 },
    LinkAdded(LinkInfo),
    LinkRemoved { id: u32 },
    ConnectionChanged,
    Ready,
    Error(String),
    /// A virtual sink created by Amplitude was discovered
    VirtualSinkDiscovered(VirtualSinkInfo),
    /// A channel sink created by Amplitude was discovered
    ChannelSinkDiscovered { name: String, id: u32 },
    /// Status of virtual sinks (response to CheckVirtualSinks command)
    VirtualSinksStatus {
        monitor_exists: bool,
        stream_exists: bool,
        monitor_id: Option<u32>,
        stream_id: Option<u32>,
    },
    /// Peak level update for a node (for level meters)
    PeakLevel { node_id: u32, left: f32, right: f32 },
    /// Peak level update for a channel (for channel level meters)
    ChannelPeakLevel { channel_name: String, left: f32, right: f32 },
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub application_name: Option<String>,
    pub media_class: String,
    pub node_type: NodeType
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    ApplicationOutput,
    ApplicationInput,
    SinkDevice,
    SourceDevice,
    Virtual,
    Unknown
}

#[derive(Debug, Clone)]
pub struct PortInfo {
    pub id: u32,
    pub node_id: u32,
    pub name: String,
    pub direction: PortDirection,
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PortDirection {
    Input,
    Output,
}

/// Information about a link between two ports
#[derive(Debug, Clone)]
pub struct LinkInfo {
    pub id: u32,
    pub output_port: u32,
    pub input_port: u32,
    pub output_node: u32,
    pub input_node: u32,
}

/// Information about a virtual sink created by Amplitude
#[derive(Debug, Clone)]
pub struct VirtualSinkInfo {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
}
