use spdlog::prelude::*;

use pipewire as pw;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::thread::{self, JoinHandle};
use tokio::sync::mpsc;

use super::events::{AudioEvent, NodeInfo, NodeType, PortDirection, PortInfo};

#[derive(Debug)]
pub enum PipeWireCommand {
    CreateLink { output_port: u32, input_port: u32 },
    DestroyLink { link_id: u32 },
    Shutdown
}

pub struct PipeWireClient {
    command_tx: std::sync::mpsc::Sender<PipeWireCommand>,
    event_rx: mpsc::UnboundedReceiver<AudioEvent>,
    thread_handle: Option<JoinHandle<()>>
}

impl PipeWireClient {
    pub async fn new() -> anyhow::Result<Self> {
        // Tokio channel for events (Pipewire -> App)
        let (event_tx, event_rx) = mpsc::unbounded_channel::<AudioEvent>();

        // Std channel for commands (App -> Pipewire)
        // We use std here because Pipewire thread isn't async
        let (command_tx, command_rx) = std::sync::mpsc::channel::<PipeWireCommand>();

        // Spawn pipewire in a dedicated OS thread
        // Pipewire has its own event loop that's not compatible with tokio
        let thread_handle = thread::Builder::new()
            .name("pipewire-main".into())
            .spawn(move || {
                if let Err(e) = run_pipewire_thread(command_rx, event_tx) {
                    error!("PipeWire thread error: {}", e);
                }
            })?;

        Ok(Self {
            command_tx,
            event_rx,
            thread_handle: Some(thread_handle)
        })
    }

    // Recieve the next event (async)
    pub async fn recv_event(&mut self) -> Option<AudioEvent> {
        self.event_rx.recv().await
    }

    // Try to receive an event without waiting
    pub fn try_recv_event(&mut self) -> Option<AudioEvent> {
        self.event_rx.try_recv().ok()
    }

    // Create a link between two ports
    pub fn create_link(&self, output_port: u32, input_port: u32) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::CreateLink {
                output_port,
                input_port
            })
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("")
            })
    }

    // destroy link
    pub fn destroy_link(&self, link_id: u32) -> anyhow::Result<()> {
        self.command_tx
            .send(PipeWireCommand::DestroyLink {
                link_id
            })
            .map_err(|e| {
                error!("Failed to send command: {}", e);
                anyhow::anyhow!("")
            })
    }

    // Shutdown the pipewire connection
    pub async fn shutdown(&mut self) {
        let _ = self.command_tx.send(PipeWireCommand::Shutdown);

        if let Some(handle) = self.thread_handle.take() {
            tokio::task::spawn_blocking(move || {
                let _ = handle.join();
            })
            .await
            .ok();
        }
    }
}

impl Drop for PipeWireClient {
    fn drop(&mut self) {
        let _ = self.command_tx.send(PipeWireCommand::Shutdown);
    }
}

struct PipeWireState {
    nodes: HashMap<u32, NodeInfo>,
    ports: HashMap<u32, PortInfo>,
    event_tx: mpsc::UnboundedSender<AudioEvent>
}

impl PipeWireState {
    fn new(event_tx: mpsc::UnboundedSender<AudioEvent>) -> Self {
        Self {
            nodes: HashMap::new(),
            ports: HashMap::new(),
            event_tx
        }
    }

    fn send_event(&self, event: AudioEvent) {
        if let Err(e) = self.event_tx.send(event) {
            error!("Failed to send audio event: {}", e);
        }
    }
}

fn run_pipewire_thread(
    command_rx: std::sync::mpsc::Receiver<PipeWireCommand>,
    event_tx: mpsc::UnboundedSender<AudioEvent>
) -> anyhow::Result<()> {
    let mainloop = pw::main_loop::MainLoopBox::new(None)?;
    let context = pw::context::ContextBox::new(&mainloop.loop_(), None)?;
    let core = context.connect(None)?;
    let registry = core.get_registry()?;

    let mainloop_ptr = &mainloop as *const _;

    let state = Rc::new(RefCell::new(PipeWireState::new(event_tx.clone())));

    // Setup registry listener for node/port discovery
    let _listener = {
        let state_add = state.clone();
        let state_remove = state.clone();

        registry
            .add_listener_local()
            .global(move |global| {
                handle_global_added(&state_add, global);
            })
            .global_remove(move |id| {
                handle_global_removed(&state_remove, id);
            })
            .register()
    };

    let timer_callback = move || {
        // Process all pending commands
        while let Ok(cmd) = command_rx.try_recv() {
            match cmd {
                PipeWireCommand::Shutdown => {
                    let mainloop: &pw::main_loop::MainLoopBox = unsafe { &*mainloop_ptr};
                    mainloop.quit();
                    break;
                }
                PipeWireCommand::CreateLink {
                    output_port,
                    input_port
                } => {
                        info!("Creating Link: {} -> {}",
                            output_port,
                            input_port
                        );

                        // TODO: Link Creation
                }
                PipeWireCommand::DestroyLink { link_id } => {
                    info!("Destroying Link: {}", link_id);
                }
            }
        }
    };


    // Create a timer source for command polling
    let timer = mainloop.loop_().add_timer(move |_| {
        timer_callback();
    });

    timer.update_timer(
        Some(std::time::Duration::from_millis(10)),
        Some(std::time::Duration::from_millis(10))
    );

    // Signal that we're ready
    event_tx.send(AudioEvent::Ready)?;

    mainloop.run();

    Ok(())
}

fn handle_global_added(
    state: &Rc<RefCell<PipeWireState>>,
    global: &pw::registry::GlobalObject<&pw::spa::utils::dict::DictRef>
) {
    let props = match global.props.as_ref() {
        Some(p) => p,
        None => return
    };

    match global.type_ {
        pw::types::ObjectType::Node => {
            handle_node_added(state, global.id, props);
        }
        pw::types::ObjectType::Port => {
            handle_port_added(state, global.id, props);
        }
        pw::types::ObjectType::Link => {
            info!("Link added: {}", global.id);
        }

        _ => {}
    }
}

// Handle a node being added
fn handle_node_added(
    state: &Rc<RefCell<PipeWireState>>,
    id: u32,
    props: &pw::spa::utils::dict::DictRef,
) {
    let name = props.get("node.name").unwrap_or("Unknown").to_string();
    let description = props.get("node.description").map(String::from);
    let application_name = props.get("application.name").map(String::from);
    let media_class = props.get("media.class").unwrap_or("").to_string();

    let node_type = classify_node(&media_class);

    // Skip nodes we don't care about
    if matches!(node_type, NodeType::Unknown) {
        return;
    }

    let node_info = NodeInfo {
        id,
        name: name.clone(),
        description,
        application_name,
        media_class,
        node_type,
    };

    info!(
        "Node discovered: {} (id={}, type={:?})",
        name,
        id,
        node_info.node_type
    );

    let mut state = state.borrow_mut();
    state.nodes.insert(id, node_info.clone());
    state.send_event(AudioEvent::NodeAdded(node_info));
}

// Handle a port being added
fn handle_port_added(
    state: &Rc<RefCell<PipeWireState>>,
    id: u32,
    props: &pw::spa::utils::dict::DictRef,
) {
    let name = props.get("port.name").unwrap_or("Unknown").to_string();

    let node_id: u32 = props
        .get("node.id")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let direction = match props.get("port.direction") {
        Some("in") => PortDirection::Input,
        Some("out") => PortDirection::Output,
        _ => return,
    };

    let channel = props.get("audio.channel").map(String::from);

    let port_info = PortInfo {
        id,
        node_id,
        name,
        direction,
        channel,
    };



    let mut state = state.borrow_mut();
    state.ports.insert(id, port_info.clone());
    state.send_event(AudioEvent::PortAdded(port_info));
}

/// Handle an object being removed
fn handle_global_removed(state: &Rc<RefCell<PipeWireState>>, id: u32) {
    let mut state = state.borrow_mut();

    if let Some(node) = state.nodes.remove(&id) {
        info!("Node removed: {} (id={})", node.name, id);
        state.send_event(AudioEvent::NodeRemoved { id });
        return;
    }

    if let Some(port) = state.ports.remove(&id) {
        state.send_event(AudioEvent::PortRemoved {
            id,
            node_id: port.node_id,
        });
    }
}

/// Classify a node based on media.class
fn classify_node(media_class: &str) -> NodeType {
    match media_class {
        "Stream/Output/Audio" => NodeType::ApplicationOutput,
        "Stream/Input/Audio" => NodeType::ApplicationInput,
        "Audio/Sink" => NodeType::SinkDevice,
        "Audio/Source" => NodeType::SourceDevice,
        "Audio/Sink/Virtual" | "Audio/Source/Virtual" => NodeType::Virtual,
        _ => NodeType::Unknown,
    }
}
