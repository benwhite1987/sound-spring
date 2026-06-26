//! DeepFilterNet3 noise suppression wrapping `deep_filter`'s `tract` streaming
//! runtime. The DFN3 model and its config are bundled via the crate's
//! `default-model` feature, so no model files are needed at runtime.
//!
//! DeepFilterNet operates on 48 kHz mono audio in fixed `hop_size` frames
//! (480 samples for DFN3). The pipeline feeds arbitrary chunk sizes, so this
//! wrapper buffers input into whole frames and emits enhanced samples. The
//! output is delayed by the model's lookahead and the count per call varies,
//! but it tracks the input sample rate over time.

use anyhow::Result;
use df::tract::{DfParams, DfTract, RuntimeParams};
use ndarray::{ArrayView2, ArrayViewMut2};
use tracing::debug;

pub struct Denoiser {
    df: DfTract,
    hop: usize,
    in_buf: Vec<f32>,
}

impl Denoiser {
    /// Build a denoiser from the bundled DeepFilterNet3 model.
    pub fn new() -> Result<Self> {
        let params = RuntimeParams::default_with_ch(1);
        let df = DfTract::new(DfParams::default(), &params)?;
        let hop = df.hop_size;
        Ok(Self {
            df,
            hop,
            in_buf: Vec::with_capacity(hop * 4),
        })
    }

    /// Feed 48 kHz mono `input`; append enhanced samples to `out`.
    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        self.in_buf.extend_from_slice(input);
        while self.in_buf.len() >= self.hop {
            let mut frame_out = vec![0.0f32; self.hop];
            let noisy = ArrayView2::from_shape((1, self.hop), &self.in_buf[..self.hop])
                .expect("noisy frame shape");
            let enh = ArrayViewMut2::from_shape((1, self.hop), &mut frame_out)
                .expect("enhanced frame shape");
            match self.df.process(noisy, enh) {
                Ok(_) => out.extend_from_slice(&frame_out),
                Err(err) => {
                    debug!("denoise frame failed: {err:#}; passing through");
                    out.extend_from_slice(&self.in_buf[..self.hop]);
                }
            }
            self.in_buf.drain(..self.hop);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Loads the bundled DeepFilterNet3 model and confirms it initializes,
    /// streams whole frames, tracks the input rate (minus lookahead buffering),
    /// and attenuates broadband noise. Ignored by default because it builds the
    /// tract models (slow); run with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn deepfilternet_reduces_noise_energy() {
        let mut d = Denoiser::new().expect("load bundled DFN3 model");
        // ~2 s of white noise at 48 kHz via a cheap LCG (deterministic).
        let n = 48_000 * 2;
        let mut seed: u32 = 0x1234_5678;
        let mut noise = Vec::with_capacity(n);
        for _ in 0..n {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            noise.push((seed as f32 / u32::MAX as f32) * 2.0 - 1.0);
        }

        let mut out = Vec::with_capacity(n);
        for chunk in noise.chunks(1024) {
            d.process(chunk, &mut out);
        }

        // Output tracks input rate within one model delay (lookahead + buffer).
        assert!(
            out.len() >= n - 48_000,
            "too few output samples: {}",
            out.len()
        );
        assert!(out.len() <= n, "more output than input: {}", out.len());

        let energy = |s: &[f32]| s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32;
        let in_e = energy(&noise);
        let out_e = energy(&out);
        assert!(
            out_e < in_e * 0.9,
            "expected noise attenuation: in={in_e:.5} out={out_e:.5}"
        );
    }
}
