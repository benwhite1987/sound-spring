//! Real-FFT spectrum analyzer producing log-frequency magnitude bins for the
//! visualization. Buffers are preallocated and reused per the spec's
//! avoid-allocations-in-the-hot-path guidance.

use std::sync::Arc;

use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};

use super::{CAPTURE_RATE, FFT_SIZE, SPECTRUM_BINS};

/// Lowest frequency mapped to the first output bin.
const FREQ_MIN: f32 = 20.0;

pub struct SpectrumAnalyzer {
    fft: Arc<dyn RealToComplex<f32>>,
    window: Vec<f32>,
    input: Vec<f32>,
    spectrum: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    /// Output bin index for each usable FFT bin (`1..=FFT_SIZE/2`).
    bin_map: Vec<usize>,
    magnitudes: Vec<f32>,
}

impl SpectrumAnalyzer {
    pub fn new() -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let input = fft.make_input_vec();
        let spectrum = fft.make_output_vec();
        let scratch = fft.make_scratch_vec();
        let half = FFT_SIZE / 2;
        let window = hann_window(FFT_SIZE);
        let bin_map = (0..=half)
            .map(|i| log_bin_index(i, FFT_SIZE, CAPTURE_RATE, SPECTRUM_BINS))
            .collect();
        Self {
            fft,
            window,
            input,
            spectrum,
            scratch,
            bin_map,
            magnitudes: vec![0.0; SPECTRUM_BINS],
        }
    }

    /// Analyze exactly [`FFT_SIZE`] samples, returning normalized (0..1)
    /// log-frequency magnitudes of length [`SPECTRUM_BINS`].
    pub fn analyze(&mut self, frame: &[f32]) -> &[f32] {
        debug_assert_eq!(frame.len(), FFT_SIZE);
        for (dst, (sample, win)) in self
            .input
            .iter_mut()
            .zip(frame.iter().zip(self.window.iter()))
        {
            *dst = sample * win;
        }

        if self
            .fft
            .process_with_scratch(&mut self.input, &mut self.spectrum, &mut self.scratch)
            .is_err()
        {
            return &self.magnitudes;
        }

        for value in self.magnitudes.iter_mut() {
            *value = 0.0;
        }
        // Max-pool FFT magnitudes into log-spaced output bins (skip DC).
        for (i, bin) in self.spectrum.iter().enumerate().skip(1) {
            let out_index = self.bin_map[i];
            let level = normalize_db(bin.norm());
            let slot = &mut self.magnitudes[out_index];
            if level > *slot {
                *slot = level;
            }
        }
        &self.magnitudes
    }
}

fn hann_window(len: usize) -> Vec<f32> {
    (0..len)
        .map(|n| {
            let x = std::f32::consts::PI * n as f32 / (len as f32 - 1.0);
            x.sin().powi(2)
        })
        .collect()
}

/// Analysis dB floor for normalized FFT magnitudes (below visible -60 dB).
pub const ANALYSIS_DB_MIN: f32 = -72.0;
/// Analysis dB ceiling; headroom above the +4 dB display top.
pub const ANALYSIS_DB_MAX: f32 = 12.0;

/// Convert a normalized analysis level (0..1) back to dB.
pub fn analysis_level_to_db(level: f32) -> f32 {
    level.clamp(0.0, 1.0) * (ANALYSIS_DB_MAX - ANALYSIS_DB_MIN) + ANALYSIS_DB_MIN
}

/// Map analysis dB back to a normalized 0..1 level.
pub fn analysis_db_to_level(db: f32) -> f32 {
    ((db - ANALYSIS_DB_MIN) / (ANALYSIS_DB_MAX - ANALYSIS_DB_MIN)).clamp(0.0, 1.0)
}

/// Apply linear amplitude gain in the analysis dB domain (avoids sub-floor cliffs).
pub fn analysis_level_apply_gain(level: f32, amplitude_gain: f32) -> f32 {
    if amplitude_gain <= 0.0 {
        return 0.0;
    }
    if (amplitude_gain - 1.0).abs() < f32::EPSILON {
        return level.clamp(0.0, 1.0);
    }
    let db = analysis_level_to_db(level) + 20.0 * amplitude_gain.log10();
    analysis_db_to_level(db)
}

/// Convert a linear FFT magnitude to a normalized 0..1 level on the analysis dB scale.
fn normalize_db(magnitude: f32) -> f32 {
    let db = 20.0 * (magnitude + 1e-9).log10();
    ((db - ANALYSIS_DB_MIN) / (ANALYSIS_DB_MAX - ANALYSIS_DB_MIN)).clamp(0.0, 1.0)
}

/// Map FFT bin `i` (of an `fft_size`-point transform at `sample_rate`) onto one
/// of `bins` log-spaced output bins spanning [`FREQ_MIN`]..Nyquist.
fn log_bin_index(i: usize, fft_size: usize, sample_rate: u32, bins: usize) -> usize {
    let freq = i as f32 * sample_rate as f32 / fft_size as f32;
    if freq <= FREQ_MIN {
        return 0;
    }
    let fmax = sample_rate as f32 / 2.0;
    let ratio = (freq / FREQ_MIN).ln() / (fmax / FREQ_MIN).ln();
    ((ratio * bins as f32) as usize).min(bins - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, sample_rate: u32, len: usize) -> Vec<f32> {
        (0..len)
            .map(|n| (2.0 * std::f32::consts::PI * freq * n as f32 / sample_rate as f32).sin())
            .collect()
    }

    #[test]
    fn pure_tone_peaks_in_expected_bin() {
        let mut analyzer = SpectrumAnalyzer::new();
        let freq = 1000.0_f32;
        let frame = sine(freq, CAPTURE_RATE, FFT_SIZE);
        let magnitudes = analyzer.analyze(&frame).to_vec();

        let max_value = magnitudes.iter().cloned().fold(0.0_f32, f32::max);
        let fft_bin = (freq * FFT_SIZE as f32 / CAPTURE_RATE as f32).round() as usize;
        let expected = log_bin_index(fft_bin, FFT_SIZE, CAPTURE_RATE, SPECTRUM_BINS);
        // The tone's main lobe spans a few neighboring bins; assert the expected
        // bin sits at the spectral peak rather than being a unique argmax.
        assert!(
            (magnitudes[expected] - max_value).abs() < 1e-6,
            "expected bin {expected} ({}) not at spectral peak {max_value}",
            magnitudes[expected]
        );
        // And nothing far away outranks it: bins beyond a small neighborhood
        // must be strictly below the peak.
        for (idx, value) in magnitudes.iter().enumerate() {
            if idx.abs_diff(expected) > 4 {
                assert!(
                    *value < max_value,
                    "distant bin {idx} matched the peak magnitude"
                );
            }
        }
    }

    #[test]
    fn log_bin_index_is_monotonic() {
        let half = FFT_SIZE / 2;
        let mut prev = 0;
        for i in 1..=half {
            let idx = log_bin_index(i, FFT_SIZE, CAPTURE_RATE, SPECTRUM_BINS);
            assert!(idx >= prev, "bin {i} mapped below its predecessor");
            assert!(idx < SPECTRUM_BINS);
            prev = idx;
        }
    }
}
