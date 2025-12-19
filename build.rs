use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("dev.zedsoftworks.amplitude")
            .qml_file("qml/main.qml")
    )
        .files([
            "src/audio/channel.rs",
            "src/audio/bus.rs",
        ])
        .build();
}
