use tokio::sync::mpsc;
use log::{info, error};
use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use pw::{main_loop::MainLoopRc, context::ContextRc};

use crate::pipewire::pw_node::{PwNode, MediaClass};

pub struct PwCore {
    thread_handle: Option<thread::JoinHandle<()>>,
    command_sender: Option<pw::channel::Sender<PwCommand>>,
    event_receiver: Arc<Mutex<mpsc::Receiver<PwEvent>>>,
    nodes: Arc<Mutex<HashMap<u32, PwNode>>>,
}

// Commands Sent FROM tokio TO pipewire
pub enum PwCommand {
    Terminate,
}

// Events sent FROM pipewire TO tokio
#[derive(Debug, Clone)]
pub enum PwEvent {
    NodeAdded(PwNode),
    NodeRemoved(u32),
}

impl PwCore {
    pub fn new() -> Self {
        let (pw_sender, pw_receiver) = pw::channel::channel::<PwCommand>();
        let (event_sender, event_receiver) = mpsc::channel::<PwEvent>(100);

        let thread_handle = thread::spawn(move || {
            pw_thread(event_sender, pw_receiver);
        });

        Self {
            thread_handle: Some(thread_handle),
            command_sender: Some(pw_sender),
            event_receiver: Arc::new(Mutex::new(event_receiver)),
            nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn send_command(&self, cmd: PwCommand) {
        if let Some(sender) = &self.command_sender {
            let _ = sender.send(cmd);
        }
    }

    pub fn get_event_receiver(&self) -> Arc<Mutex<mpsc::Receiver<PwEvent>>> {
        Arc::clone(&self.event_receiver)
    }

    pub fn process_events(&self) {
        let mut receiver = self.event_receiver.lock().unwrap();
        while let Ok(event) = receiver.try_recv() {
            match event {
                PwEvent::NodeAdded(node) => {
                    self.nodes.lock().unwrap().insert(node.id, node);
                },
                PwEvent::NodeRemoved(id) => {
                    self.nodes.lock().unwrap().remove(&id);
                }
            }
        }
    }
}

fn pw_thread(
    main_sender: mpsc::Sender<PwEvent>,
    pw_receiver: pw::channel::Receiver<PwCommand>
) {
    pw::init();

    let mainloop = match MainLoopRc::new(None) {
        Ok(mainloop) => mainloop,
        Err(err) => {
            error!("Failed to create main loop: {}", err);
            return;
        }
    };

    let context = match ContextRc::new(&mainloop, None) {
        Ok(context) => context,
        Err(err) => {
            error!("Failed to create context: {}", err);
            return;
        }
    };

    let core = match context.connect_rc(None) {
        Ok(core) => core,
        Err(err) => {
            error!("Failed to connect to context: {}", err);
            return;
        }
    };

    let registry = match core.get_registry() {
        Ok(registry) => registry,
        Err(err) => {
            error!("Failed to get registry: {}", err);
            return;
        }
    };

    let sender_add = main_sender.clone();
    let sender_remove = main_sender;

    // Listen for new nodes
    let _listener = registry.add_listener_local()
        .global(move |global| {
            if global.type_ != pw::types::ObjectType::Node {
                return;
            }

            let props: HashMap<String, String> = global
                .props
                .as_ref()
                .map(|props| {
                    props.iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let node = PwNode::from_props(global.id, &props);
            if node.media_class == MediaClass::Unknown {
                return;
            }

            let _ = sender_add.blocking_send(PwEvent::NodeAdded(node));
        })
        .global_remove(move |id| {
            let _ = sender_remove.blocking_send(PwEvent::NodeRemoved(id));
        })
        .register();

    // Handle commands from the app
    let mainloop_weak = mainloop.downgrade();
    let _receiver = pw_receiver.attach(mainloop.loop_(), move |cmd| {
        match cmd {
            PwCommand::Terminate => {
                if let Some(mainloop) = mainloop_weak.upgrade() {
                    mainloop.quit();
                }
            }
        }
    });

   mainloop.run();
}
