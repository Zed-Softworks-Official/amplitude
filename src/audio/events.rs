#[derive(Debug, Clone)]
pub enum AudioEvent {
    NodeAdded(NodeInfo),
    NodeRemoved { id: u32 },
    NodeChanged(NodeInfo),
    PortAdded(PortInfo),
    PortRemoved { id: u32, node_id: u32 },
    ConnectionChanged,
    Ready,
    Error(String)
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
    Output
}
