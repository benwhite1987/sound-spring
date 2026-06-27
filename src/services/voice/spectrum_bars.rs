//! Precomputed bar levels for the Voice panel spectrum (matches `qml/Spectrum.qml`).

use super::spectrum::{analysis_level_to_db, ANALYSIS_DB_MAX, ANALYSIS_DB_MIN};
use super::SPECTRUM_BINS;

pub const SPECTRUM_BAR_COUNT: usize = 21;
pub const SEGMENT_COUNT: usize = 11;

pub const DB_MIN: f32 = -60.0;
pub const DB_MAX: f32 = 4.0;

/// Subtracted from analysis dB before mapping to the visible -60..+4 scale.
/// Tuned so typical speech at 100% post-fader sits mid-scale.
pub const DISPLAY_INPUT_TRIM_DB: f32 = 14.0;

/// UI poll interval (matches VoicePanel.qml timer).
pub const UI_TICK_MS: f32 = 33.0;
/// VU-style release time for bar peak decay.
pub const BALLISTICS_RELEASE_MS: f32 = 300.0;

pub const SEGMENT_DB: [f32; SEGMENT_COUNT] = [
    -60.0, -40.0, -20.0, -16.0, -12.0, -8.0, -4.0, -2.0, 0.0, 2.0, 4.0,
];

/// Per-segment vertical fraction (bottom → top); sums to 1.0.
pub const SEGMENT_Y_FRAC: [f32; SEGMENT_COUNT] = [1.0 / SEGMENT_COUNT as f32; SEGMENT_COUNT];

const FREQ_MIN: f32 = 20.0;
const FREQ_MAX: f32 = 24000.0;

const BANDS: &[(f32, f32, u32)] = &[
    (20.0, 60.0, 3),
    (60.0, 250.0, 3),
    (250.0, 500.0, 3),
    (500.0, 2000.0, 3),
    (2000.0, 4000.0, 3),
    (4000.0, 6000.0, 3),
    (6000.0, 24000.0, 3),
];

fn freq_fraction(freq: f32) -> f32 {
    let f = freq.clamp(FREQ_MIN, FREQ_MAX);
    (f / FREQ_MIN).ln() / (FREQ_MAX / FREQ_MIN).ln()
}

/// Linear amplitude gain from a soundboard fader (0..100%, mute → 0).
pub fn linear_volume_gain(percent: u8, muted: bool) -> f32 {
    if muted {
        0.0
    } else {
        (percent as f32 / 100.0).clamp(0.0, 1.0)
    }
}

/// Map a 0..1 display level to dB on the visible scale.
pub fn level_to_display_db(level: f32) -> f32 {
    DB_MIN + level.clamp(0.0, 1.0) * (DB_MAX - DB_MIN)
}

/// Map visible dB to 0..1 display level.
pub fn db_to_level(db: f32) -> f32 {
    ((db - DB_MIN) / (DB_MAX - DB_MIN)).clamp(0.0, 1.0)
}

/// How many LED segments (bottom-up) are lit for a 0..1 bar level.
pub fn lit_segment_count_for_level(level: f32) -> usize {
    if level <= 1e-6 {
        return 0;
    }
    let db = level_to_display_db(level);
    SEGMENT_DB.iter().filter(|&&tick| db >= tick).count()
}

/// How many LED segments (bottom-up) are lit for a 0..1 bar level.
pub fn lit_segment_count(level: f32) -> usize {
    lit_segment_count_for_level(level)
}

fn raw_peak_to_display_level(raw: f32) -> f32 {
    let db = analysis_level_to_db(raw) - DISPLAY_INPUT_TRIM_DB;
    db_to_level(db)
}

/// Peak magnitude across magnitude bins `[bin_start, bin_end)`.
fn peak_magnitude_in_bin_range(magnitudes: &[f32], bin_start: usize, bin_end: usize) -> f32 {
    magnitudes
        .iter()
        .enumerate()
        .filter(|(i, _)| *i >= bin_start && *i < bin_end)
        .map(|(_, mag)| *mag)
        .fold(0.0_f32, f32::max)
}

/// Map post-fader normalized magnitude bins to 21 log-frequency bar levels (0..1).
pub fn compute_bar_levels_from_magnitudes(magnitudes: &[f32]) -> [f32; SPECTRUM_BAR_COUNT] {
    let bins = magnitudes.len().min(SPECTRUM_BINS);
    let mut levels = [0.0f32; SPECTRUM_BAR_COUNT];
    let mut bar_idx = 0;
    let mut next_bin = 0usize;
    for &(min_hz, max_hz, subs) in BANDS {
        let t0_base = freq_fraction(min_hz);
        let t1_base = freq_fraction(max_hz);
        let w = (t1_base - t0_base) / subs as f32;
        for sub in 0..subs {
            let t1 = if sub + 1 == subs {
                t1_base
            } else {
                t0_base + (sub + 1) as f32 * w
            };
            let bin_end = if bar_idx + 1 == SPECTRUM_BAR_COUNT {
                bins
            } else {
                (t1 * bins as f32).round() as usize
            };
            let bin_start = next_bin.min(bins);
            let bin_end = bin_end.clamp(bin_start, bins);
            let peak = peak_magnitude_in_bin_range(magnitudes, bin_start, bin_end);
            levels[bar_idx] = raw_peak_to_display_level(peak);
            next_bin = bin_end;
            bar_idx += 1;
        }
    }
    levels
}

/// Fast-attack / slow-release ballistics per bar (VU-style).
#[derive(Clone, Debug, Default)]
pub struct BarBallistics {
    held: [f32; SPECTRUM_BAR_COUNT],
}

impl BarBallistics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.held.fill(0.0);
    }

    /// Apply ballistics: instant attack, exponential release toward targets.
    pub fn update(&mut self, targets: &[f32; SPECTRUM_BAR_COUNT]) -> [f32; SPECTRUM_BAR_COUNT] {
        let decay = (-UI_TICK_MS / BALLISTICS_RELEASE_MS).exp();
        let mut out = [0.0f32; SPECTRUM_BAR_COUNT];
        for i in 0..SPECTRUM_BAR_COUNT {
            let target = targets[i].clamp(0.0, 1.0);
            if target >= self.held[i] {
                self.held[i] = target;
            } else {
                self.held[i] = (self.held[i] * decay).max(target);
            }
            out[i] = self.held[i];
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_count_matches_qml() {
        let total: u32 = BANDS.iter().map(|(_, _, subs)| subs).sum();
        assert_eq!(total as usize, SPECTRUM_BAR_COUNT);
    }

    #[test]
    fn segment_y_frac_sums_to_one() {
        let sum: f32 = SEGMENT_Y_FRAC.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "sum={sum}");
    }

    #[test]
    fn silent_input_yields_low_levels() {
        let mags = vec![0.0; SPECTRUM_BINS];
        let levels = compute_bar_levels_from_magnitudes(&mags);
        assert!(levels.iter().all(|level| *level == 0.0));
    }

    #[test]
    fn lit_segment_count_tracks_db_thresholds() {
        assert_eq!(lit_segment_count(0.0), 0);
        assert_eq!(lit_segment_count_for_level(0.01), 1);
        assert_eq!(lit_segment_count_for_level(db_to_level(-40.0)), 2);
        assert_eq!(lit_segment_count_for_level(db_to_level(-20.0)), 3);
        assert_eq!(lit_segment_count_for_level(db_to_level(0.0)), 9);
        assert_eq!(lit_segment_count_for_level(db_to_level(2.0)), 10);
        assert_eq!(lit_segment_count(1.0), SEGMENT_COUNT);
    }

    #[test]
    fn peak_at_ceiling_reaches_upper_segments() {
        let level = raw_peak_to_display_level(1.0);
        let db = level_to_display_db(level);
        assert!(db >= -4.0, "peak should reach upper segments, got {db} dB");
        assert!(lit_segment_count(level) >= 8);
    }

    #[test]
    fn sub_bass_middle_bar_picks_up_tone() {
        let mut mags = vec![0.0; SPECTRUM_BINS];
        mags[7] = 0.8;
        let levels = compute_bar_levels_from_magnitudes(&mags);
        assert!(
            levels[1] > 0.05,
            "middle sub-bass bar should map bin 7 energy, got {}",
            levels[1]
        );
    }

    #[test]
    fn speech_like_peak_stays_mid_scale() {
        let analysis_level = ((-10.0 - ANALYSIS_DB_MIN) / (ANALYSIS_DB_MAX - ANALYSIS_DB_MIN))
            .clamp(0.0, 1.0);
        let level = raw_peak_to_display_level(analysis_level);
        let db = level_to_display_db(level);
        assert!(db < 2.0, "speech-like peak should stay below red tier, got {db} dB");
        assert!(lit_segment_count(level) < SEGMENT_COUNT - 1);
    }

    #[test]
    fn ballistics_attack_is_instant() {
        let mut b = BarBallistics::new();
        let mut t = [0.0; SPECTRUM_BAR_COUNT];
        t[0] = 0.8;
        let out = b.update(&t);
        assert!((out[0] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn ballistics_release_decays() {
        let mut b = BarBallistics::new();
        let mut hot = [0.0; SPECTRUM_BAR_COUNT];
        hot[0] = 0.8;
        let _ = b.update(&hot);
        let cold = [0.0; SPECTRUM_BAR_COUNT];
        let out = b.update(&cold);
        assert!(out[0] < 0.8);
        assert!(out[0] > 0.0);
    }

    #[test]
    fn linear_volume_gain_respects_mute() {
        assert_eq!(linear_volume_gain(80, true), 0.0);
        assert!((linear_volume_gain(50, false) - 0.5).abs() < 1e-6);
    }
}
