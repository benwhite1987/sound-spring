//! Phase 2 voice-enhancement pipeline.
//!
//! Milestone 1 implements only the visualization slice: a PipeWire capture of
//! the configured mic feeds a dedicated audio thread that runs an FFT and emits
//! log-frequency spectrum frames to the UI. VAD, speaker verification, and
//! DeepFilterNet denoise arrive in later milestones; the Phase 1 mic-to-virtmic
//! loopback is intentionally left untouched here.

pub mod capture;
pub mod pipeline;
pub mod resample;
pub mod spectrum;
pub mod vad;

use anyhow::Result;
use crossbeam_queue::ArrayQueue;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

/// Capture sample rate (PipeWire mono f32).
pub const CAPTURE_RATE: u32 = 48_000;
/// Downsample target for the (future) VAD/embedding stages.
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

/// Shared, lock-light state bridging the audio thread and the Qt-side
/// `VoiceController`.
pub struct VoiceShared {
    /// Latest spectrum frames (length [`SPECTRUM_BINS`]), newest wins.
    pub spectrum: ArrayQueue<Vec<f32>>,
    /// Whether a capture session is currently running.
    pub capturing: AtomicBool,
    /// Latest VAD speech probability (0..1) stored as `f32` bits.
    vad_probability: AtomicU32,
    /// Whether VAD considers speech active (after hysteresis).
    speech_active: AtomicBool,
}

impl VoiceShared {
    fn new() -> Self {
        Self {
            spectrum: ArrayQueue::new(SPECTRUM_QUEUE_CAP),
            capturing: AtomicBool::new(false),
            vad_probability: AtomicU32::new(0),
            speech_active: AtomicBool::new(false),
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
}

static VOICE_SHARED: OnceLock<Arc<VoiceShared>> = OnceLock::new();

/// Process-wide handle to the shared voice state.
pub fn voice_shared() -> Arc<VoiceShared> {
    VOICE_SHARED
        .get_or_init(|| Arc::new(VoiceShared::new()))
        .clone()
}

/// A running capture + processing session. Dropping it tears everything down:
/// the `pw-cat` child is killed, the reader task aborted, and the audio thread
/// joined.
pub struct VoiceSession {
    // Field order matters for drop order: stop capture before the pipeline so
    // the producer side closes first.
    _capture: capture::Capture,
    _pipeline: pipeline::VoicePipeline,
}

impl VoiceSession {
    /// Start capturing `mic_source` (empty = PipeWire default source) and run
    /// the spectrum + VAD pipeline. `vad_open`/`vad_close` are the hysteresis
    /// thresholds from `[voice]` config.
    pub fn start(mic_source: &str, vad_open: f32, vad_close: f32) -> Result<Self> {
        let shared = voice_shared();
        let (producer, consumer) = rtrb::RingBuffer::<f32>::new(RING_CAPACITY);
        let pipeline =
            pipeline::VoicePipeline::spawn(consumer, shared.clone(), vad_open, vad_close)?;
        let capture = capture::Capture::start(mic_source, producer)?;
        shared.capturing.store(true, Ordering::Relaxed);
        Ok(Self {
            _capture: capture,
            _pipeline: pipeline,
        })
    }
}

impl Drop for VoiceSession {
    fn drop(&mut self) {
        let shared = voice_shared();
        shared.capturing.store(false, Ordering::Relaxed);
        shared.set_vad(0.0, false);
    }
}
