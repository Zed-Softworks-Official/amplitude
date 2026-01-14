use crate::audio::backend::{
    AudioBackend, AudioEvent, AudioNode, BackendCommand,
};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::mpsc;

pub struct CoreAudioBackend {
    thread_handle: Option<thread::JoinHandle<()>>,
    command_sender: Option<mpsc::Sender<BackendCommand>>,
    event_receiver: Arc<Mutex<mpsc::Receiver<AudioEvent>>>,
    nodes: Arc<Mutex<HashMap<u32, AudioNode>>>,
}

impl AudioBackend for CoreAudioBackend {
    fn new() -> Self {
        let (command_sender, command_receiver) =
            mpsc::channel::<BackendCommand>(100);
        let (event_sender, event_receiver) = mpsc::channel::<AudioEvent>(100);

        let thread_handle = thread::spawn(move || {
            coreaudio_thread(event_sender, command_receiver);
        });

        Self {
            thread_handle: Some(thread_handle),
            command_sender: Some(command_sender),
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
        self.event_receiver.clone()
    }

    fn process_event(&self, event: AudioEvent) {
        match event {
            AudioEvent::NodeAdded(node) => {
                self.nodes
                    .lock()
                    .unwrap()
                    .insert(node.id, node);
            }
            AudioEvent::NodeRemoved(id) => {
                self.nodes.lock().unwrap().remove(&id);
            }
        }
    }

    fn get_nodes(&self) -> Arc<Mutex<HashMap<u32, AudioNode>>> {
        Arc::clone(&self.nodes)
    }
}

fn coreaudio_thread(
    main_sender: mpsc::Sender<AudioEvent>,
    command_receiver: mpsc::Receiver<BackendCommand>,
) {
    log::info!("CoreAudio backend thread started");
}
