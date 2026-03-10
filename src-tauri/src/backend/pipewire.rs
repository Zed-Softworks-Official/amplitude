use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use pipewire as pw;
use pipewire::properties::properties;

use crate::audio::node::{MediaClass, NodeInfo};
use crate::audio::{AudioBackend, BackendCommand, BackendEvent, Sink};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Prefix applied to the internal `node.name` of every Amplitude-owned node.
const AMPLITUDE_PREFIX: &str = "amplitude-";

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

/// Pending link state: the expected (output_node_id, input_node_id) pair and
/// the one-shot reply channel that unblocks the engine once the registry
/// confirms the link's global ID.
type PendingLink = Rc<RefCell<Option<(u64, u64, crate::audio::LinkReply)>>>;

/// How long synchronous reply channels wait before returning a timeout error.
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

// ---------------------------------------------------------------------------
// Public backend struct (lives on the engine/Tauri thread)
// ---------------------------------------------------------------------------

pub struct PipewireBackend {
    /// Send commands to the PipeWire thread.
    /// Uses `pipewire::channel` so the PW mainloop wakes up on arrival.
    command_tx: pipewire::channel::Sender<BackendCommand>,
    /// Receive node events emitted by the PipeWire thread.
    /// Uses `std::sync::mpsc` so the engine can poll without blocking.
    event_rx: mpsc::Receiver<BackendEvent>,
}

impl PipewireBackend {
    pub fn new() -> Self {
        let (command_tx, command_rx) = pipewire::channel::channel();
        let (event_tx, event_rx) = mpsc::channel();

        std::thread::Builder::new()
            .name("Amplitude PipeWire Backend".to_string())
            .spawn(move || {
                if let Err(e) = pw_thread(command_rx, event_tx) {
                    eprintln!("[pipewire] thread failed: {e}");
                }
            })
            .expect("failed to spawn PipeWire thread");

        Self {
            command_tx,
            event_rx,
        }
    }
}

impl Default for PipewireBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioBackend for PipewireBackend {
    fn create_bus_sink(&mut self, name: &str) -> Result<Sink, String> {
        // At the PipeWire level a bus sink and a channel sink are both
        // null-audio-sink adapter nodes — the distinction is semantic and
        // lives in the engine. Delegate to the same command path.
        self.create_virtual_sink(name)
    }

    fn create_virtual_sink(&mut self, name: &str) -> Result<Sink, String> {
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);

        self.command_tx
            .send(BackendCommand::CreateVirtualSink {
                name: name.to_string(),
                reply: reply_tx,
            })
            .map_err(|e| {
                format!("failed to send CreateVirtualSink: {:?}", e)
            })?;

        let external_id =
            reply_rx.recv_timeout(REPLY_TIMEOUT).map_err(|e| {
                format!("sink creation timed out or channel closed: {e}")
            })??;

        Ok(Sink::new(external_id))
    }

    fn destroy_virtual_sink(&mut self, sink: &Sink) -> Result<(), String> {
        self.command_tx
            .send(BackendCommand::DestroyVirtualSink {
                external_id: sink.external_id,
            })
            .map_err(|e| format!("failed to send DestroyVirtualSink: {:?}", e))
    }

    fn create_link(
        &mut self,
        output_node_id: u64,
        input_node_id: u64,
    ) -> Result<u64, String> {
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);

        self.command_tx
            .send(BackendCommand::CreateLink {
                output_node_id,
                input_node_id,
                reply: reply_tx,
            })
            .map_err(|e| format!("failed to send CreateLink: {:?}", e))?;

        reply_rx.recv_timeout(REPLY_TIMEOUT).map_err(|e| {
            format!("link creation timed out or channel closed: {e}")
        })?
    }

    fn destroy_link(&mut self, link_id: u64) -> Result<(), String> {
        self.command_tx
            .send(BackendCommand::DestroyLink { link_id })
            .map_err(|e| format!("failed to send DestroyLink: {:?}", e))
    }

    fn poll_events(&mut self) -> Vec<BackendEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.event_rx.try_recv() {
            events.push(evt);
        }
        events
    }
}

pub fn create_backend() -> Box<dyn AudioBackend> {
    Box::new(PipewireBackend::new())
}

// ---------------------------------------------------------------------------
// PipeWire thread
// ---------------------------------------------------------------------------

fn pw_thread(
    command_rx: pipewire::channel::Receiver<BackendCommand>,
    event_tx: mpsc::Sender<BackendEvent>,
) -> Result<(), String> {
    pw::init();

    let mainloop = pw::main_loop::MainLoopRc::new(None)
        .map_err(|e| format!("failed to create mainloop: {e}"))?;
    let context = pw::context::ContextRc::new(&mainloop, None)
        .map_err(|e| format!("failed to create context: {e}"))?;
    let core = context
        .connect_rc(None)
        .map_err(|e| format!("failed to connect to PipeWire: {e}"))?;
    let registry = core
        .get_registry()
        .map_err(|e| format!("failed to get registry: {e}"))?;

    // -------------------------------------------------------------------------
    // Shared state (Rc<RefCell<_>> — PW thread is single-threaded)
    // -------------------------------------------------------------------------

    // Channel and bus sink proxies: external_id → Node proxy.
    // Kept alive so PipeWire doesn't destroy the underlying nodes.
    // Command handler inserts under u64::MAX (sentinel); registry callback
    // promotes to the real global ID.
    let our_sinks: Rc<RefCell<HashMap<u64, pw::node::Node>>> =
        Rc::new(RefCell::new(HashMap::new()));

    // Link proxies: link global ID → Link proxy.
    let our_links: Rc<RefCell<HashMap<u64, pw::link::Link>>> =
        Rc::new(RefCell::new(HashMap::new()));

    // Pending sink creation: resolved by the registry when the node appears.
    #[allow(clippy::type_complexity)]
    let pending_sink: Rc<
        RefCell<Option<(String, crate::audio::SinkReply)>>,
    > = Rc::new(RefCell::new(None));

    // Pending link creation: (output_node_id, input_node_id, reply).
    // Storing the expected node IDs prevents stale or foreign Link registry
    // events from consuming the reply before the real new link is confirmed.
    let pending_link: PendingLink = Rc::new(RefCell::new(None));

    // -------------------------------------------------------------------------
    // Registry listener
    // -------------------------------------------------------------------------

    let event_tx_reg = event_tx.clone();
    let pending_sink_reg = pending_sink.clone();
    let pending_link_reg = pending_link.clone();
    let our_sinks_reg = our_sinks.clone();
    let our_links_reg = our_links.clone();

    let _registry_listener = registry
        .add_listener_local()
        .global(move |global| match global.type_ {
            pw::types::ObjectType::Node => {
                handle_node_global(
                    global,
                    &event_tx_reg,
                    &pending_sink_reg,
                    &our_sinks_reg,
                );
            }
            pw::types::ObjectType::Link => {
                handle_link_global(global, &pending_link_reg, &our_links_reg);
            }
            _ => {}
        })
        .global_remove(move |id| {
            let _ = event_tx.send(BackendEvent::NodeRemoved(id));
        })
        .register();

    // -------------------------------------------------------------------------
    // Command channel — attached to the mainloop for wakeup on new commands
    // -------------------------------------------------------------------------

    let core_cmd = core.clone();
    let our_sinks_cmd = our_sinks.clone();
    let our_links_cmd = our_links.clone();
    let pending_link_cmd = pending_link.clone();

    let _command_listener =
        command_rx.attach(mainloop.loop_(), move |cmd| match cmd {
            BackendCommand::CreateVirtualSink { name, reply } => {
                let node_name = format!("{}{}", AMPLITUDE_PREFIX, name);
                let display_name = format!("Amplitude {}", capitalise(&name));

                *pending_sink.borrow_mut() = Some((name.clone(), reply));

                let props = properties! {
                    "factory.name"     => "support.null-audio-sink",
                    "node.name"        => node_name.as_str(),
                    "node.description" => display_name.as_str(),
                    "media.class"      => "Audio/Sink",
                    "audio.position"   => "[ FL FR ]",
                    "node.virtual"     => "true",
                    "node.driver"      => "true",
                };

                match core_cmd
                    .create_object::<pw::node::Node>("adapter", &props)
                {
                    Ok(proxy) => {
                        our_sinks_cmd.borrow_mut().insert(u64::MAX, proxy);
                    }
                    Err(e) => {
                        let mut pending = pending_sink.borrow_mut();
                        if let Some((_, reply)) = pending.take() {
                            let _ = reply.send(Err(format!(
                                "create_object failed: {e}"
                            )));
                        }
                    }
                }
            }

            BackendCommand::DestroyVirtualSink { external_id } => {
                our_sinks_cmd.borrow_mut().remove(&external_id);
            }

            BackendCommand::CreateLink {
                output_node_id,
                input_node_id,
                reply,
            } => {
                *pending_link_cmd.borrow_mut() =
                    Some((output_node_id, input_node_id, reply));

                let out_str = output_node_id.to_string();
                let in_str = input_node_id.to_string();

                let props = properties! {
                    "link.output.node" => out_str.as_str(),
                    "link.input.node"  => in_str.as_str(),
                };

                match core_cmd
                    .create_object::<pw::link::Link>("link-factory", &props)
                {
                    Ok(proxy) => {
                        // Stash under sentinel until the registry resolves
                        // the real global ID.
                        our_links_cmd.borrow_mut().insert(u64::MAX, proxy);
                    }
                    Err(e) => {
                        if let Some((_, _, reply)) =
                            pending_link_cmd.borrow_mut().take()
                        {
                            let _ = reply.send(Err(format!(
                                "link create_object failed: {e}"
                            )));
                        }
                    }
                }
            }

            BackendCommand::DestroyLink { link_id } => {
                our_links_cmd.borrow_mut().remove(&link_id);
            }
        });

    mainloop.run();

    Ok(())
}

// ---------------------------------------------------------------------------
// Registry global handlers (extracted to keep pw_thread readable)
// ---------------------------------------------------------------------------

fn handle_node_global(
    global: &pipewire::registry::GlobalObject<
        &pipewire::spa::utils::dict::DictRef,
    >,
    event_tx: &mpsc::Sender<BackendEvent>,
    pending_sink: &Rc<RefCell<Option<(String, crate::audio::SinkReply)>>>,
    our_sinks: &Rc<RefCell<HashMap<u64, pw::node::Node>>>,
) {
    let props = match global.props {
        Some(p) => p,
        None => return,
    };

    let raw_class = props.get("media.class").unwrap_or("");
    let media_class = MediaClass::parse(raw_class);

    let node_name = props.get("node.name").unwrap_or("").to_owned();
    let is_amplitude_virtual = node_name.starts_with(AMPLITUDE_PREFIX);

    if is_amplitude_virtual {
        let suffix = node_name
            .strip_prefix(AMPLITUDE_PREFIX)
            .unwrap_or("")
            .to_owned();

        // Resolve a pending channel or bus sink creation command.
        let mut pending = pending_sink.borrow_mut();
        if let Some((ref expected_name, _)) = *pending {
            if suffix == *expected_name {
                // Promote proxy from sentinel to real ID.
                let mut sinks = our_sinks.borrow_mut();
                if let Some(proxy) = sinks.remove(&u64::MAX) {
                    sinks.insert(global.id as u64, proxy);
                }
                if let Some((_, reply)) = pending.take() {
                    let _ = reply.send(Ok(global.id as u64));
                }
            }
        }
    }

    // Only forward relevant classes as NodeAdded events to the engine.
    if !media_class.is_relevant() {
        return;
    }

    let info = NodeInfo {
        id: global.id,
        name: node_name,
        description: props.get("node.description").map(str::to_owned),
        app_name: props.get("application.name").map(str::to_owned),
        app_binary: props.get("application.process.binary").map(str::to_owned),
        media_class: Some(media_class),
        icon: props.get("application.icon-name").map(str::to_owned),
        is_amplitude_virtual,
    };

    let _ = event_tx.send(BackendEvent::NodeAdded(info));
}

fn handle_link_global(
    global: &pipewire::registry::GlobalObject<
        &pipewire::spa::utils::dict::DictRef,
    >,
    pending_link: &PendingLink,
    our_links: &Rc<RefCell<HashMap<u64, pw::link::Link>>>,
) {
    // Read the node IDs this link connects so we can validate against the
    // pending creation request. Without this, any foreign or pre-existing
    // Link global event would consume the pending reply slot prematurely.
    let (link_out, link_in) = match global.props {
        Some(props) => {
            let out = props
                .get("link.output.node")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let inp = props
                .get("link.input.node")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            (out, inp)
        }
        None => (0, 0),
    };

    let mut pending = pending_link.borrow_mut();
    if let Some((expected_out, expected_in, _)) = pending.as_ref() {
        if link_out == *expected_out && link_in == *expected_in {
            // Promote proxy from sentinel to real ID.
            let mut links = our_links.borrow_mut();
            if let Some(proxy) = links.remove(&u64::MAX) {
                links.insert(global.id as u64, proxy);
            }
            if let Some((_, _, reply)) = pending.take() {
                let _ = reply.send(Ok(global.id as u64));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Upper-cases the first Unicode character of `s`.
fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
