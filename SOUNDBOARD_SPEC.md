# Sound Spring — Rust/Qt/KDE Application Spec

A PipeWire-routed soundboard with tab-cycling hotkeys, designed for KDE Plasma 6
on Wayland. Number keys 1–0 remap per tab; the same physical key plays different
sounds depending on which tab is active.

This document is the source of truth. Implement everything below; do not add
features that aren't here unless asked.

## Stack

- **Language:** Rust (edition 2021, MSRV 1.78)
- **UI:** Qt 6.7+ via [cxx-qt](https://github.com/KDAB/cxx-qt) 0.7
- **UI markup:** QML with Qt Quick Controls 2, optionally
  [Kirigami](https://invent.kde.org/frameworks/kirigami) for KDE-native widgets
- **D-Bus:** [zbus](https://github.com/dbus2/zbus) 5.x (pure Rust, async)
- **Async runtime:** Tokio 1.x, multi-thread flavor with one worker
- **Audio routing:** PipeWire via `pactl` and `paplay` (shell-out through
  `tokio::process`)
- **Config persistence:** serde + toml (config), serde + serde_json (state)
- **Filesystem watching:** `notify` crate
- **Logging:** `tracing` + `tracing-subscriber`
- **XDG paths:** `directories` crate

Avoid: the legacy `qt` crate (Qt5), `qmetaobject-rs` (less polished than cxx-qt
for new projects), `dbus-rs` (synchronous, awkward API), `keyboard`/X11 crates,
async-std, plain `std::process` for paplay (need async wait).

## Why QML and not QtWidgets

cxx-qt supports both, but QML is the more idiomatic and better-documented path.
For this project specifically: KDE Plasma is QML-heavy so the result feels
native, and local models have seen far more `Item { ... }` QML than
`cxx_qt::QObject` widget construction. The UI uses **Qt Quick Controls 2**
with the Fusion style and a project `SoundSpringTheme` palette (not Kirigami).
The binary-size penalty over widgets is ~3–4 MB, which is acceptable for the
development ergonomics.

## Directory layout

```
~/.config/soundboard/
├── config.toml                  # mic source, latency, theme
└── tabs/
    ├── 01-memes/
    │   ├── 01-airhorn.ogg
    │   ├── 02-bruh.ogg
    │   └── ...
    ├── 02-music/
    │   └── ...
    └── 03-effects/
        └── ...

~/.cache/soundboard/
└── state.json                   # last active tab, window geometry
```

- Tab dirs use `NN-name` prefix for ordering; strip prefix for display.
- Up to 10 sound files per tab; sorted lexically, mapped to slots 1–10.
- Slot 10 is triggered by the `0` key.
- When a tab has more than 10 audio files, show a warning and **ignore** files
  that cannot be placed (prefixes above 10, or excess after filling empty
  slots). Overflow files with valid prefixes (1–10) or no prefix are used to
  **fill empty slots** before any excess is dropped. There is no separate
  “unbound” UI beyond the fixed 10-slot grid.

## Architecture

Two execution contexts in one process:

**Qt thread (main)** — owns the Qt event loop, all `QObject`s, all QML state.
Rust code on this thread runs synchronously inside cxx-qt invokables.

**Tokio worker thread** — owns the async runtime. Runs zbus, file watchers,
and `paplay` process management. Communicates with the Qt thread via
`CxxQtThread::queue()` (Rust → Qt) and `tokio::sync::mpsc` channels (Qt → Rust).

### Qt-side objects (cxx-qt bridges)

- `SoundboardController` — root QObject exposed to QML as `controller`. Holds
  the observable model state: tabs list, current tab index, playing status.
  Invokables: `play_slot(i32)`, `next_tab()`, `prev_tab()`, `stop_all()`,
  `add_tab(QString)`, `rename_tab(...)`. Signals: `tabs_changed()`,
  `current_tab_changed()`, `playback_started(i32)`, `playback_ended(i32)`.
- `Settings` — QObject for the settings page, two-way bound to QML form fields.

### Rust-side services (Tokio thread)

- `pipewire::Manager` — wraps `pactl`. Methods: `setup(mic_source) -> Result<Modules>`,
  `teardown(Modules)`, `available_sources() -> Vec<MicSource>`.
- `shortcuts::Manager` — owns the zbus connection, the portal session, and the
  signal listener task. Method: `bind(shortcuts: &[ShortcutDef]) -> Result<()>`.
  Emits `ShortcutEvent` on an mpsc channel consumed by the Qt side.
- `player::Player` — `tokio::process::Command` wrapping `paplay`. Tracks active
  children in a `HashMap<u64, Child>` keyed by play-id. Methods: `play(file,
  device) -> u64`, `stop(id)`, `stop_all()`.
- `tabs::Repository` — scans `~/.config/soundboard/tabs/`, returns a sorted
  `Vec<Tab>`. Owns a `notify::RecommendedWatcher` that debounces filesystem
  events (300ms) and pushes `TabsChanged` to the Qt side.
- `config::Config` — load/save TOML at `~/.config/soundboard/config.toml`.
- `state::State` — load/save JSON at `~/.cache/soundboard/state.json`,
  debounced 500ms on save.

## UI specification

### MainWindow (QML: `Main.qml`)

- `ApplicationWindow` from QtQuick.Controls, default 800×600, restores
  geometry from state.json. Styled via `SoundSpringTheme` and a global
  `palette`.
- **Header:** tab strip (custom `ListView` delegates), drag-reorder enabled.
  Settings cog opens `SettingsDialog`. Tab navigation buttons (prev/next).
- **Content:** a single `TabPage` bound to the **currently active tab** (not a
  `StackLayout` of one page per tab).
- **Footer:** remote-output and local-monitor volume sliders with mute toggles;
  **Stop All** on the right. Settings live in the header, not the footer.

### TabPage (QML: `TabPage.qml`)

- `GridLayout`, 2 columns × 5 rows, of `SoundButton` items.
- Bound to `controller` slot helpers (`slotLabel`, `slotPlaying`, etc.) for
  the active tab’s 10 slots.

### SoundButton (QML: `SoundButton.qml`)

- `Button`, fixed minimum height 80, large font.
- Layout: slot number badge top-left, filename centered.
- Tooltip shows full path.
- `onClicked: controller.play_slot(slotNumber)` — slot index is 1-based.
- Right-click `MouseArea` opens a context `Menu` with Replace/Remove/Rename/
  Move/Open Folder.
- Empty slots: disabled, "Empty (slot N)" label.
- **Playing indicator:** green progress fill across the button background plus
  accent border (not a separate pulse animation on `playback_started`).

### SettingsDialog (QML: `SettingsDialog.qml`)

- `Dialog`, tabbed pages:
  - **Audio**: mic source `ComboBox` (model from `controller.micSources`),
    latency `Slider` 10–100ms, default 20ms, playback interruption mode,
    mute-mic-during-playback.
  - **Shortcuts**: `ListView` of 15 shortcuts, each row showing the action and
    a key-sequence text field. Backend `portal` (global via
    xdg-desktop-portal) or `local` (in-window only). "Apply" rebinds globals.
    Optional **Ignore NumLock** registers NumLock-off companion keysyms — see
    `docs/global-shortcuts.md`.
  - **General**: switches for launch-at-login, minimize-to-tray, auto-teardown
    of PipeWire modules on quit.

### System tray

- `QSystemTrayIcon` exposed from C++ as `SystemTray`. Menu: Show/Hide, Stop
  All, Quit. Left-click restores the window.

## Hotkey specification

### Default bindings

| Action       | Key                          | Shortcut ID         |
|--------------|------------------------------|---------------------|
| Play slot 1  | Num 1 (`KP_1`)               | `play_1`            |
| ...          | ...                          | ...                 |
| Play slot 9  | Num 9 (`KP_9`)               | `play_9`            |
| Play slot 10 | Num 0 (`KP_0`)               | `play_10`           |
| Next tab     | Ctrl+Num + (`Ctrl+KP_Add`)   | `tab_next`          |
| Prev tab     | Ctrl+Num − (`Ctrl+KP_Subtract`) | `tab_prev`       |
| Stop all     | Ctrl+Num . (`Ctrl+KP_Decimal`) | `stop_all`         |
| Mute output  | Alt+Num + (`Alt+KP_Add`)       | `mute_output`       |
| Mute monitor | Alt+Num − (`Alt+KP_Subtract`) | `mute_monitor`     |

### Registration via xdg-desktop-portal GlobalShortcuts (preferred)

zbus generates the proxy from a trait. The portal's full interface in three
declarations:

```rust
use zbus::{proxy, zvariant::{OwnedObjectPath, Value}};
use std::collections::HashMap;

#[proxy(
    interface = "org.freedesktop.portal.GlobalShortcuts",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait GlobalShortcuts {
    fn create_session(
        &self,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;

    fn bind_shortcuts(
        &self,
        session_handle: &zbus::zvariant::ObjectPath<'_>,
        shortcuts: &[(String, HashMap<&str, Value<'_>>)],
        parent_window: &str,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;

    #[zbus(signal)]
    fn activated(
        &self,
        session_handle: OwnedObjectPath,
        shortcut_id: String,
        timestamp: u64,
        options: HashMap<String, zbus::zvariant::OwnedValue>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    fn shortcuts_changed(
        &self,
        session_handle: OwnedObjectPath,
        shortcuts: Vec<(String, HashMap<String, zbus::zvariant::OwnedValue>)>,
    ) -> zbus::Result<()>;
}
```

That's the entire D-Bus marshaling story. zbus handles `a(sa{sv})` and the
variant dict automatically — no `QDBusArgument` wrestling.

**Flow (in `shortcuts::Manager::bind`):**

1. Connect to session bus: `zbus::Connection::session().await?`.
2. Construct the proxy: `GlobalShortcutsProxy::new(&conn).await?`.
3. Call `create_session` with a `handle_token` (random string) and
   `session_handle_token` (also random) in the options dict. The returned
   path is a Request, not the session — subscribe to the `Response` signal on
   that path to get the actual session handle.
4. Build the shortcuts vector with `description` and `preferred_trigger`
   entries per binding.
5. Call `bind_shortcuts(session, shortcuts, "", HashMap::new())`. Wait for
   the resulting Request's Response.
6. Spawn a task that listens to the `Activated` signal stream:
   `proxy.receive_activated().await?`. For each event, forward a
   `ShortcutEvent::Triggered(id)` over the mpsc channel to the Qt side.

The Qt side gets updates by queueing back onto the Qt thread from inside the
Tokio task:

```rust
// In the Activated signal handler on the Tokio side:
qt_thread.queue(move |controller: Pin<&mut SoundboardController>| {
    controller.handle_shortcut(id);
}).expect("Qt thread alive");
```

### Global shortcuts (xdg-desktop-portal)

Registration uses **xdg-desktop-portal** `GlobalShortcuts` only. Settings
offers `shortcuts.mode = "portal"` (global hotkeys) or `"local"` (in-window
keys only, no portal bind).

**Do not** register shortcuts via direct D-Bus calls to
`org.kde.KGlobalAccel` (`setForeignShortcutKeys`, `doRegister`, etc.). On
Plasma 6 / Wayland, `kglobalacceld` runs inside `kwin_wayland`; malformed
calls can crash the desktop session. Users assign global keys through the
portal **Apply** flow and KDE System Settings. Operational details and test
protocol: `docs/global-shortcuts.md`.

## PipeWire routing

Same shell-out model as the bash scripts. Use `tokio::process::Command`:

```rust
pub async fn load_null_sink(name: &str, description: &str) -> Result<u32> {
    let output = tokio::process::Command::new("pactl")
        .args([
            "load-module", "module-null-sink",
            &format!("sink_name={name}"),
            &format!("sink_properties=device.description={description}"),
        ])
        .output()
        .await?;
    if !output.status.success() {
        return Err(anyhow!("pactl failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    let id: u32 = std::str::from_utf8(&output.stdout)?.trim().parse()?;
    Ok(id)
}
```

### Setup sequence (run on first launch, idempotent)

1. List existing sinks (`pactl list short sinks`) and skip any step whose sink
   already exists.
2. Load null sink `soundboard_virtmic` with description
   `Sound-Spring-Mix` (internal mix bus; appears under Speakers).
3. Load null sink `soundboard_sfx` with description `Sound-Spring-Effects`.
4. Loopback `soundboard_sfx.monitor` → `soundboard_virtmic`, latency 20ms.
5. Loopback `<real_mic>` → `soundboard_virtmic`, latency 20ms.
6. Remap `soundboard_virtmic.monitor` → source `sound_spring_virtual_mic` with
   description `Sound-Spring-Virtual-Microphone` (appears under Microphone).
7. Store module IDs in `pipewire::Modules` for teardown.

### Playback

```rust
pub async fn play(&self, file: PathBuf) -> Result<u64> {
    let id = self.next_id();
    let mut child = tokio::process::Command::new("paplay")
        .arg("--device=soundboard_sfx")
        .arg(&file)
        .spawn()?;
    self.children.lock().await.insert(id, child);

    let children = self.children.clone();
    let qt_thread = self.qt_thread.clone();
    tokio::spawn(async move {
        if let Some(child) = children.lock().await.get_mut(&id) {
            let _ = child.wait().await;
        }
        children.lock().await.remove(&id);
        let _ = qt_thread.queue(move |c| c.playback_ended(id as i32));
    });
    Ok(id)
}
```

For "Stop All", iterate the children map and call `kill().await` on each.

### Teardown

On window close (if not minimizing to tray) or quit menu, unload the four
modules in reverse order via `pactl unload-module <id>`. Hook this from the
Qt `aboutToQuit` signal exposed through cxx-qt.

## State management

### config.toml

```toml
[audio]
mic_source = "alsa_input.usb-Blue_Microphones_Yeti_..."
latency_ms = 20
auto_teardown = true

[paths]
tabs_root = "/home/user/.config/soundboard/tabs"
state_dir = "/home/user/.cache/soundboard"

[[tabs]]
path = "/home/user/Music/memes"
name = "Memes"

[shortcuts]
mode = "portal"  # "portal" = global via xdg-desktop-portal; "local" = in-window only
ignore_numlock = false

[ui]
minimize_to_tray = true
launch_at_login = false
```

When `[[tabs]]` entries exist, only those folder paths are tabs (GUI folder picker writes here). Otherwise subdirs of `tabs_root` are scanned. `state.json` stores the active tab as an absolute path.

### state.json

```json
{
  "current_tab": "/home/user/Music/memes",
  "window_geometry": { "x": 100, "y": 100, "width": 800, "height": 600 },
  "last_session": "2026-05-22T14:32:11Z"
}
```

Both via serde with `#[derive(Serialize, Deserialize)]`. Default values via
`impl Default`. Persist on meaningful changes, debounced via a Tokio
`tokio::time::sleep` task that resets on each write request.

## File watching

`notify::RecommendedWatcher` watching `~/.config/soundboard/tabs/` recursively.
Send `EventKind::Create/Modify/Remove` over a channel. Debounce 300ms in a
Tokio task before notifying the Qt side. On change, `tabs::Repository` rescans
and pushes a fresh `Vec<Tab>` to the controller via `qt_thread.queue()`.

## cxx-qt bridge pattern

Local models often have stale or incomplete knowledge of cxx-qt. The shape of
a bridge module in 0.7:

```rust
// src/qobjects/controller.rs
use cxx_qt_lib::QString;
use std::pin::Pin;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, current_tab_index)]
        #[qproperty(QString, current_tab_name)]
        type SoundboardController = super::SoundboardControllerRust;

        #[qinvokable]
        fn play_slot(self: Pin<&mut SoundboardController>, slot: i32);

        #[qinvokable]
        fn next_tab(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn stop_all(self: Pin<&mut SoundboardController>);

        #[qsignal]
        fn playback_started(self: Pin<&mut SoundboardController>, slot: i32);

        #[qsignal]
        fn playback_ended(self: Pin<&mut SoundboardController>, slot: i32);
    }
}

#[derive(Default)]
pub struct SoundboardControllerRust {
    current_tab_index: i32,
    current_tab_name: QString,
    // Rust-only state, not exposed to QML:
    tabs: Vec<super::tabs::Tab>,
    player_tx: Option<tokio::sync::mpsc::Sender<super::services::player::Command>>,
}

impl qobject::SoundboardController {
    pub fn play_slot(mut self: Pin<&mut Self>, slot: i32) {
        let rust = self.as_mut().rust_mut();
        let Some(tab) = rust.tabs.get(rust.current_tab_index as usize) else { return };
        let Some(file) = tab.slot(slot as usize) else { return };
        if let Some(tx) = &rust.player_tx {
            let _ = tx.try_send(super::services::player::Command::Play(file.clone()));
        }
        self.playback_started(slot);
    }
    // ... rest of the methods
}
```

The build script registers QML types and compiles the C++ glue. CMake handles
Qt linking.

## Build setup

### Cargo.toml

```toml
[package]
name = "soundboard"
version = "0.1.0"
edition = "2021"

[dependencies]
cxx = "1.0"
cxx-qt = "0.7"
cxx-qt-lib = { version = "0.7", features = ["full"] }
zbus = { version = "5", default-features = false, features = ["tokio"] }
tokio = { version = "1", features = ["rt-multi-thread", "process", "macros", "sync", "time", "fs"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
serde_json = "1"
notify = "6"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
directories = "5"
thiserror = "1"
anyhow = "1"

[build-dependencies]
cxx-qt-build = { version = "0.7", features = ["qt_qml"] }
```

### build.rs

```rust
use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new()
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .qml_module(QmlModule {
            uri: "com.benkahn.soundboard",
            rust_files: &[
                "src/qobjects/controller.rs",
                "src/qobjects/settings.rs",
            ],
            qml_files: &[
                "qml/Main.qml",
                "qml/TabPage.qml",
                "qml/SoundButton.qml",
                "qml/SettingsDialog.qml",
            ],
            ..Default::default()
        })
        .build();
}
```

## Project structure

```
soundboard/
├── Cargo.toml
├── build.rs
├── README.md
├── src/
│   ├── main.rs                 # Tokio runtime + Qt event loop bootstrap
│   ├── qobjects/
│   │   ├── mod.rs
│   │   ├── controller.rs       # SoundboardController bridge
│   │   └── settings.rs         # Settings bridge
│   ├── services/
│   │   ├── mod.rs
│   │   ├── pipewire.rs
│   │   ├── shortcuts.rs        # zbus proxy + portal logic
│   │   ├── player.rs
│   │   └── tabs.rs
│   ├── config.rs
│   └── state.rs
├── qml/
│   ├── Main.qml
│   ├── TabPage.qml
│   ├── SoundButton.qml
│   ├── SettingsDialog.qml
│   └── qmldir
└── resources/
    ├── icons/
    └── soundboard.desktop
```

## Implementation notes for code generation

- **Threading discipline**: any access to cxx-qt objects from non-Qt threads
  goes through `CxxQtThread::queue`. Channels carry plain data (`String`,
  `u64`, structs); no `QObject` references cross the boundary.
- **Async on the Tokio side, sync on the Qt side**: Qt invokables are
  synchronous. They post commands to Tokio via `mpsc::Sender` and return
  immediately. Tokio replies via `CxxQtThread::queue` to update Qt state.
- **`paplay` interruption policy**: default "overlap" (multiple instances allowed
  per slot). "Interrupt" mode kills the previous before spawning. Setting in
  config.toml.
- **Shortcut binding is async-only**: do it from `tokio::spawn` at startup,
  *after* the main window has shown. Wayland portals sometimes reject early
  calls.
- **Tab cycling wraps**: next from last → first, prev from first → last.
- **Mute real mic during sound playback**: optional setting. If on, set source
  mute on first play, restore on last play end. Track active count atomically
  (`AtomicUsize`).
- **`QString` interop**: convert with `QString::from(&str)` and `String::from(&qstring)`.
  Don't use Rust `&str` directly in QML properties.
- **Logging**: initialize `tracing_subscriber` with `EnvFilter` from `RUST_LOG`
  in `main` before starting Tokio or Qt.
- **Error handling**: `anyhow::Result` at boundaries, custom `thiserror` enums
  inside modules. Don't panic from the Tokio side — log and continue.

## Acceptance criteria

The implementation is done when all of these hold:

1. `cargo build --release` produces a single stripped binary in
   `target/release/` under 15 MB. `Cargo.toml` sets `[profile.release] strip =
   true`. As of 2026-06, a typical build is ~7 MB on x86_64 Linux.
2. Launching the app creates the two null sinks if absent, with correct
   descriptions visible in `pavucontrol`.
3. Audio played through any sound button is audible on
   **Sound-Spring-Virtual-Microphone** in Discord/OBS.
4. Real mic audio also routes to the virtual microphone input.
5. Pressing Num 1 from any focused window plays the slot 1 sound of the
   currently active tab.
6. Pressing Ctrl+Num + cycles to the next tab; Num 1 now plays the slot 1 sound
   of the new tab.
7. The first run triggers a KDE portal dialog confirming the shortcut
   bindings; subsequent runs do not.
8. Stop All halts all currently playing sounds within 250ms.
9. Adding a file to a tab directory makes it appear in the UI within 1 second.
10. Closing the window minimizes to tray (if setting enabled); quitting from
    tray menu unloads the PipeWire modules and exits cleanly.
11. The app survives `pipewire` being restarted: re-creates sinks on the next
    play attempt.
12. Startup time from process start to first main-window frame is under 200 ms
    on the Thelio Mira (release build, launched via `gtk-launch sound-spring`).
    Log line: `startup: first frame in N ms` at `sound_spring=info`.

### Release build and startup check

```bash
source "$HOME/.cargo/env"
QMAKE=/usr/bin/qmake6 cargo build --release
ls -lh target/release/sound-spring    # expect < 15 MB, stripped

# Launch outside IDE/Electron cgroups (see docs/global-shortcuts.md):
RUST_LOG=sound_spring=info gtk-launch sound-spring
# Read "startup: first frame in … ms" in the terminal or journal.
```

## What to hand the model first

1. This spec (entire file).
2. The bash scripts (`sb-play`, `sb-tab`, `sb-stop`) as semantic reference for
   the routing/playback behavior.
3. The cxx-qt book chapter on QML integration:
   <https://kdab.github.io/cxx-qt/book/getting-started/index.html> — local
   models often have stale or incomplete knowledge of cxx-qt's API surface,
   and this page covers the bridge syntax in 0.7+.
4. The zbus book section on proxies:
   <https://dbus2.github.io/zbus/client.html> — same reason.

Scaffold order: `config.rs` → `state.rs` → `services/pipewire.rs` →
`services/player.rs` → `services/tabs.rs` → `services/shortcuts.rs` →
`qobjects/controller.rs` → `qml/Main.qml` → `qml/SoundButton.qml` →
remaining QML → `main.rs`.

Have the model commit after each module compiles. Local models drift badly
when asked to write multiple compilation units in one pass; the type system
will catch most drift on rebuild, but only if rebuilds happen frequently.
