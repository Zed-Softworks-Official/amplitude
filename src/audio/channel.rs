use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::core::{
    app::Message,
    icon::{
        Icon,
        get_icon
    },
};

use crate::audio::bus::BusOptions;

use iced::widget::{
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    #[serde(with = "uuid::serde::compact")]
    pub id: Uuid,
    pub icon: Icon,
    pub channel_name: String,
    pub monitor_bus_options: BusOptions,
    pub stream_bus_options: BusOptions
}

impl Channel {
    pub fn new(
        channel_name: String,
        icon: Icon
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel_name,
            icon,
            monitor_bus_options: BusOptions::new(0.8, false),
            stream_bus_options: BusOptions::new(0.8, false),
        }
    }

    pub fn from_config(channel: &Channel) -> Self {
        Self {
            id: channel.id,
            channel_name: channel.channel_name.to_string(),
            icon: channel.icon.clone(),
            monitor_bus_options: BusOptions::new(channel.monitor_bus_options.volume, channel.monitor_bus_options.muted),
            stream_bus_options: BusOptions::new(channel.stream_bus_options.volume, channel.stream_bus_options.muted),
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
                get_icon(Icon::Monitor).size(20),
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
                get_icon(Icon::Stream).size(20),
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
                    get_icon(self.icon.clone()),
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
