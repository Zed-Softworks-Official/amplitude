use uuid::Uuid;

use crate::core::app::Message;

use iced::widget::{
    Text,
    text,
    column,
    button,
    row,
    vertical_slider,
};

#[derive(Debug, Clone)]
pub struct Channel {
    pub id: Uuid,
    pub icon: fn() -> Text<'static>,
    pub channel_name: String,
    pub monitor_volume: f32,
    pub monitor_mute: bool,
    pub stream_volume: f32,
    pub stream_mute: bool
}

impl Channel {
    pub fn new(
        channel_name: String,
        icon: fn() -> iced::widget::Text<'static>
    ) -> Self {
        Channel {
            id: Uuid::new_v4(),
            channel_name,
            icon,
            monitor_volume: 0.8,
            monitor_mute: false,
            stream_volume: 0.8,
            stream_mute: false,
        }
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        let channel_name = text(self.channel_name.clone());

        let sliders = row![
            column![
                text("Monitor"),
                vertical_slider(
                    0.0..=1.0,
                    self.monitor_volume,
                    Message::MonitorVolumeChanged
                ).step(0.1),
                button(text("Mute"))
                    .on_press(Message::MonitorMuteToggled)
                    .style(match self.monitor_mute {
                        true => button::danger,
                        false => button::primary
                    })
            ].spacing(20),
            column![
                text("Stream"),
                vertical_slider(
                    0.0..=1.0,
                    self.stream_volume,
                    Message::StreamVolumeChanged
                ).step(0.1),
                button(text("Mute"))
                    .on_press(Message::StreamMuteToggled)
                    .style(match self.stream_mute {
                        true => button::danger,
                        false => button::primary
                    })
            ].spacing(20)
        ].spacing(10);

        column![
            row![
                (self.icon)(),
                channel_name
            ].spacing(10),
            sliders
        ].into()
    }
}
