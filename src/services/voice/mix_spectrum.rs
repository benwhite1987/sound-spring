//! Mixed spectrum: filtered magnitude frame + SFX-only FFT (energy sum).

use anyhow::Result;
use rtrb::Consumer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tracing::debug;

use super::spectrum::SpectrumAnalyzer;
use super::{VoiceShared, FFT_HOP, FFT_SIZE, SPECTRUM_BINS};

pub struct MixSpectrum {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl MixSpectrum {
    pub fn spawn(sfx: Option<Consumer<f32>>, shared: Arc<VoiceShared>) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let handle = std::thread::Builder::new()
            .name("voice-mix-spectrum".into())
            .spawn(move || run(sfx, shared, thread_stop))?;
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

/// Energy-sum of two normalized magnitude spectra (length [`SPECTRUM_BINS`]).
pub fn combine_magnitude_spectra(filtered: &[f32], sfx: &[f32]) -> Vec<f32> {
    debug_assert_eq!(filtered.len(), sfx.len());
    filtered
        .iter()
        .zip(sfx.iter())
        .map(|(f, s)| (f * f + s * s).sqrt())
        .collect()
}

fn run(mut sfx: Option<Consumer<f32>>, shared: Arc<VoiceShared>, stop: Arc<AtomicBool>) {
    let mut analyzer = SpectrumAnalyzer::new();
    let mut window: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);

    while !stop.load(Ordering::Relaxed) {
        let sfx_enabled = shared.sfx_mix_enabled();
        let mut got_any = false;

        if sfx_enabled {
            if let Some(sfx) = sfx.as_mut() {
                while let Ok(sample) = sfx.pop() {
                    window.push(sample);
                    got_any = true;
                    if window.len() >= FFT_SIZE * 2 {
                        break;
                    }
                }
            }
        } else if let Some(sfx) = sfx.as_mut() {
            while sfx.pop().is_ok() {}
        }

        if !got_any {
            std::thread::sleep(Duration::from_millis(2));
            continue;
        }

        while window.len() >= FFT_SIZE {
            let sfx_magnitudes = analyzer.analyze(&window[..FFT_SIZE]).to_vec();
            let filtered = shared.latest_filtered_snapshot();
            let mixed = combine_magnitude_spectra(&filtered, &sfx_magnitudes);
            shared.spectrum_mixed.force_push(mixed);
            window.drain(..FFT_HOP);
        }
    }
    debug!("mix spectrum thread stopped");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::voice::spectrum::SpectrumAnalyzer;
    use crate::services::voice::{CAPTURE_RATE, FFT_SIZE};

    fn sine(freq: f32, len: usize) -> Vec<f32> {
        (0..len)
            .map(|n| {
                (2.0 * std::f32::consts::PI * freq * n as f32 / CAPTURE_RATE as f32).sin()
            })
            .collect()
    }

    fn peak_energy(magnitudes: &[f32]) -> f32 {
        magnitudes.iter().cloned().fold(0.0_f32, f32::max)
    }

    #[test]
    fn combine_silent_filtered_with_sfx_shows_only_sfx() {
        let filtered = vec![0.0; SPECTRUM_BINS];
        let mut analyzer = SpectrumAnalyzer::new();
        let frame = sine(440.0, FFT_SIZE);
        let sfx = analyzer.analyze(&frame).to_vec();

        let mixed = combine_magnitude_spectra(&filtered, &sfx);
        assert!(peak_energy(&mixed) > 0.01);
        assert!(peak_energy(&filtered) < 1e-6);
    }

    #[test]
    fn combine_with_zero_sfx_matches_filtered() {
        let mut analyzer = SpectrumAnalyzer::new();
        let frame = sine(880.0, FFT_SIZE);
        let filtered = analyzer.analyze(&frame).to_vec();
        let silent_sfx = vec![0.0; SPECTRUM_BINS];

        let mixed = combine_magnitude_spectra(&filtered, &silent_sfx);
        assert_eq!(mixed.len(), filtered.len());
        for (m, f) in mixed.iter().zip(filtered.iter()) {
            assert!((m - f).abs() < 1e-6, "bin mismatch: mixed={m} filtered={f}");
        }
    }

    #[test]
    fn combine_adds_energy_from_both_legs() {
        let mut analyzer = SpectrumAnalyzer::new();
        let filtered = analyzer.analyze(&sine(400.0, FFT_SIZE)).to_vec();
        let sfx = analyzer.analyze(&sine(2000.0, FFT_SIZE)).to_vec();

        let mixed = combine_magnitude_spectra(&filtered, &sfx);
        assert!(peak_energy(&mixed) >= peak_energy(&filtered));
        assert!(peak_energy(&mixed) >= peak_energy(&sfx));
    }
}
