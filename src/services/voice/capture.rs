//! Mic capture via a `pw-cat --record` subprocess producing raw little-endian
//! f32 mono samples at 48 kHz, decoded and pushed into the audio thread's SPSC
//! ring.

use anyhow::{Context, Result};
use rtrb::Producer;
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use tracing::{debug, warn};

use super::{VoiceShared, CAPTURE_CHANNELS, CAPTURE_RATE};

/// A live capture session. Dropping it kills the `pw-cat` child and aborts the
/// reader task.
pub struct Capture {
    child: Child,
    reader: JoinHandle<()>,
}

impl Capture {
    pub fn start(mic_source: &str, producer: Producer<f32>, shared: Arc<VoiceShared>) -> Result<Self> {
        let mut command = Command::new("pw-cat");
        command
            .arg("--record")
            .arg("--raw")
            .arg("--rate")
            .arg(CAPTURE_RATE.to_string())
            .arg("--channels")
            .arg(CAPTURE_CHANNELS.to_string())
            .arg("--format")
            .arg("f32");
        if !mic_source.is_empty() {
            command.arg("--target").arg(mic_source);
        }
        command
            .arg("-")
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let mut child = command.spawn().context("spawn pw-cat --record")?;
        let stdout = child
            .stdout
            .take()
            .context("pw-cat --record missing stdout")?;

        let shared = shared.clone();
        let reader = tokio::spawn(async move {
            let mut producer = producer;
            let mut reader = BufReader::new(stdout);
            let mut buf = vec![0u8; 8192];
            let mut carry: Vec<u8> = Vec::with_capacity(4);
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => push_samples(&buf[..n], &mut carry, &mut producer),
                    Err(err) => {
                        warn!("voice capture read error: {err:#}");
                        shared.set_capture_status(
                            false,
                            &format!("Microphone read error: {err:#}"),
                        );
                        break;
                    }
                }
            }
            debug!("voice capture reader task ended");
            if shared.capturing.load(Ordering::Relaxed) {
                shared.set_capture_status(
                    false,
                    "Microphone capture ended unexpectedly (check mic source in Settings)",
                );
            }
        });

        Ok(Self { child, reader })
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
        self.reader.abort();
    }
}

/// Decode `bytes` (continuation of any partial sample in `carry`) into f32
/// samples and push them into `producer`, dropping samples when the ring is
/// full (the audio thread is behind; a viz gap is acceptable).
fn push_samples(bytes: &[u8], carry: &mut Vec<u8>, producer: &mut Producer<f32>) {
    let mut offset = 0;
    if !carry.is_empty() {
        let need = 4 - carry.len();
        let take = need.min(bytes.len());
        carry.extend_from_slice(&bytes[..take]);
        offset = take;
        if carry.len() == 4 {
            let sample = f32::from_le_bytes([carry[0], carry[1], carry[2], carry[3]]);
            let _ = producer.push(if sample.is_finite() { sample } else { 0.0 });
            carry.clear();
        }
    }
    let rest = &bytes[offset..];
    let full = rest.len() / 4 * 4;
    for chunk in rest[..full].chunks_exact(4) {
        let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let _ = producer.push(if sample.is_finite() { sample } else { 0.0 });
    }
    carry.extend_from_slice(&rest[full..]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_split_samples_across_reads() {
        let (mut producer, mut consumer) = rtrb::RingBuffer::<f32>::new(16);
        let values = [1.0_f32, -0.5, 0.25];
        let mut raw = Vec::new();
        for v in values {
            raw.extend_from_slice(&v.to_le_bytes());
        }
        let mut carry = Vec::new();
        // Deliver the byte stream in awkward splits to exercise the carry path.
        push_samples(&raw[..3], &mut carry, &mut producer);
        push_samples(&raw[3..7], &mut carry, &mut producer);
        push_samples(&raw[7..], &mut carry, &mut producer);

        let mut decoded = Vec::new();
        while let Ok(s) = consumer.pop() {
            decoded.push(s);
        }
        assert_eq!(decoded, values);
        assert!(carry.is_empty());
    }
}
