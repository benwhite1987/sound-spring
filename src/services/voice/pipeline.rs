//! Dedicated audio thread: drains captured samples from the SPSC ring, runs the
//! spectrum FFT over overlapping windows, runs Silero VAD on the resampled
//! stream, and (Milestone 3) accumulates speech for the speaker-verification
//! gate and enrollment. Heavy ECAPA inference is offloaded to the embed worker;
//! this thread only buffers and dispatches. Per the spec the audio path runs on
//! its own `std::thread`, not Tokio.

use anyhow::Result;
use rtrb::chunks::ChunkError;
use rtrb::{Consumer, Producer};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

use super::denoise_worker::DenoiseWorker;
use super::embed_worker::EmbedJob;
use super::resample::Resampler;
use super::spectrum::SpectrumAnalyzer;
use super::vad::Vad;
use super::{
    VoiceShared, CAPTURE_RATE, ENROLL_CMD_CANCEL, ENROLL_CMD_CLEAR, ENROLL_CMD_START,
    ENROLL_SAMPLES, FFT_HOP, FFT_SIZE, RING_CAPACITY, SPECTRUM_BINS, SPECTRUM_SOURCE_FILTERED,
    SPECTRUM_SOURCE_MIXED, SPECTRUM_SOURCE_RAW, TARGET_RATE,
};

/// Voiced 16 kHz samples accumulated before running one verification embedding (~0.75 s).
const VERIFY_WINDOW: usize = TARGET_RATE as usize * 3 / 4;

/// Output gate attack (~15 ms at 48 kHz) — soft enough to avoid clicks on reopen.
const GATE_ATTACK_MS: f32 = 15.0;

/// Stable delay cushion (~100 ms at 48 kHz) before emitting enhanced audio.
const DENOISE_TARGET_DELAY: usize = 4_800;

/// Soft cap: target plus one pipeline hop (burst absorb only).
const DENOISE_MAX_DELAY: usize = DENOISE_TARGET_DELAY + FFT_HOP;

/// Fade length when padding an underrun shortfall (never full-hop DC hold).
const UNDERRUN_FADE_SAMPLES: usize = 128;

/// Bounded wait when the delay cushion runs short after priming.
const DENOISE_UNDERRUN_WAIT: Duration = Duration::from_millis(10);

/// Brief retry when denoise_in cannot accept a full hop.
const DENOISE_INPUT_RETRY: Duration = Duration::from_millis(8);

/// Extra headroom on denoise SPSC rings vs capture/output.
const DENOISE_RING_CAPACITY: usize = RING_CAPACITY * 2;

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
    _denoise: Option<DenoiseWorker>,
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

        let (denoise_worker, denoise_in, denoise_out, _denoise_reset) = if suppression {
            let (in_prod, in_cons) = rtrb::RingBuffer::<f32>::new(DENOISE_RING_CAPACITY);
            let (out_prod, out_cons) = rtrb::RingBuffer::<f32>::new(DENOISE_RING_CAPACITY);
            match DenoiseWorker::spawn(in_cons, out_prod) {
                Ok((worker, reset)) => (Some(worker), Some(in_prod), Some(out_cons), Some(reset)),
                Err(err) => {
                    warn!("voice denoise worker unavailable: {err:#}");
                    (None, None, None, None)
                }
            }
        } else {
            (None, None, None, None)
        };

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
                    denoise_in,
                    denoise_out,
                    thread_stop,
                )
            })?;
        Ok(Self {
            stop,
            handle: Some(handle),
            _denoise: denoise_worker,
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
    mut denoise_in: Option<Producer<f32>>,
    mut denoise_out: Option<Consumer<f32>>,
    stop: Arc<AtomicBool>,
) {
    let mut analyzer = SpectrumAnalyzer::new();
    let mut gate_gain: f32 = 0.0;
    let mut out_scratch: Vec<f32> = Vec::with_capacity(FFT_HOP * 2);
    let mut out_pending: Vec<f32> = Vec::with_capacity(FFT_HOP * 2);
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
    let mut match_hold_remaining: usize = 0;
    let mut raw_spectrum_buf = vec![0.0; SPECTRUM_BINS];
    let mut filtered_spectrum_buf = vec![0.0; SPECTRUM_BINS];
    let mut pop_buf = [0.0f32; 2048];
    let mut denoise_pop_buf = [0.0f32; 2048];
    let mut denoise_pending: Vec<f32> = Vec::with_capacity(FFT_HOP * 2);
    let mut denoise_delay: VecDeque<f32> = VecDeque::with_capacity(DENOISE_MAX_DELAY);
    let mut denoise_primed = false;
    let mut denoise_last_sample: f32 = 0.0;
    let mut denoise_underruns: u64 = 0;
    let mut denoise_trims: u64 = 0;
    let mut denoise_backpressure: u64 = 0;

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
                push_spectrum_frame(&shared, &shared.spectrum, &raw_spectrum_buf);
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
                        if verify_buf.len() > VERIFY_WINDOW {
                            verify_buf.drain(..verify_buf.len() - VERIFY_WINDOW);
                        }
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
            let matched_live = shared.speaker_state().1;
            // Hold last match briefly so ECAPA window gaps / consonants do not hard-close.
            let matched = if matched_live {
                match_hold_remaining = hangover_samples(shared.gate_hangover_ms());
                true
            } else if match_hold_remaining > 0 {
                match_hold_remaining = match_hold_remaining.saturating_sub(FFT_HOP);
                true
            } else {
                false
            };
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
            let routing_output = output.is_some();
            // DFN only when actually routing to virtmic — filtered viz must not force it.
            // Always feed DFN while routing+suppression (never skip on a closed gate).
            let need_denoised_audio = routing_output && denoise_in.is_some();

            out_scratch.clear();
            if need_denoised_audio {
                if let (Some(din), Some(dout)) = (denoise_in.as_mut(), denoise_out.as_mut()) {
                    denoise_pending.clear();
                    denoise_pending.extend_from_slice(&window[..FFT_HOP]);
                    let mut pushed = flush_denoise_input(&mut denoise_pending, din);
                    if !pushed {
                        let deadline = Instant::now() + DENOISE_INPUT_RETRY;
                        while Instant::now() < deadline && !pushed {
                            drain_denoise_out(dout, &mut denoise_delay, &mut denoise_pop_buf);
                            pushed = flush_denoise_input(&mut denoise_pending, din);
                            if !pushed {
                                std::thread::sleep(Duration::from_micros(200));
                            }
                        }
                    }
                    if !pushed {
                        denoise_backpressure = denoise_backpressure.saturating_add(1);
                        if denoise_backpressure == 1 || denoise_backpressure.is_multiple_of(50) {
                            warn!(
                                count = denoise_backpressure,
                                "denoise_in backpressure; dropping newest hop"
                            );
                        }
                        denoise_pending.clear();
                    }

                    drain_denoise_out(dout, &mut denoise_delay, &mut denoise_pop_buf);

                    if !denoise_primed {
                        if denoise_delay.len() >= DENOISE_TARGET_DELAY {
                            denoise_primed = true;
                        } else {
                            // Warmup: silence only — never splice raw over delayed DFN.
                            out_scratch.resize(FFT_HOP, 0.0);
                        }
                    }

                    if denoise_primed {
                        let want = DENOISE_TARGET_DELAY + FFT_HOP;
                        if denoise_delay.len() < want {
                            let deadline = Instant::now() + DENOISE_UNDERRUN_WAIT;
                            while Instant::now() < deadline && denoise_delay.len() < want {
                                drain_denoise_out(dout, &mut denoise_delay, &mut denoise_pop_buf);
                                if denoise_delay.len() < want {
                                    std::thread::sleep(Duration::from_micros(200));
                                }
                            }
                        }

                        if denoise_delay.len() < FFT_HOP {
                            denoise_underruns = denoise_underruns.saturating_add(1);
                            if denoise_underruns == 1 || denoise_underruns.is_multiple_of(25) {
                                warn!(
                                    count = denoise_underruns,
                                    buffered = denoise_delay.len(),
                                    "denoise delay underrun; fading shortfall to silence"
                                );
                            }
                        }
                        // Always advance wall-clock: pop available, fade-pad shortfall.
                        // Never invent silence while leaving delayed samples behind.
                        emit_denoise_hop(
                            &mut denoise_delay,
                            &mut out_scratch,
                            &mut denoise_last_sample,
                        );
                    }

                    // Soft-cap burst absorb only (not the underrun growth path).
                    let mut trimmed = 0usize;
                    while denoise_delay.len() > DENOISE_MAX_DELAY {
                        denoise_delay.pop_front();
                        trimmed += 1;
                    }
                    if trimmed > 0 {
                        denoise_trims = denoise_trims.saturating_add(1);
                        if denoise_trims == 1 || denoise_trims.is_multiple_of(25) {
                            warn!(
                                count = denoise_trims,
                                samples = trimmed,
                                "denoise delay trim; dropped oldest burst samples"
                            );
                        }
                    }
                } else {
                    out_scratch.extend_from_slice(&window[..FFT_HOP]);
                }
            } else {
                out_scratch.extend_from_slice(&window[..FFT_HOP]);
            }

            for &sample in &out_scratch {
                if gate_gain < target {
                    gate_gain = (gate_gain + attack_step).min(target);
                } else if gate_gain > target {
                    gate_gain = (gate_gain - release_step).max(target);
                }
                let gated = sample * gate_gain;
                if output.is_some() {
                    out_pending.push(gated);
                }
                if need_filtered_spectrum {
                    filtered_window.push(gated);
                }
            }
            if let Some(out) = output.as_mut() {
                flush_output_samples(&mut out_pending, out);
            }

            while filtered_window.len() >= FFT_SIZE {
                if need_filtered_spectrum {
                    let magnitudes = filtered_analyzer.analyze(&filtered_window[..FFT_SIZE]);
                    filtered_spectrum_buf.copy_from_slice(magnitudes);
                    shared.set_latest_filtered(&filtered_spectrum_buf);
                    push_spectrum_frame(&shared, &shared.spectrum_filtered, &filtered_spectrum_buf);
                }
                filtered_window.drain(..FFT_HOP);
            }

            window.drain(..FFT_HOP);
        }
    }
    debug!("voice pipeline thread stopped");
}

fn push_spectrum_frame(
    shared: &VoiceShared,
    queue: &crossbeam_queue::ArrayQueue<Vec<f32>>,
    frame: &[f32],
) {
    let mut buf = queue
        .pop()
        .unwrap_or_else(|| shared.take_spectrum_frame_buf());
    if buf.len() != SPECTRUM_BINS {
        buf.resize(SPECTRUM_BINS, 0.0);
    }
    buf.copy_from_slice(frame);
    let _ = queue.force_push(buf);
}

fn drain_denoise_out(
    dout: &mut Consumer<f32>,
    delay: &mut VecDeque<f32>,
    pop_buf: &mut [f32; 2048],
) {
    loop {
        let (filled, _) = dout.pop_partial_slice(pop_buf);
        if filled.is_empty() {
            break;
        }
        delay.extend(filled.iter().copied());
    }
}

/// Emit one pipeline hop from the delay line. Pops all available up to FFT_HOP,
/// then fade-pads any shortfall — never a full-hop DC hold, never silence while
/// leaving delayed samples unconsumed.
fn emit_denoise_hop(delay: &mut VecDeque<f32>, out: &mut Vec<f32>, last: &mut f32) {
    let available = delay.len().min(FFT_HOP);
    for _ in 0..available {
        let sample = delay.pop_front().unwrap_or(*last);
        *last = sample;
        out.push(sample);
    }
    let shortfall = FFT_HOP - available;
    if shortfall == 0 {
        return;
    }
    pad_fade_to_silence(out, *last, shortfall);
    *last = 0.0;
}

fn pad_fade_to_silence(out: &mut Vec<f32>, from: f32, shortfall: usize) {
    if shortfall == 0 {
        return;
    }
    let fade_n = shortfall.min(UNDERRUN_FADE_SAMPLES).max(1);
    for i in 0..fade_n {
        let gain = 1.0 - (i as f32 + 1.0) / fade_n as f32;
        out.push(from * gain);
    }
    for _ in fade_n..shortfall {
        out.push(0.0);
    }
}

/// Push a full denoise hop; returns false if the ring cannot accept it entirely.
fn flush_denoise_input(pending: &mut Vec<f32>, producer: &mut Producer<f32>) -> bool {
    if pending.is_empty() {
        return true;
    }
    if producer.slots() < pending.len() {
        return false;
    }
    match producer.push_entire_slice(pending) {
        Ok(()) => {
            pending.clear();
            true
        }
        Err(_) => false,
    }
}

fn flush_output_samples(pending: &mut Vec<f32>, producer: &mut Producer<f32>) {
    while !pending.is_empty() {
        match producer.push_entire_slice(pending) {
            Ok(()) => {
                pending.clear();
                break;
            }
            Err(ChunkError::TooFewSlots(n)) if n > 0 => {
                let _ = producer.push_entire_slice(&pending[..n]);
                pending.drain(..n);
            }
            Err(_) => {
                // Drop oldest half; keep newest so playback stays closer to realtime.
                let drop_n = (pending.len() / 2).max(1);
                pending.drain(..drop_n);
                break;
            }
        }
    }
    if pending.len() > RING_CAPACITY {
        pending.drain(..pending.len() - RING_CAPACITY);
    }
}
