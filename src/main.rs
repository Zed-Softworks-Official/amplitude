mod core;
mod audio;
mod pipewire;

use lucide_icons::LUCIDE_FONT_BYTES;
use crate::core::app::App;

use iced::{
    Theme,
    Color,
};
use iced::theme::Palette;

#[tokio::main]
async fn main() -> iced::Result {
    let settings = iced::Settings {
        fonts: vec![LUCIDE_FONT_BYTES.into()],
        ..Default::default()
    };

    iced::application(App::new, App::update, App::view)
        .theme(theme())
        .settings(settings)
        .title("Amplitude")
        .run()
}

pub fn theme() -> Theme {
    Theme::custom("Amplitude".to_string(), Palette {
        background: Color::from_rgb(0.051, 0.051, 0.051), // #0d0d0d
        text: Color::from_rgb(0.980, 0.980, 0.980),       // #fafafa
        primary: Color::from_rgb(0.392, 0.412, 0.941),    // #6469F0
        success: Color::from_rgb(0.180, 0.545, 0.341),    // #2e8b57
        warning: Color::from_rgb(0.900, 0.580, 0.200),    // #e69433
        danger: Color::from_rgb(0.600, 0.176, 0.176),     // #992d2d
    })
}
