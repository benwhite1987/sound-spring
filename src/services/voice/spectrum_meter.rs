//! Post-fader spectrum metering: dB-domain fader taps, mixed compose, bar mapping.

use super::spectrum::analysis_level_apply_gain;
use super::spectrum_bars::{
    compute_bar_levels_from_magnitudes, BarBallistics, SPECTRUM_BAR_COUNT,
};

/// Apply a post-fader gain in the analysis dB domain (0..1 fader → dB attenuation).
pub fn apply_fader_db(magnitudes: &[f32], gain: f32, out: &mut [f32]) {
    debug_assert_eq!(magnitudes.len(), out.len());
    for (dst, &mag) in out.iter_mut().zip(magnitudes.iter()) {
        *dst = analysis_level_apply_gain(mag, gain);
    }
}

/// Energy-sum of mic and SFX legs after independent post-fader dB scaling.
pub fn compose_mixed_into(
    out: &mut [f32],
    filtered: &[f32],
    sfx: &[f32],
    mic_gain: f32,
    output_gain: f32,
) {
    debug_assert_eq!(filtered.len(), sfx.len());
    debug_assert_eq!(out.len(), filtered.len());
    for ((slot, &f), &s) in out.iter_mut().zip(filtered.iter()).zip(sfx.iter()) {
        let f = analysis_level_apply_gain(f, mic_gain);
        let s = analysis_level_apply_gain(s, output_gain);
        *slot = (f * f + s * s).sqrt();
    }
}

/// Mic-only mixed frame (post-fader).
pub fn compose_mic_only_into(out: &mut [f32], filtered: &[f32], mic_gain: f32) {
    apply_fader_db(filtered, mic_gain, out);
}

/// Map post-fader magnitude bins to raw 0..1 bar targets (no ballistics).
pub fn map_to_bar_targets(magnitudes: &[f32]) -> [f32; SPECTRUM_BAR_COUNT] {
    compute_bar_levels_from_magnitudes(magnitudes)
}

/// Map magnitudes to display bar levels with VU-style ballistics.
pub fn map_to_bar_levels(
    magnitudes: &[f32],
    ballistics: &mut BarBallistics,
) -> [f32; SPECTRUM_BAR_COUNT] {
    let targets = map_to_bar_targets(magnitudes);
    ballistics.update(&targets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::voice::spectrum::SpectrumAnalyzer;
    use crate::services::voice::spectrum_bars::{level_to_display_db, linear_volume_gain};
    use crate::services::voice::SPECTRUM_BINS;
    use crate::services::voice::{CAPTURE_RATE, FFT_SIZE};

    fn sine(freq: f32, len: usize) -> Vec<f32> {
        (0..len)
            .map(|n| (2.0 * std::f32::consts::PI * freq * n as f32 / CAPTURE_RATE as f32).sin())
            .collect()
    }

    fn peak(mags: &[f32]) -> f32 {
        mags.iter().cloned().fold(0.0_f32, f32::max)
    }

    #[test]
    fn fader_half_attenuates_display_by_6db() {
        let mut analyzer = SpectrumAnalyzer::new();
        let mags = analyzer.analyze(&sine(1000.0, FFT_SIZE)).to_vec();
        let mut full = vec![0.0; SPECTRUM_BINS];
        let mut half = vec![0.0; SPECTRUM_BINS];
        apply_fader_db(&mags, 1.0, &mut full);
        apply_fader_db(&mags, 0.5, &mut half);
        let full_peak = map_to_bar_targets(&full)
            .iter()
            .cloned()
            .fold(0.0_f32, f32::max);
        let half_peak = map_to_bar_targets(&half)
            .iter()
            .cloned()
            .fold(0.0_f32, f32::max);
        let full_db = level_to_display_db(full_peak);
        let half_db = level_to_display_db(half_peak);
        assert!(half_peak < full_peak);
        assert!((full_db - half_db - 6.0).abs() < 1.5, "full_db={full_db} half_db={half_db}");
    }

    #[test]
    fn compose_mixed_independent_legs() {
        let mut analyzer = SpectrumAnalyzer::new();
        let filtered = analyzer.analyze(&sine(400.0, FFT_SIZE)).to_vec();
        let sfx = analyzer.analyze(&sine(2000.0, FFT_SIZE)).to_vec();
        let mut mic_only = vec![0.0; SPECTRUM_BINS];
        let mut sfx_only = vec![0.0; SPECTRUM_BINS];
        compose_mic_only_into(&mut mic_only, &filtered, 0.5);
        compose_mixed_into(&mut sfx_only, &vec![0.0; SPECTRUM_BINS], &sfx, 1.0, 0.5);
        let mic_peak = peak(&mic_only);
        let sfx_peak = peak(&sfx_only);
        assert!(mic_peak < peak(&filtered));
        assert!(sfx_peak < peak(&sfx));
    }

    #[test]
    fn linear_volume_gain_respects_mute() {
        assert_eq!(linear_volume_gain(80, true), 0.0);
        assert!((linear_volume_gain(50, false) - 0.5).abs() < 1e-6);
    }
}
