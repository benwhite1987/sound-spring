//! Phase 2 voice-enhancement pipeline.
//!
//! Milestone 1 added the visualization slice (PipeWire capture -> FFT spectrum),
//! Milestone 2 added Silero VAD. Milestone 3 added ECAPA-TDNN speaker embedding,
//! enrollment, and a cosine verification gate. Milestone 4 routes the gated mic
//! into the virtmic: when gating is active the session feeds processed audio to
//! the output sink and the backend removes the raw mic loopback. DeepFilterNet
//! denoise arrives later in the same chain.

pub mod capture;
pub mod denoise;
pub mod embed_worker;
pub mod embedding;
pub mod mix_spectrum;
pub mod output;
pub mod pipeline;
pub mod resample;
pub mod spectrum;
pub mod spectrum_bars;
pub mod vad;
pub mod verifier;
pub mod voiceprint;

use anyhow::Result;
use crossbeam_queue::ArrayQueue;
use rtrb::Producer;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// Capture sample rate (PipeWire mono f32).
pub const CAPTURE_RATE: u32 = 48_000;
/// Downsample target for the VAD/embedding stages.
pub const TARGET_RATE: u32 = 16_000;
/// Capture channel count.
pub const CAPTURE_CHANNELS: u32 = 1;
/// FFT window size for the spectrum analyzer.
pub const FFT_SIZE: usize = 2048;
/// Hop between successive FFT windows (50% overlap).
pub const FFT_HOP: usize = FFT_SIZE / 2;
/// Number of log-spaced magnitude bins handed to the UI.
pub const SPECTRUM_BINS: usize = 128;
/// SPSC ring capacity in samples (~340 ms at 48 kHz) between the capture task
/// and the audio thread.
pub const RING_CAPACITY: usize = 16_384;
/// Bounded spectrum-frame queue depth. Dropping spectrum frames is fine;
/// dropping audio frames is not.
const SPECTRUM_QUEUE_CAP: usize = 4;

/// Enrollment recording length in 16 kHz samples (~30 s per the spec).
pub const ENROLL_SAMPLES: usize = TARGET_RATE as usize * 30;

/// Enroll command codes carried by [`VoiceShared::enroll_command`].
pub const ENROLL_CMD_NONE: u8 = 0;
pub const ENROLL_CMD_START: u8 = 1;
pub const ENROLL_CMD_CANCEL: u8 = 2;
pub const ENROLL_CMD_CLEAR: u8 = 3;

/// Spectrum display source codes for [`VoiceShared::spectrum_source`].
pub const SPECTRUM_SOURCE_RAW: u8 = 0;
pub const SPECTRUM_SOURCE_FILTERED: u8 = 1;
pub const SPECTRUM_SOURCE_MIXED: u8 = 2;

/// Shared, lock-light state bridging the audio thread, the embedding worker, and
/// the Qt-side `VoiceController`.
pub struct VoiceShared {
    /// Latest raw capture spectrum frames (length [`SPECTRUM_BINS`]), newest wins.
    pub spectrum: ArrayQueue<Vec<f32>>,
    /// Post-denoise × gate spectrum frames for the filtered view.
    pub spectrum_filtered: ArrayQueue<Vec<f32>>,
    /// Virtmic monitor mix spectrum frames (mic + soundboard).
    pub spectrum_mixed: ArrayQueue<Vec<f32>>,
    /// Whether a capture session is currently running.
    pub capturing: AtomicBool,
    /// Latest VAD speech probability (0..1) stored as `f32` bits.
    vad_probability: AtomicU32,
    /// Whether VAD considers speech active (after hysteresis).
    speech_active: AtomicBool,

    // --- speaker verification: control (UI -> workers) ---
    verification_enabled: AtomicBool,
    match_threshold: AtomicU32,
    enroll_command: AtomicU8,

    // --- speaker verification: outputs (workers -> UI) ---
    /// Latest cosine similarity to the enrolled voiceprint (`f32` bits).
    match_score: AtomicU32,
    /// Whether the latest embedding matched the enrolled speaker (post-hysteresis).
    speaker_match: AtomicBool,
    /// Gate state: speech present AND (verification off OR speaker matched).
    passing: AtomicBool,
    /// Whether a voiceprint is enrolled.
    enrolled: AtomicBool,
    /// Whether an enrollment recording/processing is in progress.
    enroll_active: AtomicBool,
    /// Enrollment progress 0..1 (`f32` bits).
    enroll_progress: AtomicU32,
    /// Bumped each time an enrollment completes successfully.
    enroll_done_seq: AtomicU32,
    /// Human-readable capture failure message for the Voice panel (empty when ok).
    capture_error: Mutex<String>,
    /// Bumped when [`Self::set_capture_status`] updates [`Self::capture_error`].
    capture_error_seq: AtomicU32,
    /// VAD open/close thresholds (f32 bits); hot-updated from the Voice panel.
    vad_open: AtomicU32,
    vad_close: AtomicU32,
    /// When false, VAD inference is skipped and all audio is treated as voiced.
    vad_enabled: AtomicBool,
    /// Active spectrum display source (0=raw, 1=filtered, 2=mixed).
    spectrum_source: AtomicU8,
    /// Voice panel is showing (spectrum visualization is active).
    spectrum_panel_visible: AtomicBool,
    /// Latest filtered magnitude frame for magnitude-domain mixed spectrum.
    latest_filtered: Mutex<Vec<f32>>,
    /// Bumped when [`Self::set_latest_filtered`] publishes a new frame.
    filtered_seq: AtomicU32,
    /// When false, mixed spectrum mirrors filtered (no SFX monitor contribution).
    sfx_mix_enabled: AtomicBool,
    /// Playback decode feed for mixed-spectrum SFX leg (replaces monitor capture).
    sfx_spectrum_producer: Mutex<Option<Producer<f32>>>,
    /// Config: hold gate open after VAD drops (milliseconds).
    gate_hangover_ms: AtomicU32,
    /// Config: output gate release ramp (milliseconds).
    gate_release_ms: AtomicU32,
    /// Config: allow passthrough until first failed speaker check.
    verification_warmup_enabled: AtomicBool,
    /// Runtime: warm-up gate active until first confident non-match.
    verify_warmup: AtomicBool,
}

impl VoiceShared {
    fn new() -> Self {
        Self {
            spectrum: ArrayQueue::new(SPECTRUM_QUEUE_CAP),
            spectrum_filtered: ArrayQueue::new(SPECTRUM_QUEUE_CAP),
            spectrum_mixed: ArrayQueue::new(SPECTRUM_QUEUE_CAP),
            capturing: AtomicBool::new(false),
            vad_probability: AtomicU32::new(0),
            speech_active: AtomicBool::new(false),
            verification_enabled: AtomicBool::new(false),
            match_threshold: AtomicU32::new(0.6_f32.to_bits()),
            enroll_command: AtomicU8::new(ENROLL_CMD_NONE),
            match_score: AtomicU32::new(0),
            speaker_match: AtomicBool::new(false),
            passing: AtomicBool::new(false),
            enrolled: AtomicBool::new(false),
            enroll_active: AtomicBool::new(false),
            enroll_progress: AtomicU32::new(0),
            enroll_done_seq: AtomicU32::new(0),
            capture_error: Mutex::new(String::new()),
            capture_error_seq: AtomicU32::new(0),
            vad_open: AtomicU32::new(0.45_f32.to_bits()),
            vad_close: AtomicU32::new(0.20_f32.to_bits()),
            vad_enabled: AtomicBool::new(true),
            spectrum_source: AtomicU8::new(SPECTRUM_SOURCE_RAW),
            spectrum_panel_visible: AtomicBool::new(false),
            latest_filtered: Mutex::new(vec![0.0; SPECTRUM_BINS]),
            filtered_seq: AtomicU32::new(0),
            sfx_mix_enabled: AtomicBool::new(false),
            sfx_spectrum_producer: Mutex::new(None),
            gate_hangover_ms: AtomicU32::new(200),
            gate_release_ms: AtomicU32::new(100),
            verification_warmup_enabled: AtomicBool::new(true),
            verify_warmup: AtomicBool::new(false),
        }
    }

    /// Publish the latest VAD result from the audio thread.
    pub fn set_vad(&self, probability: f32, active: bool) {
        self.vad_probability
            .store(probability.to_bits(), Ordering::Relaxed);
        self.speech_active.store(active, Ordering::Relaxed);
    }

    /// Read the latest VAD result (probability, speech_active).
    pub fn vad_state(&self) -> (f32, bool) {
        (
            f32::from_bits(self.vad_probability.load(Ordering::Relaxed)),
            self.speech_active.load(Ordering::Relaxed),
        )
    }

    pub fn set_verification_enabled(&self, on: bool) {
        self.verification_enabled.store(on, Ordering::Relaxed);
    }

    pub fn verification_enabled(&self) -> bool {
        self.verification_enabled.load(Ordering::Relaxed)
    }

    pub fn set_match_threshold(&self, threshold: f32) {
        self.match_threshold
            .store(threshold.to_bits(), Ordering::Relaxed);
    }

    pub fn match_threshold(&self) -> f32 {
        f32::from_bits(self.match_threshold.load(Ordering::Relaxed))
    }

    /// Request the start of an enrollment recording.
    pub fn request_enroll_start(&self) {
        self.enroll_command
            .store(ENROLL_CMD_START, Ordering::Relaxed);
    }

    /// Request cancellation of an in-progress enrollment.
    pub fn request_enroll_cancel(&self) {
        self.enroll_command
            .store(ENROLL_CMD_CANCEL, Ordering::Relaxed);
    }

    /// Ask a running session to drop its in-memory voiceprint (the file is
    /// removed by the UI). No-op if no session is running.
    pub fn request_enroll_clear(&self) {
        self.enroll_command
            .store(ENROLL_CMD_CLEAR, Ordering::Relaxed);
    }

    /// Consume the pending enroll command (returns it and resets to NONE).
    pub fn take_enroll_command(&self) -> u8 {
        self.enroll_command.swap(ENROLL_CMD_NONE, Ordering::Relaxed)
    }

    pub fn set_speaker(&self, score: f32, matched: bool) {
        self.match_score.store(score.to_bits(), Ordering::Relaxed);
        self.speaker_match.store(matched, Ordering::Relaxed);
    }

    /// (cosine_score, matched).
    pub fn speaker_state(&self) -> (f32, bool) {
        (
            f32::from_bits(self.match_score.load(Ordering::Relaxed)),
            self.speaker_match.load(Ordering::Relaxed),
        )
    }

    pub fn set_passing(&self, passing: bool) {
        self.passing.store(passing, Ordering::Relaxed);
    }

    pub fn is_passing(&self) -> bool {
        self.passing.load(Ordering::Relaxed)
    }

    pub fn set_enrolled(&self, enrolled: bool) {
        self.enrolled.store(enrolled, Ordering::Relaxed);
    }

    pub fn is_enrolled(&self) -> bool {
        self.enrolled.load(Ordering::Relaxed)
    }

    pub fn set_enroll_active(&self, active: bool) {
        self.enroll_active.store(active, Ordering::Relaxed);
    }

    pub fn enroll_active(&self) -> bool {
        self.enroll_active.load(Ordering::Relaxed)
    }

    pub fn set_enroll_progress(&self, progress: f32) {
        self.enroll_progress
            .store(progress.to_bits(), Ordering::Relaxed);
    }

    pub fn enroll_progress(&self) -> f32 {
        f32::from_bits(self.enroll_progress.load(Ordering::Relaxed))
    }

    pub fn bump_enroll_done(&self) {
        self.enroll_done_seq.fetch_add(1, Ordering::Relaxed);
    }

    pub fn enroll_done_seq(&self) -> u32 {
        self.enroll_done_seq.load(Ordering::Relaxed)
    }

    /// Update capture status surfaced in the Voice panel.
    pub fn set_capture_status(&self, active: bool, error: &str) {
        self.capturing.store(active, Ordering::Relaxed);
        if let Ok(mut msg) = self.capture_error.lock() {
            if *msg != error {
                *msg = error.to_string();
                self.capture_error_seq.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Latest capture-error generation; cheap to poll from the UI thread.
    pub fn capture_error_seq(&self) -> u32 {
        self.capture_error_seq.load(Ordering::Relaxed)
    }

    /// Read the capture error without blocking the audio thread.
    pub fn read_capture_error(&self) -> Option<String> {
        self.capture_error
            .try_lock()
            .ok()
            .map(|msg| msg.clone())
    }

    pub fn set_vad_thresholds(&self, open: f32, close: f32) {
        self.vad_open.store(open.to_bits(), Ordering::Relaxed);
        self.vad_close.store(close.to_bits(), Ordering::Relaxed);
    }

    pub fn vad_thresholds(&self) -> (f32, f32) {
        (
            f32::from_bits(self.vad_open.load(Ordering::Relaxed)),
            f32::from_bits(self.vad_close.load(Ordering::Relaxed)),
        )
    }

    pub fn set_vad_enabled(&self, enabled: bool) {
        self.vad_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn vad_enabled(&self) -> bool {
        self.vad_enabled.load(Ordering::Relaxed)
    }

    pub fn set_spectrum_source(&self, source: u8) {
        let source = source.min(2);
        let prev = self.spectrum_source.swap(source, Ordering::Relaxed);
        if source == SPECTRUM_SOURCE_MIXED && prev != SPECTRUM_SOURCE_MIXED {
            self.filtered_seq.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn spectrum_source(&self) -> u8 {
        self.spectrum_source.load(Ordering::Relaxed)
    }

    pub fn set_spectrum_panel_visible(&self, visible: bool) {
        self.spectrum_panel_visible
            .store(visible, Ordering::Relaxed);
    }

    pub fn spectrum_panel_visible(&self) -> bool {
        self.spectrum_panel_visible.load(Ordering::Relaxed)
    }

    pub fn set_latest_filtered(&self, magnitudes: &[f32]) {
        if let Ok(mut latest) = self.latest_filtered.lock() {
            if latest.len() != magnitudes.len() {
                *latest = magnitudes.to_vec();
            } else {
                latest.copy_from_slice(magnitudes);
            }
        }
        self.filtered_seq.fetch_add(1, Ordering::Relaxed);
    }

    pub fn filtered_seq(&self) -> u32 {
        self.filtered_seq.load(Ordering::Relaxed)
    }

    pub fn latest_filtered_copy_into(&self, out: &mut [f32]) -> bool {
        if let Ok(latest) = self.latest_filtered.lock() {
            if latest.len() == out.len() {
                out.copy_from_slice(&latest);
                return true;
            }
        }
        false
    }

    pub fn set_sfx_mix_enabled(&self, enabled: bool) {
        self.sfx_mix_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn sfx_mix_enabled(&self) -> bool {
        self.sfx_mix_enabled.load(Ordering::Relaxed)
    }

    pub fn attach_sfx_spectrum_producer(&self, producer: Producer<f32>) {
        if let Ok(mut guard) = self.sfx_spectrum_producer.lock() {
            *guard = Some(producer);
        }
    }

    pub fn detach_sfx_spectrum_producer(&self) {
        if let Ok(mut guard) = self.sfx_spectrum_producer.lock() {
            *guard = None;
        }
    }

    pub fn flush_sfx_spectrum_pending(&self, pending: &mut Vec<f32>) {
        if pending.is_empty() {
            return;
        }
        let Ok(mut guard) = self.sfx_spectrum_producer.lock() else {
            pending.clear();
            return;
        };
        let Some(producer) = guard.as_mut() else {
            pending.clear();
            return;
        };
        while !pending.is_empty() {
            match producer.push_entire_slice(pending) {
                Ok(()) => {
                    pending.clear();
                    break;
                }
                Err(rtrb::chunks::ChunkError::TooFewSlots(n)) if n > 0 => {
                    let _ = producer.push_entire_slice(&pending[..n]);
                    pending.drain(..n);
                }
                Err(_) => {
                    pending.clear();
                    break;
                }
            }
        }
        if pending.len() > RING_CAPACITY {
            pending.drain(..pending.len() - RING_CAPACITY);
        }
    }

    pub fn set_gate_hangover_ms(&self, ms: u32) {
        self.gate_hangover_ms.store(ms, Ordering::Relaxed);
    }

    pub fn gate_hangover_ms(&self) -> u32 {
        self.gate_hangover_ms.load(Ordering::Relaxed)
    }

    pub fn set_gate_release_ms(&self, ms: u32) {
        self.gate_release_ms
            .store(ms.clamp(20, 200), Ordering::Relaxed);
    }

    pub fn gate_release_ms(&self) -> u32 {
        self.gate_release_ms.load(Ordering::Relaxed)
    }

    pub fn set_verification_warmup_enabled(&self, enabled: bool) {
        self.verification_warmup_enabled
            .store(enabled, Ordering::Relaxed);
    }

    pub fn verification_warmup_enabled(&self) -> bool {
        self.verification_warmup_enabled.load(Ordering::Relaxed)
    }

    pub fn set_verify_warmup(&self, active: bool) {
        self.verify_warmup.store(active, Ordering::Relaxed);
    }

    pub fn verify_warmup(&self) -> bool {
        self.verify_warmup.load(Ordering::Relaxed)
    }
}

/// Map a config/UI spectrum source string to [`SPECTRUM_SOURCE_*`].
pub fn spectrum_source_from_str(s: &str) -> u8 {
    match s {
        "filtered" => SPECTRUM_SOURCE_FILTERED,
        "mixed" => SPECTRUM_SOURCE_MIXED,
        _ => SPECTRUM_SOURCE_RAW,
    }
}

/// Derive a VAD close threshold from the open threshold (minimum 0.02 hysteresis).
pub fn vad_close_for_open(open: f32) -> f32 {
    let close = (open - 0.12).max(open * 0.5);
    let hi = open - 0.02;
    let lo = 0.08_f32.min(hi);
    close.clamp(lo, hi)
}

static VOICE_SHARED: OnceLock<Arc<VoiceShared>> = OnceLock::new();

/// Process-wide handle to the shared voice state.
pub fn voice_shared() -> Arc<VoiceShared> {
    VOICE_SHARED
        .get_or_init(|| Arc::new(VoiceShared::new()))
        .clone()
}

/// Parameters for starting a capture + processing session.
pub struct VoiceParams {
    /// Mic source name (empty = PipeWire default source).
    pub mic_source: String,
    pub vad_open: f32,
    pub vad_close: f32,
    pub verification_enabled: bool,
    pub match_threshold: f32,
    /// Absolute path of the enrolled voiceprint file.
    pub voiceprint_path: PathBuf,
    /// When set, the session feeds processed audio into `output_sink` (replacing
    /// the raw mic loopback) instead of running visualization-only.
    pub gating: bool,
    /// Target sink for processed output (typically the virtmic). Empty = none.
    pub output_sink: String,
    /// Apply DeepFilterNet3 noise suppression on the routed output path.
    pub suppression: bool,
    /// When false, VAD inference is skipped and all audio is treated as voiced.
    pub vad_enabled: bool,
    pub gate_hangover_ms: u32,
    pub gate_release_ms: u32,
    pub verification_warmup: bool,
}

/// A running capture + processing session. Dropping it tears everything down:
/// the `pw-cat` child is killed, the reader task aborted, the audio thread
/// joined, and the embedding worker joined.
pub struct VoiceSession {
    // Field order matters for drop order: stop capture first (closes the ring
    // producer), then the pipeline (drops the embed-job sender), then the embed
    // worker (its receiver disconnects and the thread exits).
    _capture: capture::Capture,
    _pipeline: pipeline::VoicePipeline,
    // Dropped after the pipeline so its producer is gone before the writer dies.
    _output: Option<output::Output>,
    _mix_spectrum: Option<mix_spectrum::MixSpectrum>,
    _embed: embed_worker::EmbedWorker,
}

impl VoiceSession {
    /// Start capturing and run the spectrum + VAD + verification pipeline.
    pub fn start(params: VoiceParams) -> Result<Self> {
        let shared = voice_shared();
        shared.set_verification_enabled(params.verification_enabled);
        shared.set_match_threshold(params.match_threshold);
        shared.set_vad_thresholds(params.vad_open, params.vad_close);
        shared.set_vad_enabled(params.vad_enabled);
        shared.set_gate_hangover_ms(params.gate_hangover_ms);
        shared.set_gate_release_ms(params.gate_release_ms);
        shared.set_verification_warmup_enabled(params.verification_warmup);
        shared.set_verify_warmup(params.verification_warmup && params.verification_enabled);

        let busy = Arc::new(AtomicBool::new(false));
        let (job_tx, job_rx) = std::sync::mpsc::channel::<embed_worker::EmbedJob>();

        let embed = embed_worker::EmbedWorker::spawn(
            job_rx,
            shared.clone(),
            busy.clone(),
            params.voiceprint_path,
            params.match_threshold,
        );

        // When gating, the pipeline emits processed samples into a second ring
        // that a `pw-cat --playback` writer feeds into the output sink.
        let (out_producer, output) = if params.gating && !params.output_sink.is_empty() {
            let (out_producer, out_consumer) = rtrb::RingBuffer::<f32>::new(RING_CAPACITY);
            let output = output::Output::start(&params.output_sink, out_consumer)?;
            (Some(out_producer), Some(output))
        } else {
            (None, None)
        };

        // Denoise runs whenever suppression is enabled (including visualization-only).
        let suppression = params.suppression;
        let (producer, consumer) = rtrb::RingBuffer::<f32>::new(RING_CAPACITY);
        let pipeline = pipeline::VoicePipeline::spawn(
            consumer,
            shared.clone(),
            params.vad_open,
            params.vad_close,
            job_tx,
            busy,
            out_producer,
            suppression,
        )?;
        let capture = capture::Capture::start(&params.mic_source, producer, Some(shared.clone()))?;

        let mix_spectrum = {
            let (sfx_producer, sfx_consumer) = rtrb::RingBuffer::<f32>::new(RING_CAPACITY);
            shared.attach_sfx_spectrum_producer(sfx_producer);
            Some(mix_spectrum::MixSpectrum::spawn(
                Some(sfx_consumer),
                shared.clone(),
            )?)
        };

        shared.set_capture_status(true, "");
        Ok(Self {
            _capture: capture,
            _pipeline: pipeline,
            _output: output,
            _mix_spectrum: mix_spectrum,
            _embed: embed,
        })
    }
}

impl Drop for VoiceSession {
    fn drop(&mut self) {
        let shared = voice_shared();
        shared.detach_sfx_spectrum_producer();
        shared.set_capture_status(false, "");
        shared.set_vad(0.0, false);
        shared.set_passing(false);
        shared.set_enroll_active(false);
        shared.set_enroll_progress(0.0);
    }
}

#[cfg(test)]
mod vad_threshold_tests {
    use super::vad_close_for_open;

    #[test]
    fn close_stays_below_open_with_hysteresis() {
        for &open in &[0.45_f32, 0.30, 0.15, 0.10, 0.08, 0.06, 0.95] {
            let close = vad_close_for_open(open);
            assert!(close < open, "open={open} close={close}");
            assert!(open - close >= 0.02, "open={open} close={close}");
        }
    }

    #[test]
    fn low_open_values_do_not_panic() {
        let mut open = 0.05_f32;
        while open <= 0.15 {
            let _ = vad_close_for_open(open);
            open += 0.001;
        }
    }

    #[test]
    fn default_open_maps_to_sane_close() {
        let close = vad_close_for_open(0.45);
        assert!((close - 0.22).abs() < 0.02 || close >= 0.20);
    }
}
