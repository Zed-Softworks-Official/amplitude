use crate::audio::backend::{
    AudioBackend,
    BackendCommand,
    AudioEvent
};

use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use std::thread;

pub struct CoreAudioBackend {
    thread_handle: Option<thread::JoinHandle<()>>,
    command_sender: Option<String>,
    event_receiver: Arc<Mutex<mpsc::Receiver<AudioEvent>>>,
}

impl AudioBackend for CoreAudioBackend {
    fn new() -> Self {
        let (_event_sender, event_receiver) = mpsc::channel::<AudioEvent>(100);

        Self {
            thread_handle: None,
            command_sender: None,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
        }
    }

    fn send_command(&self, cmd: BackendCommand) {
    }

    fn get_event_receiver(&self) -> Arc<Mutex<mpsc::Receiver<AudioEvent>>> {
        self.event_receiver.clone()
    }

    fn process_events(&self) {
    }
}
