//! Mixed spectrum: filtered magnitude frame + SFX playback FFT (energy sum).

use anyhow::Result;
use rtrb::Consumer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tracing::debug;

use super::spectrum::SpectrumAnalyzer;
use super::{VoiceShared, FFT_HOP, FFT_SIZE, SPECTRUM_BINS, SPECTRUM_SOURCE_MIXED};

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
#[cfg_attr(not(test), allow(dead_code))]
pub fn combine_magnitude_spectra(filtered: &[f32], sfx: &[f32]) -> Vec<f32> {
    let mut out = vec![0.0; filtered.len().min(sfx.len())];
    combine_magnitude_spectra_into(&mut out, filtered, sfx);
    out
}

/// In-place energy-sum of two normalized magnitude spectra.
pub fn combine_magnitude_spectra_into(out: &mut [f32], filtered: &[f32], sfx: &[f32]) {
    debug_assert_eq!(filtered.len(), sfx.len());
    debug_assert_eq!(out.len(), filtered.len());
    for ((slot, f), s) in out.iter_mut().zip(filtered.iter()).zip(sfx.iter()) {
        *slot = (f * f + s * s).sqrt();
    }
}

fn run(mut sfx: Option<Consumer<f32>>, shared: Arc<VoiceShared>, stop: Arc<AtomicBool>) {
    let mut analyzer = SpectrumAnalyzer::new();
    let mut window: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);
    let mut mixed_buf = vec![0.0; SPECTRUM_BINS];
    let mut last_sfx_magnitudes = vec![0.0; SPECTRUM_BINS];
    let mut last_filtered_seq = 0u32;

    while !stop.load(Ordering::Relaxed) {
        let want_mix = shared.spectrum_source() == SPECTRUM_SOURCE_MIXED;
        let sfx_playing = shared.sfx_mix_enabled();
        if !sfx_playing {
            last_sfx_magnitudes.fill(0.0);
        }
        let mut got_any = false;

        if let Some(sfx) = sfx.as_mut() {
            while let Ok(sample) = sfx.pop() {
                window.push(sample);
                got_any = true;
                if window.len() >= FFT_SIZE * 2 {
                    break;
                }
            }
        }

        let should_process = window.len() >= FFT_SIZE && (got_any || sfx_playing || want_mix);
        if should_process {
            while window.len() >= FFT_SIZE {
                let sfx_magnitudes = analyzer.analyze(&window[..FFT_SIZE]);
                last_sfx_magnitudes.copy_from_slice(sfx_magnitudes);
                let filtered = shared.latest_filtered_snapshot();
                combine_magnitude_spectra_into(&mut mixed_buf, &filtered, sfx_magnitudes);
                push_spectrum_frame(&shared.spectrum_mixed, &mixed_buf);
                window.drain(..FFT_HOP);
            }
        } else if want_mix {
            let seq = shared.filtered_seq();
            if seq != last_filtered_seq {
                last_filtered_seq = seq;
                let filtered = shared.latest_filtered_snapshot();
                let last_sfx_peak = last_sfx_magnitudes.iter().cloned().fold(0.0_f32, f32::max);
                if sfx_playing && last_sfx_peak > 1e-4 {
                    combine_magnitude_spectra_into(&mut mixed_buf, &filtered, &last_sfx_magnitudes);
                } else {
                    mixed_buf.copy_from_slice(&filtered);
                }
                push_spectrum_frame(&shared.spectrum_mixed, &mixed_buf);
            }
            if !got_any {
                std::thread::sleep(Duration::from_millis(2));
            }
        } else if !got_any {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    debug!("mix spectrum thread stopped");
}

fn push_spectrum_frame(queue: &crossbeam_queue::ArrayQueue<Vec<f32>>, frame: &[f32]) {
    let mut buf = queue.pop().unwrap_or_else(|| vec![0.0; SPECTRUM_BINS]);
    if buf.len() != SPECTRUM_BINS {
        buf.resize(SPECTRUM_BINS, 0.0);
    }
    buf.copy_from_slice(frame);
    let _ = queue.force_push(buf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::voice::spectrum::SpectrumAnalyzer;
    use crate::services::voice::SPECTRUM_BINS;
    use crate::services::voice::{CAPTURE_RATE, FFT_SIZE};

    fn sine(freq: f32, len: usize) -> Vec<f32> {
        (0..len)
            .map(|n| (2.0 * std::f32::consts::PI * freq * n as f32 / CAPTURE_RATE as f32).sin())
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
