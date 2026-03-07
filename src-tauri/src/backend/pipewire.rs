use crate::audio::{AudioBackend, BackendCommand, BackendEvent, Sink};
use std::sync::mpsc;

use pipewire::{context::ContextBox, main_loop::MainLoopBox};

pub struct PipewireBackend {
    command_tx: mpsc::Sender<BackendCommand>,
    event_rx: pipewire::channel::Receiver<BackendEvent>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl PipewireBackend {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = pipewire::channel::channel();

        let thread_handle = std::thread::Builder::new()
            .name("Amplitude Pipewire Backend".to_string())
            .spawn(move || {
                if let Err(e) = pw_thread(command_rx, event_tx) {
                    println!("Pipewire thread failed: {}", e);
                };
            })
            .map_err(|e| format!("Failed to spawn Pipewire thread: {}", e))
            .unwrap();

        Self {
            command_tx,
            event_rx,
            thread_handle: Some(thread_handle),
        }
    }
}

impl AudioBackend for PipewireBackend {
    fn create_virtual_sink(&mut self, name: &str) -> Result<Sink, String> {
        // TODO: Create a real PipeWire virtual sink node
        Ok(Sink::new(format!("pipewire:{}", name)))
    }

    fn destroy_virtual_sink(&mut self, _sink: &Sink) -> Result<(), String> {
        // TODO: Destroy the PipeWire virtual sink node
        Ok(())
    }
}

pub fn create_backend() -> Box<dyn AudioBackend> {
    Box::new(PipewireBackend::new())
}

fn pw_thread(
    command_rx: mpsc::Receiver<BackendCommand>,
    event_tx: pipewire::channel::Sender<BackendEvent>,
) -> Result<(), String> {
    pipewire::init();

    let mainloop = MainLoopBox::new(None).expect("Failed to create mainloop");
    let context = ContextBox::new(mainloop.loop_(), None)
        .expect("Failed to create context");
    let core = context.connect(None).expect("Failed to connect to context");
    let registry = core.get_registry().expect("Failed to get registry");

    let _listener = registry
        .add_listener_local()
        .global(|global| {
            println!("Global event: {:?}", global);
        })
        .register();

    mainloop.run();

    Ok(())
}
