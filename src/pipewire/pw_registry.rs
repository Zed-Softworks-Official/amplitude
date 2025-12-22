use pw::{registry::GlobalObject};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AudioNode {
    pub id: u32,
    pub name: String,
    pub app_name: Option<String>,
    pub binary_name: Option<String>,
    pub media_class: String,
    pub node_type: NodeType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    AudioSink,
    AudioSource,
    StreamSink,
    StreamSource,
    VirtualSink,
    Unknown
}

pub struct PwRegistry {
    nodes: HashMap<u32, AudioNode>
}

impl PwRegistry {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new()
        }
    }

    pub fn add_node(&mut self, global: &GlobalObject<&pw::spa::utils::dict::DictRef>) {
        if global.type_ != pw::types::ObjectType::Node {
            return;
        }

        let props = global.props.as_ref();
        let media_class = props
            .and_then(|p| p.get("media.class"))
            .unwrap_or("")
            .to_string();

        // Filter only audio nodes
        if !media_class.starts_with("Audio/") && !media_class.starts_with("Stream/") {
            return;
        }

        let node = AudioNode {
            id: global.id,
            name: props
                .and_then(|p| p.get("node.name"))
                .unwrap_or("Unknown")
                .to_string(),
            app_name: props
                .and_then(|p| p.get("application.name"))
                .map(|s| s.to_string()),
            binary_name: props
                .and_then(|p| p.get("application.process.binary"))
                .map(|s| s.to_string()),
            media_class: media_class.clone(),
            node_type: Self::classify_node(&media_class)
        };

        log::info!("Node added: {:?}", node);

        self.nodes.insert(global.id, node);
    }

    pub fn remove_node(&mut self, id: &u32) {
        log::info!("Node removed: {}", id);
        self.nodes.remove(id);
    }

    pub fn get_node(&self, id: u32) -> Option<&AudioNode> {
        self.nodes.get(&id)
    }

    pub fn get_playback_stream(&self) -> Vec<&AudioNode> {
        self.nodes
            .values()
            .filter(|n| n.node_type == NodeType::StreamSink)
            .collect()
    }

    fn classify_node(media_class: &str) -> NodeType {
        match media_class {
            "Audio/Sink" => NodeType::AudioSink,
            "Audio/Source" => NodeType::AudioSource,
            "Stream/Output/Audio" => NodeType::StreamSink,
            "Stream/Input/Audio" => NodeType::StreamSource,
            _ => NodeType::Unknown
        }
    }
}
