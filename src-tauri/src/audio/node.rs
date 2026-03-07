use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Media class enum
// ---------------------------------------------------------------------------

/// Typed representation of PipeWire's `media.class` property.
/// Only classes relevant to Amplitude are modelled; everything else is
/// captured by `Other(String)` so unknown nodes are never silently dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
pub enum MediaClass {
    /// A physical or virtual audio output (sink).
    AudioSink,
    /// A physical or virtual audio input (source).
    AudioSource,
    /// An application stream producing audio (e.g. Spotify, browser).
    StreamOutputAudio,
    /// An application stream consuming audio.
    StreamInputAudio,
    /// Any other class not explicitly handled.
    Other(String),
}

impl MediaClass {
    /// Parse the raw `media.class` string value from PipeWire props.
    pub fn parse(s: &str) -> Self {
        match s {
            "Audio/Sink" => Self::AudioSink,
            "Audio/Source" => Self::AudioSource,
            "Stream/Output/Audio" => Self::StreamOutputAudio,
            "Stream/Input/Audio" => Self::StreamInputAudio,
            other => Self::Other(other.to_owned()),
        }
    }

    /// Returns `true` for classes that Amplitude surfaces to the frontend.
    pub fn is_relevant(&self) -> bool {
        matches!(
            self,
            Self::AudioSink
                | Self::AudioSource
                | Self::StreamOutputAudio
                | Self::StreamInputAudio
        )
    }
}

// ---------------------------------------------------------------------------
// NodeInfo
// ---------------------------------------------------------------------------

/// Metadata describing a single PipeWire node seen on the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    /// PipeWire global object ID.
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub app_name: Option<String>,
    pub app_binary: Option<String>,
    pub media_class: Option<MediaClass>,
    pub icon: Option<String>,
    /// True when this node was created by Amplitude itself.
    pub is_amplitude_virtual: bool,
}
