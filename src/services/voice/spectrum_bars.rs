//! Precomputed bar levels for the Voice panel spectrum (matches `qml/Spectrum.qml`).

use super::SPECTRUM_BINS;

pub const SPECTRUM_BAR_COUNT: usize = 21;

const FREQ_MIN: f32 = 20.0;
const FREQ_MAX: f32 = 24000.0;
const NOISE_FLOOR: f32 = 0.38;
const DISPLAY_GAMMA: f32 = 1.6;

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

fn freq_from_fraction(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    FREQ_MIN * (FREQ_MAX / FREQ_MIN).powf(t)
}

fn display_level(raw: f32) -> f32 {
    let t = ((raw - NOISE_FLOOR) / (1.0 - NOISE_FLOOR)).max(0.0);
    t.powf(DISPLAY_GAMMA).min(1.0)
}

/// Map normalized magnitude bins to the 21 log-frequency bar levels shown in QML.
pub fn compute_bar_levels(magnitudes: &[f32]) -> [f32; SPECTRUM_BAR_COUNT] {
    let bins = magnitudes.len().min(SPECTRUM_BINS);
    let mut levels = [0.0f32; SPECTRUM_BAR_COUNT];
    let mut bar_idx = 0;
    for &(min_hz, max_hz, subs) in BANDS {
        let t0_base = freq_fraction(min_hz);
        let t1_base = freq_fraction(max_hz);
        let w = (t1_base - t0_base) / subs as f32;
        for sub in 0..subs {
            let t0 = t0_base + sub as f32 * w;
            let t1 = t0 + w;
            let hz0 = freq_from_fraction(t0);
            let hz1 = freq_from_fraction(t1);
            let mut peak = 0.0f32;
            for (i, magnitude) in magnitudes.iter().enumerate().take(bins) {
                let t = i as f32 / (bins.saturating_sub(1).max(1)) as f32;
                let hz = freq_from_fraction(t);
                if hz >= hz0 && hz < hz1 {
                    peak = peak.max(*magnitude);
                }
            }
            if bar_idx < SPECTRUM_BAR_COUNT {
                levels[bar_idx] = display_level(peak);
                bar_idx += 1;
            }
        }
    }
    levels
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
    fn silent_input_yields_low_levels() {
        let mags = vec![0.0; SPECTRUM_BINS];
        let levels = compute_bar_levels(&mags);
        assert!(levels.iter().all(|level| *level == 0.0));
    }
}
