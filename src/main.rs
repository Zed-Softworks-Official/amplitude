mod audio;
mod core;
mod platform;

use crate::core::app::App;
use lucide_icons::LUCIDE_FONT_BYTES;

use env_logger::Env;
use iced::theme::Palette;
use iced::{Color, Theme};

fn main() -> iced::Result {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();

    let settings = iced::Settings {
        fonts: vec![LUCIDE_FONT_BYTES.into()],
        ..Default::default()
    };

    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .theme(theme())
        .settings(settings)
        .title("Amplitude")
        .run()
}

pub fn theme() -> Theme {
    Theme::custom(
        "Amplitude".to_string(),
        Palette {
            background: Color::from_rgb(0.051, 0.051, 0.051), // #0d0d0d
            text: Color::from_rgb(0.980, 0.980, 0.980),       // #fafafa
            primary: Color::from_rgb(0.392, 0.412, 0.941),    // #6469F0
            success: Color::from_rgb(0.180, 0.545, 0.341),    // #2e8b57
            warning: Color::from_rgb(0.900, 0.580, 0.200),    // #e69433
            danger: Color::from_rgb(0.600, 0.176, 0.176),     // #992d2d
        },
    )
}
