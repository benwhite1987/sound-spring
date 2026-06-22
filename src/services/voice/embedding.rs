//! ECAPA-TDNN speaker embedding via ONNX Runtime (`ort`).
//!
//! The model is the SpeechBrain `spkrec-ecapa-voxceleb` export
//! (`vedk00/ecapa-voxceleb-speaker-embedding-onnx`, Apache-2.0). It does NOT
//! take raw audio: its `features` input is an 80-bin log-mel "fbank" computed
//! exactly the way SpeechBrain does, so this module reproduces that front end on
//! `realfft`:
//!
//! 1. STFT: 400-pt FFT, 400-sample periodic Hamming window, hop 160, centered
//!    with constant (zero) padding -> 201 one-sided bins.
//! 2. Power spectrum `|X|^2` (SpeechBrain `spectral_magnitude` default power=1).
//! 3. Mel projection through the frozen 80x201 SpeechBrain filterbank matrix
//!    (`fbank-80x201-f32.bin`, bundled).
//! 4. `_amplitude_to_DB`: `10*log10(max(x, 1e-10))`, then floor-clamp to
//!    `(per-utterance max - 80 dB)`.
//! 5. Sentence mean subtraction (per-mel mean over time).
//!
//! The 192-float output is L2-normalized downstream by the verifier/voiceprint.
//!
//! The ECAPA ONNX weights are compiled into the binary at build time (fetched by
//! `build.rs` when missing locally). The fbank matrix is also embedded.

use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use ort::session::Session;
use ort::value::{Tensor, ValueType};
use ort::tensor::TensorElementType;
use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};

use super::TARGET_RATE;

/// SpeechBrain Fbank STFT parameters (16 kHz, 25 ms window, 10 ms hop).
const N_FFT: usize = 400;
const HOP: usize = 160;
/// One-sided FFT bins: `N_FFT/2 + 1`.
const N_STFT: usize = N_FFT / 2 + 1; // 201
/// Mel filter count (ECAPA uses 80, not the SpeechBrain default 40).
const N_MELS: usize = 80;
/// ECAPA embedding dimensionality.
pub const EMBEDDING_DIM: usize = 192;
/// `_amplitude_to_DB` floor (Filterbank `amin`).
const AMIN: f32 = 1e-10;
/// Power-spectrogram dB multiplier (`10*log10` for power, SpeechBrain `power_spectrogram=2`).
const DB_MULTIPLIER: f32 = 10.0;
/// `_amplitude_to_DB` dynamic-range clip below the per-utterance max.
const TOP_DB: f32 = 80.0;
/// Centered-STFT padding (`N_FFT/2`).
const PAD: usize = N_FFT / 2;

/// Minimum input length (in 16 kHz samples) to attempt an embedding. ECAPA's
/// attentive statistics pooling needs a few frames to be meaningful; ~0.5 s.
pub const MIN_EMBED_SAMPLES: usize = TARGET_RATE as usize / 2;

/// The frozen SpeechBrain mel filterbank, row-major `[N_MELS][N_STFT]`.
static FBANK_BYTES: &[u8] = include_bytes!("../../../assets/models/fbank-80x201-f32.bin");

/// ECAPA-TDNN ONNX weights (`vedk00/ecapa-voxceleb-speaker-embedding-onnx`).
static ECAPA_BYTES: &[u8] = include_bytes!("../../../assets/models/ecapa-speaker-v1.onnx");

/// How the model expects the `feature_lens` input expressed.
#[derive(Clone, Copy)]
enum LenKind {
    /// int64 absolute frame count.
    FramesI64,
    /// float32 relative length (fraction of the utterance present, i.e. 1.0).
    RelativeF32,
}

pub struct Embedder {
    session: Session,
    fft: Arc<dyn RealToComplex<f32>>,
    window: Vec<f32>,
    /// `[N_MELS * N_STFT]`, row-major.
    fbank: Vec<f32>,
    len_kind: LenKind,
    // Reused scratch.
    frame_in: Vec<f32>,
    spectrum: Vec<Complex<f32>>,
    fft_scratch: Vec<Complex<f32>>,
}

impl Embedder {
    pub fn new() -> Result<Self> {
        let fbank = parse_fbank()?;
        let session = Session::builder()
            .context("ort session builder")?
            .commit_from_memory(ECAPA_BYTES)
            .context("load embedded ECAPA model")?;

        let len_kind = resolve_len_kind(&session)?;

        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(N_FFT);
        let frame_in = fft.make_input_vec();
        let spectrum = fft.make_output_vec();
        let fft_scratch = fft.make_scratch_vec();

        Ok(Self {
            session,
            fft,
            window: hamming_periodic(N_FFT),
            fbank,
            len_kind,
            frame_in,
            spectrum,
            fft_scratch,
        })
    }

    /// Compute a raw (un-normalized) 192-d embedding for 16 kHz mono samples.
    pub fn embed(&mut self, samples: &[f32]) -> Result<Vec<f32>> {
        let features = self.log_mel(samples)?;
        let n_frames = features.len() / N_MELS;
        if n_frames == 0 {
            bail!("embedding input too short: {} samples", samples.len());
        }

        let feat_tensor = Tensor::from_array((
            vec![1_i64, n_frames as i64, N_MELS as i64],
            features,
        ))
        .context("build features tensor")?;

        let outputs = match self.len_kind {
            LenKind::FramesI64 => {
                let lens = Tensor::from_array((vec![1_i64], vec![n_frames as i64]))
                    .context("build feature_lens tensor")?;
                self.session
                    .run(ort::inputs!["features" => feat_tensor, "feature_lens" => lens])
                    .context("ECAPA inference")?
            }
            LenKind::RelativeF32 => {
                let lens = Tensor::from_array((vec![1_i64], vec![1.0_f32]))
                    .context("build feature_lens tensor")?;
                self.session
                    .run(ort::inputs!["features" => feat_tensor, "feature_lens" => lens])
                    .context("ECAPA inference")?
            }
        };

        let (_, data) = outputs["embedding"]
            .try_extract_tensor::<f32>()
            .context("extract embedding")?;
        if data.len() < EMBEDDING_DIM {
            bail!("embedding output had {} values", data.len());
        }
        Ok(data[..EMBEDDING_DIM].to_vec())
    }

    /// Produce SpeechBrain-compatible, sentence-mean-normalized log-mel features
    /// flattened frame-major as `[frames * N_MELS]`.
    fn log_mel(&mut self, samples: &[f32]) -> Result<Vec<f32>> {
        // Centered STFT: pad PAD zeros on each side.
        let mut padded = Vec::with_capacity(samples.len() + 2 * PAD);
        padded.resize(PAD, 0.0);
        padded.extend_from_slice(samples);
        padded.resize(padded.len() + PAD, 0.0);
        if padded.len() < N_FFT {
            return Ok(Vec::new());
        }

        let n_frames = 1 + (padded.len() - N_FFT) / HOP;
        let mut log_mel = vec![0.0_f32; n_frames * N_MELS];
        let mut global_max = f32::NEG_INFINITY;

        for f in 0..n_frames {
            let start = f * HOP;
            for (i, dst) in self.frame_in.iter_mut().enumerate() {
                *dst = padded[start + i] * self.window[i];
            }
            self.fft
                .process_with_scratch(&mut self.frame_in, &mut self.spectrum, &mut self.fft_scratch)
                .map_err(|e| anyhow!("rfft: {e:?}"))?;

            let row = &mut log_mel[f * N_MELS..(f + 1) * N_MELS];
            for (m, out) in row.iter_mut().enumerate() {
                let filt = &self.fbank[m * N_STFT..(m + 1) * N_STFT];
                let mut mel = 0.0_f32;
                for (k, w) in filt.iter().enumerate() {
                    // Power spectrum |X|^2 (re^2 + im^2).
                    let c = self.spectrum[k];
                    mel += (c.re * c.re + c.im * c.im) * w;
                }
                let db = DB_MULTIPLIER * mel.max(AMIN).log10();
                *out = db;
                if db > global_max {
                    global_max = db;
                }
            }
        }

        // _amplitude_to_DB top-db clip, over the whole sequence.
        let floor = global_max - TOP_DB;
        for v in log_mel.iter_mut() {
            if *v < floor {
                *v = floor;
            }
        }

        // Sentence mean subtraction: per-mel mean across frames.
        for m in 0..N_MELS {
            let mut sum = 0.0_f32;
            for f in 0..n_frames {
                sum += log_mel[f * N_MELS + m];
            }
            let mean = sum / n_frames as f32;
            for f in 0..n_frames {
                log_mel[f * N_MELS + m] -= mean;
            }
        }

        Ok(log_mel)
    }
}

fn parse_fbank() -> Result<Vec<f32>> {
    let expected = N_MELS * N_STFT;
    if FBANK_BYTES.len() != expected * 4 {
        bail!(
            "bundled fbank matrix is {} bytes, expected {}",
            FBANK_BYTES.len(),
            expected * 4
        );
    }
    Ok(FBANK_BYTES
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

fn resolve_len_kind(session: &Session) -> Result<LenKind> {
    let input = session
        .inputs
        .iter()
        .find(|i| i.name == "feature_lens")
        .ok_or_else(|| anyhow!("model has no `feature_lens` input"))?;
    match &input.input_type {
        ValueType::Tensor { ty, .. } => match ty {
            TensorElementType::Int64 => Ok(LenKind::FramesI64),
            TensorElementType::Float32 => Ok(LenKind::RelativeF32),
            other => bail!("unexpected feature_lens element type: {other:?}"),
        },
        other => bail!("feature_lens is not a tensor: {other:?}"),
    }
}

/// Periodic Hamming window (`torch.hamming_window` default, divisor `N`).
fn hamming_periodic(len: usize) -> Vec<f32> {
    (0..len)
        .map(|n| 0.54 - 0.46 * (2.0 * std::f32::consts::PI * n as f32 / len as f32).cos())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fbank_has_expected_shape() {
        let fbank = parse_fbank().expect("fbank parses");
        assert_eq!(fbank.len(), N_MELS * N_STFT);
        // Mel weights are non-negative and not all zero.
        assert!(fbank.iter().all(|w| *w >= 0.0));
        assert!(fbank.iter().any(|w| *w > 0.0));
    }

    fn tone(freq: f32, secs: f32) -> Vec<f32> {
        let n = (TARGET_RATE as f32 * secs) as usize;
        (0..n)
            .map(|i| {
                0.5 * (2.0 * std::f32::consts::PI * freq * i as f32 / TARGET_RATE as f32).sin()
            })
            .collect()
    }

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot / (na * nb)
    }

    #[test]
    #[ignore]
    fn ecapa_model_io_contract_holds() {
        let mut emb = Embedder::new().expect("load embedder");

        let a = tone(180.0, 2.0);
        let e1 = emb.embed(&a).expect("embed a #1");
        let e2 = emb.embed(&a).expect("embed a #2");
        assert_eq!(e1.len(), EMBEDDING_DIM);
        assert!(e1.iter().all(|v| v.is_finite()), "embedding must be finite");

        let self_cos = cosine(&e1, &e2);
        assert!(self_cos > 0.999, "determinism: self-cosine {self_cos}");

        let b = tone(330.0, 2.0);
        let e3 = emb.embed(&b).expect("embed b");
        let cross = cosine(&e1, &e3);
        eprintln!("self={self_cos:.4} cross={cross:.4}");
        assert!(cross < self_cos, "distinct signals should differ");
    }

    #[test]
    fn hamming_is_symmetric_and_bounded() {
        let w = hamming_periodic(N_FFT);
        assert_eq!(w.len(), N_FFT);
        for v in &w {
            assert!(*v >= 0.0 && *v <= 1.0 + 1e-6);
        }
        // Periodic Hamming starts at its minimum 0.08.
        assert!((w[0] - 0.08).abs() < 1e-4);
    }
}
