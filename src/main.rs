mod core;
mod audio;

use lucide_icons::LUCIDE_FONT_BYTES;
use iced::Theme;
use crate::core::app::{App};

fn main() -> iced::Result {
    let settings = iced::Settings {
        fonts: vec![LUCIDE_FONT_BYTES.into()],
        ..Default::default()
    };

    iced::application(App::new, App::update, App::view)
        .theme(Theme::Dark)
        .settings(settings)
        .title("Amplitude")
        .run()
}
