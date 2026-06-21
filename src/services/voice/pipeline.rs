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
use tracing::debug;

use super::resample::Resampler;
use super::spectrum::SpectrumAnalyzer;
use super::{VoiceShared, FFT_HOP, FFT_SIZE};

pub struct VoicePipeline {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl VoicePipeline {
    pub fn spawn(consumer: Consumer<f32>, shared: Arc<VoiceShared>) -> Result<Self> {
        let resampler = Resampler::new()?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let handle = std::thread::Builder::new()
            .name("voice-pipeline".into())
            .spawn(move || run(consumer, shared, resampler, thread_stop))?;
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
    stop: Arc<AtomicBool>,
) {
    let mut analyzer = SpectrumAnalyzer::new();
    let mut window: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);
    // Reused scratch for the (currently consumer-less) resampled stream so the
    // resampler stays exercised ahead of later milestones.
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
            }

            window.drain(..FFT_HOP);
        }
    }
    debug!("voice pipeline thread stopped");
}
