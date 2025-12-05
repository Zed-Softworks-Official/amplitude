use spdlog::prelude::*;

use gtk4 as gtk;
use gtk::glib;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::audio::{AudioEvent, AudioManager, ChannelManager, NodeInfo, NodeType};
use crate::config::Channel;

pub struct AmplitudeApplication {
    gtk_app: Application,
    audio_manager: Arc<RwLock<AudioManager>>,
}

impl AmplitudeApplication {
    pub async fn new() -> anyhow::Result<Self> {
        let gtk_app = Application::builder()
            .application_id("dev.zedsoftworks.amplitude")
            .build();

        let audio_manager = Arc::new(RwLock::new(AudioManager::new().await?));

        Ok(Self {
            gtk_app,
            audio_manager,
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

/// Shared UI state accessible from callbacks
struct UiState {
    channel_manager: ChannelManager,
    /// Maps node IDs to their info
    known_nodes: HashMap<u32, NodeInfo>,
    /// Maps channel names to their UI widgets
    channel_widgets: HashMap<String, ChannelWidget>,
    /// Container for channel strips
    channel_container: gtk::Box,
    /// Container for unassigned apps
    unassigned_container: gtk::Box,
    /// Maps node IDs to their unassigned app widgets
    unassigned_widgets: HashMap<u32, gtk::Box>,
    /// Main window reference for dialogs
    window: ApplicationWindow,
}

struct ChannelWidget {
    container: gtk::Frame,
    app_list: gtk::Box,
    monitor_scale: gtk::Scale,
    monitor_mute: gtk::ToggleButton,
    stream_scale: gtk::Scale,
    stream_mute: gtk::ToggleButton,
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

    // Content area with channels and unassigned section
    let content_box = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    content_box.set_margin_top(16);
    content_box.set_margin_bottom(16);
    content_box.set_margin_start(16);
    content_box.set_margin_end(16);
    content_box.set_vexpand(true);

    // Channel Container (left side - scrollable)
    let channel_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    channel_box.set_halign(gtk::Align::Start);

    // Add channel button
    let add_channel_btn = create_add_channel_button();
    channel_box.append(&add_channel_btn);

    let channel_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .child(&channel_box)
        .hexpand(true)
        .build();

    content_box.append(&channel_scroll);

    // Separator
    let separator = gtk::Separator::new(gtk::Orientation::Vertical);
    content_box.append(&separator);

    // Unassigned apps section (right side)
    let unassigned_frame = gtk::Frame::new(Some("Unassigned Apps"));
    let unassigned_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    unassigned_box.set_margin_top(8);
    unassigned_box.set_margin_bottom(8);
    unassigned_box.set_margin_start(8);
    unassigned_box.set_margin_end(8);

    let unassigned_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&unassigned_box)
        .width_request(200)
        .build();

    unassigned_frame.set_child(Some(&unassigned_scroll));
    content_box.append(&unassigned_frame);

    main_box.append(&content_box);
    window.set_child(Some(&main_box));

    // Create shared UI state
    let state = Rc::new(RefCell::new(UiState {
        channel_manager: ChannelManager::new(),
        known_nodes: HashMap::new(),
        channel_widgets: HashMap::new(),
        channel_container: channel_box.clone(),
        unassigned_container: unassigned_box,
        unassigned_widgets: HashMap::new(),
        window: window.clone(),
    }));

    // Load existing channels from config
    {
        let mut st = state.borrow_mut();
        let channels: Vec<Channel> = st.channel_manager.channels().to_vec();
        for channel in channels {
            create_channel_widget(&mut st, &channel.name);
        }
    }

    // Connect add channel button
    let state_clone = state.clone();
    add_channel_btn.connect_clicked(move |_| {
        show_create_channel_dialog(state_clone.clone());
    });

    // Setup event handler
    setup_event_handler(audio_manager, state);

    window.present();
}

fn setup_event_handler(audio_manager: Arc<RwLock<AudioManager>>, state: Rc<RefCell<UiState>>) {
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
                if tx.send(event).await.is_err() {
                    break;
                }
            }
        });
    });

    // Handle events on the GTK main thread
    glib::spawn_future_local(async move {
        while let Ok(event) = rx.recv().await {
            match event {
                AudioEvent::NodeAdded(info) => {
                    if matches!(info.node_type, NodeType::ApplicationOutput) {
                        handle_node_added(state.clone(), info);
                    }
                }
                AudioEvent::NodeRemoved { id } => {
                    handle_node_removed(state.clone(), id);
                }
                _ => {}
            }
        }
    });
}

fn handle_node_added(state: Rc<RefCell<UiState>>, info: NodeInfo) {
    let mut st = state.borrow_mut();

    // Get app name for matching
    let app_name = info
        .application_name
        .clone()
        .unwrap_or_else(|| info.name.clone());

    // Check if this node is already known
    if st.known_nodes.contains_key(&info.id) {
        return;
    }

    st.known_nodes.insert(info.id, info.clone());

    // Try to auto-assign to a channel
    if let Some(channel_name) = st.channel_manager.on_node_added(info.id, &app_name) {
        // Add to channel's app list in UI
        add_app_to_channel_ui(&mut st, &channel_name, &info);
    } else {
        // Add to unassigned section
        add_unassigned_app_ui(&mut st, &info, state.clone());
    }
}

fn handle_node_removed(state: Rc<RefCell<UiState>>, node_id: u32) {
    let mut st = state.borrow_mut();

    // Remove from channel manager
    st.channel_manager.on_node_removed(node_id);

    // Remove from known nodes
    st.known_nodes.remove(&node_id);

    // Remove from unassigned UI if present
    if let Some(widget) = st.unassigned_widgets.remove(&node_id) {
        st.unassigned_container.remove(&widget);
    }

    // Remove from channel UIs (search all channels)
    for (_, channel_widget) in &st.channel_widgets {
        // Find and remove the app widget by iterating children
        let mut child = channel_widget.app_list.first_child();
        while let Some(widget) = child {
            // Check if this widget has our node_id stored
            // Safety: We only store u32 values with this key
            if let Some(id) = unsafe { widget.data::<u32>("node_id") } {
                if unsafe { *id.as_ref() } == node_id {
                    channel_widget.app_list.remove(&widget);
                    break;
                }
            }
            child = widget.next_sibling();
        }
    }
}

fn create_channel_widget(state: &mut UiState, name: &str) {
    let channel_frame = gtk::Frame::new(Some(name));
    let channel = gtk::Box::new(gtk::Orientation::Vertical, 4);
    channel.set_margin_start(4);
    channel.set_margin_end(4);
    channel.set_margin_top(4);
    channel.set_margin_bottom(4);
    channel.set_width_request(150);

    // App list area
    let app_list_label = gtk::Label::new(Some("Applications:"));
    app_list_label.set_halign(gtk::Align::Start);
    channel.append(&app_list_label);

    let app_list = gtk::Box::new(gtk::Orientation::Vertical, 2);
    app_list.set_margin_bottom(8);
    channel.append(&app_list);

    // Monitor Section
    let monitor_label = gtk::Label::new(Some("Monitor"));
    channel.append(&monitor_label);

    let monitor_scale = gtk::Scale::with_range(gtk::Orientation::Vertical, 0.0, 1.0, 0.01);
    monitor_scale.set_inverted(true);
    monitor_scale.set_vexpand(true);
    monitor_scale.set_size_request(-1, 100);

    // Load saved value
    if let Some(ch) = state.channel_manager.config().get_channel(name) {
        monitor_scale.set_value(ch.monitor_volume);
    } else {
        monitor_scale.set_value(0.8);
    }
    channel.append(&monitor_scale);

    let monitor_mute = gtk::ToggleButton::with_label("M");
    if let Some(ch) = state.channel_manager.config().get_channel(name) {
        monitor_mute.set_active(ch.monitor_muted);
    }
    channel.append(&monitor_mute);

    // Stream Section
    let stream_label = gtk::Label::new(Some("Stream"));
    channel.append(&stream_label);

    let stream_scale = gtk::Scale::with_range(gtk::Orientation::Vertical, 0.0, 1.0, 0.01);
    stream_scale.set_inverted(true);
    stream_scale.set_vexpand(true);
    stream_scale.set_size_request(-1, 100);

    if let Some(ch) = state.channel_manager.config().get_channel(name) {
        stream_scale.set_value(ch.stream_volume);
    } else {
        stream_scale.set_value(0.7);
    }
    channel.append(&stream_scale);

    let stream_mute = gtk::ToggleButton::with_label("M");
    if let Some(ch) = state.channel_manager.config().get_channel(name) {
        stream_mute.set_active(ch.stream_muted);
    }
    channel.append(&stream_mute);

    channel_frame.set_child(Some(&channel));

    // Insert before the add button (which is always first)
    state.channel_container.append(&channel_frame);

    // Store widget references
    state.channel_widgets.insert(
        name.to_string(),
        ChannelWidget {
            container: channel_frame,
            app_list,
            monitor_scale,
            monitor_mute,
            stream_scale,
            stream_mute,
        },
    );
}

fn add_app_to_channel_ui(state: &mut UiState, channel_name: &str, info: &NodeInfo) {
    if let Some(channel_widget) = state.channel_widgets.get(channel_name) {
        let app_name = info
            .application_name
            .clone()
            .unwrap_or_else(|| info.name.clone());

        let app_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);

        let label = gtk::Label::new(Some(&app_name));
        label.set_max_width_chars(15);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        label.set_hexpand(true);
        label.set_halign(gtk::Align::Start);
        app_row.append(&label);

        // Store node_id for later removal
        unsafe {
            app_row.set_data("node_id", info.id);
        }

        channel_widget.app_list.append(&app_row);
    }
}

fn add_unassigned_app_ui(state: &mut UiState, info: &NodeInfo, ui_state: Rc<RefCell<UiState>>) {
    let app_name = info
        .application_name
        .clone()
        .unwrap_or_else(|| info.name.clone());

    let app_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    app_row.set_margin_top(2);
    app_row.set_margin_bottom(2);

    let label = gtk::Label::new(Some(&app_name));
    label.set_max_width_chars(15);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.set_hexpand(true);
    label.set_halign(gtk::Align::Start);
    app_row.append(&label);

    // Assign button
    let assign_btn = gtk::Button::with_label("+");
    assign_btn.set_tooltip_text(Some("Assign to channel"));

    let node_id = info.id;
    let app_name_clone = app_name.clone();
    let state_clone = ui_state.clone();
    assign_btn.connect_clicked(move |_| {
        show_assign_app_dialog(state_clone.clone(), node_id, app_name_clone.clone());
    });
    app_row.append(&assign_btn);

    state.unassigned_container.append(&app_row);
    state.unassigned_widgets.insert(info.id, app_row);
}

fn create_add_channel_button() -> gtk::Button {
    let button = gtk::Button::new();
    button.set_label("+");
    button.set_tooltip_text(Some("Create new channel"));
    button.set_halign(gtk::Align::Center);
    button.set_vexpand(true);
    button.set_valign(gtk::Align::Center);
    button.set_width_request(50);

    button
}

fn show_create_channel_dialog(state: Rc<RefCell<UiState>>) {
    let st = state.borrow();
    let dialog = gtk::Dialog::with_buttons(
        Some("Create Channel"),
        Some(&st.window),
        gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
        &[("Cancel", gtk::ResponseType::Cancel), ("Create", gtk::ResponseType::Accept)],
    );
    drop(st);

    dialog.set_default_width(300);

    let content = dialog.content_area();
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.set_spacing(8);

    let label = gtk::Label::new(Some("Channel Name:"));
    label.set_halign(gtk::Align::Start);
    content.append(&label);

    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some("e.g., Games, Music, Discord"));
    content.append(&entry);

    let state_clone = state.clone();
    let entry_clone = entry.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let name = entry_clone.text().to_string();
            if !name.is_empty() {
                let mut st = state_clone.borrow_mut();
                if let Err(e) = st.channel_manager.create_channel(name.clone()) {
                    warn!("Failed to create channel: {}", e);
                } else {
                    create_channel_widget(&mut st, &name);
                }
            }
        }
        dialog.close();
    });

    dialog.present();
}

fn show_assign_app_dialog(state: Rc<RefCell<UiState>>, node_id: u32, app_name: String) {
    let st = state.borrow();

    // Get channel names
    let channel_names: Vec<String> = st
        .channel_manager
        .channels()
        .iter()
        .map(|c| c.name.clone())
        .collect();

    if channel_names.is_empty() {
        // No channels, show a message
        let dialog = gtk::MessageDialog::new(
            Some(&st.window),
            gtk::DialogFlags::MODAL,
            gtk::MessageType::Info,
            gtk::ButtonsType::Ok,
            "No channels available. Create a channel first.",
        );
        dialog.connect_response(|d, _| d.close());
        dialog.present();
        return;
    }

    let dialog = gtk::Dialog::with_buttons(
        Some(&format!("Assign '{}'", app_name)),
        Some(&st.window),
        gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
        &[("Cancel", gtk::ResponseType::Cancel), ("Assign", gtk::ResponseType::Accept)],
    );
    drop(st);

    dialog.set_default_width(300);

    let content = dialog.content_area();
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.set_spacing(8);

    let label = gtk::Label::new(Some("Select channel:"));
    label.set_halign(gtk::Align::Start);
    content.append(&label);

    let dropdown = gtk::DropDown::from_strings(&channel_names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    content.append(&dropdown);

    let state_clone = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let selected_idx = dropdown.selected() as usize;
            if selected_idx < channel_names.len() {
                let channel_name = &channel_names[selected_idx];
                let mut st = state_clone.borrow_mut();

                // Remove from unassigned UI
                if let Some(widget) = st.unassigned_widgets.remove(&node_id) {
                    st.unassigned_container.remove(&widget);
                }

                // Assign in channel manager
                if let Err(e) = st
                    .channel_manager
                    .assign_node_to_channel(node_id, &app_name, channel_name)
                {
                    warn!("Failed to assign app: {}", e);
                } else {
                    // Add to channel UI
                    if let Some(info) = st.known_nodes.get(&node_id).cloned() {
                        add_app_to_channel_ui(&mut st, channel_name, &info);
                    }
                }
            }
        }
        dialog.close();
    });

    dialog.present();
}
