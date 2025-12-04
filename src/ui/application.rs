use spdlog::prelude::*;

use gtk4 as gtk;
use gtk::glib;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::audio::{AudioEvent, AudioManager, NodeType};

pub struct AmplitudeApplication {
    gtk_app: Application,
    audio_manager: Arc<RwLock<AudioManager>>
}

impl AmplitudeApplication {
    pub async fn new() -> anyhow::Result<Self> {
        let gtk_app = Application::builder()
            .application_id("dev.zedsoftworks.amplitude")
            .build();

        let audio_manager = Arc::new(RwLock::new(AudioManager::new().await?));

        Ok(Self {
            gtk_app,
            audio_manager
        })
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let audio_manager = self.audio_manager.clone();

        self.gtk_app.connect_activate(move |app| {
            let am = audio_manager.clone();
            build_ui(app, am);
        });

        let am = self.audio_manager.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut manager = am.write().await;
                if let Err(e) = manager.run().await {
                    error!("Audio Manager error: {}", e);
                }
            });
        });

        self.gtk_app.run();
        Ok(())
    }
}

fn build_ui(app: &Application, audio_manager: Arc<RwLock<AudioManager>>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Amplitude")
        .default_width(1280)
        .default_height(720)
        .build();

    // Main Layout
    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    // Header
    let header = gtk::HeaderBar::new();
    let title = gtk::Label::new(Some("Amplitude"));
    header.set_title_widget(Some(&title));
    main_box.append(&header);

    // Channel Container
    let channel_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    channel_box.set_margin_top(16);
    channel_box.set_margin_bottom(16);
    channel_box.set_margin_start(16);
    channel_box.set_margin_end(16);
    channel_box.set_halign(gtk::Align::Start);

    // Wrap in scrollable area
    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .child(&channel_box)
        .vexpand(true)
        .build();

    main_box.append(&scrolled);
    window.set_child(Some(&main_box));

    // Setup event handler
    setup_event_handler(audio_manager, channel_box);

    window.present();
}

fn setup_event_handler(
    audio_manager: Arc<RwLock<AudioManager>>,
    channel_box: gtk::Box
) {
    // Create an async channel for sending events to the GTK main thread
    let (tx, rx) = async_channel::unbounded::<AudioEvent>();

    // Spawn a background thread that runs a Tokio runtime to receive/broadcast audio events
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            // First, fetch any existing nodes that were discovered before we subscribed
            {
                let manager = audio_manager.read().await;
                let existing_apps = manager.get_playback_applications().await;
                for info in existing_apps {
                    let _ = tx.send(AudioEvent::NodeAdded(info)).await;
                }
            }

            // Now subscribe and listen for new events
            let manager = audio_manager.read().await;
            let mut sub = manager.subscribe();

            while let Ok(event) = sub.recv().await {
                // TODO: Remove this later when do more work for background tasks
                if tx.send(event).await.is_err() {
                    break;
                }
            }
        });
    });

    // Handle events on the GTK main thread using spawn_local
    glib::spawn_future_local(async move {
        while let Ok(event) = rx.recv().await {
            match event {
                AudioEvent::NodeAdded(info) => {
                    if matches!(info.node_type, NodeType::ApplicationOutput) {
                        let channel = create_channel_strip(&info.name);
                        channel_box.append(&channel);
                    }
                }
                AudioEvent::NodeRemoved { id } => {
                    warn!("Would remove channel for node {}", id);
                }
                _ => {}
            }
        }
    });
}

fn create_channel_strip(name: &str) -> gtk::Box {
    let channel = gtk::Box::new(gtk::Orientation::Vertical, 4);
    channel.set_margin_start(4);
    channel.set_margin_end(4);

    // Label
    let label = gtk::Label::new(Some(name));
    label.set_max_width_chars(10);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    channel.append(&label);

    // Monitor Selection
    let monitor_label = gtk::Label::new(Some("MON"));
    channel.append(&monitor_label);

    let monitor_scale = gtk::Scale::with_range(gtk::Orientation::Vertical, 0.0, 1.0, 0.01);
    monitor_scale.set_inverted(true);
    monitor_scale.set_value(0.8);
    monitor_scale.set_vexpand(true);
    channel.append(&monitor_scale);

    let monitor_mute = gtk::ToggleButton::with_label("M");
    channel.append(&monitor_mute);

    // Stream Section
    let stream_label = gtk::Label::new(Some("STR"));
    channel.append(&stream_label);

    let stream_scale = gtk::Scale::with_range(gtk::Orientation::Vertical, 0.0, 1.0, 0.01);
    stream_scale.set_inverted(true);
    stream_scale.set_value(0.7);
    stream_scale.set_vexpand(true);
    channel.append(&stream_scale);

    let stream_mute = gtk::ToggleButton::with_label("M");
    channel.append(&stream_mute);

    channel
}
