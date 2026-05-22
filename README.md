# Sound Spring

A PipeWire-routed soundboard with tab-cycling hotkeys, designed for KDE Plasma on Wayland (CachyOS and similar distros).

Sounds play through a dedicated virtual sink and are mixed with your real microphone into a single **virtual microphone** that Discord, Zoom, and OBS can capture.

## Quick start

```bash
./install.sh
```

1. Pick your hardware microphone when prompted (or skip for Sound Spring audio only).
2. Add audio files to tab directories under `~/.config/soundboard/tabs/`.
3. Bind hotkeys (see below).

In Discord/OBS, set **Microphone** to **Sound-Spring-Virtual-Microphone**.

## Directory layout

```
~/.config/soundboard/
├── config.toml              # mic, paths, and optional tab folder list
└── tabs/                    # default tabs root (subfolder per tab)
    ├── 01-memes/
    └── ...

~/.cache/soundboard/
└── state.json               # current tab path (shared with future GUI)
```

Number-prefix files so lexical sort maps to slots 1–10. Tab folder names are for display and order only.

Supported audio formats: **OGG**, **WAV**, **FLAC**, **MP3**, plus Opus/M4A/AAC. WAV and OGG play via `paplay`; MP3 uses `ffmpeg` piped to `paplay` when available.

### Custom tab folders

By default, each subdirectory under `tabs_root` is a tab. To use folders anywhere on disk (for example folders picked in the future GUI), add them to `config.toml`:

```toml
[paths]
tabs_root = "/home/you/.config/soundboard/tabs"
state_dir = "/home/you/.cache/soundboard"

[[tabs]]
path = "/home/you/Music/memes"
name = "Memes"

[[tabs]]
path = "/media/sounds/clips"
name = "Clips"
```

When any `[[tabs]]` entries exist, only those folders are used (scan mode is disabled). `state.json` stores the full path of the active tab.

## Commands

Installed to `~/.local/bin/` by `./install.sh`:

| Command | Description |
|---------|-------------|
| `sb-play <0-9>` | Play slot on current tab (0 = slot 10) |
| `sb-tab next\|prev` | Cycle tabs (wraps around) |
| `sb-stop` | Stop all Sound Spring playback |

Playback target: `paplay --device=soundboard_sfx`.

## Hotkeys

### X11 (sxhkd)

Bindings are installed to `~/.config/sxhkd/soundboard.conf` and included from `sxhkdrc`:

- `Super+1` … `Super+9`, `Super+0` → `sb-play`
- `Super+]` → `sb-tab next`
- `Super+[` → `sb-tab prev`
- `Super+Escape` → `sb-stop`

Reload: `pkill -USR1 sxhkd`

### KDE Plasma Wayland

`sxhkd` is X11-only. Create **Custom Shortcuts** in System Settings → Shortcuts with the same `sb-play`, `sb-tab`, and `sb-stop` commands.

Suggested defaults:

| Action | Command |
|--------|---------|
| Play slot 1–9 | `sb-play 1` … `sb-play 9` |
| Play slot 10 | `sb-play 0` |
| Next tab | `sb-tab next` |
| Previous tab | `sb-tab prev` |
| Stop all | `sb-stop` |

## PipeWire routing

`./install.sh` creates two null sinks, loopbacks, and a remapped virtual microphone input:

| Device name | Type | Purpose |
|-------------|------|---------|
| **Sound-Spring-Virtual-Microphone** | Microphone (input) | Select this in Discord/OBS |
| **Sound-Spring-Effects** | Speaker (output) | Internal playback sink |
| **Sound-Spring-Mix** | Speaker (output) | Internal mix bus — do not use as mic |

Your real mic (if configured) and Sound Spring audio are both looped into the mix, then exposed as **Sound-Spring-Virtual-Microphone**.

### Persistence across reboot

A systemd user service keeps routing alive after login:

```bash
systemctl --user status soundboard-pipewire
```

If PipeWire is restarted while the service is inactive, `sb-play` will attempt to re-create the sinks automatically.

### Re-run install

```bash
./install.sh                  # interactive; reuses saved mic by default
./install.sh --mic <source>   # non-interactive mic selection
./install.sh --skip-mic       # Sound Spring audio only, no mic loopback
SOUNDBOARD_MIC=<source> ./install.sh
```

Mic choice is saved to `~/.config/soundboard/config.toml`.

## Uninstall

```bash
./uninstall.sh
```

Removes scripts, systemd service, PipeWire modules, and runtime state. Tab audio folders are kept by default; custom folders registered in `config.toml` are never deleted automatically.

## Future GUI

The full Rust/Qt/KDE application is specified in [SOUNDBOARD_SPEC.md](SOUNDBOARD_SPEC.md). The bash layer uses the same config and state file paths so migration will be seamless.

## See also

- [PROJECT.md](PROJECT.md) — bash layer architecture and manual PipeWire commands
- [SOUNDBOARD_SPEC.md](SOUNDBOARD_SPEC.md) — full application specification
