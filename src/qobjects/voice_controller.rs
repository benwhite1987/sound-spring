#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[auto_cxx_name]
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
        #[qproperty(f32, vad_open_threshold)]
        #[qproperty(bool, suppression_enabled)]
        #[qproperty(bool, vad_enabled)]
        #[qproperty(bool, mic_muted)]
        #[qproperty(QString, spectrum_source)]
        #[qproperty(QString, capture_error)]
        #[qproperty(i32, spectrum_bar_count)]
        #[qproperty(i32, spectrum_segment_count)]
        #[qproperty(f32, spectrum_db_min)]
        #[qproperty(f32, spectrum_db_max)]
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
        fn bar_level_at(self: &VoiceController, index: i32) -> f64;

        #[qinvokable]
        fn lit_segment_count_at(self: &VoiceController, level: f64) -> i32;

        #[qinvokable]
        fn spectrum_segment_db_at(self: &VoiceController, index: i32) -> f64;

        #[qinvokable]
        fn spectrum_segment_y_frac_at(self: &VoiceController, index: i32) -> f64;

        #[qinvokable]
        fn open_pavucontrol(self: Pin<&mut VoiceController>);

        #[qinvokable]
        fn set_verification(self: Pin<&mut VoiceController>, enabled: bool);

        #[qinvokable]
        fn set_threshold(self: Pin<&mut VoiceController>, threshold: f32);

        #[qinvokable]
        fn set_vad_threshold(self: Pin<&mut VoiceController>, open: f32);

        #[qinvokable]
        fn set_suppression(self: Pin<&mut VoiceController>, enabled: bool);

        #[qinvokable]
        fn persist_vad_enabled(self: Pin<&mut VoiceController>, enabled: bool);

        #[qinvokable]
        fn toggle_mic_mute(self: Pin<&mut VoiceController>);

        #[qinvokable]
        fn persist_mic_muted(self: Pin<&mut VoiceController>, muted: bool);

        #[qinvokable]
        fn persist_spectrum_source(self: Pin<&mut VoiceController>, source: QString);

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
use cxx_qt_lib::QString;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::qobjects::controller::{BackendCommand, send_backend, try_send_backend};
use crate::services::voice::{
    spectrum_bars::{
        lit_segment_count_for_level, BarBallistics, DB_MAX, DB_MIN, SEGMENT_COUNT, SEGMENT_DB,
        SEGMENT_Y_FRAC, SPECTRUM_BAR_COUNT,
    },
    spectrum_meter::{apply_fader_db, map_to_bar_targets},
    spectrum_source_from_str, vad_close_for_open, voice_shared, VoiceShared, SPECTRUM_BINS,
    SPECTRUM_SOURCE_FILTERED, SPECTRUM_SOURCE_MIXED,
};

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
    vad_open_threshold: f32,
    suppression_enabled: bool,
    vad_enabled: bool,
    mic_muted: bool,
    spectrum_source: QString,
    capture_error: QString,
    spectrum_bar_count: i32,
    spectrum_segment_count: i32,
    spectrum_db_min: f32,
    spectrum_db_max: f32,
    capture_error_buf: String,
    last_capture_error_seq: u32,
    shared: Arc<VoiceShared>,
    latest: Vec<f32>,
    bar_levels: [f32; SPECTRUM_BAR_COUNT],
    spectrum_source_code: u8,
    last_enroll_done_seq: u32,
    last_progress_percent: i32,
    last_spectrum_generation: u32,
    bar_ballistics: BarBallistics,
    scaled_buf: Vec<f32>,
}

impl Default for VoiceControllerRust {
    fn default() -> Self {
        let config = crate::config::load_config().unwrap_or_default();
        let enrolled = crate::config::voiceprint_path(&config).is_file();
        let shared = voice_shared();
        shared.set_enrolled(enrolled);
        shared.set_verification_enabled(config.voice.verification_enabled);
        shared.set_match_threshold(config.voice.match_threshold);
        shared.set_vad_thresholds(
            config.voice.vad_open_threshold,
            config.voice.vad_close_threshold,
        );
        shared.set_vad_enabled(config.voice.vad_enabled);
        shared.set_spectrum_source(spectrum_source_from_str(&config.voice.spectrum_source));
        let last_spectrum_generation = shared.spectrum_generation();
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
            vad_open_threshold: config.voice.vad_open_threshold,
            suppression_enabled: config.voice.suppression_enabled,
            vad_enabled: config.voice.vad_enabled,
            mic_muted: config.audio.mic_muted,
            spectrum_source: QString::from(config.voice.spectrum_source.as_str()),
            capture_error: QString::from(""),
            spectrum_bar_count: SPECTRUM_BAR_COUNT as i32,
            spectrum_segment_count: SEGMENT_COUNT as i32,
            spectrum_db_min: DB_MIN,
            spectrum_db_max: DB_MAX,
            capture_error_buf: String::new(),
            last_capture_error_seq: 0,
            shared,
            latest: vec![0.0; SPECTRUM_BINS],
            bar_levels: [0.0; SPECTRUM_BAR_COUNT],
            spectrum_source_code: spectrum_source_from_str(&config.voice.spectrum_source),
            last_enroll_done_seq: shared_done_seq(),
            last_progress_percent: 0,
            last_spectrum_generation,
            bar_ballistics: BarBallistics::new(),
            scaled_buf: vec![0.0; SPECTRUM_BINS],
        }
    }
}

fn shared_done_seq() -> u32 {
    voice_shared().enroll_done_seq()
}

impl qobject::VoiceController {
    pub fn set_visualization_active(self: Pin<&mut Self>, active: bool) {
        let command = if active {
            BackendCommand::StartVoiceCapture
        } else {
            BackendCommand::StopVoiceCapture
        };
        send_backend(command);
    }

    /// Drain the shared spectrum queue (keeping only the newest frame) and bump
    /// `spectrum_version` so QML re-reads `spectrum_value_at`. Also refreshes the
    /// VAD and speaker-verification properties. Driven by a QML timer.
    pub fn process_spectrum(mut self: Pin<&mut Self>) {
        let capturing = self.rust().shared.capturing.load(Ordering::Relaxed);
        if capturing != self.rust().is_capturing {
            self.as_mut().set_is_capturing(capturing);
        }

        let (seq, last_seq, error_opt, prev_error) = {
            let rust = self.rust();
            (
                rust.shared.capture_error_seq(),
                rust.last_capture_error_seq,
                rust.shared.read_capture_error(),
                rust.capture_error.clone(),
            )
        };
        if seq != last_seq {
            if let Some(error) = error_opt {
                let error_q = QString::from(error.as_str());
                self.as_mut().rust_mut().last_capture_error_seq = seq;
                self.as_mut().rust_mut().capture_error_buf = error;
                if error_q != prev_error {
                    self.as_mut().set_capture_error(error_q);
                }
            }
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

        let generation = self.rust().shared.spectrum_generation();
        if generation != self.rust().last_spectrum_generation {
            self.as_mut().rust_mut().last_spectrum_generation = generation;
            self.as_mut().rust_mut().bar_ballistics.clear();
            self.as_mut().rust_mut().bar_levels = [0.0; SPECTRUM_BAR_COUNT];
            self.as_mut().rust_mut().latest.fill(0.0);
            let next = self.rust().spectrum_version.wrapping_add(1);
            self.as_mut().set_spectrum_version(next);
        }

        let mut newest: Option<Vec<f32>> = None;
        let queue = match self.rust().spectrum_source_code {
            SPECTRUM_SOURCE_FILTERED => &self.rust().shared.spectrum_filtered,
            SPECTRUM_SOURCE_MIXED => &self.rust().shared.spectrum_mixed,
            _ => &self.rust().shared.spectrum,
        };
        while let Some(frame) = queue.pop() {
            newest = Some(frame);
        }
        if let Some(frame) = newest {
            let (mic_gain, _) = self.rust().shared.spectrum_volume_gains();
            let source = self.rust().spectrum_source_code;
            let targets = if source == SPECTRUM_SOURCE_MIXED {
                map_to_bar_targets(&frame)
            } else {
                apply_fader_db(&frame, mic_gain, &mut self.as_mut().rust_mut().scaled_buf);
                map_to_bar_targets(&self.rust().scaled_buf)
            };
            let levels = self
                .as_mut()
                .rust_mut()
                .bar_ballistics
                .update(&targets);
            self.as_mut().rust_mut().bar_levels = levels;
            self.as_mut().rust_mut().latest = frame;
            let next = self.rust().spectrum_version.wrapping_add(1);
            self.as_mut().set_spectrum_version(next);
        } else {
            let silent = [0.0f32; SPECTRUM_BAR_COUNT];
            let decayed = self
                .as_mut()
                .rust_mut()
                .bar_ballistics
                .update(&silent);
            if decayed != self.rust().bar_levels {
                self.as_mut().rust_mut().bar_levels = decayed;
                let next = self.rust().spectrum_version.wrapping_add(1);
                self.as_mut().set_spectrum_version(next);
            }
        }
    }

    pub fn spectrum_bin_count(&self) -> i32 {
        SPECTRUM_BINS as i32
    }

    pub fn bar_level_at(&self, index: i32) -> f64 {
        self.rust()
            .bar_levels
            .get(index as usize)
            .copied()
            .unwrap_or(0.0) as f64
    }

    pub fn lit_segment_count_at(&self, level: f64) -> i32 {
        lit_segment_count_for_level(level as f32) as i32
    }

    pub fn spectrum_segment_db_at(&self, index: i32) -> f64 {
        SEGMENT_DB
            .get(index as usize)
            .copied()
            .unwrap_or(DB_MIN) as f64
    }

    pub fn spectrum_segment_y_frac_at(&self, index: i32) -> f64 {
        SEGMENT_Y_FRAC
            .get(index as usize)
            .copied()
            .unwrap_or(0.0) as f64
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

    pub fn set_vad_threshold(mut self: Pin<&mut Self>, open: f32) {
        let open = open.clamp(0.05, 0.95);
        let close = vad_close_for_open(open);
        self.rust().shared.set_vad_thresholds(open, close);
        self.as_mut().set_vad_open_threshold(open);
        persist_vad_threshold(open, close);
    }

    pub fn set_suppression(mut self: Pin<&mut Self>, enabled: bool) {
        self.as_mut().set_suppression_enabled(enabled);
        send_backend(BackendCommand::SetVoiceSuppression { enabled });
    }

    pub fn persist_vad_enabled(mut self: Pin<&mut Self>, enabled: bool) {
        self.rust().shared.set_vad_enabled(enabled);
        self.as_mut().set_vad_enabled(enabled);
        try_send_backend(BackendCommand::SetVoiceVad { enabled });
    }

    pub fn toggle_mic_mute(mut self: Pin<&mut Self>) {
        let muted = !self.rust().mic_muted;
        self.as_mut().persist_mic_muted(muted);
    }

    pub fn persist_mic_muted(mut self: Pin<&mut Self>, muted: bool) {
        self.as_mut().set_mic_muted(muted);
        let config = crate::config::load_config().unwrap_or_default();
        try_send_backend(BackendCommand::SetMicVolume {
            percent: config.audio.mic_volume,
            muted,
        });
    }

    pub fn persist_spectrum_source(mut self: Pin<&mut Self>, source: QString) {
        let source_str = source.to_string();
        let code = spectrum_source_from_str(&source_str);
        self.rust().shared.set_spectrum_source(code);
        self.as_mut().rust_mut().spectrum_source_code = code;
        self.as_mut().set_spectrum_source(source);
        self.as_mut().rust_mut().latest = vec![0.0; SPECTRUM_BINS];
        self.as_mut().rust_mut().bar_levels = [0.0; SPECTRUM_BAR_COUNT];
        self.as_mut().rust_mut().bar_ballistics.clear();
        self.as_mut().rust_mut().last_spectrum_generation = self.rust().shared.spectrum_generation();
        let next = self.rust().spectrum_version.wrapping_add(1);
        self.as_mut().set_spectrum_version(next);
        try_send_backend(BackendCommand::SetSpectrumSource { source: source_str });
    }

    pub fn start_enrollment(mut self: Pin<&mut Self>) {
        // Enrollment needs a live capture; ensure it is running.
        send_backend(BackendCommand::StartVoiceCapture);
        self.rust().shared.request_enroll_start();
        self.as_mut().set_enroll_active(true);
        self.as_mut().set_enroll_progress(0.0);
        self.as_mut().rust_mut().last_progress_percent = 0;
    }

    pub fn cancel_enrollment(mut self: Pin<&mut Self>) {
        self.rust().shared.request_enroll_cancel();
        self.as_mut().set_enroll_active(false);
        self.as_mut().set_enroll_progress(0.0);
        self.as_mut().rust_mut().last_progress_percent = 0;
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
    let mut config = crate::config::load_config().unwrap_or_default();
    config.voice.verification_enabled = enabled;
    config.voice.match_threshold = threshold;
    if let Err(err) = crate::config::save_config(&config) {
        tracing::warn!("failed to persist voice verification settings: {err:#}");
    }
    send_backend(BackendCommand::SetVoiceVerification { enabled, threshold });
}

fn persist_vad_threshold(open: f32, close: f32) {
    let mut config = crate::config::load_config().unwrap_or_default();
    config.voice.vad_open_threshold = open;
    config.voice.vad_close_threshold = close;
    if let Err(err) = crate::config::save_config(&config) {
        tracing::warn!("failed to persist VAD threshold: {err:#}");
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
