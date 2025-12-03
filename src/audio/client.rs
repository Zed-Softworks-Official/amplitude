use pipewire::{main_loop::MainLoopBox, context::ContextBox};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct PipeWireClient {
    main_loop: pw::MainLoop,
    context: pw::Context,
    core: pw::Core,
    registry: pw::Registry,
    event_sender: mpsc::UnboundedSender<AudioEvent>,
}

#[derive(Debug, Clone)]
pub enum AudioEvent {
    NodeAdded { id: u32, info: NodeInfo },
    NodeRemoved { id: u32 },
    ConnectionChanged
}

impl PipeWireClient {
    pub fn new() -> anyhow::Result<Self> {
        let main_loop = MainLoopBox::new(None)?;
        let context = ContextBox::new(&main_loop)?;
        let core = context.connect(None)?;

        let (event_sender, _event_receiver) = mpsc::unbounded_channel();

        let registry = core.get_registry()?;

        Ok(Self {
            main_loop,
            context,
            core,
            registry,
            event_sender
        })
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        self.setup_registry_listener()?;

        self.main_loop.run();

        Ok(())
    }

    fn setup_registry_listener(&self) -> anyhow::Result<()> {
        // Implementation for discovering nodes
        Ok(())
    }
}
