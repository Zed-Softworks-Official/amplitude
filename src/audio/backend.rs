use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};

pub trait AudioBackend {
    fn new() -> Self where Self: Sized;
    fn send_command(&self, cmd: BackendCommand);
    fn get_event_receiver(&self) -> Arc<Mutex<mpsc::Receiver<AudioEvent>>>;
    fn process_events(&self);
}

// Commands Sent FROM tokio TO audio backend
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
    pub format: Option<String>
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
    Unknown
}

//pub trait MediaClass {
//    fn from_str(class: &str) -> Self;
//}
