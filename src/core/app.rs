use iced::widget::{button, column, text, Column};

#[derive(Debug, Default, Clone)]
pub struct App {
    value: i32,
}

#[derive(Debug, Clone)]
pub enum Message {
    Increment,
    Decrement
}

impl App {
    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Increment => self.value += 1,
            Message::Decrement => self.value -= 1,
        };
    }

    pub fn view(&self) -> Column<Message> {
        let increment = button("+").on_press(Message::Increment);
        let decrement = button("-").on_press(Message::Decrement);

        let counter = text(self.value);

        let interface = column![increment, counter, decrement];

        interface
    }
}
