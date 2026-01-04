use uuid::Uuid;
use std::time::Duration;
use std::sync::Arc;
use log::info;
use lucide_icons::iced::icon_plus;
use futures::SinkExt;

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
    Alignment,
    Theme,
    Border,
};

use crate::audio::{
    NewChannelData,
    audio_manager::{AudioManager, ChannelBus}
};

use crate::core::{
    config::Config,
    modal::{Modal, modal}
};

use crate::pipewire::pw_core::{PwCore, PwEvent};


pub struct App {
    audio_manager: AudioManager,
    config: Config,

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

    // PipeWire
    PipeWireEvent(PwEvent),

    // Create Channel Modal
    ShowCreateChannelModal,
    HideCreateChannelModal,
    NewChannelContentChanged(String),

    // App Names Modal
    ShowAppNamesModal(Uuid),
}

impl App {
    pub fn subscription(&self) -> iced::Subscription<Message> {
        pw_event_subscription(self.audio_manager.get_event_receiver())
    }

    pub fn new() -> Self {
        let config = Config::load();

        Self {
            audio_manager: AudioManager::new(config.clone(), PwCore::new()),
            config,
            create_channel_modal: Modal::new(NewChannelData {
                name: "".to_string()
            }),
        }
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::ShowCreateChannelModal => {
                self.create_channel_modal.show();
            },
            Message::HideCreateChannelModal => {
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

                self.config.save(Some(self.audio_manager.get_channels().clone()));
            },
            Message::MonitorVolumeChanged(uuid, volume) => {
                info!("Monitor Volume Changed: {} (uuid: {})", volume, uuid);
                self.audio_manager.update_volume(uuid, volume, ChannelBus::Monitor);
            },
            Message::StreamVolumeChanged(uuid, volume) => {
                info!("Stream Volume Changed: {} (uuid: {})", volume, uuid);
                self.audio_manager.update_volume(uuid, volume, ChannelBus::Stream);
            }
            Message::MonitorMuteToggled(uuid) => {
                info!("Monitor Mute Toggled: {}", uuid);
                self.audio_manager.toggle_mute(uuid, ChannelBus::Monitor);
            },
            Message::StreamMuteToggled(uuid) => {
                info!("Stream Mute Toggled: {}", uuid);
                self.audio_manager.toggle_mute(uuid, ChannelBus::Stream);
            }
            Message::PipeWireEvent(event) => {
                self.audio_manager.process_events();
                info!("PipeWire Event: {:?}", event);

                // TODO: Actually Do Things
                // Handle the event - you can add your logic here
                // For example, update UI state based on node additions/removals
            }
            Message::NewChannelContentChanged(content) => {
                self.create_channel_modal.data.as_mut().unwrap().name = content;
            },
            Message::ShowAppNamesModal(uuid) => {
                info!("Show App Names Modal: {}", uuid);
            },
        };
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        // Channel Button
        let button_content = container(column![
            icon_plus().size(35),
            text("Add Channel").size(20).center()
        ].spacing(10).align_x(Alignment::Center)
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center);

        let add_channel_button = button(button_content)
            .on_press(Message::ShowCreateChannelModal)
            .width(Length::Fixed(100.0))
            .height(Length::Fill)
            .style(|theme: &Theme, status| {
                let mut style = button::secondary(theme, status);
                style.border = Border {
                    radius: 8.0.into(),
                    ..style.border
                };

                style
            });

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
            modal(interface, self.new_channel_modal(), Message::HideCreateChannelModal)
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
                        .on_press(Message::HideCreateChannelModal)
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

    fn app_names_modal(&self, uuid: Uuid) -> iced::Element<'_, Message> {
        let channel = self.audio_manager.get_channels().get(&uuid).clone();
        let nodes = self.audio_manager.get_nodes();

        container(column![
            text("Routing to this channel").center(),
            text(channel.as_ref().unwrap().app_names.join(", ")),
            text("Souces not routed to this channel").center(),
            row(
                nodes
                    .iter()
                    .filter(|(_id, node)| node.media_class.is_input())
                    .map(|(_id, node)| text(node.name.clone()).into())
            ).spacing(10),
        ].spacing(10))
            .padding(padding::vertical(10).horizontal(20))
            .style(container::rounded_box)
            .max_width(400)
            .into()
    }
}

// Subscription for PipeWire events
#[derive(Clone)]
struct PwEventReceiver(Arc<std::sync::Mutex<tokio::sync::mpsc::Receiver<PwEvent>>>);

impl std::hash::Hash for PwEventReceiver {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&*self.0, state);
    }
}

fn pw_event_subscription(
    receiver: Arc<std::sync::Mutex<tokio::sync::mpsc::Receiver<PwEvent>>>
) -> iced::Subscription<Message> {
    iced::Subscription::run_with(
        PwEventReceiver(receiver),
        pw_event_worker
    )
}

fn pw_event_worker(
    receiver_wrapper: &PwEventReceiver
) -> iced::futures::stream::BoxStream<'static, Message> {
    let receiver = Arc::clone(&receiver_wrapper.0);

    Box::pin(iced::stream::channel(100, move |mut output: futures::channel::mpsc::Sender<Message>| {
        let receiver = Arc::clone(&receiver);
        async move {
            loop {
                // Try to receive events from PipeWire
                let event = {
                    let mut rx = receiver.lock().unwrap();
                    rx.try_recv().ok()
                };

                if let Some(event) = event {
                    let _ = output.send(Message::PipeWireEvent(event)).await;
                }

                tokio::time::sleep(Duration::from_millis(16)).await;
            }
        }
    }))
}

