use uuid::Uuid;

use iced::widget::{
    button,
    column,
    text_input,
    row,
    text,
    container,
    Scrollable,
    scrollable::{Direction, Scrollbar},
};

use iced::{
    Length,
    padding,
};

use crate::audio::audio_manager::{AudioManager, ChannelBus};
use crate::audio::NewChannelData;
use crate::core::modal::{Modal, modal};

#[derive(Default)]
pub struct App {
    audio_manager: AudioManager,
    create_channel_modal: Modal<NewChannelData>,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Channels
    AddChannel,

    // Audio
    MonitorVolumeChanged(Uuid, f32),
    StreamVolumeChanged(Uuid, f32),

    MonitorMuteToggled(Uuid),
    StreamMuteToggled(Uuid),

    // Modal
    ShowModal,
    HideModal,
    NewChannelContentChanged(String)
}

impl App {
    pub fn new() -> Self {
        Self {
            audio_manager: AudioManager::new(),
            create_channel_modal: Modal::new(NewChannelData {
                name: "".to_string()
            })
        }
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::ShowModal => {
                self.create_channel_modal.show();
            },
            Message::HideModal => {
                self.create_channel_modal.hide();
            },
            Message::AddChannel => {
                self.audio_manager.add_channel(
                    self.create_channel_modal.data
                        .as_ref()
                        .unwrap()
                        .name
                        .as_str()
                );
            },
            Message::MonitorVolumeChanged(uuid, volume) => {
                println!("Monitor Volume Changed: {} (uuid: {})", volume, uuid);
                self.audio_manager.update_volume(uuid, volume, ChannelBus::Monitor);
            },
            Message::StreamVolumeChanged(uuid, volume) => {
                println!("Stream Volume Changed: {} (uuid: {})", volume, uuid);
                self.audio_manager.update_volume(uuid, volume, ChannelBus::Stream);
            }
            Message::MonitorMuteToggled(uuid) => {
                println!("Monitor Mute Toggled");
                self.audio_manager.toggle_mute(uuid, ChannelBus::Monitor);
            },
            Message::StreamMuteToggled(uuid) => {
                println!("Stream Mute Toggled");
                self.audio_manager.toggle_mute(uuid, ChannelBus::Stream);
            }
            Message::NewChannelContentChanged(content) => {
                self.create_channel_modal.data.as_mut().unwrap().name = content;
            }
        };
    }

    pub fn view(&self) -> iced::Element<Message> {
        // Channel Button
        let add_channel_button = button(text("+").center())
            .on_press(Message::ShowModal)
            .height(Length::Fill);

        // Channel Strips
        let channels = row(
            self.audio_manager.get_channels()
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

        // Buses
        let busses = column(
            self.audio_manager.get_busses()
                .iter()
                .map(|(_name, bus)| bus.view().into())
        ).spacing(10);

        let bus_strip = container(column![text("OUTPUT"), busses].spacing(10))
            .padding(padding::top(10).bottom(20).horizontal(20));

        let interface = column![channel_section, bus_strip]
            .spacing(20);

        if self.create_channel_modal.show_modal {
            modal(interface, self.new_channel_modal(), Message::HideModal)
        } else {
            interface.into()
        }
    }

    fn new_channel_modal(&self) -> iced::Element<'_, Message> {
        container(column![
            text("Create a new channel"),
            text_input(
                "Channel Name",
                &self.create_channel_modal.data.as_ref().unwrap().name,
            )
                .on_input(Message::NewChannelContentChanged)
                .padding(5),
            row![
                container(
                    button(text("Cancel"))
                        .on_press(Message::HideModal)
                        .style(button::danger)
                ).align_left(iced::Fill),
                container(
                    button(text("Create"))
                        .on_press(Message::AddChannel)
                        .style(button::primary)

                ).align_right(iced::Fill),
            ].width(Length::Fill)
        ].spacing(10))
            .padding(padding::vertical(10).horizontal(20))
            .style(container::rounded_box)
            .max_width(400)
            .into()
    }
}
