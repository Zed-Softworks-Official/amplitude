use uuid::Uuid;

use crate::core::app::Message;

use iced::widget::{
    Text,
    text,
    container,
    column,
    button,
    row,
    vertical_slider,
};

use iced::{
    padding,
    Background,
    Color,
    Border,
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

        let sliders = container(row![
            column![
                text("Monitor"),
                vertical_slider(
                    0.0..=1.0,
                    self.monitor_volume,
                    |v| Message::MonitorVolumeChanged(self.id, v)
                ).step(0.1),
                button(text("Mute"))
                    .on_press(Message::MonitorMuteToggled(self.id))
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
                    |v| Message::StreamVolumeChanged(self.id, v)
                ).step(0.1),
                button(text("Mute"))
                    .on_press(Message::StreamMuteToggled(self.id))
                    .style(match self.stream_mute {
                        true => button::danger,
                        false => button::primary
                    })
            ].spacing(20)
        ].spacing(10));

        container(
            column![
                row![
                    (self.icon)(),
                    channel_name
                ].spacing(10),
                sliders
            ])
            .padding(padding::top(20).left(10).right(10).bottom(20))
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.15))),
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: Color::from_rgb(0.15, 0.15, 0.15),
                },
                ..Default::default()
            })
            .into()
    }
}
