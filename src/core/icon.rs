use serde::{Serialize, Deserialize};

use lucide_icons::iced::{
    icon_headphones,
    icon_headphone_off,
    icon_wifi,
    icon_wifi_off,
    icon_speaker
};

#[derive(Debug, PartialEq, Clone, Eq, Serialize, Deserialize)]
pub enum Icon {
    Speaker,
    Monitor,
    MonitorMuted,
    Stream,
    StreamMuted
}

pub fn get_icon(icon: Icon) -> iced::widget::Text<'static> {
    match icon {
        Icon::Speaker => icon_speaker(),
        Icon::Monitor => icon_headphones(),
        Icon::MonitorMuted => icon_headphone_off(),
        Icon::Stream => icon_wifi(),
        Icon::StreamMuted => icon_wifi_off()
    }
}
