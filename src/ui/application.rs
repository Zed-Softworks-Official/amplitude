use spdlog::prelude::*;

use gtk4 as gtk;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, CssProvider};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::audio::{AudioCommandSender, AudioEvent, AudioManager, ChannelManager, NodeInfo, NodeType};
use crate::config::Channel;

pub struct AmplitudeApplication {
    gtk_app: Application,
    audio_manager: Arc<RwLock<AudioManager>>,
    /// Event subscriber created before audio thread starts (to avoid deadlock)
    event_subscriber: tokio::sync::broadcast::Receiver<AudioEvent>,
    /// Command sender for routing - can be used without locking AudioManager
    command_sender: AudioCommandSender,
}

impl AmplitudeApplication {
    pub async fn new() -> anyhow::Result<Self> {
        let gtk_app = Application::builder()
            .application_id("dev.zedsoftworks.amplitude")
            .build();

        let audio_manager = Arc::new(RwLock::new(AudioManager::new().await?));

        // Get event subscriber and command sender BEFORE starting the audio thread
        // The audio thread holds a write lock forever, so we must get these first
        let (event_subscriber, command_sender) = {
            let manager = audio_manager.read().await;
            (manager.subscribe(), manager.command_sender())
        };

        Ok(Self {
            gtk_app,
            audio_manager,
            event_subscriber,
            command_sender,
        })
    }

    pub fn run(self) -> anyhow::Result<()> {
        let audio_manager = self.audio_manager.clone();

        // Create the event channel that bridges the subscriber thread and GTK
        let (event_tx, event_rx) = async_channel::unbounded::<AudioEvent>();

        // Start the subscriber thread BEFORE the audio thread to ensure we don't miss events
        // This thread reads from the broadcast subscriber and forwards to the async_channel
        let event_subscriber = self.event_subscriber;
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async move {
                let mut sub = event_subscriber;
                while let Ok(event) = sub.recv().await {
                    if event_tx.send(event).await.is_err() {
                        break;
                    }
                }
            });
        });

        // Load config and set startup channels BEFORE starting the audio thread
        // This is done synchronously to avoid race conditions
        let startup_channels = {
            use crate::audio::ChannelManager;
            let cm = ChannelManager::new();
            cm.channels().iter().map(|c| c.name.clone()).collect::<Vec<_>>()
        };

        // Now spawn the audio thread - events will be captured by the subscriber thread above
        let am = self.audio_manager.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut manager = am.write().await;
                // Set startup channels before running
                manager.set_startup_channels(startup_channels);
                if let Err(e) = manager.run().await {
                    error!("Audio Manager error: {}", e);
                }
            });
        });

        let cmd_sender = self.command_sender.clone();
        self.gtk_app.connect_activate(move |app| {
            let am = audio_manager.clone();
            let cs = cmd_sender.clone();
            build_ui(app, am, event_rx.clone(), cs);
        });

        self.gtk_app.run();
        Ok(())
    }
}

/// Load application CSS styles
fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(include_str!("style.css"));

    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not get default display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
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
    /// Container for unassigned apps (inside popover)
    unassigned_container: gtk::Box,
    /// Maps node IDs to their unassigned app widgets
    unassigned_widgets: HashMap<u32, gtk::Box>,
    /// Button that shows unassigned apps count
    unassigned_button: gtk::MenuButton,
    /// Main window reference for dialogs
    window: ApplicationWindow,
    /// Audio manager for routing
    audio_manager: Arc<RwLock<AudioManager>>,
    /// Command sender for routing (doesn't require lock)
    command_sender: AudioCommandSender,
    /// Monitor bus level meter
    monitor_bus_meter: gtk::Box,
    /// Stream bus level meter
    stream_bus_meter: gtk::Box,
}

struct ChannelWidget {
    container: gtk::Frame,
    app_list: gtk::Box,
    monitor_scale: gtk::Scale,
    monitor_mute: gtk::ToggleButton,
    stream_scale: gtk::Scale,
    stream_mute: gtk::ToggleButton,
    level_meter: gtk::Box,
}

fn build_ui(app: &Application, audio_manager: Arc<RwLock<AudioManager>>, event_rx: async_channel::Receiver<AudioEvent>, command_sender: AudioCommandSender) {
    // Load CSS
    load_css();

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
    
    // Unassigned apps button with popover (in header)
    let unassigned_button = gtk::MenuButton::new();
    unassigned_button.set_icon_name("audio-speakers-symbolic");
    unassigned_button.set_tooltip_text(Some("Unassigned Apps"));
    unassigned_button.add_css_class("unassigned-apps-button");
    
    // Create popover content
    let popover_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    popover_box.set_margin_top(12);
    popover_box.set_margin_bottom(12);
    popover_box.set_margin_start(12);
    popover_box.set_margin_end(12);
    
    let popover_title = gtk::Label::new(Some("Unassigned Apps"));
    popover_title.add_css_class("popover-title");
    popover_title.set_halign(gtk::Align::Start);
    popover_box.append(&popover_title);
    
    let unassigned_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    unassigned_box.set_margin_top(8);
    
    let unassigned_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&unassigned_box)
        .width_request(250)
        .height_request(300)
        .build();
    
    popover_box.append(&unassigned_scroll);
    
    let popover = gtk::Popover::new();
    popover.set_child(Some(&popover_box));
    unassigned_button.set_popover(Some(&popover));
    
    header.pack_end(&unassigned_button);
    main_box.append(&header);

    // Content area - full width for channels
    let content_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    content_box.set_margin_top(16);
    content_box.set_margin_bottom(16);
    content_box.set_margin_start(16);
    content_box.set_margin_end(16);
    content_box.set_vexpand(true);

    // Channel Container (scrollable, takes most space)
    let channel_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    channel_box.set_halign(gtk::Align::Start);
    channel_box.set_margin_bottom(8); // Add padding above scrollbar

    // Add channel button
    let add_channel_btn = create_add_channel_button();
    channel_box.append(&add_channel_btn);

    let channel_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .child(&channel_box)
        .hexpand(true)
        .build();
    channel_scroll.add_css_class("channel-scroll");

    content_box.append(&channel_scroll);

    // Separator before bus section
    let separator = gtk::Separator::new(gtk::Orientation::Vertical);
    separator.set_margin_start(16);
    separator.set_margin_end(16);
    content_box.append(&separator);

    // Bus section (Monitor and Stream)
    let bus_section = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    bus_section.add_css_class("bus-section");
    
    // Create Monitor Bus strip
    let (monitor_bus_strip, monitor_bus_meter) = create_bus_strip("Monitor", "MON", true);
    bus_section.append(&monitor_bus_strip);
    
    // Create Stream Bus strip
    let (stream_bus_strip, stream_bus_meter) = create_bus_strip("Stream", "STR", false);
    bus_section.append(&stream_bus_strip);
    
    content_box.append(&bus_section);

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
        unassigned_button: unassigned_button.clone(),
        window: window.clone(),
        audio_manager: audio_manager.clone(),
        command_sender,
        monitor_bus_meter,
        stream_bus_meter,
    }));

    // Load existing channels from config and create their UI widgets
    // Note: Channel sinks are created by the audio manager on startup (set_startup_channels)
    {
        let mut st = state.borrow_mut();
        let channels: Vec<Channel> = st.channel_manager.channels().to_vec();
        
        for channel in channels {
            create_channel_widget_with_state(&mut st, &channel.name, Some(state.clone()));
        }
    }

    // Connect add channel button
    let state_clone = state.clone();
    add_channel_btn.connect_clicked(move |_| {
        show_create_channel_dialog(state_clone.clone());
    });

    // Setup event handler
    setup_event_handler(state.clone(), event_rx);

    // Setup level meter update timer
    setup_level_meter_timer(state);

    window.present();
}

/// Setup a timer to update level meters based on channel activity
fn setup_level_meter_timer(state: Rc<RefCell<UiState>>) {
    // Update level meters every 50ms
    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        let st = state.borrow();
        
        let mut total_channel_activity = 0usize;
        
        for (channel_name, channel_widget) in &st.channel_widgets {
            // Get the nodes assigned to this channel
            let node_count = st.channel_manager.get_channel_nodes(channel_name).len();
            total_channel_activity += node_count;
            
            if node_count > 0 {
                // Channel has active apps - show simulated activity
                // In a full implementation, this would come from actual PipeWire peak levels
                let base_level = 0.3 + (node_count as f32 * 0.1).min(0.4);
                let variation = (rand_variation() * 0.3) as f32;
                let level = (base_level + variation).min(0.95);
                let peak = (level + 0.05).min(1.0);
                
                update_level_meter(&channel_widget.level_meter, level, peak);
            } else {
                // No active apps - show minimal activity
                let level = rand_variation() as f32 * 0.05;
                update_level_meter(&channel_widget.level_meter, level, level);
            }
        }
        
        // Update bus meters - they aggregate all channel activity
        let unassigned_count = st.unassigned_widgets.len();
        let total_activity = total_channel_activity + unassigned_count;
        
        if total_activity > 0 {
            // Monitor bus - shows all audio going to speakers
            let monitor_base = 0.4 + (total_activity as f32 * 0.05).min(0.3);
            let monitor_variation = (rand_variation() * 0.2) as f32;
            let monitor_level = (monitor_base + monitor_variation).min(0.9);
            update_level_meter(&st.monitor_bus_meter, monitor_level, (monitor_level + 0.05).min(1.0));
            
            // Stream bus - shows audio going to stream
            let stream_base = 0.35 + (total_activity as f32 * 0.05).min(0.25);
            let stream_variation = (rand_variation() * 0.25) as f32;
            let stream_level = (stream_base + stream_variation).min(0.85);
            update_level_meter(&st.stream_bus_meter, stream_level, (stream_level + 0.05).min(1.0));
        } else {
            // No activity - show minimal levels
            let noise = rand_variation() as f32 * 0.03;
            update_level_meter(&st.monitor_bus_meter, noise, noise);
            update_level_meter(&st.stream_bus_meter, noise, noise);
        }
        
        glib::ControlFlow::Continue
    });
}

/// Generate a pseudo-random variation for level simulation
fn rand_variation() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    // Simple pseudo-random based on time
    ((nanos % 1000) as f64) / 1000.0
}

fn setup_event_handler(
    state: Rc<RefCell<UiState>>,
    event_rx: async_channel::Receiver<AudioEvent>,
) {
    // Handle events on the GTK main thread
    // The subscriber thread is already running and forwarding events to event_rx
    glib::spawn_future_local(async move {
        while let Ok(event) = event_rx.recv().await {
            match event {
                AudioEvent::NodeAdded(info) => {
                    if matches!(info.node_type, NodeType::ApplicationOutput) {
                        handle_node_added(state.clone(), info);
                    }
                }
                AudioEvent::NodeRemoved { id } => {
                    handle_node_removed(state.clone(), id);
                }
                AudioEvent::ChannelPeakLevel { channel_name, left, right } => {
                    handle_channel_peak_level(state.clone(), &channel_name, left, right);
                }
                AudioEvent::ChannelSinkDiscovered { name, id } => {
                    info!("Channel sink '{}' discovered with id {}", name, id);
                }
                _ => {}
            }
        }
    });
}

fn handle_channel_peak_level(state: Rc<RefCell<UiState>>, channel_name: &str, left: f32, right: f32) {
    let st = state.borrow();
    if let Some(channel_widget) = st.channel_widgets.get(channel_name) {
        // Use the average of left and right for the meter
        let level = (left + right) / 2.0;
        // Use max of left/right as peak
        let peak = left.max(right);
        update_level_meter(&channel_widget.level_meter, level, peak);
    }
}

fn handle_node_added(state: Rc<RefCell<UiState>>, info: NodeInfo) {
    let node_id = info.id;
    let command_sender;
    let channel_name_for_routing: Option<String>;

    {
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

        // Check if this app is assigned to a channel
        if let Some(channel_name) = st.channel_manager.on_node_added(info.id, &app_name) {
            // App is assigned to a channel - add to channel's app list in UI
            add_app_to_channel_ui(&mut st, &channel_name, &info, Some(state.clone()));
            channel_name_for_routing = Some(channel_name);
        } else {
            // App is unassigned - show in unassigned apps panel
            add_unassigned_app_ui(&mut st, &info, state.clone());
            channel_name_for_routing = None;
        }

        // Get command sender - no lock needed to use it
        command_sender = st.command_sender.clone();
    }

    // Route the app using command sender - this is fully synchronous and non-blocking
    if let Some(channel_name) = channel_name_for_routing {
        // Route to the assigned channel sink
        if let Err(e) = command_sender.route_app_to_channel(node_id, &channel_name) {
            warn!("Failed to route app {} to channel '{}': {}", node_id, channel_name, e);
            // Fallback: route to monitor/stream sinks directly
            if let Err(e) = command_sender.route_app_to_virtual_sinks(node_id) {
                warn!("Fallback routing also failed: {}", e);
            }
        }
    } else {
        // Unassigned apps still go directly to monitor/stream sinks
        // This ensures audio plays even for unassigned apps
        if let Err(e) = command_sender.route_app_to_virtual_sinks(node_id) {
            warn!("Failed to route unassigned app {} to virtual sinks: {}", node_id, e);
        }
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
        // Update button count
        update_unassigned_button(&st.unassigned_button, st.unassigned_widgets.len());
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
    create_channel_widget_with_state(state, name, None);
}

fn create_channel_widget_with_state(
    state: &mut UiState,
    name: &str,
    ui_state_rc: Option<Rc<RefCell<UiState>>>,
) {
    let channel_frame = gtk::Frame::new(None::<&str>);
    channel_frame.add_css_class("channel-strip");

    let is_builtin = state.channel_manager.is_builtin_channel(name);

    let channel = gtk::Box::new(gtk::Orientation::Vertical, 4);
    channel.set_margin_start(8);
    channel.set_margin_end(8);
    channel.set_margin_top(8);
    channel.set_margin_bottom(8);
    channel.set_width_request(180);

    // Channel name header
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let name_label = gtk::Label::new(Some(name));
    name_label.add_css_class("channel-name");
    name_label.set_hexpand(true);
    name_label.set_halign(gtk::Align::Start);
    header.append(&name_label);

    // Show lock icon for builtin channels
    if is_builtin {
        let lock_icon = gtk::Label::new(Some("🔒"));
        lock_icon.set_tooltip_text(Some("Built-in channel"));
        header.append(&lock_icon);
    } else {
        // Show drag handle for non-builtin channels
        let drag_icon = gtk::Label::new(Some("⋮⋮"));
        drag_icon.set_tooltip_text(Some("Drag to reorder"));
        drag_icon.add_css_class("drag-handle");
        header.append(&drag_icon);
    }
    channel.append(&header);

    // Separator
    let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep.set_margin_top(4);
    sep.set_margin_bottom(4);
    channel.append(&sep);

    // App list area
    let app_list_label = gtk::Label::new(Some("Applications"));
    app_list_label.add_css_class("section-label");
    app_list_label.set_halign(gtk::Align::Start);
    channel.append(&app_list_label);

    let app_list = gtk::Box::new(gtk::Orientation::Vertical, 2);
    app_list.set_margin_bottom(8);
    app_list.set_vexpand(false);
    channel.append(&app_list);

    // Sliders section - horizontal layout with level meter
    let sliders_section = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    sliders_section.set_vexpand(true);
    sliders_section.set_halign(gtk::Align::Center);

    // Level meter
    let level_meter = create_level_meter();
    sliders_section.append(&level_meter);

    // Monitor slider column
    let monitor_col = gtk::Box::new(gtk::Orientation::Vertical, 2);
    monitor_col.set_halign(gtk::Align::Center);

    let monitor_label = gtk::Label::new(Some("MON"));
    monitor_label.add_css_class("slider-label");
    monitor_col.append(&monitor_label);

    let monitor_scale = gtk::Scale::with_range(gtk::Orientation::Vertical, 0.0, 1.0, 0.01);
    monitor_scale.set_inverted(true);
    monitor_scale.set_vexpand(true);
    monitor_scale.set_size_request(30, 150);
    monitor_scale.set_draw_value(false);

    if let Some(ch) = state.channel_manager.config().get_channel(name) {
        monitor_scale.set_value(ch.monitor_volume);
    } else {
        monitor_scale.set_value(0.8);
    }
    monitor_col.append(&monitor_scale);

    let monitor_mute = gtk::ToggleButton::with_label("M");
    monitor_mute.add_css_class("mute-button");
    monitor_mute.set_tooltip_text(Some("Mute Monitor"));
    if let Some(ch) = state.channel_manager.config().get_channel(name) {
        monitor_mute.set_active(ch.monitor_muted);
    }
    monitor_col.append(&monitor_mute);

    sliders_section.append(&monitor_col);

    // Stream slider column
    let stream_col = gtk::Box::new(gtk::Orientation::Vertical, 2);
    stream_col.set_halign(gtk::Align::Center);

    let stream_label = gtk::Label::new(Some("STR"));
    stream_label.add_css_class("slider-label");
    stream_col.append(&stream_label);

    let stream_scale = gtk::Scale::with_range(gtk::Orientation::Vertical, 0.0, 1.0, 0.01);
    stream_scale.set_inverted(true);
    stream_scale.set_vexpand(true);
    stream_scale.set_size_request(30, 150);
    stream_scale.set_draw_value(false);

    if let Some(ch) = state.channel_manager.config().get_channel(name) {
        stream_scale.set_value(ch.stream_volume);
    } else {
        stream_scale.set_value(0.7);
    }
    stream_col.append(&stream_scale);

    let stream_mute = gtk::ToggleButton::with_label("S");
    stream_mute.add_css_class("mute-button");
    stream_mute.set_tooltip_text(Some("Mute Stream"));
    if let Some(ch) = state.channel_manager.config().get_channel(name) {
        stream_mute.set_active(ch.stream_muted);
    }
    stream_col.append(&stream_mute);

    sliders_section.append(&stream_col);

    // Connect volume slider signals
    if let Some(ref state_rc) = ui_state_rc {
        // Monitor volume slider
        let state_for_mon_vol = state_rc.clone();
        let channel_name_for_mon_vol = name.to_string();
        monitor_scale.connect_value_changed(move |scale| {
            let volume = scale.value();
            let mut st = state_for_mon_vol.borrow_mut();
            if let Err(e) = st.channel_manager.set_channel_monitor_volume(&channel_name_for_mon_vol, volume) {
                warn!("Failed to set monitor volume: {}", e);
            }
            // Also update the audio system
            let am = st.audio_manager.clone();
            let ch_name = channel_name_for_mon_vol.clone();
            drop(st);
            glib::spawn_future_local(async move {
                let manager = am.read().await;
                if let Err(e) = manager.set_channel_volume(&ch_name, volume as f32) {
                    warn!("Failed to set channel volume in audio manager: {}", e);
                }
            });
        });

        // Monitor mute button
        let state_for_mon_mute = state_rc.clone();
        let channel_name_for_mon_mute = name.to_string();
        monitor_mute.connect_toggled(move |btn| {
            let muted = btn.is_active();
            let mut st = state_for_mon_mute.borrow_mut();
            if let Err(e) = st.channel_manager.set_channel_monitor_muted(&channel_name_for_mon_mute, muted) {
                warn!("Failed to set monitor muted: {}", e);
            }
            // Update button appearance
            if muted {
                btn.add_css_class("muted");
            } else {
                btn.remove_css_class("muted");
            }
            // Also update the audio system
            let am = st.audio_manager.clone();
            let ch_name = channel_name_for_mon_mute.clone();
            drop(st);
            glib::spawn_future_local(async move {
                let manager = am.read().await;
                if let Err(e) = manager.set_channel_muted(&ch_name, muted) {
                    warn!("Failed to set channel muted in audio manager: {}", e);
                }
            });
        });

        // Stream volume slider
        let state_for_str_vol = state_rc.clone();
        let channel_name_for_str_vol = name.to_string();
        stream_scale.connect_value_changed(move |scale| {
            let volume = scale.value();
            let mut st = state_for_str_vol.borrow_mut();
            if let Err(e) = st.channel_manager.set_channel_stream_volume(&channel_name_for_str_vol, volume) {
                warn!("Failed to set stream volume: {}", e);
            }
        });

        // Stream mute button
        let state_for_str_mute = state_rc.clone();
        let channel_name_for_str_mute = name.to_string();
        stream_mute.connect_toggled(move |btn| {
            let muted = btn.is_active();
            let mut st = state_for_str_mute.borrow_mut();
            if let Err(e) = st.channel_manager.set_channel_stream_muted(&channel_name_for_str_mute, muted) {
                warn!("Failed to set stream muted: {}", e);
            }
            // Update button appearance
            if muted {
                btn.add_css_class("muted");
            } else {
                btn.remove_css_class("muted");
            }
        });
    }

    channel.append(&sliders_section);

    channel_frame.set_child(Some(&channel));

    // Store channel name as data on the frame for drag-and-drop
    let name_string = name.to_string();
    unsafe {
        channel_frame.set_data("channel_name", name_string.clone());
    }

    // Setup drag-and-drop for non-builtin channels
    if !is_builtin {
        // Create drag source
        let drag_source = gtk::DragSource::new();
        drag_source.set_actions(gdk::DragAction::MOVE);

        let name_for_drag = name.to_string();
        drag_source.connect_prepare(move |_source, _x, _y| {
            let content = gdk::ContentProvider::for_value(&name_for_drag.to_value());
            Some(content)
        });

        channel_frame.add_controller(drag_source);
    }

    // Create drop target (all channels can receive drops)
    let drop_target = gtk::DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE);
    let target_name = name.to_string();
    let state_clone = ui_state_rc.clone();

    drop_target.connect_drop(move |_target, value, _x, _y| {
        if let Ok(source_name) = value.get::<String>() {
            if source_name != target_name {
                if let Some(ref state_rc) = state_clone {
                    let mut st = state_rc.borrow_mut();
                    // Get target position
                    let target_pos = st
                        .channel_manager
                        .channels()
                        .iter()
                        .position(|c| c.name == target_name)
                        .unwrap_or(0);

                    // Move the channel
                    if let Err(e) = st.channel_manager.move_channel(&source_name, target_pos) {
                        warn!("Failed to move channel: {}", e);
                    } else {
                        // Reorder widgets in the UI
                        reorder_channel_widgets(&mut st);
                    }
                }
            }
        }
        true
    });

    channel_frame.add_controller(drop_target);

    // Insert after existing channels
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
            level_meter,
        },
    );
}

/// Reorder channel widgets in the UI to match the config order
fn reorder_channel_widgets(state: &mut UiState) {
    // Get the ordered channel names from config
    let ordered_names: Vec<String> = state
        .channel_manager
        .channels()
        .iter()
        .map(|c| c.name.clone())
        .collect();

    // Remove all channel widgets from the container
    for name in &ordered_names {
        if let Some(widget) = state.channel_widgets.get(name) {
            state.channel_container.remove(&widget.container);
        }
    }

    // Re-add them in the correct order
    // First, find and keep track of the add button (first child after channels)
    let add_button = state.channel_container.first_child();

    // Re-add channels in order
    for name in &ordered_names {
        if let Some(widget) = state.channel_widgets.get(name) {
            state.channel_container.append(&widget.container);
        }
    }

    // Make sure add button stays at the beginning (before channels)
    if let Some(btn) = add_button {
        state.channel_container.reorder_child_after(&btn, None::<&gtk::Widget>);
    }
}

/// Number of segments in the level meter
const METER_SEGMENTS: usize = 12;
const BUS_METER_SEGMENTS: usize = 8;

/// Create a smaller level meter for bus strips
fn create_bus_level_meter() -> gtk::Box {
    let meter_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    meter_container.set_width_request(16);
    meter_container.set_vexpand(true);
    meter_container.add_css_class("level-meter");

    let drawing_area = gtk::DrawingArea::new();
    drawing_area.set_vexpand(true);
    drawing_area.set_content_width(12);
    drawing_area.set_content_height(100);

    let level = Rc::new(RefCell::new(0.0f32));
    let peak = Rc::new(RefCell::new(0.0f32));

    let level_clone = level.clone();
    let peak_clone = peak.clone();

    drawing_area.set_draw_func(move |_area, cr, width, height| {
        let current_level = *level_clone.borrow();
        let current_peak = *peak_clone.borrow();

        let width = width as f64;
        let height = height as f64;

        cr.set_source_rgb(0.05, 0.05, 0.1);
        let _ = cr.paint();

        let segment_height = height / BUS_METER_SEGMENTS as f64;
        let segment_gap = 2.0;
        let segment_width = width - 2.0;

        for i in 0..BUS_METER_SEGMENTS {
            let segment_idx = BUS_METER_SEGMENTS - 1 - i;
            let y = i as f64 * segment_height + 1.0;

            let (r_off, g_off, b_off, r_on, g_on, b_on) = if segment_idx >= 6 {
                (0.35, 0.15, 0.15, 0.94, 0.27, 0.27)
            } else if segment_idx >= 5 {
                (0.35, 0.35, 0.15, 0.98, 0.80, 0.08)
            } else {
                (0.15, 0.35, 0.20, 0.29, 0.87, 0.50)
            };

            let threshold = segment_idx as f32 / BUS_METER_SEGMENTS as f32;
            let is_lit = current_level > threshold;
            let is_peak = (current_peak - threshold).abs() < (1.0 / BUS_METER_SEGMENTS as f32);

            if is_lit || is_peak {
                cr.set_source_rgb(r_on, g_on, b_on);
            } else {
                cr.set_source_rgb(r_off, g_off, b_off);
            }

            let rect_y = y + segment_gap / 2.0;
            let rect_height = segment_height - segment_gap;
            let radius = 1.5;

            cr.new_path();
            cr.arc(1.0 + radius, rect_y + radius, radius, std::f64::consts::PI, 1.5 * std::f64::consts::PI);
            cr.arc(1.0 + segment_width - radius, rect_y + radius, radius, 1.5 * std::f64::consts::PI, 0.0);
            cr.arc(1.0 + segment_width - radius, rect_y + rect_height - radius, radius, 0.0, 0.5 * std::f64::consts::PI);
            cr.arc(1.0 + radius, rect_y + rect_height - radius, radius, 0.5 * std::f64::consts::PI, std::f64::consts::PI);
            cr.close_path();
            let _ = cr.fill();
        }
    });

    meter_container.append(&drawing_area);

    unsafe {
        meter_container.set_data("drawing_area", drawing_area);
        meter_container.set_data("level", level);
        meter_container.set_data("peak", peak);
    }

    meter_container
}

/// Create a segmented LED-style level meter using DrawingArea
fn create_level_meter() -> gtk::Box {
    let meter_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    meter_container.set_width_request(24);
    meter_container.set_vexpand(true);
    meter_container.add_css_class("level-meter");

    // Create a single DrawingArea for the entire meter
    let drawing_area = gtk::DrawingArea::new();
    drawing_area.set_vexpand(true);
    drawing_area.set_content_width(20);
    drawing_area.set_content_height(150);

    // Store the current level (0.0 - 1.0) in a shared state
    let level = Rc::new(RefCell::new(0.0f32));
    let peak = Rc::new(RefCell::new(0.0f32));

    let level_clone = level.clone();
    let peak_clone = peak.clone();

    drawing_area.set_draw_func(move |_area, cr, width, height| {
        let current_level = *level_clone.borrow();
        let current_peak = *peak_clone.borrow();

        let width = width as f64;
        let height = height as f64;

        // Background
        cr.set_source_rgb(0.05, 0.05, 0.1);
        let _ = cr.paint();

        let segment_height = height / METER_SEGMENTS as f64;
        let segment_gap = 2.0;
        let segment_width = width - 4.0;

        // Draw segments from bottom to top
        for i in 0..METER_SEGMENTS {
            let segment_idx = METER_SEGMENTS - 1 - i; // Reverse order (bottom to top)
            let y = i as f64 * segment_height + 1.0;

            // Determine color based on segment position
            let (r_off, g_off, b_off, r_on, g_on, b_on) = if segment_idx >= 10 {
                // Red zone (top 2 segments)
                (0.35, 0.15, 0.15, 0.94, 0.27, 0.27)
            } else if segment_idx >= 8 {
                // Yellow zone (2 segments)
                (0.35, 0.35, 0.15, 0.98, 0.80, 0.08)
            } else {
                // Green zone (bottom 8 segments)
                (0.15, 0.35, 0.20, 0.29, 0.87, 0.50)
            };

            // Calculate if this segment should be lit
            let threshold = segment_idx as f32 / METER_SEGMENTS as f32;
            let is_lit = current_level > threshold;
            let is_peak = (current_peak - threshold).abs() < (1.0 / METER_SEGMENTS as f32);

            // Draw segment
            if is_lit || is_peak {
                cr.set_source_rgb(r_on, g_on, b_on);
            } else {
                cr.set_source_rgb(r_off, g_off, b_off);
            }

            // Draw rounded rectangle
            let rect_y = y + segment_gap / 2.0;
            let rect_height = segment_height - segment_gap;
            let radius = 2.0;

            cr.new_path();
            cr.arc(2.0 + radius, rect_y + radius, radius, std::f64::consts::PI, 1.5 * std::f64::consts::PI);
            cr.arc(2.0 + segment_width - radius, rect_y + radius, radius, 1.5 * std::f64::consts::PI, 0.0);
            cr.arc(2.0 + segment_width - radius, rect_y + rect_height - radius, radius, 0.0, 0.5 * std::f64::consts::PI);
            cr.arc(2.0 + radius, rect_y + rect_height - radius, radius, 0.5 * std::f64::consts::PI, std::f64::consts::PI);
            cr.close_path();
            let _ = cr.fill();
        }
    });

    meter_container.append(&drawing_area);

    // Store references for updating
    unsafe {
        meter_container.set_data("drawing_area", drawing_area);
        meter_container.set_data("level", level);
        meter_container.set_data("peak", peak);
    }

    meter_container
}

/// Update the level meter with new values
fn update_level_meter(meter: &gtk::Box, level: f32, peak: f32) {
    unsafe {
        if let Some(level_rc) = meter.data::<Rc<RefCell<f32>>>("level") {
            *level_rc.as_ref().borrow_mut() = level.clamp(0.0, 1.0);
        }
        if let Some(peak_rc) = meter.data::<Rc<RefCell<f32>>>("peak") {
            *peak_rc.as_ref().borrow_mut() = peak.clamp(0.0, 1.0);
        }
        if let Some(drawing_area) = meter.data::<gtk::DrawingArea>("drawing_area") {
            drawing_area.as_ref().queue_draw();
        }
    }
}

fn add_app_to_channel_ui(
    state: &mut UiState,
    channel_name: &str,
    info: &NodeInfo,
    ui_state: Option<Rc<RefCell<UiState>>>,
) {
    if let Some(channel_widget) = state.channel_widgets.get(channel_name) {
        let app_name = info
            .application_name
            .clone()
            .unwrap_or_else(|| info.name.clone());

        let app_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        app_row.add_css_class("app-item");

        // App icon placeholder
        let icon = gtk::Label::new(Some("🔊"));
        icon.set_margin_end(4);
        app_row.append(&icon);

        let label = gtk::Label::new(Some(&app_name));
        label.set_max_width_chars(12);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        label.set_hexpand(true);
        label.set_halign(gtk::Align::Start);
        app_row.append(&label);

        // Reassign button
        if let Some(state_rc) = ui_state {
            let reassign_btn = gtk::Button::new();
            reassign_btn.set_icon_name("view-more-symbolic");
            reassign_btn.add_css_class("flat");
            reassign_btn.add_css_class("app-menu-btn");
            reassign_btn.set_tooltip_text(Some("Move to another channel"));

            let node_id = info.id;
            let app_name_clone = app_name.clone();
            reassign_btn.connect_clicked(move |_| {
                show_assign_app_dialog(state_rc.clone(), node_id, app_name_clone.clone());
            });
            app_row.append(&reassign_btn);
        }

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
    app_row.add_css_class("unassigned-app-row");

    // App icon
    let icon = gtk::Label::new(Some("🔊"));
    icon.set_margin_end(4);
    app_row.append(&icon);

    let label = gtk::Label::new(Some(&app_name));
    label.set_max_width_chars(20);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.set_hexpand(true);
    label.set_halign(gtk::Align::Start);
    app_row.append(&label);

    // Assign button
    let assign_btn = gtk::Button::new();
    assign_btn.set_icon_name("list-add-symbolic");
    assign_btn.add_css_class("flat");
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
    
    // Update button count
    update_unassigned_button(&state.unassigned_button, state.unassigned_widgets.len());
}

/// Create a bus strip (Monitor or Stream) for routing verification
fn create_bus_strip(name: &str, _short_name: &str, is_monitor: bool) -> (gtk::Frame, gtk::Box) {
    let frame = gtk::Frame::new(None::<&str>);
    frame.add_css_class("bus-strip");
    if is_monitor {
        frame.add_css_class("monitor-bus");
    } else {
        frame.add_css_class("stream-bus");
    }

    let container = gtk::Box::new(gtk::Orientation::Vertical, 2);
    container.set_margin_start(6);
    container.set_margin_end(6);
    container.set_margin_top(6);
    container.set_margin_bottom(6);
    container.set_width_request(70);

    // Bus name header - compact
    let name_label = gtk::Label::new(Some(name));
    name_label.add_css_class("bus-name");
    name_label.set_halign(gtk::Align::Center);
    name_label.set_tooltip_text(Some(if is_monitor { "Monitor → Speakers" } else { "Stream → OBS" }));
    container.append(&name_label);

    // Sliders section - compact layout
    let sliders_section = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    sliders_section.set_vexpand(true);
    sliders_section.set_halign(gtk::Align::Center);
    sliders_section.set_margin_top(4);

    // Level meter (smaller for bus)
    let level_meter = create_bus_level_meter();
    sliders_section.append(&level_meter);

    // Volume slider column
    let vol_col = gtk::Box::new(gtk::Orientation::Vertical, 2);
    vol_col.set_halign(gtk::Align::Center);

    let vol_scale = gtk::Scale::with_range(gtk::Orientation::Vertical, 0.0, 1.0, 0.01);
    vol_scale.set_inverted(true);
    vol_scale.set_vexpand(true);
    vol_scale.set_size_request(20, 100);
    vol_scale.set_draw_value(false);
    vol_scale.set_value(1.0);
    vol_col.append(&vol_scale);

    let mute_btn = gtk::ToggleButton::with_label("M");
    mute_btn.add_css_class("mute-button");
    mute_btn.add_css_class("bus-mute");
    mute_btn.set_tooltip_text(Some(&format!("Mute {}", name)));
    vol_col.append(&mute_btn);

    sliders_section.append(&vol_col);
    container.append(&sliders_section);

    frame.set_child(Some(&container));

    (frame, level_meter)
}

/// Update the unassigned apps button label to show count
fn update_unassigned_button(button: &gtk::MenuButton, count: usize) {
    if count > 0 {
        button.set_label(&format!("{}", count));
        button.add_css_class("has-unassigned");
    } else {
        button.set_label("");
        button.remove_css_class("has-unassigned");
    }
}

fn create_add_channel_button() -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("add-channel-button");
    button.set_tooltip_text(Some("Create new channel"));

    // Create a container that matches channel strip dimensions
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    container.set_width_request(180);
    container.set_vexpand(true);
    container.set_valign(gtk::Align::Fill);
    container.set_halign(gtk::Align::Center);

    // Large plus icon in the center
    let plus_label = gtk::Label::new(Some("+"));
    plus_label.add_css_class("add-channel-icon");
    plus_label.set_vexpand(true);
    plus_label.set_valign(gtk::Align::Center);
    plus_label.set_halign(gtk::Align::Center);
    container.append(&plus_label);

    // Subtitle text
    let subtitle = gtk::Label::new(Some("Add Channel"));
    subtitle.add_css_class("add-channel-subtitle");
    subtitle.set_margin_bottom(16);
    container.append(&subtitle);

    button.set_child(Some(&container));

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
    let state_for_widget = state.clone();
    let entry_clone = entry.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let name = entry_clone.text().to_string();
            if !name.is_empty() {
                let mut st = state_clone.borrow_mut();
                if let Err(e) = st.channel_manager.create_channel(name.clone()) {
                    warn!("Failed to create channel: {}", e);
                } else {
                    create_channel_widget_with_state(&mut st, &name, Some(state_for_widget.clone()));
                    
                    // Request the virtual sink creation for this channel
                    // Uses command sender - no lock needed
                    if let Err(e) = st.command_sender.create_channel_sink(&name) {
                        error!("Failed to request channel sink for '{}': {}", name, e);
                    }
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
    let state_for_ui = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let selected_idx = dropdown.selected() as usize;
            if selected_idx < channel_names.len() {
                let channel_name = channel_names[selected_idx].clone();
                let mut st = state_clone.borrow_mut();

                // Remove from current channel UI first
                for (_, channel_widget) in &st.channel_widgets {
                    let mut child = channel_widget.app_list.first_child();
                    while let Some(widget) = child {
                        if let Some(id) = unsafe { widget.data::<u32>("node_id") } {
                            if unsafe { *id.as_ref() } == node_id {
                                channel_widget.app_list.remove(&widget);
                                break;
                            }
                        }
                        child = widget.next_sibling();
                    }
                }

                // Remove from unassigned UI
                if let Some(widget) = st.unassigned_widgets.remove(&node_id) {
                    st.unassigned_container.remove(&widget);
                    // Update button count
                    update_unassigned_button(&st.unassigned_button, st.unassigned_widgets.len());
                }

                // Assign in channel manager
                if let Err(e) = st
                    .channel_manager
                    .assign_node_to_channel(node_id, &app_name, &channel_name)
                {
                    warn!("Failed to assign app: {}", e);
                } else {
                    // Add to channel UI
                    if let Some(info) = st.known_nodes.get(&node_id).cloned() {
                        add_app_to_channel_ui(
                            &mut st,
                            &channel_name,
                            &info,
                            Some(state_for_ui.clone()),
                        );
                    }
                    
                    // Route the app to the channel sink - uses command sender, no lock needed
                    if let Err(e) = st.command_sender.route_app_to_channel(node_id, &channel_name) {
                        warn!("Failed to route app {} to channel '{}': {}", node_id, channel_name, e);
                    }
                }
            }
        }
        dialog.close();
    });

    dialog.present();
}
