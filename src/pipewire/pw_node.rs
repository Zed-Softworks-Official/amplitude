use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PwNode {
    pub id: u32,
    pub name: String,
    pub nick: Option<String>,
    pub media_class: MediaClass,
    pub app_info: Option<AppInfo>,
    pub device_info: Option<DeviceInfo>,
    pub sample_rate: Option<u32>,
    pub format: Option<String>
}

impl PwNode {
    pub fn from_props(id: u32, props: &HashMap<String, String>) -> Self {
        let media_class = props
            .get("media.class")
            .map(|class| MediaClass::from_str(class))
            .unwrap_or(MediaClass::Unknown);

        let app_info = if media_class.is_stream() {
            Some(AppInfo {
                name: props.get("application.name").cloned(),
                binary: props.get("application.process.binary").cloned(),
                pid: props
                    .get("application.process.id")
                    .and_then(|pid| pid.parse().ok()),
                icon_name: props
                    .get("application.icon-name")
                    .or_else(|| props.get("application.process.binary"))
                    .cloned(),
            })
        } else {
            None
        };

        let device_info = if media_class.is_device() {
            Some(DeviceInfo {
                description: props.get("device.description").cloned(),
                card_name: props.get("device.name").cloned(),
                is_default: false,
            })
        } else {
            None
        };

        Self {
            id,
            name: props
                .get("node.name")
                .cloned()
                .unwrap_or_else(|| format!("Node {}", id)),
            nick: props.get("node.nick").cloned(),
            media_class,
            app_info,
            device_info,
            sample_rate: props
                .get("audio.rate")
                .and_then(|rate| rate.parse().ok()),
            format: props.get("audio.format").cloned()
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
    Unknown
}

impl MediaClass {
    pub fn from_str(class: &str) -> Self {
        match class {
            "Audio/Sink" => Self::AudioSink,
            "Audio/Source" => Self::AudioSource,
            "Stream/Output/Audio" => Self::StreamOutputAudio,
            "Stream/Input/Audio" => Self::StreamInputAudio,
            _ => Self::Unknown
        }
    }

    pub fn is_device(&self) -> bool {
        matches!(self, Self::AudioSink | Self::AudioSource)
    }

    pub fn is_stream(&self) -> bool {
        matches!(self, Self::StreamOutputAudio | Self::StreamInputAudio)
    }

    pub fn is_output(&self) -> bool {
        matches!(self, Self::StreamOutputAudio | Self::AudioSink)
    }

    pub fn is_input(&self) -> bool {
        matches!(self, Self::StreamInputAudio | Self::AudioSource)
    }
}
