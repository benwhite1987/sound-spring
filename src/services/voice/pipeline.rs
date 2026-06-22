//! Dedicated audio thread: drains captured samples from the SPSC ring, runs the
//! spectrum FFT over overlapping windows, runs Silero VAD on the resampled
//! stream, and (Milestone 3) accumulates speech for the speaker-verification
//! gate and enrollment. Heavy ECAPA inference is offloaded to the embed worker;
//! this thread only buffers and dispatches. Per the spec the audio path runs on
//! its own `std::thread`, not Tokio.

use anyhow::Result;
use rtrb::Consumer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tracing::{debug, warn};

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
    stop: Arc<AtomicBool>,
) {
    let mut analyzer = SpectrumAnalyzer::new();
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

            let active = match vad.as_mut() {
                Some(vad) => {
                    let (prob, active) = vad.process(&resampled);
                    shared.set_vad(prob, active);
                    active
                }
                None => false,
            };
            // When VAD is unavailable we can't gate, so treat audio as voiced.
            let voiced = active || !vad_available;

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

            window.drain(..FFT_HOP);
        }
    }
    debug!("voice pipeline thread stopped");
}
