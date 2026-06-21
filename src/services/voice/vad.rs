//! Voice activity detection via the Silero VAD v5 model (bundled in the
//! `voice_activity_detector` crate, ONNX inference through `ort`).
//!
//! Runs on the 16 kHz resampled stream in fixed 512-sample windows (the only
//! window size Silero v5 supports at 16 kHz) and applies open/close hysteresis
//! so gating doesn't chatter at sentence boundaries.

use anyhow::Result;
use voice_activity_detector::VoiceActivityDetector;

use super::TARGET_RATE;

/// Silero v5 window size at 16 kHz (the only supported size).
const VAD_CHUNK: usize = 512;

pub struct Vad {
    detector: VoiceActivityDetector,
    open: f32,
    close: f32,
    active: bool,
    last_prob: f32,
    pending: Vec<f32>,
}

impl Vad {
    pub fn new(open_threshold: f32, close_threshold: f32) -> Result<Self> {
        let detector = VoiceActivityDetector::builder()
            .sample_rate(TARGET_RATE as i64)
            .chunk_size(VAD_CHUNK)
            .build()
            .map_err(|err| anyhow::anyhow!("build Silero VAD: {err}"))?;
        Ok(Self {
            detector,
            open: open_threshold,
            close: close_threshold,
            active: false,
            last_prob: 0.0,
            pending: Vec::with_capacity(VAD_CHUNK * 2),
        })
    }

    /// Feed 16 kHz mono samples; runs inference on each full 512-sample window
    /// and updates the hysteresis gate. Returns the most recent
    /// (probability, speech_active).
    pub fn process(&mut self, samples: &[f32]) -> (f32, bool) {
        self.pending.extend_from_slice(samples);
        while self.pending.len() >= VAD_CHUNK {
            let chunk: Vec<f32> = self.pending.drain(..VAD_CHUNK).collect();
            let prob = self.detector.predict(chunk);
            self.last_prob = prob;
            self.active = next_active(self.active, prob, self.open, self.close);
        }
        (self.last_prob, self.active)
    }
}

/// Hysteresis state transition: open above `open`, close below `close`.
fn next_active(active: bool, prob: f32, open: f32, close: f32) -> bool {
    if active {
        prob >= close
    } else {
        prob > open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hysteresis_requires_open_then_close() {
        // Below open while idle: stays closed.
        assert!(!next_active(false, 0.65, 0.7, 0.3));
        // Above open while idle: opens.
        assert!(next_active(false, 0.75, 0.7, 0.3));
        // In the hysteresis band while open: stays open.
        assert!(next_active(true, 0.4, 0.7, 0.3));
        // Below close while open: closes.
        assert!(!next_active(true, 0.25, 0.7, 0.3));
    }

    #[test]
    fn silence_does_not_trigger_speech() {
        let mut vad = Vad::new(0.7, 0.3).expect("build vad");
        let (prob, active) = vad.process(&vec![0.0_f32; VAD_CHUNK * 4]);
        assert!(!active, "silence should not be flagged as speech");
        assert!(prob < 0.7, "silence probability {prob} exceeded open threshold");
    }
}
