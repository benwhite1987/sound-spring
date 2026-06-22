//! FFT-only thread for the mixed spectrum: filtered × gated mic + soundboard SFX.

use anyhow::Result;
use rtrb::Consumer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tracing::debug;

use super::spectrum::SpectrumAnalyzer;
use super::{VoiceShared, FFT_HOP, FFT_SIZE};

pub struct MixSpectrum {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl MixSpectrum {
    pub fn spawn(
        mic: Consumer<f32>,
        sfx: Consumer<f32>,
        shared: Arc<VoiceShared>,
    ) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let handle = std::thread::Builder::new()
            .name("voice-mix-spectrum".into())
            .spawn(move || run(mic, sfx, shared, thread_stop))?;
        Ok(Self {
            stop,
            handle: Some(handle),
        })
    }
}

impl Drop for MixSpectrum {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run(
    mut mic: Consumer<f32>,
    mut sfx: Consumer<f32>,
    shared: Arc<VoiceShared>,
    stop: Arc<AtomicBool>,
) {
    let mut analyzer = SpectrumAnalyzer::new();
    let mut window: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);

    while !stop.load(Ordering::Relaxed) {
        let mut got_any = false;

        while let Ok(mic_sample) = mic.pop() {
            let sfx_sample = sfx.pop().unwrap_or(0.0);
            window.push(mic_sample + sfx_sample);
            got_any = true;
            if window.len() >= FFT_SIZE * 2 {
                break;
            }
        }
        while let Ok(sfx_sample) = sfx.pop() {
            window.push(sfx_sample);
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
            shared.spectrum_mixed.force_push(magnitudes);
            window.drain(..FFT_HOP);
        }
    }
    debug!("mix spectrum thread stopped");
}
