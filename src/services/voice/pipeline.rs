//! Dedicated audio thread: drains captured samples from the SPSC ring, runs the
//! spectrum FFT over overlapping windows, runs Silero VAD on the resampled
//! stream, and (Milestone 3) accumulates speech for the speaker-verification
//! gate and enrollment. Heavy ECAPA inference is offloaded to the embed worker;
//! this thread only buffers and dispatches. Per the spec the audio path runs on
//! its own `std::thread`, not Tokio.

use anyhow::Result;
use rtrb::{Consumer, Producer};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tracing::{debug, warn};

use super::denoise::Denoiser;
use super::embed_worker::EmbedJob;
use super::resample::Resampler;
use super::spectrum::SpectrumAnalyzer;
use super::vad::Vad;
use super::{
    VoiceShared, ENROLL_CMD_CANCEL, ENROLL_CMD_CLEAR, ENROLL_CMD_START, ENROLL_SAMPLES, FFT_HOP,
    FFT_SIZE, TARGET_RATE,
};

/// Voiced 16 kHz samples accumulated before running one verification embedding (~1.5 s).
const VERIFY_WINDOW: usize = TARGET_RATE as usize * 3 / 2;

/// Per-sample gain step for the output gate (~10 ms attack/release at 48 kHz),
/// keeping open/close transitions click-free.
const GATE_STEP: f32 = 1.0 / 480.0;

pub struct VoicePipeline {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl VoicePipeline {
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        consumer: Consumer<f32>,
        shared: Arc<VoiceShared>,
        vad_open: f32,
        vad_close: f32,
        job_tx: Sender<EmbedJob>,
        busy: Arc<AtomicBool>,
        output: Option<Producer<f32>>,
        suppression: bool,
    ) -> Result<Self> {
        let resampler = Resampler::new()?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let handle = std::thread::Builder::new()
            .name("voice-pipeline".into())
            .spawn(move || {
                run(
                    consumer,
                    shared,
                    resampler,
                    vad_open,
                    vad_close,
                    job_tx,
                    busy,
                    output,
                    suppression,
                    thread_stop,
                )
            })?;
        Ok(Self {
            stop,
            handle: Some(handle),
        })
    }
}

impl Drop for VoicePipeline {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    mut consumer: Consumer<f32>,
    shared: Arc<VoiceShared>,
    mut resampler: Resampler,
    vad_open: f32,
    vad_close: f32,
    job_tx: Sender<EmbedJob>,
    busy: Arc<AtomicBool>,
    mut output: Option<Producer<f32>>,
    suppression: bool,
    stop: Arc<AtomicBool>,
) {
    let mut analyzer = SpectrumAnalyzer::new();
    // Current output-gate gain, ramped toward 0/1 to avoid clicks.
    let mut gate_gain: f32 = 0.0;
    // DeepFilterNet3 denoiser for the routed output path. A load failure
    // degrades to passthrough (gating still works, just without suppression).
    let mut denoiser = if suppression {
        match Denoiser::new() {
            Ok(d) => Some(d),
            Err(err) => {
                warn!("voice denoise disabled: {err:#}");
                None
            }
        }
    } else {
        None
    };
    // Reused scratch for denoised (or passthrough) output samples.
    let mut out_scratch: Vec<f32> = Vec::with_capacity(FFT_HOP * 2);
    let mut filtered_analyzer = SpectrumAnalyzer::new();
    let mut filtered_window: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);
    // A VAD failure (e.g. ONNX runtime unavailable) degrades to spectrum-only;
    // verification then runs ungated.
    let mut vad = match Vad::new(vad_open, vad_close) {
        Ok(vad) => Some(vad),
        Err(err) => {
            warn!("voice VAD disabled: {err:#}");
            None
        }
    };
    let vad_available = vad.is_some();

    let mut window: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);
    // 16 kHz stream feeding the VAD; contiguous across iterations.
    let mut resampled = Vec::with_capacity(FFT_SIZE);

    let mut enrolling = false;
    let mut enroll_buf: Vec<f32> = Vec::new();
    let mut verify_buf: Vec<f32> = Vec::with_capacity(VERIFY_WINDOW + FFT_SIZE);

    while !stop.load(Ordering::Relaxed) {
        match shared.take_enroll_command() {
            ENROLL_CMD_START => {
                enrolling = true;
                enroll_buf.clear();
                enroll_buf.reserve(ENROLL_SAMPLES);
                verify_buf.clear();
                shared.set_enroll_active(true);
                shared.set_enroll_progress(0.0);
            }
            ENROLL_CMD_CANCEL => {
                enrolling = false;
                enroll_buf.clear();
                shared.set_enroll_active(false);
                shared.set_enroll_progress(0.0);
            }
            ENROLL_CMD_CLEAR => {
                verify_buf.clear();
                let _ = job_tx.send(EmbedJob::Clear);
            }
            _ => {}
        }

        let mut got_any = false;
        while let Ok(sample) = consumer.pop() {
            window.push(sample);
            got_any = true;
            if window.len() >= FFT_SIZE * 2 {
                break;
            }
        }

        if !got_any {
            std::thread::sleep(Duration::from_millis(2));
            continue;
        }

        while window.len() >= FFT_SIZE {
            let magnitudes = analyzer.analyze(&window[..FFT_SIZE]).to_vec();
            // Newest frame wins; the UI only renders the latest.
            shared.spectrum.force_push(magnitudes);

            resampled.clear();
            if let Err(err) = resampler.process(&window[..FFT_HOP], &mut resampled) {
                debug!("voice resample error: {err:#}");
                window.drain(..FFT_HOP);
                continue;
            }

            let vad_on = shared.vad_enabled();
            let active = if vad_on {
                match vad.as_mut() {
                    Some(vad) => {
                        let (open, close) = shared.vad_thresholds();
                        vad.set_thresholds(open, close);
                        let (prob, active) = vad.process(&resampled);
                        shared.set_vad(prob, active);
                        active
                    }
                    None => false,
                }
            } else {
                shared.set_vad(0.0, false);
                false
            };
            // When VAD is unavailable or disabled we can't gate on speech.
            let voiced = active || !vad_available || !vad_on;

            if enrolling {
                enroll_buf.extend_from_slice(&resampled);
                let progress = (enroll_buf.len() as f32 / ENROLL_SAMPLES as f32).min(1.0);
                shared.set_enroll_progress(progress);
                if enroll_buf.len() >= ENROLL_SAMPLES {
                    let buf = std::mem::take(&mut enroll_buf);
                    let _ = job_tx.send(EmbedJob::Enroll(buf));
                    enrolling = false;
                    shared.set_enroll_progress(1.0);
                }
            } else if shared.verification_enabled() && shared.is_enrolled() {
                if voiced {
                    verify_buf.extend_from_slice(&resampled);
                }
                if verify_buf.len() >= VERIFY_WINDOW && !busy.load(Ordering::Relaxed) {
                    let buf = std::mem::take(&mut verify_buf);
                    verify_buf = Vec::with_capacity(VERIFY_WINDOW + FFT_SIZE);
                    let _ = job_tx.send(EmbedJob::Verify(buf));
                }
            } else if !verify_buf.is_empty() {
                verify_buf.clear();
            }

            // Gate state for the UI: speech present and (verification off or the
            // enrolled speaker matched).
            let verifying = shared.verification_enabled() && shared.is_enrolled();
            let passing = if verifying {
                voiced && shared.speaker_state().1
            } else {
                voiced
            };
            shared.set_passing(passing);

            // Denoise then gate — same path for virtmic output and filtered spectrum.
            let gate_open = if verifying {
                voiced && shared.speaker_state().1
            } else {
                true
            };
            let target = if gate_open { 1.0 } else { 0.0 };

            out_scratch.clear();
            match denoiser.as_mut() {
                Some(d) => d.process(&window[..FFT_HOP], &mut out_scratch),
                None => out_scratch.extend_from_slice(&window[..FFT_HOP]),
            }

            for &sample in &out_scratch {
                if gate_gain < target {
                    gate_gain = (gate_gain + GATE_STEP).min(target);
                } else if gate_gain > target {
                    gate_gain = (gate_gain - GATE_STEP).max(target);
                }
                let gated = sample * gate_gain;
                if let Some(out) = output.as_mut() {
                    let _ = out.push(gated);
                }
                filtered_window.push(gated);
            }

            while filtered_window.len() >= FFT_SIZE {
                let magnitudes = filtered_analyzer
                    .analyze(&filtered_window[..FFT_SIZE])
                    .to_vec();
                shared.set_latest_filtered(&magnitudes);
                shared.spectrum_filtered.force_push(magnitudes.clone());
                if !shared.sfx_mix_enabled() {
                    shared.spectrum_mixed.force_push(magnitudes);
                }
                filtered_window.drain(..FFT_HOP);
            }

            window.drain(..FFT_HOP);
        }
    }
    debug!("voice pipeline thread stopped");
}
