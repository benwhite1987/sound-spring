# Sound Spring — Phase 2: Voice Enhancement Panel

This document extends the core Sound Spring spec with a separate Voice
Enhancement panel for processing the live microphone before it reaches the
virtual mic that Discord/OBS captures. It is implemented as a sibling top-level
panel to the Soundboard panel, with its own routing, its own QML pages, and
its own service modules. The Soundboard panel from Phase 1 is unchanged.

Phase 2 is implemented only after Phase 1 satisfies all its acceptance
criteria.

## Why this exists

When you use Discord in Studio mode (Krisp routing is bypassed for low-latency
audio), all of Discord's built-in noise suppression and echo cancellation is
disabled. This is the right choice for music streaming and high-quality voice,
but it puts the burden on you to provide your own clean signal. Sound Spring
becomes that signal chain: real mic → enhancement panel → virtual mic →
Discord.

The enhancement panel must do four things in real time, in order:

1. **Voice activity detection** — gate frames where no one is speaking, so
   noise during silence doesn't bleed through.
2. **Target speaker extraction / verification** — pass through *your* voice;
   reject your wife's voice when it's picked up by your mic at gaming-room
   distance.
3. **Noise suppression** — remove the keyboard, mechanical fan, room HVAC,
   game audio bleeding from headphones.
4. **Spectrum visualization** — a real-time spectral display so you can see
   what's getting through and tune thresholds visually.

Total budget: <30ms end-to-end latency, including PipeWire's own loopback
overhead. Each stage targets <10ms.

## Component selection

### Noise suppression: DeepFilterNet3

Use DeepFilterNet3 over RNNoise. The reasons are concrete and matter to this
project:

- **Female-voice support**: RNNoise was trained on a corpus heavily skewed
  toward male speakers and operates on 22 perceptual bands tuned to common
  speech-formant ranges. It audibly notches and warbles on fundamentals above
  ~220 Hz, which covers a lot of female and child vocal range. DeepFilterNet3
  operates on full-spectrum complex masks per FFT bin and is trained on the
  Deep Noise Suppression Challenge corpus, which is materially more diverse.
- **Better generalization on complex noise**: research from 2025 shows
  DeepFilterNet3 winning on non-stationary noise — TV in another room, game
  audio bleed, overlapping speech, mechanical keyboard clicks during
  speech — exactly your use case.
- **Native Rust integration**: DeepFilterNet is written in Rust. The `libdf`
  crate is published and supports streaming inference. No FFI to Python or C++.
- **PipeWire-native option exists**: the project ships a LADSPA plugin for
  `pipewire-filter-chain`, which you can use as a reference for the inference
  loop.
- **Latency**: 10–20ms, dominated by the STFT window. Acceptable.

RNNoise stays available as a fallback for very low-end machines or as a
comparison toggle. Don't make it the default.

### Voice activity detection: Silero VAD

Use Silero VAD v5 over WebRTC VAD. Silero is a small (~1MB) neural VAD that
runs in ONNX, supports streaming at 16kHz with a fixed 512-sample window
(~32ms), and is dramatically more accurate than the legacy WebRTC GMM
implementation. Three Rust crates wrap it:

- `voice_activity_detector` (uses Silero v5, MIT)
- `silero-vad-rs` (also Silero v5, MIT)
- `wavekat-vad` (multi-backend trait, includes Silero and WebRTC for A/B)

Recommend `wavekat-vad` for the trait abstraction — it lets you swap backends
without changing the rest of the pipeline. Avoid the TEN VAD backend
included in wavekat-vad: its license has a non-compete clause that is not
real open source.

VAD output is a continuous probability per chunk, not a binary decision.
Apply hysteresis (e.g., open-threshold 0.7, close-threshold 0.3) to avoid
chattering at sentence boundaries.

### Speaker verification / target extraction: ECAPA-TDNN + custom gate

Two approaches, in order of complexity:

**Approach A — Verification gate (recommended for v1):**

1. Enrollment: user records ~30 seconds of their voice; the app computes the
   mean ECAPA-TDNN embedding (192-dim vector) and stores it as
   `enrolled_voiceprint`.
2. Runtime: every speech-active chunk (gated by VAD) is converted to its own
   ECAPA-TDNN embedding, compared to the enrolled voiceprint via cosine
   similarity, and gated on a threshold.
3. Output: if cosine sim > threshold, pass; otherwise mute.

This is conceptually simple, latency-cheap (one neural network pass on each
chunk plus a dot product), and good enough for the "wife in the same room"
case because her voice arrives at your mic *much* quieter than yours and is
spectrally distinct.

Pre-trained ECAPA-TDNN models are available from SpeechBrain (PyTorch, can be
exported to ONNX) and NVIDIA NeMo TitaNet (also exportable). Pick TitaNet
small (~6M params, 192-dim output) for the latency target.

**Approach B — Target speaker extraction (deferred):**

Use a VoiceFilter-Lite-style model: a small causal LSTM that takes the noisy
spectrogram *plus* your enrolled voiceprint embedding as conditioning input
and produces a per-frame mask that isolates your voice from any interfering
voice in the mix.

This is dramatically harder to do well — open-source implementations
(`mindslab-ai/voicefilter`, `seungwonpark/voicefilter-lite`) need retraining
on a domain-appropriate dataset, the masks are more aggressive and can
introduce artifacts, and inference latency is higher (40–60ms typical).

Don't implement Approach B in v1. Implement A, see if it's sufficient, and
defer B to a future revision if not.

### A note on the "gender recognition" idea

The reframing from the response above belongs in the spec too: do not
implement a binary M/F classifier. Speaker verification answers the actual
question ("is this Ben?") with better accuracy than gender classification
answers a question you didn't quite mean to ask ("is this someone with a
male-typed voice?"). Gender classification also degrades sharply on voices
under headset compression, tired voices, voices at the edge of typical pitch
ranges, kids' voices, and non-binary or transitioning speakers — failure
modes that don't exist in speaker verification.

## Architecture

```
real_mic
   │
   ▼
┌─────────────┐
│ Capture     │  PipeWire monitor of mic, 48kHz mono f32
│  buffer     │  ring-buffer fed by tokio task
└─────┬───────┘
      │ 10ms frames (480 samples @ 48kHz)
      ▼
┌─────────────┐
│ Resampler   │  48kHz → 16kHz for VAD and embeddings
└─────┬───────┘
      │
      ├──────────────► VAD (Silero)        ─► speech_active: bool
      │
      ├──────────────► Embedder (ECAPA)    ─► chunk_embedding (when active)
      │                       │
      │                       ▼
      │                ┌────────────┐
      │                │ Cosine     │  vs enrolled_voiceprint
      │                │ comparator │  ─► speaker_match: bool
      │                └────────────┘
      │
      ▼
┌─────────────┐
│ Gate        │  Pass through iff speech_active && speaker_match
└─────┬───────┘
      │
      ▼
┌─────────────┐
│ DeepFilter  │  Full-band 48kHz noise suppression
│  Net3       │
└─────┬───────┘
      │
      ▼
┌─────────────┐
│ FFT tap     │  Branch for spectrum visualization
└─────┬───────┘
      │
      ▼
soundboard_virtmic (existing PipeWire sink from Phase 1)
```

The Phase 1 mic-to-virtmic loopback gets replaced by this chain. The
`soundboard_sfx → soundboard_virtmic` loopback stays as it is.

## Service module additions

New Rust modules under `src/services/`:

- `voice/capture.rs` — PipeWire input via `pipewire-rs` (or shell-out to
  `pw-cat --record` initially). Ring buffer in `parking_lot::Mutex<VecDeque>`.
- `voice/resample.rs` — 48kHz↔16kHz via `rubato` crate. Single instance
  reused across frames.
- `voice/vad.rs` — wraps `wavekat-vad` with Silero backend. Hysteresis state
  machine.
- `voice/embedding.rs` — ONNX inference of ECAPA-TDNN via the `ort` crate.
  Loads the model once at startup, reuses session.
- `voice/verifier.rs` — owns the enrolled voiceprint, cosine comparator,
  threshold + hysteresis.
- `voice/denoise.rs` — DeepFilterNet3 via `df` crate. Streaming inference.
- `voice/pipeline.rs` — composes all of the above into a single async task.
  Receives raw frames from capture, emits processed frames into the virtual
  sink. Also emits a downsampled spectrum frame to the QML side at ~30Hz.
- `voice/spectrum.rs` — FFT-based spectrum analyzer for the visualization.
  Uses `realfft` (real-valued FFT, half the work of complex FFT).

New cxx-qt bridge under `src/qobjects/`:

- `voice_controller.rs` — `VoiceController` QObject exposing:
  - Properties: `is_enrolled: bool`, `speaker_match_score: f32`,
    `noise_suppression_db: f32`, `vad_probability: f32`, `is_passing: bool`
  - Invokables: `start_enrollment()`, `cancel_enrollment()`,
    `clear_enrollment()`, `set_threshold(f32)`, `set_suppression_enabled(bool)`,
    `set_verification_enabled(bool)`
  - Signals: `enrollment_progress(percent: i32)`, `enrollment_complete()`,
    `spectrum_frame(values: QList<f32>)`

## UI specification

### Panel switching

The main window gets a top-level tab switcher with two panels: **Soundboard**
(Phase 1) and **Voice** (Phase 2). The soundboard tab UI from Phase 1 lives
inside the Soundboard panel; the new voice enhancement UI lives in the Voice
panel. Both panels stay mounted (their services run continuously), only the
displayed page changes.

In QML: a top-level `Kirigami.PageRow` or a simpler `TabBar` + `StackLayout`.

### Voice panel layout

Single page, vertical layout:

**Top: Spectrum analyzer (`Spectrum.qml`)**

- Full-width, ~200px tall, dark background.
- Two layers drawn on a QML `Canvas` (or better, a custom
  `QQuickPaintedItem` exposed from Rust for performance):
  - Live spectrum: 1024-bin log-frequency display, drawn as a filled curve
    in a bright color, updated at 30Hz from the `spectrum_frame` signal.
  - Trailing waterfall (optional toggle): scrolling 2D heatmap of the last
    5 seconds, scrolls right-to-left, intensity = magnitude.
- Three vertical band markers showing what's being suppressed:
  - Sub-100Hz (rumble, AC hum) — usually heavy suppression
  - 100–4000Hz (speech band) — minimal suppression
  - 4000Hz+ (sibilance, keyboard clicks) — moderate suppression
- Color-code the spectrum red when the gate is closed (mute), green when
  open (passing). Gives you immediate visual confirmation that your voice
  is getting through.

**Middle: Status row**

Three large indicators side by side:

- **VAD**: speech probability as a meter (0–100%), with threshold marker.
  Label "Speaking" / "Silent".
- **Speaker match**: cosine similarity to enrolled voiceprint, as a meter.
  Threshold marker. Label "You" / "Not you" / "Unknown".
- **Noise floor**: estimated noise reduction in dB, updated from
  DeepFilterNet stats. Just a numeric readout.

**Bottom: Controls**

A `Kirigami.FormLayout` with:

- **Enrollment** section:
  - "Enrolled voiceprint: <status>" — shows "None" or
    "Enrolled YYYY-MM-DD HH:MM, 30s of audio".
  - "Re-enroll" button → opens enrollment dialog.
  - "Clear enrollment" button → confirms then wipes.
- **Verification** section:
  - Toggle: "Enable speaker verification" (default off until enrolled).
  - Slider: "Match threshold" (0.0–1.0, default 0.6). Lower = more permissive.
  - Hysteresis offset: implicit (close-threshold = match - 0.1).
- **Noise suppression** section:
  - Toggle: "Enable noise suppression" (default on).
  - Combo: "Model" — `DeepFilterNet3` (default), `RNNoise (fallback)`,
    `Off (passthrough)`.
- **Output routing** section:
  - Read-only label: "Routing to: soundboard_virtmic".
  - Link/button: "Open in pavucontrol" — runs `pavucontrol` for inspection.

### Enrollment dialog

Modal `Kirigami.Dialog`, full-screen on mobile-class windows, sized on
desktop.

- Instructions: "Read the following passage for 30 seconds. Try to read
  naturally, at normal volume, in your usual gaming position."
- A short paragraph of phonetically diverse text (use the Harvard sentences
  or the Rainbow Passage — both public domain).
- "Start recording" button.
- During recording:
  - Live waveform display.
  - 30-second countdown.
  - Spectrum tap also visible.
  - Pause/Resume/Cancel.
- After recording:
  - "Save voiceprint" → embedding computed, stored to disk.
  - "Re-record" if unhappy.
- Storage: `~/.config/soundboard/voiceprints/default.bin` — raw f32 array,
  192 floats, little-endian, plus a 16-byte header (`SSPV` magic, version,
  timestamp, embedding dim).

## State additions

`config.toml` gains a new section:

```toml
[voice]
verification_enabled = false
suppression_enabled = true
suppression_model = "deepfilternet3"   # or "rnnoise" or "off"
match_threshold = 0.6
vad_open_threshold = 0.7
vad_close_threshold = 0.3
enrollment_path = "voiceprints/default.bin"  # relative to config dir
spectrum_fps = 30
```

## Model assets

Three ONNX/binary files need to ship with the app (or be downloaded on
first run):

- `silero_vad.onnx` (~1.5 MB) — from snakers4/silero-vad
- `titanet_small.onnx` (~25 MB) — exported from NeMo, or use SpeechBrain
  ECAPA-TDNN export (~22 MB)
- `DeepFilterNet3.tar.gz` (~9 MB) — official release

Total: ~35 MB. Either bundle in the binary (via `include_bytes!`, bloats
binary but trivial deployment), or download on first run to
`~/.cache/soundboard/models/` with SHA-256 verification.

Recommendation: bundle Silero (it's tiny), download the other two on first
run with a progress dialog. Models can be updated without rebuilding the
app, and a 100 MB statically-linked binary is gross.

## Cargo additions

```toml
[dependencies]
# ... existing deps from Phase 1 ...

# Phase 2 additions:
ort = { version = "2", features = ["load-dynamic"] }
df = "0.5"                          # DeepFilterNet
wavekat-vad = { version = "0.3", default-features = false, features = ["silero"] }
rubato = "0.16"                     # resampling
realfft = "3"                       # spectrum FFT
parking_lot = "0.12"                # faster mutexes for the audio path
ndarray = "0.16"                    # tensor manipulation for ONNX I/O
sha2 = "0.10"                       # model integrity
reqwest = { version = "0.12", features = ["stream"] }  # model download
```

## Performance budget

Targets, measured on the Thelio Mira (i9-14900K):

| Stage              | Target latency | Memory  |
|--------------------|---------------:|--------:|
| Capture + ring     |          <2ms  |   <5 MB |
| Resample 48→16     |          <1ms  |   <2 MB |
| Silero VAD         |          <3ms  |    2 MB |
| ECAPA-TDNN         |          <8ms  |   25 MB |
| Cosine compare     |        <0.1ms  |       0 |
| DeepFilterNet3     |        <15ms   |   10 MB |
| Spectrum FFT       |          <2ms  |   <1 MB |
| **Total**          |        **<30ms**| **<50 MB** |

CPU usage target: <10% of one core at steady state on the Thelio. Profile
with `cargo flamegraph` and `tracy` if anything exceeds budget.

## Implementation notes for code generation

- **Audio path runs on its own thread, not Tokio.** The audio pipeline is
  hard-real-time-adjacent and benefits from a dedicated `std::thread` with a
  bounded SPSC channel (`rtrb` or `ringbuf` crate) for samples. Tokio is fine
  for the control plane (UI events, model loading) but not for per-frame
  audio dispatch.
- **ONNX session caching is mandatory.** Creating an `ort::Session` takes
  ~100ms. Create all three (VAD, embedder, denoiser) at app startup, hold
  them in an `Arc`, and reuse for the app's lifetime.
- **Avoid allocations in the hot path.** Pre-allocate all working buffers
  (FFT input/output, ONNX tensor staging, resampler scratch) at pipeline
  construction and reuse. Profile shows allocator pressure is the #1 cause
  of audio glitches in naive implementations.
- **Spectrum updates are decoupled from audio rate.** The audio thread pushes
  spectrum frames to a separate `crossbeam::ArrayQueue` (capacity 4); a
  30Hz Tokio task drains it and forwards the latest to the QML side via
  `CxxQtThread::queue`. Dropping spectrum frames is fine; dropping audio
  frames is not.
- **Verification is gated by VAD.** Don't run the ECAPA-TDNN model on silence.
  Wastes CPU and the embeddings of silence are noise. Run it only on frames
  where Silero says speech_prob > 0.5.
- **Enrollment is offline.** During enrollment, capture audio to disk first,
  then run ECAPA-TDNN over windowed slices and average the embeddings (L2-
  normalize first, then mean, then re-normalize). This is more accurate than
  a single forward pass on 30 seconds of audio.
- **Hysteresis everywhere.** VAD: open at 0.7, close at 0.3. Speaker match:
  open at threshold, close at threshold − 0.1. Without hysteresis you get
  chattery gating that makes consonants disappear.
- **DeepFilterNet attenuation cap.** Set the max suppression to ~30dB. Above
  that, residual speech artifacts become audible musical noise. The `df`
  crate exposes this as a parameter.
- **Test with your wife in the room.** The acceptance criteria below assume
  you can recruit her for a 5-minute test session. This is the only honest
  way to validate the feature.

## Acceptance criteria

In addition to all Phase 1 criteria continuing to hold:

1. Enrollment of a 30-second voiceprint completes within 35 seconds (30s
   recording + ≤5s embedding) and produces a 192-float file on disk.
2. With verification enabled and you speaking normally, the cosine match
   score sits above 0.75 the vast majority of the time.
3. With verification enabled and your wife speaking 3 meters from your mic
   at normal conversation volume, the cosine match score sits below the
   threshold and her voice is gated (≤5% of her syllables make it through).
4. Both of you speaking simultaneously: your voice is preserved, hers is
   attenuated by ≥15 dB at the virtual mic monitor.
5. DeepFilterNet3 reduces mechanical keyboard noise by ≥20 dB during silence
   and ≥10 dB during overlapping speech.
6. Total mic-to-virtmic latency is under 30ms as measured by a loopback
   round-trip test (clap into the mic, see the spike in OBS waveform).
7. CPU usage at steady state is under 10% of one core on the Thelio Mira.
8. Spectrum visualization updates at 30 Hz without noticeable stutter.
9. With suppression and verification both off, the pipeline is a pure
   passthrough — audio sounds identical to a direct mic→Discord routing.
10. Toggling any setting while audio is active does not produce a pop,
    click, or dropout.

## What to hand the model first

When starting Phase 2:

1. This Phase 2 spec.
2. The completed Phase 1 codebase as context (especially `services/pipewire.rs`
   and the cxx-qt bridge patterns).
3. The DeepFilterNet README and the `df` crate docs:
   <https://docs.rs/df/latest/df/>
4. The wavekat-vad README: <https://github.com/wavekat/wavekat-vad>
5. The SpeechBrain ECAPA-TDNN docs (for understanding the embedding
   semantics; the actual inference is via ONNX):
   <https://huggingface.co/speechbrain/spkrec-ecapa-voxceleb>

Scaffold order:

`voice/capture.rs` (with a sine-wave test source first, then real PipeWire) →
`voice/resample.rs` → `voice/spectrum.rs` (visualize before processing) →
`voice/vad.rs` → `voice/embedding.rs` (test that enrollment produces stable
embeddings) → `voice/verifier.rs` → `voice/denoise.rs` →
`voice/pipeline.rs` (compose everything) → `qobjects/voice_controller.rs` →
QML for the voice panel → enrollment dialog.

Each module gets unit tests with synthetic audio (sine waves, white noise,
recordings from `~/audio_test/` if you want real data). The end-to-end
loopback test is the validation gate.
