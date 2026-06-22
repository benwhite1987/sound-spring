//! Streaming 48 kHz -> 16 kHz mono resampler.
//!
//! Milestone 1 builds and exercises this ahead of the VAD/embedding consumers
//! that arrive in later milestones (the spectrum analyzer runs on the 48 kHz
//! capture frames directly). Buffers are preallocated and reused so the later
//! real-time path stays allocation-free.

use anyhow::Result;
use rubato::{FftFixedIn, Resampler as _};

use super::{CAPTURE_RATE, TARGET_RATE};

pub struct Resampler {
    inner: FftFixedIn<f32>,
    chunk_in: usize,
    pending: Vec<f32>,
    out_scratch: Vec<Vec<f32>>,
}

impl Resampler {
    pub fn new() -> Result<Self> {
        let inner = FftFixedIn::<f32>::new(
            CAPTURE_RATE as usize,
            TARGET_RATE as usize,
            1024,
            1,
            1,
        )?;
        let chunk_in = inner.input_frames_next();
        let out_max = inner.output_frames_max();
        Ok(Self {
            inner,
            chunk_in,
            pending: Vec::with_capacity(chunk_in * 2),
            out_scratch: vec![vec![0.0_f32; out_max]],
        })
    }

    /// Feed 48 kHz mono samples; appends any produced 16 kHz samples to `out`.
    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) -> Result<()> {
        self.pending
            .extend(input.iter().map(|&s| if s.is_finite() { s } else { 0.0 }));
        while self.pending.len() >= self.chunk_in {
            let n = self.chunk_in;
            let (_, written) =
                self.inner
                    .process_into_buffer(&[&self.pending[..n]], &mut self.out_scratch, None)?;
            out.extend_from_slice(&self.out_scratch[0][..written]);
            self.pending.drain(..n);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downsamples_by_three() {
        let mut resampler = Resampler::new().expect("resampler");
        // One second of 48 kHz silence -> ~one second of 16 kHz output.
        let input = vec![0.0_f32; CAPTURE_RATE as usize];
        let mut out = Vec::new();
        resampler.process(&input, &mut out).expect("process");
        let expected = TARGET_RATE as usize;
        let tolerance = expected / 20; // 5%
        assert!(
            out.len().abs_diff(expected) <= tolerance,
            "expected ~{expected} samples, got {}",
            out.len()
        );
    }

    #[test]
    fn sanitizes_non_finite_samples() {
        let mut resampler = Resampler::new().expect("resampler");
        let hop = super::super::FFT_HOP;
        let mut out = Vec::new();
        for i in 0..200 {
            let input: Vec<f32> = (0..hop)
                .map(|j| {
                    if (i + j) % 17 == 0 {
                        f32::NAN
                    } else if (i + j) % 23 == 0 {
                        f32::INFINITY
                    } else {
                        0.0
                    }
                })
                .collect();
            resampler.process(&input, &mut out).expect("process");
        }
    }

    #[test]
    fn preserves_a_constant_signal() {
        let mut resampler = Resampler::new().expect("resampler");
        let input = vec![0.5_f32; CAPTURE_RATE as usize];
        let mut out = Vec::new();
        resampler.process(&input, &mut out).expect("process");
        // Skip transient edges; the steady-state level should track the input.
        let mid = &out[out.len() / 4..out.len() * 3 / 4];
        let mean = mid.iter().sum::<f32>() / mid.len() as f32;
        assert!((mean - 0.5).abs() < 0.05, "mean drifted to {mean}");
    }
}
