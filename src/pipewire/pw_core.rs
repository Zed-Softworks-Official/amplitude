use tokio::sync::mpsc;
use std::thread;
use std::collections::HashMap;
use pw::{main_loop::MainLoopRc, context::ContextRc};

use crate::pipewire::pw_node::{PwNode, MediaClass};

#[derive(Default)]
pub struct PwCore {
    thread_handle: Option<thread::JoinHandle<()>>,
    command_sender: Option<pw::channel::Sender<PwCommand>>,
    event_receiver: Option<mpsc::Receiver<PwEvent>>,
}

// Commands Sent FROM tokio TO pipewire
pub enum PwCommand {
    Terminate,
}

// Events sent FROM pipewire TO tokio
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
            event_receiver: Some(event_receiver),
        }
    }

    pub fn send_command(&self, cmd: PwCommand) {
        if let Some(sender) = &self.command_sender {
            let _ = sender.send(cmd);
        }
    }

    pub fn try_recv(&mut self) -> Option<PwEvent> {
        self.event_receiver.as_mut()?.try_recv().ok()
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
            eprintln!("Failed to create main loop: {}", err);
            return;
        }
    };

    let context = match ContextRc::new(&mainloop, None) {
        Ok(context) => context,
        Err(err) => {
            eprintln!("Failed to create context: {}", err);
            return;
        }
    };

    let core = match context.connect_rc(None) {
        Ok(core) => core,
        Err(err) => {
            eprintln!("Failed to connect to context: {}", err);
            return;
        }
    };

    let registry = match core.get_registry() {
        Ok(registry) => registry,
        Err(err) => {
            eprintln!("Failed to get registry: {}", err);
            return;
        }
    };

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

            println!("Node Added: {:?}", node);
        })
        .register();

    mainloop.run();
}
