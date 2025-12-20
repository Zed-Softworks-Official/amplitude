#[cxx_qt::bridge]
pub mod qobject {

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type PwCore = super::PwCoreObject;

        #[qinvokable]
        fn initialize(self: Pin<&mut Self>) -> bool;
    }
}

use pw::{main_loop::MainLoopRc, context::ContextRc, core::CoreRc};
use cxx_qt::CxxQtType;
use core::pin::Pin;
use log::{info, error};

#[derive(Default)]
pub struct PwCoreObject {
    mainloop: Option<MainLoopRc>,
    context: Option<ContextRc>,
    core: Option<CoreRc>
}

impl qobject::PwCore {
    fn initialize(mut self: Pin<&mut Self>) -> bool {
        let mainloop = match MainLoopRc::new(None) {
            Ok(mainloop) => mainloop,
            Err(err) => {
                error!("failed to create mainloop: {}", err);
                return false;
            }
        };

        let context = match ContextRc::new(&mainloop, None) {
            Ok(context) => context,
            Err(err) => {
                error!("failed to create context: {}", err);
                return false;
            }
        };

        let core = match context.connect_rc(None) {
            Ok(core) => core,
            Err(err) => {
                error!("failed to connect context: {}", err);
                return false;
            }
        };

        self.as_mut().rust_mut().mainloop = Some(mainloop);
        self.as_mut().rust_mut().context = Some(context);
        self.as_mut().rust_mut().core = Some(core);

        info!("pipewire initialized");

        true
    }
}
