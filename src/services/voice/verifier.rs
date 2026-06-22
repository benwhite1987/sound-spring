//! Speaker verification: compares a runtime embedding against the enrolled
//! voiceprint via cosine similarity, with open/close hysteresis so the gate
//! doesn't chatter near the threshold.
//!
//! Per the spec the close-threshold trails the (configurable) match threshold
//! by a fixed 0.1 offset.

use super::voiceprint::{l2_normalize, Voiceprint};

/// How far below the match threshold the gate stays open once it has opened.
const HYSTERESIS_OFFSET: f32 = 0.1;

pub struct Verifier {
    /// Enrolled, L2-normalized reference embedding.
    reference: Option<Vec<f32>>,
    threshold: f32,
    matched: bool,
    last_score: f32,
}

impl Verifier {
    pub fn new(threshold: f32) -> Self {
        Self {
            reference: None,
            threshold,
            matched: false,
            last_score: 0.0,
        }
    }

    pub fn set_voiceprint(&mut self, voiceprint: Option<Voiceprint>) {
        self.reference = voiceprint.map(|vp| vp.vector);
        self.matched = false;
        self.last_score = 0.0;
    }

    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold.clamp(0.0, 1.0);
    }

    /// Compare a raw embedding to the enrolled voiceprint. Returns
    /// `(cosine_score, matched)`. With no enrollment, returns `(0.0, false)`.
    pub fn verify(&mut self, embedding: &[f32]) -> (f32, bool) {
        let Some(reference) = self.reference.as_ref() else {
            self.matched = false;
            self.last_score = 0.0;
            return (0.0, false);
        };
        let score = cosine_prenorm_reference(reference, embedding);
        self.last_score = score;
        self.matched = next_matched(self.matched, score, self.threshold, HYSTERESIS_OFFSET);
        (score, self.matched)
    }
}

/// Cosine similarity where `reference` is already L2-normalized and `candidate`
/// is normalized here.
fn cosine_prenorm_reference(reference: &[f32], candidate: &[f32]) -> f32 {
    if reference.len() != candidate.len() {
        return 0.0;
    }
    let candidate = l2_normalize(candidate);
    reference
        .iter()
        .zip(candidate.iter())
        .map(|(a, b)| a * b)
        .sum()
}

/// Hysteresis transition: open at `threshold`, close at `threshold - offset`.
fn next_matched(matched: bool, score: f32, threshold: f32, offset: f32) -> bool {
    if matched {
        score >= threshold - offset
    } else {
        score >= threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hysteresis_opens_and_closes_with_offset() {
        // Idle: needs to reach the threshold to open.
        assert!(!next_matched(false, 0.59, 0.6, 0.1));
        assert!(next_matched(false, 0.61, 0.6, 0.1));
        // Open: stays open within the 0.1 band.
        assert!(next_matched(true, 0.52, 0.6, 0.1));
        // Open: closes once below threshold - offset.
        assert!(!next_matched(true, 0.49, 0.6, 0.1));
    }

    #[test]
    fn identical_embedding_scores_one() {
        let mut v = Verifier::new(0.6);
        v.set_voiceprint(Some(Voiceprint::from_embedding(&[1.0, 2.0, 2.0])));
        let (score, matched) = v.verify(&[1.0, 2.0, 2.0]);
        assert!((score - 1.0).abs() < 1e-5, "self-similarity should be ~1");
        assert!(matched);
    }

    #[test]
    fn orthogonal_embedding_is_rejected() {
        let mut v = Verifier::new(0.6);
        v.set_voiceprint(Some(Voiceprint::from_embedding(&[1.0, 0.0])));
        let (score, matched) = v.verify(&[0.0, 1.0]);
        assert!(score.abs() < 1e-5);
        assert!(!matched);
    }

    #[test]
    fn unenrolled_never_matches() {
        let mut v = Verifier::new(0.6);
        let (score, matched) = v.verify(&[1.0, 0.0]);
        assert_eq!(score, 0.0);
        assert!(!matched);
    }
}
