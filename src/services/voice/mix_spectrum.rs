//! Mixed spectrum: filtered magnitude frame + SFX playback FFT (energy sum).

use anyhow::Result;
use rtrb::Consumer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tracing::debug;

use super::spectrum::SpectrumAnalyzer;
use super::spectrum_meter::{compose_mic_only_into, compose_mixed_into};
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
    compose_mixed_into(&mut out, filtered, sfx, 1.0, 1.0);
    out
}

/// In-place energy-sum of two normalized magnitude spectra.
pub fn combine_magnitude_spectra_into(out: &mut [f32], filtered: &[f32], sfx: &[f32]) {
    compose_mixed_into(out, filtered, sfx, 1.0, 1.0);
}

/// In-place energy-sum with per-leg post-fader dB gains.
pub fn combine_magnitude_spectra_with_gains_into(
    out: &mut [f32],
    filtered: &[f32],
    sfx: &[f32],
    mic_gain: f32,
    output_gain: f32,
) {
    compose_mixed_into(out, filtered, sfx, mic_gain, output_gain);
}

fn run(mut sfx: Option<Consumer<f32>>, shared: Arc<VoiceShared>, stop: Arc<AtomicBool>) {
    let mut analyzer = SpectrumAnalyzer::new();
    let mut window: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);
    let mut mixed_buf = vec![0.0; SPECTRUM_BINS];
    let mut filtered_buf = vec![0.0; SPECTRUM_BINS];
    let mut last_sfx_magnitudes = vec![0.0; SPECTRUM_BINS];
    let mut last_filtered_seq = 0u32;
    let mut last_output_gain = f32::NAN;
    let mut last_idle_push = Instant::now();

    while !stop.load(Ordering::Relaxed) {
        if shared.take_spectrum_reset_requested() {
            window.clear();
            last_sfx_magnitudes.fill(0.0);
            last_idle_push = Instant::now();
        }

        let want_mix = shared.spectrum_source() == SPECTRUM_SOURCE_MIXED;
        let sfx_playing = shared.sfx_mix_enabled();
        let output_gain = shared.spectrum_volume_gains().1;
        if output_gain != last_output_gain {
            window.clear();
            last_sfx_magnitudes.fill(0.0);
            last_output_gain = output_gain;
        }
        if !sfx_playing {
            if !window.is_empty() {
                window.clear();
            }
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
                let filtered_ok = shared.latest_filtered_copy_into(&mut filtered_buf);
                if !filtered_ok {
                    filtered_buf.fill(0.0);
                }
                let (mic_g, out_g) = shared.spectrum_volume_gains();
                compose_mixed_into(
                    &mut mixed_buf,
                    &filtered_buf,
                    sfx_magnitudes,
                    mic_g,
                    out_g,
                );
                push_spectrum_frame(&shared.spectrum_mixed, &mixed_buf);
                window.drain(..FFT_HOP);
            }
        } else if want_mix {
            let seq = shared.filtered_seq();
            let idle = !sfx_playing;
            let heartbeat_due =
                idle && last_idle_push.elapsed() >= Duration::from_millis(33);
            if seq != last_filtered_seq || heartbeat_due {
                last_filtered_seq = seq;
                if heartbeat_due {
                    last_idle_push = Instant::now();
                }
                let filtered_ok = shared.latest_filtered_copy_into(&mut filtered_buf);
                if !filtered_ok {
                    filtered_buf.fill(0.0);
                }
                let (mic_g, out_g) = shared.spectrum_volume_gains();
                if sfx_playing {
                    let last_sfx_peak =
                        last_sfx_magnitudes.iter().cloned().fold(0.0_f32, f32::max);
                    if last_sfx_peak > 1e-4 {
                        compose_mixed_into(
                            &mut mixed_buf,
                            &filtered_buf,
                            &last_sfx_magnitudes,
                            mic_g,
                            out_g,
                        );
                    } else {
                        compose_mic_only_into(&mut mixed_buf, &filtered_buf, mic_g);
                    }
                } else {
                    compose_mic_only_into(&mut mixed_buf, &filtered_buf, mic_g);
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
    use crate::services::voice::spectrum_meter::apply_fader_db;
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

    #[test]
    fn mic_gain_scales_filtered_leg_in_db() {
        let mut analyzer = SpectrumAnalyzer::new();
        let filtered = analyzer.analyze(&sine(400.0, FFT_SIZE)).to_vec();
        let silent_sfx = vec![0.0; SPECTRUM_BINS];
        let mut mixed = vec![0.0; SPECTRUM_BINS];
        combine_magnitude_spectra_with_gains_into(&mut mixed, &filtered, &silent_sfx, 0.5, 1.0);
        let mut half_only = vec![0.0; SPECTRUM_BINS];
        apply_fader_db(&filtered, 0.5, &mut half_only);
        let full = peak_energy(&filtered);
        let half = peak_energy(&mixed);
        assert!(half < full);
    }

    #[test]
    fn mixed_sfx_stop_mic_only_frame() {
        let mut analyzer = SpectrumAnalyzer::new();
        let filtered = analyzer.analyze(&sine(400.0, FFT_SIZE)).to_vec();
        let silent_sfx = vec![0.0; SPECTRUM_BINS];
        let mut mixed = vec![0.0; SPECTRUM_BINS];
        compose_mic_only_into(&mut mixed, &filtered, 1.0);
        assert!(peak_energy(&mixed) > 0.01);
        let mut cleared = vec![0.0; SPECTRUM_BINS];
        compose_mixed_into(&mut cleared, &filtered, &silent_sfx, 1.0, 0.0);
        assert!(peak_energy(&cleared) > 0.01);
    }
}
