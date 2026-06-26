//! Off-audio-thread embedding worker.
//!
//! ECAPA inference on the ~80 MB model is far too heavy for the hard-real-time
//! audio thread (a single forward pass can take tens of milliseconds), so the
//! pipeline thread hands it windows of 16 kHz speech and enrollment buffers over
//! a channel. This thread owns the [`Embedder`] and [`Verifier`], publishes
//! match results to [`VoiceShared`], and persists the enrolled voiceprint.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::JoinHandle;

use tracing::{info, warn};

use super::embedding::{Embedder, MIN_EMBED_SAMPLES};
use super::verifier::Verifier;
use super::voiceprint::{l2_normalize, Voiceprint};
use super::{VoiceShared, TARGET_RATE};

/// Enrollment embedding window (3 s) and hop (1.5 s) at 16 kHz.
const ENROLL_WINDOW: usize = TARGET_RATE as usize * 3;
const ENROLL_HOP: usize = TARGET_RATE as usize * 3 / 2;

pub enum EmbedJob {
    /// A window of speech-active 16 kHz samples to verify against the voiceprint.
    Verify(Vec<f32>),
    /// A full enrollment recording (16 kHz) to average into a new voiceprint.
    Enroll(Vec<f32>),
    /// Drop the in-memory voiceprint (the file is removed by the UI).
    Clear,
}

pub struct EmbedWorker {
    handle: Option<JoinHandle<()>>,
}

impl EmbedWorker {
    pub fn spawn(
        rx: Receiver<EmbedJob>,
        shared: Arc<VoiceShared>,
        busy: Arc<AtomicBool>,
        voiceprint_path: PathBuf,
        threshold: f32,
    ) -> Self {
        let handle = std::thread::Builder::new()
            .name("voice-embed".into())
            .spawn(move || run(rx, shared, busy, voiceprint_path, threshold))
            .ok();
        Self { handle }
    }
}

impl Drop for EmbedWorker {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run(
    rx: Receiver<EmbedJob>,
    shared: Arc<VoiceShared>,
    busy: Arc<AtomicBool>,
    voiceprint_path: PathBuf,
    threshold: f32,
) {
    let mut verifier = Verifier::new(threshold);

    // Load an existing voiceprint, if any.
    if voiceprint_path.is_file() {
        match Voiceprint::load(&voiceprint_path) {
            Ok(vp) => {
                info!("loaded voiceprint ({} dims)", vp.dim());
                verifier.set_voiceprint(Some(vp));
                shared.set_enrolled(true);
                if shared.verification_warmup_enabled() && shared.verification_enabled() {
                    shared.set_verify_warmup(true);
                }
            }
            Err(err) => warn!("failed to load voiceprint: {err:#}"),
        }
    }

    let mut embedder: Option<Embedder> = match Embedder::new() {
        Ok(e) => {
            info!("loaded ECAPA embedder (embedded)");
            Some(e)
        }
        Err(err) => {
            warn!("speaker embedding disabled: {err:#}");
            None
        }
    };

    while let Ok(job) = rx.recv() {
        busy.store(true, Ordering::Relaxed);
        match job {
            EmbedJob::Verify(samples) => {
                if let Some(embedder) = embedder.as_mut() {
                    verifier.set_threshold(shared.match_threshold());
                    match embedder.embed(&samples) {
                        Ok(emb) => {
                            let (score, matched) = verifier.verify(&emb);
                            shared.set_speaker(score, matched);
                            if shared.verification_warmup_enabled() && !matched {
                                shared.set_verify_warmup(false);
                            }
                        }
                        Err(err) => warn!("verify embedding failed: {err:#}"),
                    }
                }
            }
            EmbedJob::Enroll(samples) => {
                match embedder.as_mut() {
                    Some(embedder) => match enroll(embedder, &samples) {
                        Ok(vp) => {
                            if let Err(err) = vp.save(&voiceprint_path) {
                                warn!("failed to save voiceprint: {err:#}");
                            } else {
                                info!("enrolled voiceprint -> {}", voiceprint_path.display());
                            }
                            verifier.set_voiceprint(Some(vp));
                            shared.set_enrolled(true);
                            if shared.verification_warmup_enabled() && shared.verification_enabled()
                            {
                                shared.set_verify_warmup(true);
                            }
                            shared.bump_enroll_done();
                        }
                        Err(err) => warn!("enrollment failed: {err:#}"),
                    },
                    None => warn!("enrollment failed: embedding model unavailable"),
                }
                shared.set_enroll_active(false);
                shared.set_enroll_progress(0.0);
            }
            EmbedJob::Clear => {
                verifier.set_voiceprint(None);
                shared.set_enrolled(false);
                shared.set_speaker(0.0, false);
                shared.set_verify_warmup(false);
            }
        }
        busy.store(false, Ordering::Relaxed);
    }
}

/// Average windowed embeddings into a single voiceprint: embed each window,
/// L2-normalize, mean, then re-normalize (handled by `Voiceprint::from_embedding`).
fn enroll(embedder: &mut Embedder, samples: &[f32]) -> anyhow::Result<Voiceprint> {
    let mut sum: Vec<f32> = Vec::new();
    let mut count = 0usize;

    let mut start = 0usize;
    while start + MIN_EMBED_SAMPLES <= samples.len() {
        let end = (start + ENROLL_WINDOW).min(samples.len());
        let window = &samples[start..end];
        match embedder.embed(window) {
            Ok(emb) => {
                let norm = l2_normalize(&emb);
                if sum.is_empty() {
                    sum = vec![0.0; norm.len()];
                }
                for (acc, v) in sum.iter_mut().zip(norm.iter()) {
                    *acc += v;
                }
                count += 1;
            }
            Err(err) => warn!("enrollment window skipped: {err:#}"),
        }
        if end == samples.len() {
            break;
        }
        start += ENROLL_HOP;
    }

    if count == 0 {
        anyhow::bail!("no usable enrollment windows");
    }
    for v in sum.iter_mut() {
        *v /= count as f32;
    }
    Ok(Voiceprint::from_embedding(&sum))
}
