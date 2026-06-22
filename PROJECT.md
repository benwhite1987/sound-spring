## Sound Spring — Architecture

Sound Spring is a single Rust/Qt binary that manages PipeWire routing, soundboard playback, and optional voice processing (VAD, noise suppression, speaker verification).

### Startup flow

1. Load or create `~/.config/soundboard/config.toml` and seed default tab directories.
2. Apply PipeWire routing (`PipewireManager::setup`) — null sinks, mic loopback, virtual microphone.
3. Start the soundboard backend (tab scan, player, shortcut portal listener).
4. Optionally start the voice pipeline when suppression or verification routing is enabled.

Enable **Launch at login** in Settings to re-apply routing on each session.

### PipeWire routing

The app creates:

```text
soundboard_sfx          → playback sink for soundboard clips
soundboard_virtmic      → internal mix bus
sound_spring_virtual_mic → remapped source exposed to Discord/OBS
```

Mic loopback connects the configured hardware source into the mix bus. Voice processing (when enabled) replaces the raw mic loopback with a gated/denoised feed.

### Config and data paths

| Path | Purpose |
|------|---------|
| `~/.config/soundboard/config.toml` | All settings |
| `~/.config/soundboard/tabs/` | Default sound folders (scan mode) |
| `~/.cache/soundboard/state.json` | Active tab |
| `~/.config/soundboard/voiceprints/` | Enrolled speaker profile |

### Global shortcuts

Shortcuts bind through `xdg-desktop-portal` GlobalShortcuts, not direct KGlobalAccel D-Bus. See [docs/global-shortcuts.md](docs/global-shortcuts.md) before changing shortcut code.

### Embedded assets

Compiled into the binary at build time:

- QML UI (cxx-qt)
- Silero VAD ONNX (via `voice_activity_detector` crate)
- DeepFilterNet3 (via `deep_filter` crate)
- ECAPA-TDNN speaker embedding ONNX (~80 MB, fetched by `build.rs` when missing)
- SpeechBrain fbank matrix (`fbank-80x201-f32.bin`)

User sound files and voiceprints remain on disk under XDG paths.
