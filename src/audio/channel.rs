use uuid::Uuid;

use crate::core::app::Message;
use crate::audio::bus::{BusOptions};

use iced::widget::{
    Text,
    text,
    container,
    column,
    progress_bar,
    button,
    row,
    vertical_slider,
};

use iced::{
    padding,
    Background,
    Color,
    Border,
    Alignment
};

use lucide_icons::iced::{
    icon_headphones,
    icon_podcast
};

#[derive(Debug, Clone)]
pub struct Channel {
    pub id: Uuid,
    pub icon: fn() -> Text<'static>,
    pub channel_name: String,
    pub monitor_bus_options: BusOptions,
    pub stream_bus_options: BusOptions
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
            monitor_bus_options: BusOptions::new(0.8, false),
            stream_bus_options: BusOptions::new(0.8, false),
        }
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        let channel_name = text(self.channel_name.clone());

        let sliders = container(row![
            column![
                vertical_slider(
                    0.0..=1.0,
                    self.monitor_bus_options.volume,
                    |v| Message::MonitorVolumeChanged(self.id, v)
                ).step(0.1),
                icon_headphones().size(20),
                button(text("Mute"))
                    .on_press(Message::MonitorMuteToggled(self.id))
                    .style(match self.monitor_bus_options.muted {
                        true => button::danger,
                        false => button::primary
                    })
            ].spacing(20).align_x(Alignment::Center),
            column![
                vertical_slider(
                    0.0..=1.0,
                    self.stream_bus_options.volume,
                    |v| Message::StreamVolumeChanged(self.id, v)
                ).step(0.1),
                icon_podcast().size(20),
                button(text("Mute"))
                    .on_press(Message::StreamMuteToggled(self.id))
                    .style(match self.stream_bus_options.muted {
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
                progress_bar(0.0..=1.0, self.monitor_bus_options.level),
                sliders
            ].spacing(10))
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

#[derive(Debug, Clone, Default)]
pub struct NewChannelData {
    pub name: String,
}
