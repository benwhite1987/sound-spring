//! Phase 2 voice-enhancement pipeline.
//!
//! Milestone 1 added the visualization slice (PipeWire capture -> FFT spectrum),
//! Milestone 2 added Silero VAD. Milestone 3 added ECAPA-TDNN speaker embedding,
//! enrollment, and a cosine verification gate. Milestone 4 routes the gated mic
//! into the virtmic: when gating is active the session feeds processed audio to
//! the output sink and the backend removes the raw mic loopback. DeepFilterNet
//! denoise arrives later in the same chain.

pub mod capture;
pub mod embed_worker;
pub mod embedding;
pub mod output;
pub mod pipeline;
pub mod resample;
pub mod spectrum;
pub mod vad;
pub mod verifier;
pub mod voiceprint;

use anyhow::Result;
use crossbeam_queue::ArrayQueue;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::{Arc, OnceLock};

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

/// Shared, lock-light state bridging the audio thread, the embedding worker, and
/// the Qt-side `VoiceController`.
pub struct VoiceShared {
    /// Latest spectrum frames (length [`SPECTRUM_BINS`]), newest wins.
    pub spectrum: ArrayQueue<Vec<f32>>,
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
}

impl VoiceShared {
    fn new() -> Self {
        Self {
            spectrum: ArrayQueue::new(SPECTRUM_QUEUE_CAP),
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
        self.enroll_command
            .swap(ENROLL_CMD_NONE, Ordering::Relaxed)
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
    /// When set, the session feeds gated audio into `output_sink` (replacing the
    /// raw mic loopback) instead of running visualization-only.
    pub gating: bool,
    /// Target sink for gated output (typically the virtmic). Empty = none.
    pub output_sink: String,
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
    _embed: embed_worker::EmbedWorker,
}

impl VoiceSession {
    /// Start capturing and run the spectrum + VAD + verification pipeline.
    pub fn start(params: VoiceParams) -> Result<Self> {
        let shared = voice_shared();
        shared.set_verification_enabled(params.verification_enabled);
        shared.set_match_threshold(params.match_threshold);

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

        let (producer, consumer) = rtrb::RingBuffer::<f32>::new(RING_CAPACITY);
        let pipeline = pipeline::VoicePipeline::spawn(
            consumer,
            shared.clone(),
            params.vad_open,
            params.vad_close,
            job_tx,
            busy,
            out_producer,
        )?;
        let capture = capture::Capture::start(&params.mic_source, producer)?;
        shared.capturing.store(true, Ordering::Relaxed);
        Ok(Self {
            _capture: capture,
            _pipeline: pipeline,
            _output: output,
            _embed: embed,
        })
    }
}

impl Drop for VoiceSession {
    fn drop(&mut self) {
        let shared = voice_shared();
        shared.capturing.store(false, Ordering::Relaxed);
        shared.set_vad(0.0, false);
        shared.set_passing(false);
        shared.set_enroll_active(false);
        shared.set_enroll_progress(0.0);
    }
}
