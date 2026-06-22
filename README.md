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

## GUI application

The Rust/Qt GUI shares config and state with the bash layer (`config.toml`, `state.json`). See [`SOUNDBOARD_SPEC.md`](SOUNDBOARD_SPEC.md) for the soundboard specification — **Phase 1 implementation is complete** (all acceptance criteria met). The **Voice** enhancement panel ([`SOUNDBOARD_SPEC_PHASE2.md`](SOUNDBOARD_SPEC_PHASE2.md)) is also complete: live spectrum, Silero VAD, ECAPA speaker verification and enrollment, DeepFilterNet3 noise suppression, and routed output to the virtual mic.

**Requirements:** Rust (rustup), Qt 6 (`qmake6`), PipeWire, `paplay`, and optionally `ffmpeg` for MP3.

```bash
source "$HOME/.cargo/env"
QMAKE=/usr/bin/qmake6 cargo build --release
ls -lh target/release/sound-spring   # stripped; expect well under 15 MB
RUST_LOG=sound_spring=info ./target/release/sound-spring
```

On first launch, the log line `startup: first frame in N ms` reports time from
process start to main-window `Component.onCompleted` (acceptance target: < 200
ms on the Thelio Mira, release build, launched via `gtk-launch sound-spring`).

Global shortcuts use **xdg-desktop-portal** (`shortcuts.mode = "portal"` or `"auto"` in config). They are **not** registered at launch — open **Settings → Shortcuts** and click **Apply** to bind globals with KDE. You may see a permission dialog the first time. In-window numpad keys work immediately without Apply.

Direct KGlobalAccel D-Bus calls are intentionally avoided. On Plasma 6 / Wayland the `org.kde.kglobalaccel` service is hosted inside `kwin_wayland` itself, so a malformed call can crash the entire desktop session.

### Testing global shortcuts — must run outside Cursor / Electron / Chromium

`xdg-desktop-portal` identifies the calling application by walking the
caller's systemd cgroup scope. If Sound Spring is launched from a terminal
embedded inside another desktop app (Cursor IDE, VS Code, Chromium-based
browsers, any Electron shell), portal-kde sees the **parent app's** `app_id`
(e.g. `org.chromium.Chromium`) and shares its already-bound portal session.
The portal then returns 15 shortcuts with empty `trigger_description` in
~10 ms, no assignment dialog appears, and no `[sound-spring]` section is
ever written to `~/.config/kglobalshortcutsrc`.

To test global shortcuts, launch the binary so it lands in a **top-level
`app-sound-spring-*.scope`** under `app.slice`, not nested inside a parent
terminal's scope. `systemd-run --user --scope` from inside another desktop
app's terminal creates a *child* scope of that app — portal-kde then walks
up and resolves the first `app-*.scope` it finds, which is still the
parent.

These launchers go through the session bus and create a proper top-level
scope:

```bash
# Recommended — uses the installed .desktop file via GIO:
gtk-launch sound-spring

# Or via gio launch:
gio launch ~/.local/share/applications/sound-spring.desktop

# Or from KRunner / the app menu (Alt+Space → "Sound Spring")
```

These do **not** escape the parent scope and will report the wrong
`app_id` if launched from Konsole/Cursor/VS Code:

```bash
# Wrong — inherits parent app's cgroup, app_id resolves to parent
./target/release/sound-spring
systemd-run --user --scope --collect ./target/release/sound-spring

# Only safe from a TTY (Ctrl+Alt+F2) or a terminal that itself runs in
# its own top-level app.scope and has no app-*.scope ancestor.
```

When launched correctly, the journal will show
`xdg-desktop-portal-kde[...]: CreateSession ... app_id: "sound-spring"`, the
BindShortcuts dialog will take seconds to complete (not 10 ms), and a
`[sound-spring]` section will land in `~/.config/kglobalshortcutsrc`. The
app will then appear in **System Settings → Shortcuts** for further editing.

See [docs/global-shortcuts.md](docs/global-shortcuts.md) for the full
diagnostic protocol, things that look like fixes but make it worse, and the
list of architectural constraints any change to the shortcut path must
respect.

Open **Settings** (⚙ in the header) to change mic source, paths, custom tab folders, and shortcut bindings.

See [SOUNDBOARD_SPEC.md](SOUNDBOARD_SPEC.md) for the soundboard specification and [SOUNDBOARD_SPEC_PHASE2.md](SOUNDBOARD_SPEC_PHASE2.md) for the Voice enhancement panel.

## See also

- [PROJECT.md](PROJECT.md) — bash layer architecture and manual PipeWire commands
- [SOUNDBOARD_SPEC.md](SOUNDBOARD_SPEC.md) — soundboard application specification
- [SOUNDBOARD_SPEC_PHASE2.md](SOUNDBOARD_SPEC_PHASE2.md) — Voice enhancement panel specification
