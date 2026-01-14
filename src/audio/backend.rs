use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub trait AudioBackend {
    fn new() -> Self
    where
        Self: Sized;
    fn send_command(&self, cmd: BackendCommand);
    fn get_event_receiver(&self) -> Arc<Mutex<mpsc::Receiver<AudioEvent>>>;
    fn process_event(&self, event: AudioEvent);
    fn get_nodes(&self) -> Arc<Mutex<HashMap<u32, AudioNode>>>;
}

// Commands Sent FROM tokio TO audio backend
#[derive(Debug, Clone)]
pub enum BackendCommand {
    Terminate,
}

// Events sent FROM audio backend TO tokio
#[derive(Debug, Clone)]
pub enum AudioEvent {
    NodeAdded(AudioNode),
    NodeRemoved(u32),
}

#[derive(Debug, Clone)]
pub struct AudioNode {
    pub id: u32,
    pub name: String,
    pub nick: Option<String>,
    pub media_class: MediaClass,
    pub app_info: Option<AppInfo>,
    pub device_info: Option<DeviceInfo>,
    pub sample_rate: Option<u32>,
    pub format: Option<String>,
}

impl AudioNode {
    #[cfg(target_os = "linux")]
    pub fn from_props(id: u32, props: &HashMap<String, String>) -> Self {
        let name = props
            .get("name")
            .cloned()
            .unwrap_or_default();
        let nick = props.get("nick").cloned();
        let media_class = MediaClass::from_str(
            &props
                .get("media.class")
                .cloned()
                .unwrap_or_default(),
        );
        let app_info = props
            .get("application.name")
            .cloned()
            .map(|name| AppInfo {
                name: Some(name),
                binary: None,
                pid: None,
                icon_name: None,
            });
        let device_info = props
            .get("device.description")
            .cloned()
            .map(|desc| DeviceInfo {
                description: Some(desc),
                card_name: None,
                is_default: false,
            });
        let sample_rate = props
            .get("format.sample_rate")
            .cloned()
            .map(|rate| rate.parse::<u32>().unwrap_or_default());
        let format = props.get("format.format").cloned();

        Self {
            id,
            name,
            nick,
            media_class,
            app_info,
            device_info,
            sample_rate,
            format,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppInfo {
    pub name: Option<String>,
    pub binary: Option<String>,
    pub pid: Option<u32>,
    pub icon_name: Option<String>,
}

impl AppInfo {
    pub fn display_name(&self) -> &str {
        self.name
            .as_deref()
            .or(self.binary.as_deref())
            .unwrap_or("Unknown Application")
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub description: Option<String>,
    pub card_name: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MediaClass {
    AudioSink,
    AudioSource,
    StreamOutputAudio,
    StreamInputAudio,
    Unknown,
}

impl MediaClass {
    #[cfg(target_os = "linux")]
    pub fn from_str(class: &str) -> Self {
        match class {
            "Audio/Sink" => MediaClass::AudioSink,
            "Audio/Source" => MediaClass::AudioSource,
            "Stream/Output/Audio" => MediaClass::StreamOutputAudio,
            "Stream/Input/Audio" => MediaClass::StreamInputAudio,
            _ => MediaClass::Unknown,
        }
    }
}
