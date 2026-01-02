mod core;

use crate::core::app::{App};

fn main() -> iced::Result {
    iced::run(App::update, App::view)
}
