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
// Name prefix used to tag all Amplitude-owned virtual sink nodes
// ---------------------------------------------------------------------------

const AMPLITUDE_PREFIX: &str = "amplitude-";

/// How long `create_virtual_sink` waits for the PW thread to confirm the node
/// before giving up and returning an error instead of blocking forever.
const SINK_CREATE_TIMEOUT: Duration = Duration::from_secs(5);

// ---------------------------------------------------------------------------
// Public backend struct (lives on the engine/Tauri thread)
// ---------------------------------------------------------------------------

pub struct PipewireBackend {
    /// Send commands to the PipeWire thread.
    /// Uses pipewire::channel so the PW mainloop wakes up when a command arrives.
    command_tx: pipewire::channel::Sender<BackendCommand>,
    /// Receive node events emitted by the PipeWire thread.
    /// Uses std::sync::mpsc so the engine can poll without blocking.
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

        // Use a timeout so a misconfigured PipeWire server (e.g. missing
        // support.null-audio-sink factory) surfaces as a clean error instead
        // of blocking the Tauri setup thread forever.
        let external_id =
            reply_rx.recv_timeout(SINK_CREATE_TIMEOUT).map_err(|e| {
                format!(
                    "PipeWire sink creation timed out or channel closed: {e}"
                )
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

    // Proxies for our own virtual sinks: external_id → Node proxy.
    // Must stay alive or PipeWire will destroy the corresponding node.
    // Shared between the command handler (which inserts under u64::MAX) and
    // the registry callback (which promotes to the real global ID).
    let our_sinks: Rc<RefCell<HashMap<u64, pw::node::Node>>> =
        Rc::new(RefCell::new(HashMap::new()));

    // Pending sink creation: set by the command handler, resolved by the
    // registry global callback when the new node appears.
    #[allow(clippy::type_complexity)]
    let pending_create: Rc<
        RefCell<Option<(String, crate::audio::SinkReply)>>,
    > = Rc::new(RefCell::new(None));

    // -------------------------------------------------------------------------
    // Registry listener
    // -------------------------------------------------------------------------

    let event_tx_reg = event_tx.clone();
    let pending_create_reg = pending_create.clone();
    let our_sinks_reg = our_sinks.clone();

    let _registry_listener = registry
        .add_listener_local()
        .global(move |global| {
            if global.type_ != pw::types::ObjectType::Node {
                return;
            }

            let props = match global.props {
                Some(p) => p,
                None => return,
            };

            let raw_class = props.get("media.class").unwrap_or("");
            let media_class = MediaClass::parse(raw_class);

            let node_name = props.get("node.name").unwrap_or("").to_owned();
            let is_amplitude_virtual = node_name.starts_with(AMPLITUDE_PREFIX);

            // Resolve a pending sink creation if this is our new node.
            // Do this before the relevance check so we always catch our own
            // nodes even if their media.class isn't in the relevant set.
            if is_amplitude_virtual {
                let mut pending = pending_create_reg.borrow_mut();
                if let Some((ref expected_name, _)) = *pending {
                    let full_name =
                        format!("{}{}", AMPLITUDE_PREFIX, expected_name);
                    if node_name == full_name {
                        // Promote the proxy from the sentinel slot to the
                        // real global ID so destroy_virtual_sink can find it.
                        let mut sinks = our_sinks_reg.borrow_mut();
                        if let Some(proxy) = sinks.remove(&u64::MAX) {
                            sinks.insert(global.id as u64, proxy);
                        }

                        if let Some((_, reply)) = pending.take() {
                            let _ = reply.send(Ok(global.id as u64));
                        }
                    }
                }
            }

            // Only surface relevant classes to the engine.
            if !media_class.is_relevant() {
                return;
            }

            let info = NodeInfo {
                id: global.id,
                name: node_name,
                description: props.get("node.description").map(str::to_owned),
                app_name: props.get("application.name").map(str::to_owned),
                app_binary: props
                    .get("application.process.binary")
                    .map(str::to_owned),
                media_class: Some(media_class),
                icon: props.get("application.icon-name").map(str::to_owned),
                is_amplitude_virtual,
            };

            let _ = event_tx_reg.send(BackendEvent::NodeAdded(info));
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

    let _command_listener =
        command_rx.attach(mainloop.loop_(), move |cmd| match cmd {
            BackendCommand::CreateVirtualSink { name, reply } => {
                let node_name = format!("{}{}", AMPLITUDE_PREFIX, name);

                *pending_create.borrow_mut() = Some((name.clone(), reply));

                // Use the "adapter" factory with "factory.name" pointing at
                // the SPA null-audio-sink.  Passing "support.null-audio-sink"
                // directly as the factory name to create_object does not work
                // because it is a SPA factory, not a PipeWire server factory.
                let props = properties! {
                    "factory.name"     => "support.null-audio-sink",
                    "node.name"        => node_name.as_str(),
                    "node.description" => name.as_str(),
                    "media.class"      => "Audio/Sink",
                    "audio.position"   => "[ FL FR ]",
                    "node.virtual"     => "true",
                };

                match core_cmd
                    .create_object::<pw::node::Node>("adapter", &props)
                {
                    Ok(node) => {
                        // Stash under sentinel until the registry global
                        // callback fires and promotes it to the real ID.
                        our_sinks_cmd.borrow_mut().insert(u64::MAX, node);
                    }
                    Err(e) => {
                        let mut pending = pending_create.borrow_mut();
                        if let Some((_, reply)) = pending.take() {
                            let _ = reply.send(Err(format!(
                                "create_object failed: {e}"
                            )));
                        }
                    }
                }
            }

            BackendCommand::DestroyVirtualSink { external_id } => {
                // Dropping the proxy signals PipeWire to destroy the node.
                our_sinks_cmd.borrow_mut().remove(&external_id);
            }
        });

    mainloop.run();

    Ok(())
}
