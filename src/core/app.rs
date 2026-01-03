use uuid::Uuid;

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

use crate::audio::channel_manager::{ChannelManager, ChannelBus};

#[derive(Debug, Default, Clone)]
pub struct App {
    channel_manager: ChannelManager,
}

#[derive(Debug, Clone)]
pub enum Message {
    AddChannel,

    MonitorVolumeChanged(Uuid, f32),
    StreamVolumeChanged(Uuid, f32),

    MonitorMuteToggled(Uuid),
    StreamMuteToggled(Uuid),
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
            Message::MonitorVolumeChanged(uuid, volume) => {
                println!("Monitor Volume Changed: {} (uuid: {})", volume, uuid);
                self.channel_manager.update_volume(uuid, volume, ChannelBus::Monitor);
            },
            Message::StreamVolumeChanged(uuid, volume) => {
                println!("Stream Volume Changed: {} (uuid: {})", volume, uuid);
                self.channel_manager.update_volume(uuid, volume, ChannelBus::Stream);
            }
            Message::MonitorMuteToggled(uuid) => {
                println!("Monitor Mute Toggled");
                self.channel_manager.toggle_mute(uuid, ChannelBus::Monitor);
            },
            Message::StreamMuteToggled(uuid) => {
                println!("Stream Mute Toggled");
                self.channel_manager.toggle_mute(uuid, ChannelBus::Stream);
            }
        };
    }

    pub fn view(&self) -> Column<Message> {
        // Channel Button
        let add_channel_button = button(text("+").center())
            .on_press(Message::AddChannel)
            .height(Length::Fill);

        // Channel Strips
        let channels = row(
            self.channel_manager.get_channels()
                .iter()
                .map(|(_id, channel)| channel.view().into())
        ).spacing(10);

        let channel_strip = Scrollable::new(channels)
            .direction(Direction::Horizontal(Scrollbar::new()));

        // Channel Section
        let channel_section = container(
            column![
                text("INPUTS"),
                row![channel_strip, add_channel_button]
                    .spacing(20)
            ].spacing(10)
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
