//! Processed-audio sink via a `pw-cat --playback` subprocess. The audio thread
//! pushes gated (and later denoised) 48 kHz mono f32 samples into an SPSC ring;
//! a Tokio task drains the ring and writes raw little-endian f32 to the child's
//! stdin, which plays into `soundboard_virtmic`. This is the gated replacement
//! for the Phase 1 raw mic-to-virtmic loopback.
//!
//! Playback is stereo L=R interleaved so PipeWire does not invent a mono→stereo
//! SPA adaptor on the default stereo virtmic sink.

use anyhow::{Context, Result};
use rtrb::Consumer;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};
use tracing::{debug, warn};

use super::CAPTURE_RATE;
use crate::services::host_audio::host_audio_command;

/// Virtmic playback channel count (L=R duplicate of mono pipeline samples).
const PLAYBACK_CHANNELS: u32 = 2;

/// Silence pad when the ring is empty (~3 ms mono frames → stereo interleaved).
const SILENCE_PAD_FRAMES: usize = (CAPTURE_RATE as usize * 3) / 1000;

/// A live playback session. Dropping it aborts the writer task (and its `pw-cat`
/// child).
pub struct Output {
    writer: JoinHandle<()>,
}

impl Output {
    /// Start a `pw-cat --playback` writer targeting `sink`, fed by `consumer`.
    pub fn start(sink: &str, consumer: Consumer<f32>, latency_ms: u32) -> Result<Self> {
        // Prefer ≥40 ms on the routed path; config 20 ms is too thin for DFN delay.
        let latency_ms = latency_ms.max(40).clamp(40, 100);
        let mut command = host_audio_command("pw-cat");
        command
            .arg("--playback")
            .arg("--raw")
            .arg("--rate")
            .arg(CAPTURE_RATE.to_string())
            .arg("--channels")
            .arg(PLAYBACK_CHANNELS.to_string())
            .arg("--format")
            .arg("f32")
            .arg("--latency")
            .arg(format!("{latency_ms}ms"));
        if !sink.is_empty() {
            command.arg("--target").arg(sink);
        }
        command
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        let mut child = command.spawn().context("spawn pw-cat --playback")?;
        let stdin = child
            .stdin
            .take()
            .context("pw-cat --playback missing stdin")?;
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.is_empty() {
                        debug!("pw-cat playback: {line}");
                    }
                }
            });
        }

        let writer = tokio::spawn(async move {
            let mut child = child;
            let mut consumer = consumer;
            let mut stdin = BufWriter::new(stdin);
            let mut bytes: Vec<u8> = Vec::with_capacity(8192);
            loop {
                if let Ok(Some(status)) = child.try_wait() {
                    warn!("pw-cat playback exited: {status}");
                    break;
                }

                bytes.clear();
                while let Ok(sample) = consumer.pop() {
                    // Interleave mono → stereo L=R.
                    let le = sample.to_le_bytes();
                    bytes.extend_from_slice(&le);
                    bytes.extend_from_slice(&le);
                    if bytes.len() >= 8192 {
                        break;
                    }
                }
                if bytes.is_empty() {
                    // Keep pw-cat stdin fed so PipeWire does not underrun.
                    let pad = SILENCE_PAD_FRAMES.max(64);
                    let zero = 0f32.to_le_bytes();
                    for _ in 0..pad {
                        bytes.extend_from_slice(&zero);
                        bytes.extend_from_slice(&zero);
                    }
                    sleep(Duration::from_millis(2)).await;
                }
                if let Err(err) = stdin.write_all(&bytes).await {
                    warn!("voice output write error: {err:#}");
                    break;
                }
                let _ = stdin.flush().await;
            }
            debug!("voice output writer task ended");
        });

        Ok(Self { writer })
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        self.writer.abort();
    }
}
