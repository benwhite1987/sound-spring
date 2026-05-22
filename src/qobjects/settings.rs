#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, mic_source)]
        #[qproperty(i32, latency_ms)]
        type Settings = super::SettingsRust;

        #[qinvokable]
        fn apply(self: Pin<&mut Settings>);
    }
}

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

#[derive(Default)]
pub struct SettingsRust {
    mic_source: QString,
    latency_ms: i32,
}

impl qobject::Settings {
    pub fn apply(self: Pin<&mut Self>) {
        let _mic = String::from(self.mic_source());
        let _latency = *self.latency_ms();
    }
}
