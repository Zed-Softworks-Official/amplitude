use iced::widget::{
    button,
    column,
    row,
    text,
    Column,
    container,
    Scrollable,
    scrollable::{Direction, Scrollbar},
};

use iced::{
    Length,
    padding,
};

use crate::audio::channel_manager::ChannelManager;

#[derive(Debug, Default, Clone)]
pub struct App {
    channel_manager: ChannelManager,
}

#[derive(Debug, Clone)]
pub enum Message {
    AddChannel,

    MonitorVolumeChanged(f32),
    StreamVolumeChanged(f32),

    MonitorMuteToggled,
    StreamMuteToggled,
}

impl App {
    pub fn new() -> Self {
        Self {
            channel_manager: ChannelManager::new(),
        }
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::AddChannel => {
                self.channel_manager.add_channel("Channel 1");
            },
            Message::MonitorVolumeChanged(volume) => {
                println!("Monitor Volume Changed: {}", volume);
            },
            Message::StreamVolumeChanged(volume) => {
                println!("Stream Volume Changed: {}", volume);
            }
            Message::MonitorMuteToggled => {
                println!("Monitor Mute Toggled");
            },
            Message::StreamMuteToggled => {
                println!("Stream Mute Toggled");
            }
        };
    }

    pub fn view(&self) -> Column<Message> {
        // Channel Button
        let add_channel_button = button(text("+").center())
            .on_press(Message::AddChannel)
            .height(Length::Fill);

        // Channel Strips
        let channels = column(
            self.channel_manager.get_channels()
                .iter()
                .map(|(_id, channel)| channel.view())
        ).spacing(10);

        let channel_strip = Scrollable::new(channels)
            .direction(Direction::Vertical(Scrollbar::new()));

        // Channel Section
        let channel_section = container(
            row![channel_strip, add_channel_button]
                .spacing(20)
        )
            .padding(padding::vertical(10).left(20).right(20))
            .style(container::rounded_box)
            .width(Length::Fill)
            .height(Length::Fill);

        let interface = column![channel_section]
            .spacing(20);

        interface
    }
}
