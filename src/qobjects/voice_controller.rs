#[cxx_qt::bridge]
pub mod qobject {
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, spectrum_version)]
        #[qproperty(bool, is_capturing)]
        #[qproperty(f32, vad_probability)]
        #[qproperty(bool, is_speaking)]
        #[qproperty(bool, is_passing)]
        #[qproperty(f32, speaker_match_score)]
        #[qproperty(bool, is_enrolled)]
        #[qproperty(bool, enroll_active)]
        #[qproperty(f32, enroll_progress)]
        #[qproperty(bool, verification_enabled)]
        #[qproperty(f32, match_threshold)]
        #[qproperty(bool, suppression_enabled)]
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

        #[qinvokable]
        fn set_verification(self: Pin<&mut VoiceController>, enabled: bool);

        #[qinvokable]
        fn set_threshold(self: Pin<&mut VoiceController>, threshold: f32);

        #[qinvokable]
        fn set_suppression(self: Pin<&mut VoiceController>, enabled: bool);

        #[qinvokable]
        fn start_enrollment(self: Pin<&mut VoiceController>);

        #[qinvokable]
        fn cancel_enrollment(self: Pin<&mut VoiceController>);

        #[qinvokable]
        fn clear_enrollment(self: Pin<&mut VoiceController>);

        #[qsignal]
        fn enrollment_progress(self: Pin<&mut VoiceController>, percent: i32);

        #[qsignal]
        fn enrollment_complete(self: Pin<&mut VoiceController>);
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
    vad_probability: f32,
    is_speaking: bool,
    is_passing: bool,
    speaker_match_score: f32,
    is_enrolled: bool,
    enroll_active: bool,
    enroll_progress: f32,
    verification_enabled: bool,
    match_threshold: f32,
    suppression_enabled: bool,
    shared: Arc<VoiceShared>,
    latest: Vec<f32>,
    last_enroll_done_seq: u32,
    last_progress_percent: i32,
}

impl Default for VoiceControllerRust {
    fn default() -> Self {
        let config = crate::config::load_config().unwrap_or_default();
        let enrolled = crate::config::voiceprint_path(&config).is_file();
        let shared = voice_shared();
        shared.set_enrolled(enrolled);
        shared.set_verification_enabled(config.voice.verification_enabled);
        shared.set_match_threshold(config.voice.match_threshold);
        Self {
            spectrum_version: 0,
            is_capturing: false,
            vad_probability: 0.0,
            is_speaking: false,
            is_passing: false,
            speaker_match_score: 0.0,
            is_enrolled: enrolled,
            enroll_active: false,
            enroll_progress: 0.0,
            verification_enabled: config.voice.verification_enabled,
            match_threshold: config.voice.match_threshold,
            suppression_enabled: config.voice.suppression_enabled,
            shared,
            latest: vec![0.0; SPECTRUM_BINS],
            last_enroll_done_seq: shared_done_seq(),
            last_progress_percent: 0,
        }
    }
}

fn shared_done_seq() -> u32 {
    voice_shared().enroll_done_seq()
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
    /// `spectrum_version` so QML re-reads `spectrum_value_at`. Also refreshes the
    /// VAD and speaker-verification properties. Driven by a QML timer.
    pub fn process_spectrum(mut self: Pin<&mut Self>) {
        let capturing = self.rust().shared.capturing.load(Ordering::Relaxed);
        if capturing != self.rust().is_capturing {
            self.as_mut().set_is_capturing(capturing);
        }

        let (probability, speaking) = self.rust().shared.vad_state();
        if speaking != self.rust().is_speaking {
            self.as_mut().set_is_speaking(speaking);
        }
        if (probability - self.rust().vad_probability).abs() > f32::EPSILON {
            self.as_mut().set_vad_probability(probability);
        }

        let (score, _matched) = self.rust().shared.speaker_state();
        if (score - self.rust().speaker_match_score).abs() > f32::EPSILON {
            self.as_mut().set_speaker_match_score(score);
        }
        let passing = self.rust().shared.is_passing();
        if passing != self.rust().is_passing {
            self.as_mut().set_is_passing(passing);
        }
        let enrolled = self.rust().shared.is_enrolled();
        if enrolled != self.rust().is_enrolled {
            self.as_mut().set_is_enrolled(enrolled);
        }
        let enroll_active = self.rust().shared.enroll_active();
        if enroll_active != self.rust().enroll_active {
            self.as_mut().set_enroll_active(enroll_active);
        }

        let progress = self.rust().shared.enroll_progress();
        if (progress - self.rust().enroll_progress).abs() > f32::EPSILON {
            self.as_mut().set_enroll_progress(progress);
        }
        let percent = (progress * 100.0).round() as i32;
        if percent != self.rust().last_progress_percent {
            self.as_mut().rust_mut().last_progress_percent = percent;
            self.as_mut().enrollment_progress(percent);
        }

        let done_seq = self.rust().shared.enroll_done_seq();
        if done_seq != self.rust().last_enroll_done_seq {
            self.as_mut().rust_mut().last_enroll_done_seq = done_seq;
            self.as_mut().set_is_enrolled(true);
            self.as_mut().enrollment_complete();
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

    pub fn set_verification(mut self: Pin<&mut Self>, enabled: bool) {
        self.rust().shared.set_verification_enabled(enabled);
        self.as_mut().set_verification_enabled(enabled);
        persist_verification(enabled, self.rust().match_threshold);
    }

    pub fn set_threshold(mut self: Pin<&mut Self>, threshold: f32) {
        let threshold = threshold.clamp(0.0, 1.0);
        self.rust().shared.set_match_threshold(threshold);
        self.as_mut().set_match_threshold(threshold);
        persist_verification(self.rust().verification_enabled, threshold);
    }

    pub fn set_suppression(mut self: Pin<&mut Self>, enabled: bool) {
        self.as_mut().set_suppression_enabled(enabled);
        if let Some(tx) = BACKEND_TX.get() {
            let _ = tx.blocking_send(BackendCommand::SetVoiceSuppression { enabled });
        }
    }

    pub fn start_enrollment(self: Pin<&mut Self>) {
        // Enrollment needs a live capture; ensure it is running.
        if let Some(tx) = BACKEND_TX.get() {
            let _ = tx.blocking_send(BackendCommand::StartVoiceCapture);
        }
        self.rust().shared.request_enroll_start();
    }

    pub fn cancel_enrollment(self: Pin<&mut Self>) {
        self.rust().shared.request_enroll_cancel();
    }

    pub fn clear_enrollment(mut self: Pin<&mut Self>) {
        let config = crate::config::load_config().unwrap_or_default();
        let path = crate::config::voiceprint_path(&config);
        if path.exists() {
            if let Err(err) = std::fs::remove_file(&path) {
                tracing::warn!("failed to remove voiceprint {}: {err:#}", path.display());
            }
        }
        self.rust().shared.set_enrolled(false);
        self.rust().shared.set_speaker(0.0, false);
        self.rust().shared.request_enroll_clear();
        self.as_mut().set_is_enrolled(false);
        self.as_mut().set_speaker_match_score(0.0);
    }
}

fn persist_verification(enabled: bool, threshold: f32) {
    if let Some(tx) = BACKEND_TX.get() {
        let _ = tx.blocking_send(BackendCommand::SetVoiceVerification { enabled, threshold });
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
