use std::{
    thread,
    sync::{Arc, Mutex},
    collections::HashMap
};

use log::error;

use pw::{
    main_loop::MainLoopRc,
    context::ContextRc,
};

use crate::audio::backend::{
    AudioBackend,
    AudioEvent,
    AudioNode,
    BackendCommand,
    MediaClass,
};

use tokio::sync::mpsc;

pub struct PipewireBackend {
    thread_handle: Option<thread::JoinHandle<()>>,
    command_sender: Option<pw::channel::Sender<BackendCommand>>,
    event_receiver: Arc<Mutex<mpsc::Receiver<AudioEvent>>>,
    nodes: Arc<Mutex<HashMap<u32, AudioNode>>>,
}

impl AudioBackend for PipewireBackend {
    fn new() -> Self {
        let (pw_sender, pw_receiver) = pw::channel::channel::<BackendCommand>();
        let (event_sender, event_receiver) = mpsc::channel::<AudioEvent>(100);

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

    fn send_command(&self, cmd: BackendCommand) {
        if let Some(sender) = &self.command_sender {
            let _ = sender.send(cmd);
        }
    }

    fn get_event_receiver(&self) -> Arc<Mutex<mpsc::Receiver<AudioEvent>>> {
        Arc::clone(&self.event_receiver)
    }

    fn process_event(&self, event: AudioEvent) {
        match event {
            AudioEvent::NodeAdded(node) => {
                self.nodes.lock().unwrap().insert(node.id, node);
            },
            AudioEvent::NodeRemoved(id) => {
                self.nodes.lock().unwrap().remove(&id);
            }
        }
    }

    fn get_nodes(&self) -> Arc<Mutex<HashMap<u32, AudioNode>>> {
        Arc::clone(&self.nodes)
    }
}

fn pw_thread(
    main_sender: mpsc::Sender<AudioEvent>,
    pw_receiver: pw::channel::Receiver<BackendCommand>
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

            let node = AudioNode::from_props(global.id, &props);
            if node.media_class == MediaClass::Unknown {
                return;
            }

            let _ = sender_add.blocking_send(AudioEvent::NodeAdded(node));
        })
        .global_remove(move |id| {
            let _ = sender_remove.blocking_send(AudioEvent::NodeRemoved(id));
        })
        .register();

    // Handle commands from the app
    let mainloop_weak = mainloop.downgrade();
    let _receiver = pw_receiver.attach(mainloop.loop_(), move |cmd| {
        match cmd {
            BackendCommand::Terminate => {
                if let Some(mainloop) = mainloop_weak.upgrade() {
                    mainloop.quit();
                }
            }
        }
    });

   mainloop.run();
}
