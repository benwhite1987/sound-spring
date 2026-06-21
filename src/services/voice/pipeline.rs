//! Dedicated audio thread: drains captured samples from the SPSC ring, runs the
//! spectrum FFT over overlapping windows, and publishes magnitude frames to the
//! shared queue the Qt-side `VoiceController` drains. Per the spec the audio
//! path runs on its own `std::thread`, not Tokio.

use anyhow::Result;
use rtrb::Consumer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tracing::{debug, warn};

use super::resample::Resampler;
use super::spectrum::SpectrumAnalyzer;
use super::vad::Vad;
use super::{VoiceShared, FFT_HOP, FFT_SIZE};

pub struct VoicePipeline {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl VoicePipeline {
    pub fn spawn(
        consumer: Consumer<f32>,
        shared: Arc<VoiceShared>,
        vad_open: f32,
        vad_close: f32,
    ) -> Result<Self> {
        let resampler = Resampler::new()?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let handle = std::thread::Builder::new()
            .name("voice-pipeline".into())
            .spawn(move || run(consumer, shared, resampler, vad_open, vad_close, thread_stop))?;
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

fn run(
    mut consumer: Consumer<f32>,
    shared: Arc<VoiceShared>,
    mut resampler: Resampler,
    vad_open: f32,
    vad_close: f32,
    stop: Arc<AtomicBool>,
) {
    let mut analyzer = SpectrumAnalyzer::new();
    // A VAD failure (e.g. ONNX runtime unavailable) degrades to spectrum-only.
    let mut vad = match Vad::new(vad_open, vad_close) {
        Ok(vad) => Some(vad),
        Err(err) => {
            warn!("voice VAD disabled: {err:#}");
            None
        }
    };
    let mut window: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);
    // 16 kHz stream feeding the VAD; contiguous across iterations.
    let mut resampled = Vec::with_capacity(FFT_SIZE);

    while !stop.load(Ordering::Relaxed) {
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
            } else if let Some(vad) = vad.as_mut() {
                let (prob, active) = vad.process(&resampled);
                shared.set_vad(prob, active);
            }

            window.drain(..FFT_HOP);
        }
    }
    debug!("voice pipeline thread stopped");
}
