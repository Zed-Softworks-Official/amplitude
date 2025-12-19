#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, name)]
        #[qproperty(f32, volume)]
        #[qproperty(bool, muted)]
        #[qproperty(f32, level)]
        type Bus = super::BusObject;
    }

}

use cxx_qt_lib::QString;

#[derive(Default)]
pub struct BusObject {
    name: QString,
    volume: f32,
    muted: bool,
    level: f32,
}
