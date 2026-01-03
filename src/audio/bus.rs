use crate::core::app::Message;

use iced::widget::{
    text,
    progress_bar,
    button,
    row,
    container
};

use iced::{
    Border,
    Color,
    Background,
    padding,
};

#[derive(Debug, Clone)]
pub struct Bus {
    name: String,
    level: f32,
    muted: bool,
}

impl Bus {
    pub fn new(bus_name: String) -> Self {
        Bus {
            name: bus_name,
            level: 0.8,
            muted: false,
        }
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        let bus_strip = row![
            text(self.name.clone()),
            progress_bar(0.0..=1.0, self.level),
            button("Mute").style(match self.muted {
                true => button::danger,
                false => button::primary,
            })
        ].spacing(10);

        container(bus_strip)
            .padding(padding::vertical(10).horizontal(20))
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgb(0.1, 0.1, 0.1))),
                border: Border {
                    color: Color::from_rgb(0.1, 0.1, 0.1),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}
