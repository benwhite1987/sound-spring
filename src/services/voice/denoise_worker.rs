//! Off-audio-thread DeepFilterNet worker.
//!
//! DFN inference is too heavy for the hard-real-time pipeline hop loop, so hops
//! are handed over an SPSC ring (mirroring the ECAPA embed worker pattern). The
//! pipeline keeps a watermarked delay FIFO of enhanced samples and applies the
//! output gate on that delayed stream — never splicing raw over DFN.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::Result;
use rtrb::chunks::ChunkError;
use rtrb::{Consumer, Producer};
use tracing::{debug, warn};

use super::denoise::Denoiser;

/// Max DFN frames processed per wake (keeps the pipeline delay cushion fed steadily).
const MAX_FRAMES_PER_WAKE: usize = 8;

/// When the input scratch backlog is large, catch up harder so the delay refill
/// does not fall behind wall-clock hops.
const MAX_FRAMES_CATCH_UP: usize = 12;

pub struct DenoiseWorker {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl DenoiseWorker {
    pub fn spawn(input: Consumer<f32>, output: Producer<f32>) -> Result<(Self, Arc<AtomicBool>)> {
        let stop = Arc::new(AtomicBool::new(false));
        let reset = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread_reset = reset.clone();
        let handle = std::thread::Builder::new()
            .name("voice-denoise".into())
            .spawn(move || run(input, output, thread_stop, thread_reset))?;
        Ok((
            Self {
                stop,
                handle: Some(handle),
            },
            reset,
        ))
    }
}

impl Drop for DenoiseWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run(
    mut input: Consumer<f32>,
    mut output: Producer<f32>,
    stop: Arc<AtomicBool>,
    reset: Arc<AtomicBool>,
) {
    let mut denoiser = match Denoiser::new() {
        Ok(d) => d,
        Err(err) => {
            warn!("voice denoise worker disabled: {err:#}");
            while !stop.load(Ordering::Relaxed) {
                while input.pop().is_ok() {}
                std::thread::sleep(Duration::from_millis(5));
            }
            return;
        }
    };

    let hop = denoiser.hop_size().max(1);
    let max_batch = hop * MAX_FRAMES_CATCH_UP;
    let mut in_scratch = Vec::with_capacity(max_batch);
    let mut out_scratch = Vec::with_capacity(max_batch);
    let mut output_block_events: u64 = 0;

    while !stop.load(Ordering::Relaxed) {
        if reset.swap(false, Ordering::Relaxed) {
            if let Err(err) = denoiser.reset() {
                warn!("voice denoise reset failed: {err:#}");
            }
            in_scratch.clear();
            out_scratch.clear();
        }

        while in_scratch.len() < hop {
            match input.pop() {
                Ok(sample) => in_scratch.push(sample),
                Err(_) => break,
            }
            if in_scratch.len() >= max_batch {
                break;
            }
        }

        if in_scratch.len() < hop {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }

        // Process whole frames only; leave a remainder < hop for the next wake.
        let frame_cap = if in_scratch.len() >= hop * MAX_FRAMES_PER_WAKE {
            MAX_FRAMES_CATCH_UP
        } else {
            MAX_FRAMES_PER_WAKE
        };
        let take = (in_scratch.len() / hop).min(frame_cap) * hop;
        out_scratch.clear();
        denoiser.process(&in_scratch[..take], &mut out_scratch);
        in_scratch.drain(..take);

        flush_output_blocking(&mut out_scratch, &mut output, &stop, &mut output_block_events);
    }
    debug!("voice denoise worker stopped");
}

fn flush_output_blocking(
    pending: &mut Vec<f32>,
    producer: &mut Producer<f32>,
    stop: &AtomicBool,
    block_events: &mut u64,
) {
    while !pending.is_empty() && !stop.load(Ordering::Relaxed) {
        match producer.push_entire_slice(pending) {
            Ok(()) => {
                pending.clear();
                break;
            }
            Err(ChunkError::TooFewSlots(n)) if n > 0 => {
                let _ = producer.push_entire_slice(&pending[..n]);
                pending.drain(..n);
            }
            Err(_) => {
                *block_events = block_events.saturating_add(1);
                if *block_events == 1 || (*block_events).is_multiple_of(200) {
                    debug!(
                        count = *block_events,
                        "denoise worker waiting on full output ring"
                    );
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}
