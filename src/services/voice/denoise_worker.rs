//! Off-audio-thread DeepFilterNet worker.
//!
//! DFN inference is too heavy for the hard-real-time pipeline hop loop, so hops
//! are handed over an SPSC ring (mirroring the ECAPA embed worker pattern). The
//! pipeline applies gate/attack on the delayed enhanced stream.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::Result;
use rtrb::chunks::ChunkError;
use rtrb::{Consumer, Producer};
use tracing::{debug, warn};

use super::denoise::Denoiser;

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
            // Drain input so the pipeline does not block forever on a full ring.
            while !stop.load(Ordering::Relaxed) {
                while input.pop().is_ok() {}
                std::thread::sleep(Duration::from_millis(5));
            }
            return;
        }
    };

    let mut in_scratch = Vec::with_capacity(2048);
    let mut out_scratch = Vec::with_capacity(2048);

    while !stop.load(Ordering::Relaxed) {
        if reset.swap(false, Ordering::Relaxed) {
            if let Err(err) = denoiser.reset() {
                warn!("voice denoise reset failed: {err:#}");
            }
            in_scratch.clear();
            out_scratch.clear();
        }

        let mut got_any = false;
        while let Ok(sample) = input.pop() {
            in_scratch.push(sample);
            got_any = true;
            if in_scratch.len() >= 4096 {
                break;
            }
        }

        if !got_any {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }

        out_scratch.clear();
        denoiser.process(&in_scratch, &mut out_scratch);
        in_scratch.clear();

        flush_output(&mut out_scratch, &mut output);
    }
    debug!("voice denoise worker stopped");
}

fn flush_output(pending: &mut Vec<f32>, producer: &mut Producer<f32>) {
    while !pending.is_empty() {
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
                // Drop oldest backlog rather than growing without bound.
                let drop_n = pending.len() / 2;
                pending.drain(..drop_n);
                break;
            }
        }
    }
}
