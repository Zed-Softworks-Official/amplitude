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

#[derive(Default)]
pub struct PwCoreObject {
    mainloop: Option<MainLoopRc>,
    context: Option<ContextRc>,
    core: Option<CoreRc>
}

impl qobject::PwCore {
    pub fn initialize(mut self: Pin<&mut Self>) -> bool {
        let mainloop = match MainLoopRc::new(None) {
            Ok(mainloop) => mainloop,
            Err(err) => {
                eprintln!("Failed to create mainloop: {}", err);
                return false;
            }
        };

        let context = match ContextRc::new(&mainloop, None) {
            Ok(context) => context,
            Err(err) => {
                eprintln!("Failed to create context: {}", err);
                return false;
            }
        };

        let core = match context.connect_rc(None) {
            Ok(core) => core,
            Err(err) => {
                eprintln!("Failed to connect to context: {}", err);
                return false;
            }
        };

        self.as_mut().rust_mut().mainloop = Some(mainloop);
        self.as_mut().rust_mut().context = Some(context);
        self.as_mut().rust_mut().core = Some(core);

        println!("Initialized pipewire");

        true
    }
}
