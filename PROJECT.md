## Sound Spring — Bash Script Soundboard

Two layers: PipeWire setup for the virtual mic plumbing, then bash scripts for playback and tab cycling.

**1. PipeWire routing (run `./install.sh`, or the systemd user service after install)**

`./install.sh` creates two null sinks, loopbacks, and a remapped virtual microphone input. A systemd user service (`soundboard-pipewire.service`) re-applies routing at login. Manual equivalent:

```bash
# Internal mix bus (shows under Speakers — do not use as mic)
pactl load-module module-null-sink \
  sink_name=soundboard_virtmic \
  sink_properties=device.description=Sound-Spring-Mix

# Sound Spring playback sink (shows under Speakers)
pactl load-module module-null-sink \
  sink_name=soundboard_sfx \
  sink_properties=device.description=Sound-Spring-Effects

# Loop Sound Spring audio into mix bus
pactl load-module module-loopback \
  source=soundboard_sfx.monitor sink=soundboard_virtmic latency_msec=20

# Loop your real mic into mix bus (replace with your actual source name from `pactl list sources short`)
pactl load-module module-loopback \
  source=alsa_input.usb-YOUR_MIC sink=soundboard_virtmic latency_msec=20

# Virtual microphone input for Discord/OBS (shows under Microphone)
pactl load-module module-remap-source \
  master=soundboard_virtmic.monitor \
  source_name=sound_spring_virtual_mic \
  source_properties=device.description=Sound-Spring-Virtual-Microphone
```

Mic source and latency are persisted in `~/.config/soundboard/config.toml`. Check service status: `systemctl --user status soundboard-pipewire`.

In Discord/Zoom/OBS, set **Microphone** to **Sound-Spring-Virtual-Microphone**.

**2. Directory layout**

```
~/.config/soundboard/
├── config.toml
└── tabs/                    # default tabs root; subdirs become tabs

~/.cache/soundboard/
└── state.json               # current tab (absolute path)
```

Optional `[[tabs]]` entries in `config.toml` register folders anywhere on disk as tabs (used by the future GUI). When present, only those paths are tabs — `tabs_root` scan mode is skipped.

Number-prefix files so lexical sort gives slot 1, 2, 3… Tab folder names are for display/order only.

Supported audio: OGG, WAV, FLAC, MP3 (also Opus, M4A, AAC). MP3 is decoded through `ffmpeg` when installed.

**3. The three scripts**

Installed from `scripts/` by `./install.sh` into `~/.local/bin/`:

- `sb-play <0-9>` — play slot (0 = slot 10) on current tab
- `sb-tab next|prev` — cycle tabs
- `sb-stop` — kill all Sound Spring playback

Playback target: `paplay --device=soundboard_sfx`. If sinks are missing (e.g. after a PipeWire restart), `sb-play` re-runs setup from config.

**4. Hotkey bindings**

`sxhkd` fragment at `~/.config/sxhkd/soundboard.conf` (included from `sxhkdrc` on install):

```
super + {1,2,3,4,5,6,7,8,9,0}  → sb-play {1,2,3,4,5,6,7,8,9,0}
super + bracketright           → sb-tab next
super + bracketleft            → sb-tab prev
super + Escape                 → sb-stop
```

Reload with `pkill -USR1 sxhkd` (or start it: `sxhkd &`).

**Wayland caveat:** sxhkd is X11-only. On KDE Plasma Wayland (CachyOS default), bind the same commands in System Settings → Shortcuts → Custom Shortcuts. On Hyprland or Sway, use the compositor's native bind config.

**5. GUI global shortcuts (Rust/Qt)**

The Rust/Qt GUI binds shortcuts via `xdg-desktop-portal`'s `GlobalShortcuts`
interface, not via direct KGlobalAccel D-Bus calls. The full architecture,
testing protocol, cgroup `app_id` gotcha, and list of regressions to never
re-introduce live in [docs/global-shortcuts.md](docs/global-shortcuts.md).
Read that file before touching `src/services/shortcuts/`,
`src/cpp/app_identity.cpp`, or the `bind_shortcuts` flow in `src/main.rs`.
