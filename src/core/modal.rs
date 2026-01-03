use iced::widget::{
    stack,
    opaque,
    mouse_area,
    center,
    container
};

use iced::Color;

#[derive(Default)]
pub struct Modal<T> {
    pub show_modal: bool,
    pub data: Option<T>
}

impl<T> Modal<T> {
    pub fn new(data: T) -> Self {
        Self {
            show_modal: false,
            data: Some(data)
        }
    }

    pub fn show(&mut self) {
        self.show_modal = true;
    }

    pub fn hide(&mut self) {
        self.show_modal = false;
    }
}

pub fn modal<'a, Message>(
    base: impl Into<iced::Element<'a, Message>>,
    content: impl Into<iced::Element<'a, Message>>,
    on_blur: Message,
) -> iced::Element<'a, Message>
    where
    Message: Clone + 'a,
{
    stack![
        base.into(),
        opaque(
            mouse_area(center(opaque(content)).style(|_theme| {
                container::Style {
                    background: Some(Color {
                        a: 0.8,
                        ..Color::BLACK
                    }.into()
                    ),
                    ..container::Style::default()
                }
            }))
                .on_press(on_blur)
        )
    ].into()
}
