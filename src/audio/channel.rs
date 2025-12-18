#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, id)]
        #[qproperty(QString, channel_name)]
        #[qproperty(f32, montior_volume)]
        #[qproperty(bool, monitor_muted)]
        #[qproperty(f32, stream_volume)]
        #[qproperty(bool, stream_muted)]
        type Channel = super::ChannelObject;
    }
}

use cxx_qt_lib::QString;

#[derive(Default)]
pub struct ChannelObject {
    id: QString,
    channel_name: QString,
    montior_volume: f32,
    monitor_muted: bool,
    stream_volume: f32,
    stream_muted: bool,
}
