//! Processed-audio sink via a `pw-cat --playback` subprocess. The audio thread
//! pushes gated (and later denoised) 48 kHz mono f32 samples into an SPSC ring;
//! a Tokio task drains the ring and writes raw little-endian f32 to the child's
//! stdin, which plays into `soundboard_virtmic`. This is the gated replacement
//! for the Phase 1 raw mic-to-virtmic loopback.

use anyhow::{Context, Result};
use rtrb::Consumer;
use std::process::Stdio;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};
use tracing::{debug, warn};

use super::{CAPTURE_CHANNELS, CAPTURE_RATE};

/// A live playback session. Dropping it kills the `pw-cat` child and aborts the
/// writer task.
pub struct Output {
    child: Child,
    writer: JoinHandle<()>,
}

impl Output {
    /// Start a `pw-cat --playback` writer targeting `sink`, fed by `consumer`.
    pub fn start(sink: &str, consumer: Consumer<f32>) -> Result<Self> {
        let mut command = Command::new("pw-cat");
        command
            .arg("--playback")
            .arg("--rate")
            .arg(CAPTURE_RATE.to_string())
            .arg("--channels")
            .arg(CAPTURE_CHANNELS.to_string())
            .arg("--format")
            .arg("f32");
        if !sink.is_empty() {
            command.arg("--target").arg(sink);
        }
        command
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let mut child = command.spawn().context("spawn pw-cat --playback")?;
        let stdin = child
            .stdin
            .take()
            .context("pw-cat --playback missing stdin")?;

        let writer = tokio::spawn(async move {
            let mut consumer = consumer;
            let mut stdin = BufWriter::new(stdin);
            let mut bytes: Vec<u8> = Vec::with_capacity(4096);
            loop {
                bytes.clear();
                while let Ok(sample) = consumer.pop() {
                    bytes.extend_from_slice(&sample.to_le_bytes());
                    if bytes.len() >= 4096 {
                        break;
                    }
                }
                if bytes.is_empty() {
                    // No samples ready; yield briefly rather than busy-spin.
                    sleep(Duration::from_millis(2)).await;
                    continue;
                }
                if let Err(err) = stdin.write_all(&bytes).await {
                    warn!("voice output write error: {err:#}");
                    break;
                }
                let _ = stdin.flush().await;
            }
            debug!("voice output writer task ended");
        });

        Ok(Self { child, writer })
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
        self.writer.abort();
    }
}
