use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("dev.zedsoftworks.amplitude")
            .qml_file("qml/main.qml")
    )
        .files([
            "src/pipewire/pw_core.rs",
        ])
        .build();
}
