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
    VoiceShared, CAPTURE_RATE, ENROLL_CMD_CANCEL, ENROLL_CMD_CLEAR, ENROLL_CMD_START,
    ENROLL_SAMPLES, FFT_HOP, FFT_SIZE, SPECTRUM_BINS, SPECTRUM_SOURCE_FILTERED,
    SPECTRUM_SOURCE_MIXED, SPECTRUM_SOURCE_RAW, TARGET_RATE,
};

/// Voiced 16 kHz samples accumulated before running one verification embedding (~0.75 s).
const VERIFY_WINDOW: usize = TARGET_RATE as usize * 3 / 4;

/// Output gate attack (~3 ms at 48 kHz).
const GATE_ATTACK_MS: f32 = 3.0;

fn gate_ramp_steps(release_ms: u32) -> (f32, f32) {
    let attack_samples = CAPTURE_RATE as f32 * GATE_ATTACK_MS / 1000.0;
    let release_samples = CAPTURE_RATE as f32 * release_ms as f32 / 1000.0;
    (
        1.0 / attack_samples.max(1.0),
        1.0 / release_samples.max(1.0),
    )
}

fn hangover_samples(hangover_ms: u32) -> usize {
    (CAPTURE_RATE as u64 * hangover_ms as u64 / 1000) as usize
}

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
    let mut gate_gain: f32 = 0.0;
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
    let mut out_scratch: Vec<f32> = Vec::with_capacity(FFT_HOP * 2);
    let mut filtered_analyzer = SpectrumAnalyzer::new();
    let mut filtered_window: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);
    let mut vad = match Vad::new(vad_open, vad_close) {
        Ok(vad) => Some(vad),
        Err(err) => {
            warn!("voice VAD disabled: {err:#}");
            None
        }
    };
    let vad_available = vad.is_some();

    let mut window: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);
    let mut resampled = Vec::with_capacity(FFT_SIZE);

    let mut enrolling = false;
    let mut enroll_buf: Vec<f32> = Vec::new();
    let mut verify_buf: Vec<f32> = Vec::with_capacity(VERIFY_WINDOW + FFT_SIZE);
    let mut hangover_remaining: usize = 0;
    let mut raw_spectrum_buf = vec![0.0; SPECTRUM_BINS];
    let mut filtered_spectrum_buf = vec![0.0; SPECTRUM_BINS];
    let mut pop_buf = [0.0f32; 2048];

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
        loop {
            let (filled, _) = consumer.pop_partial_slice(&mut pop_buf);
            if filled.is_empty() {
                break;
            }
            window.extend_from_slice(filled);
            got_any = true;
            if window.len() >= FFT_SIZE * 2 {
                break;
            }
        }

        if !got_any {
            std::thread::sleep(Duration::from_millis(2));
            continue;
        }

        let (attack_step, release_step) = gate_ramp_steps(shared.gate_release_ms());

        while window.len() >= FFT_SIZE {
            let spectrum_visible = shared.spectrum_panel_visible();
            let source = shared.spectrum_source();
            let need_raw_spectrum = spectrum_visible && source == SPECTRUM_SOURCE_RAW;
            let need_filtered_spectrum = spectrum_visible
                && matches!(source, SPECTRUM_SOURCE_FILTERED | SPECTRUM_SOURCE_MIXED);

            if need_raw_spectrum {
                let magnitudes = analyzer.analyze(&window[..FFT_SIZE]);
                raw_spectrum_buf.copy_from_slice(magnitudes);
                push_spectrum_frame(&shared.spectrum, &raw_spectrum_buf);
            }

            let vad_on = shared.vad_enabled();
            let verifying_path = shared.verification_enabled() && shared.is_enrolled();
            let need_resampled = enrolling || vad_on || verifying_path;

            let effective_voiced = if need_resampled {
                resampled.clear();
                if let Err(err) = resampler.process(&window[..FFT_HOP], &mut resampled) {
                    debug!("voice resample error: {err:#}");
                    window.drain(..FFT_HOP);
                    continue;
                }

                let vad_active = if vad_on {
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

                let voiced = vad_active || !vad_available || !vad_on;

                let effective_voiced = if !vad_on || !vad_available {
                    true
                } else if vad_active {
                    hangover_remaining = hangover_samples(shared.gate_hangover_ms());
                    true
                } else if hangover_remaining > 0 {
                    hangover_remaining = hangover_remaining.saturating_sub(FFT_HOP);
                    true
                } else {
                    false
                };

                if !effective_voiced
                    && shared.verification_warmup_enabled()
                    && !shared.speaker_state().1
                {
                    shared.set_verify_warmup(true);
                }

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
                } else if verifying_path {
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

                effective_voiced
            } else {
                shared.set_vad(0.0, false);
                if !verify_buf.is_empty() {
                    verify_buf.clear();
                }
                true
            };

            let verifying = verifying_path;
            let matched = shared.speaker_state().1;
            let passing = if verifying {
                effective_voiced && (matched || shared.verify_warmup())
            } else {
                effective_voiced
            };
            shared.set_passing(passing);

            let gate_open = if verifying {
                if !shared.verification_warmup_enabled() {
                    effective_voiced && matched
                } else {
                    effective_voiced && (matched || shared.verify_warmup())
                }
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
                    gate_gain = (gate_gain + attack_step).min(target);
                } else if gate_gain > target {
                    gate_gain = (gate_gain - release_step).max(target);
                }
                let gated = sample * gate_gain;
                if let Some(out) = output.as_mut() {
                    let _ = out.push(gated);
                }
                filtered_window.push(gated);
            }

            while filtered_window.len() >= FFT_SIZE {
                if need_filtered_spectrum {
                    let magnitudes = filtered_analyzer.analyze(&filtered_window[..FFT_SIZE]);
                    filtered_spectrum_buf.copy_from_slice(magnitudes);
                    shared.set_latest_filtered(&filtered_spectrum_buf);
                    push_spectrum_frame(&shared.spectrum_filtered, &filtered_spectrum_buf);
                    if !shared.sfx_mix_enabled()
                        && shared.spectrum_source() != SPECTRUM_SOURCE_MIXED
                    {
                        push_spectrum_frame(&shared.spectrum_mixed, &filtered_spectrum_buf);
                    }
                }
                filtered_window.drain(..FFT_HOP);
            }

            window.drain(..FFT_HOP);
        }
    }
    debug!("voice pipeline thread stopped");
}

fn push_spectrum_frame(queue: &crossbeam_queue::ArrayQueue<Vec<f32>>, frame: &[f32]) {
    let mut buf = queue.pop().unwrap_or_else(|| vec![0.0; SPECTRUM_BINS]);
    if buf.len() != SPECTRUM_BINS {
        buf.resize(SPECTRUM_BINS, 0.0);
    }
    buf.copy_from_slice(frame);
    let _ = queue.force_push(buf);
}
