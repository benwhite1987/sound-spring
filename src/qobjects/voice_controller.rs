#[cxx_qt::bridge]
pub mod qobject {
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, spectrum_version)]
        #[qproperty(bool, is_capturing)]
        type VoiceController = super::VoiceControllerRust;

        #[qinvokable]
        fn set_visualization_active(self: Pin<&mut VoiceController>, active: bool);

        #[qinvokable]
        fn process_spectrum(self: Pin<&mut VoiceController>);

        #[qinvokable]
        fn spectrum_bin_count(self: &VoiceController) -> i32;

        #[qinvokable]
        fn spectrum_value_at(self: &VoiceController, index: i32) -> f64;

        #[qinvokable]
        fn open_pavucontrol(self: Pin<&mut VoiceController>);
    }

    impl cxx_qt::Constructor<()> for VoiceController {}
}

use core::pin::Pin;
use cxx_qt::{Constructor, CxxQtType};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::qobjects::controller::{BackendCommand, BACKEND_TX};
use crate::services::voice::{voice_shared, VoiceShared, SPECTRUM_BINS};

pub struct VoiceControllerRust {
    spectrum_version: i32,
    is_capturing: bool,
    shared: Arc<VoiceShared>,
    latest: Vec<f32>,
}

impl Default for VoiceControllerRust {
    fn default() -> Self {
        Self {
            spectrum_version: 0,
            is_capturing: false,
            shared: voice_shared(),
            latest: vec![0.0; SPECTRUM_BINS],
        }
    }
}

impl qobject::VoiceController {
    pub fn set_visualization_active(self: Pin<&mut Self>, active: bool) {
        if let Some(tx) = BACKEND_TX.get() {
            let command = if active {
                BackendCommand::StartVoiceCapture
            } else {
                BackendCommand::StopVoiceCapture
            };
            let _ = tx.blocking_send(command);
        }
    }

    /// Drain the shared spectrum queue (keeping only the newest frame) and bump
    /// `spectrum_version` so QML re-reads `spectrum_value_at`. Driven by a QML
    /// timer at the configured spectrum fps.
    pub fn process_spectrum(mut self: Pin<&mut Self>) {
        let capturing = self.rust().shared.capturing.load(Ordering::Relaxed);
        if capturing != self.rust().is_capturing {
            self.as_mut().set_is_capturing(capturing);
        }

        let mut newest: Option<Vec<f32>> = None;
        while let Some(frame) = self.rust().shared.spectrum.pop() {
            newest = Some(frame);
        }
        if let Some(frame) = newest {
            self.as_mut().rust_mut().latest = frame;
            let next = self.rust().spectrum_version.wrapping_add(1);
            self.as_mut().set_spectrum_version(next);
        }
    }

    pub fn spectrum_bin_count(&self) -> i32 {
        SPECTRUM_BINS as i32
    }

    pub fn spectrum_value_at(&self, index: i32) -> f64 {
        self.rust()
            .latest
            .get(index as usize)
            .copied()
            .unwrap_or(0.0) as f64
    }

    pub fn open_pavucontrol(self: Pin<&mut Self>) {
        std::thread::spawn(|| {
            if let Err(err) = std::process::Command::new("pavucontrol").spawn() {
                tracing::warn!("failed to launch pavucontrol: {err:#}");
            }
        });
    }
}

impl Constructor<()> for qobject::VoiceController {
    type NewArguments = ();
    type BaseArguments = ();
    type InitializeArguments = ();

    fn route_arguments(
        (): (),
    ) -> (
        Self::NewArguments,
        Self::BaseArguments,
        Self::InitializeArguments,
    ) {
        ((), (), ())
    }

    fn new((): ()) -> VoiceControllerRust {
        VoiceControllerRust::default()
    }

    fn initialize(self: Pin<&mut Self>, (): ()) {
        let _ = self;
    }
}
