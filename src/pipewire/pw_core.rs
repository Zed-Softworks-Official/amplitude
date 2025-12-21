#[cxx_qt::bridge]
pub mod qobject {

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type PwCore = super::PwCoreObject;

        #[qinvokable]
        fn initialize(self: Pin<&mut Self>) -> bool;

        #[qinvokable]
        fn cleanup(self: Pin<&mut Self>);
    }
}

use pw::{main_loop::MainLoopRc, context::ContextRc};
use crate::pipewire::pw_registry::{AudioNode, PwRegistry};
use std::thread;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Mutex};
use cxx_qt::CxxQtType;

#[derive(Default)]
pub struct PwCoreObject {
    channel: Option<pw::channel::Sender<PwCommand>>,
    event_rx: Option<std::sync::mpsc::Receiver<PwEvent>>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

pub enum PwCommand {
    Stop,
}

#[derive(Debug, Clone)]
pub enum PwEvent {
    NodeAdded(AudioNode),
    NodeRemoved(u32),
}

impl qobject::PwCore {
    fn initialize(mut self: Pin<&mut Self>) -> bool {
        log::info!("Initializing pipewire thread");

        let (cmd_tx, cmd_rx) = pw::channel::channel();
        let (event_tx, event_rx) = std::sync::mpsc::channel();

        // Spawn pipewire thread
        let handle = thread::spawn(move || {
            pw_thread(cmd_rx, event_tx);
        });

        self.as_mut().rust_mut().thread_handle = Some(handle);
        self.as_mut().rust_mut().channel = Some(cmd_tx);
        self.as_mut().rust_mut().event_rx = Some(event_rx);

        log::info!("pipewire thread started");

        true
    }

    fn cleanup(mut self: Pin<&mut Self>) {
        log::info!("Cleaning up pipewire thread");

        let mut rust = self.as_mut().rust_mut();

        if let Some(sender) = rust.channel.take() {
            let _ = sender.send(PwCommand::Stop);
        }

        if let Some(handle) = rust.thread_handle.take() {
            let _ = handle.join();
        }

        log::info!("pipewire thread stopped");
    }

}

fn pw_thread(
    cmd_rx: pw::channel::Receiver<PwCommand>,
    evt_tx: std::sync::mpsc::Sender<PwEvent>,
) {
    log::info!("Pipewire thread running");

    let mainloop = match MainLoopRc::new(None) {
        Ok(ml) => ml,
        Err(err) => {
            log::error!("failed to create mainloop: {}", err);
            return;
        }
    };

    let context = match ContextRc::new(&mainloop, None) {
        Ok(ctx) => ctx,
        Err(err) => {
            log::error!("failed to create context: {}", err);
            return;
        }
    };

    let core = match context.connect_rc(None) {
        Ok(core) => core,
        Err(err) => {
            log::error!("failed to connect to context: {}", err);
            return;
        }
    };

    let registry_data = Rc::new(Mutex::new(PwRegistry::new()));
    let registry = match core.get_registry_rc() {
        Ok(reg) => reg,
        Err(err) => {
            log::error!("failed to get registry: {}", err);
            return;
        }
    };

    let registry_clone = registry_data.clone();
    let evt_tx_clone = evt_tx.clone();

    let _listener = registry
        .add_listener_local()
        .global(move |global| {
            let mut reg = registry_clone.lock().unwrap();
            reg.add_node(global);

            if let Some(node) = reg.get_node(global.id).cloned() {
                let _ = evt_tx_clone.send(PwEvent::NodeAdded(node));
            }
        })
        .register();
            /*
        .global_remove(move |id| {
            let mut reg = registry_clone.lock().unwrap();
            reg.remove_node(&id);

            let _ = evt_tx_clone.send(PwEvent::NodeRemoved(id));
        });
*/

    let _receiver = cmd_rx.attach(&mainloop.loop_(), {
        let mainloop = mainloop.clone();
        move |msg| match msg {
            PwCommand::Stop => {
                log::info!("Received stop command");
                mainloop.quit();
            }
        }
    });

    log::info!("Starting pipewire mainloop");
    mainloop.run();
    log::info!("Pipewire mainloop stopped");
}
