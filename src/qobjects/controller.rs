#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, current_tab_index)]
        #[qproperty(QString, current_tab_name)]
        #[qproperty(i32, tab_count)]
        #[qproperty(i32, playing_version)]
        #[qproperty(i32, mic_source_count)]
        #[qproperty(i32, mic_sources_version)]
        #[qproperty(i32, audio_sink_count)]
        #[qproperty(i32, audio_sinks_version)]
        #[qproperty(i32, tab_version)]
        #[qproperty(i32, progress_version)]
        #[qproperty(i32, shortcut_version)]
        #[qproperty(i32, ui_version)]
        #[qproperty(i32, output_volume)]
        #[qproperty(i32, monitor_volume)]
        #[qproperty(i32, mic_volume)]
        #[qproperty(bool, output_muted)]
        #[qproperty(bool, monitor_muted)]
        #[qproperty(bool, mic_muted)]
        #[qproperty(QString, global_shortcuts_status)]
        #[qproperty(QString, tab_warning)]
        type SoundboardController = super::SoundboardControllerRust;

        #[qinvokable]
        fn play_slot(self: Pin<&mut SoundboardController>, slot: i32);

        #[qinvokable]
        fn next_tab(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn prev_tab(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn select_tab(self: Pin<&mut SoundboardController>, index: i32);

        #[qinvokable]
        fn stop_all(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn invoke_shortcut(self: Pin<&mut SoundboardController>, id: QString);

        #[qinvokable]
        fn handle_key_event(
            self: Pin<&mut SoundboardController>,
            key: i32,
            modifiers: i32,
            native_scan_code: u32,
        ) -> bool;

        #[qinvokable]
        fn update_output_volume(self: Pin<&mut SoundboardController>, volume: i32);

        #[qinvokable]
        fn update_monitor_volume(self: Pin<&mut SoundboardController>, volume: i32);

        #[qinvokable]
        fn toggle_output_mute(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn toggle_monitor_mute(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn update_mic_volume(self: Pin<&mut SoundboardController>, volume: i32);

        #[qinvokable]
        fn toggle_mic_mute(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn set_playback_keys_enabled(self: Pin<&mut SoundboardController>, enabled: bool);

        #[qinvokable]
        fn set_window_active(self: Pin<&mut SoundboardController>, active: bool);

        #[qinvokable]
        fn refresh_portal_parent_window(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn bind_global_shortcuts(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn configure_global_shortcuts(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn refresh_shortcut_bindings(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn process_events(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn note_first_paint(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn tab_name_at(self: &SoundboardController, index: i32) -> QString;

        #[qinvokable]
        fn tab_path_at(self: &SoundboardController, index: i32) -> QString;

        #[qinvokable]
        fn tab_uses_custom_list(self: &SoundboardController) -> bool;

        #[qinvokable]
        fn add_tab(self: Pin<&mut SoundboardController>, path: QString, name: QString) -> bool;

        #[qinvokable]
        fn rename_tab(self: Pin<&mut SoundboardController>, index: i32, name: QString) -> bool;

        #[qinvokable]
        fn move_tab(self: Pin<&mut SoundboardController>, from_index: i32, to_index: i32) -> bool;

        #[qinvokable]
        fn remove_tab(self: Pin<&mut SoundboardController>, index: i32) -> bool;

        #[qinvokable]
        fn slot_label(self: &SoundboardController, slot: i32) -> QString;

        #[qinvokable]
        fn slot_path_at(self: &SoundboardController, slot: i32) -> QString;

        #[qinvokable]
        fn slot_empty(self: &SoundboardController, slot: i32) -> bool;

        #[qinvokable]
        fn replace_slot(self: Pin<&mut SoundboardController>, slot: i32, path: QString) -> bool;

        #[qinvokable]
        fn remove_slot(self: Pin<&mut SoundboardController>, slot: i32) -> bool;

        #[qinvokable]
        fn rename_slot(self: Pin<&mut SoundboardController>, slot: i32, name: QString) -> bool;

        #[qinvokable]
        fn move_slot(self: Pin<&mut SoundboardController>, from_slot: i32, to_slot: i32) -> bool;

        #[qinvokable]
        fn open_tab_folder(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn slot_playing(self: &SoundboardController, slot: i32) -> bool;

        #[qinvokable]
        fn slot_shortcut_label(self: &SoundboardController, slot: i32) -> QString;

        #[qinvokable]
        fn slot_progress(self: &SoundboardController, slot: i32) -> f64;

        #[qinvokable]
        fn shortcut_sequence(self: &SoundboardController, id: QString) -> QString;

        #[qinvokable]
        fn mic_source_name_at(self: &SoundboardController, index: i32) -> QString;

        #[qinvokable]
        fn mic_source_id_at(self: &SoundboardController, index: i32) -> QString;

        #[qinvokable]
        fn mic_source_description_at(self: &SoundboardController, index: i32) -> QString;

        #[qinvokable]
        fn refresh_mic_sources(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn audio_sink_id_at(self: &SoundboardController, index: i32) -> QString;

        #[qinvokable]
        fn audio_sink_description_at(self: &SoundboardController, index: i32) -> QString;

        #[qinvokable]
        fn refresh_audio_devices(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn reload_from_config(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn sync_global_shortcuts_status(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn refresh_global_shortcuts_status(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn needs_global_shortcut_apply(self: &SoundboardController) -> bool;

        #[qinvokable]
        fn dismiss_global_shortcuts_prompt(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn needs_close_action_prompt(self: &SoundboardController) -> bool;

        #[qinvokable]
        fn apply_close_action_choice(
            self: Pin<&mut SoundboardController>,
            minimize_to_tray: bool,
            remember: bool,
        );

        #[qinvokable]
        fn has_saved_window_geometry(self: &SoundboardController) -> bool;

        #[qinvokable]
        fn saved_window_x(self: &SoundboardController) -> i32;

        #[qinvokable]
        fn saved_window_y(self: &SoundboardController) -> i32;

        #[qinvokable]
        fn saved_window_width(self: &SoundboardController) -> i32;

        #[qinvokable]
        fn saved_window_height(self: &SoundboardController) -> i32;

        #[qinvokable]
        fn save_window_geometry(
            self: Pin<&mut SoundboardController>,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
        );

        #[qinvokable]
        fn save_session_on_quit(
            self: Pin<&mut SoundboardController>,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
        );

        #[qinvokable]
        fn shutdown_backend(self: Pin<&mut SoundboardController>);

        #[qsignal]
        fn tabs_changed(self: Pin<&mut SoundboardController>);

        #[qsignal]
        fn current_tab_changed(self: Pin<&mut SoundboardController>);

        #[qsignal]
        fn playback_started(self: Pin<&mut SoundboardController>, slot: i32);

        #[qsignal]
        fn playback_ended(self: Pin<&mut SoundboardController>, slot: i32);

        #[qsignal]
        fn playing_state_changed(self: Pin<&mut SoundboardController>);
    }

    impl cxx_qt::Constructor<()> for SoundboardController {}
}

use core::pin::Pin;
use cxx_qt::{Constructor, CxxQtType};
use cxx_qt_lib::QString;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Receiver as StdReceiver, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender as TokioSender;

extern "C" {
    fn sound_spring_refresh_portal_parent_window();
}

use crate::config::{self, Config, TabEntry};
use crate::services::pipewire::{AudioSink, MicSource};
use crate::services::player::{PlayerCommand, VolumeState};
use crate::services::shortcuts::{
    accept_shortcut, format_global_shortcut_status, global_shortcut_status, play_slot_from_qt_key,
    qt_shortcut_sequence, trigger_display, trigger_from_qt, GlobalShortcutStatus, ShortcutDef,
    ShortcutsManager,
};
use crate::services::tabs::{
    normalize_slot, tab_name_from_path, uses_custom_tabs, Tab, TabsRepository,
};
use crate::state::{State, WindowGeometry};

#[derive(Debug)]
pub enum BackendCommand {
    ApplyConfig(Box<Config>),
    BindShortcuts,
    ConfigurePortalShortcuts,
    Player(PlayerCommand),
    RefreshMicSources,
    RefreshAudioSinks,
    RestartTabWatch,
    ApplyVolumes(VolumeState),
    StartVoiceCapture,
    StopVoiceCapture,
    SetVoiceVerification { enabled: bool, threshold: f32 },
    SetVoiceSuppression { enabled: bool },
    SetVoiceVad { enabled: bool },
    SetMicVolume { percent: u8, muted: bool },
    SetSpectrumSource { source: String },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum BackendEvent {
    PlaybackEnded {
        tab_index: i32,
        slot: i32,
    },
    ShortcutTriggered {
        id: String,
    },
    GlobalShortcutStatusChanged,
    ConfigApplied,
    TabsRescanned {
        tabs: Vec<crate::services::tabs::Tab>,
    },
    MicSourcesUpdated,
    AudioSinksUpdated,
    VoiceCaptureStatus {
        active: bool,
        error: String,
    },
}

pub static BACKEND_TX: OnceLock<TokioSender<BackendCommand>> = OnceLock::new();
pub static BACKEND_EVENT_RX: OnceLock<Mutex<StdReceiver<BackendEvent>>> = OnceLock::new();

pub static MIC_SOURCES: OnceLock<Mutex<Vec<MicSource>>> = OnceLock::new();
pub static AUDIO_SINKS: OnceLock<Mutex<Vec<AudioSink>>> = OnceLock::new();
pub static SHORTCUT_BINDINGS: OnceLock<Mutex<Vec<ShortcutDef>>> = OnceLock::new();
pub static WINDOW_ACTIVE: AtomicBool = AtomicBool::new(true);

const KEY_DEDUPE_MS: u64 = 120;
const MIN_PLAY_BEFORE_TOGGLE_MS: u64 = 300;

struct ActivePlayback {
    started: Instant,
    duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SessionKey {
    tab_index: i32,
    slot: i32,
}

#[derive(Debug, Clone, Copy)]
struct PlayCoalesce {
    tab_index: i32,
    slot: i32,
    at: Instant,
}

// Compatibility with existing playback wiring.
pub static PLAYER_TX: OnceLock<TokioSender<PlayerCommand>> = OnceLock::new();

#[derive(Default)]
pub struct SoundboardControllerRust {
    current_tab_index: i32,
    current_tab_name: QString,
    tab_count: i32,
    playing_version: i32,
    mic_source_count: i32,
    mic_sources_version: i32,
    audio_sink_count: i32,
    audio_sinks_version: i32,
    tab_version: i32,
    progress_version: i32,
    shortcut_version: i32,
    ui_version: i32,
    output_volume: i32,
    monitor_volume: i32,
    mic_volume: i32,
    output_muted: bool,
    monitor_muted: bool,
    mic_muted: bool,
    global_shortcuts_status: QString,
    tab_warning: QString,
    tabs: Vec<Tab>,
    active_playbacks: HashMap<SessionKey, ActivePlayback>,
    play_coalesce: Option<PlayCoalesce>,
    duration_cache: HashMap<PathBuf, u64>,
    tabs_root: PathBuf,
    state_path: PathBuf,
    window_geometry: Option<WindowGeometry>,
}

#[derive(Debug, Default)]
struct KeyEventResult {
    handled: bool,
    tab_changed: bool,
    playback_changed: bool,
    mute_changed: bool,
}

#[derive(Debug, Default)]
struct ShortcutHandleResult {
    tab_changed: bool,
    mute_changed: bool,
}

impl SoundboardControllerRust {
    fn send_player_command(&self, command: PlayerCommand) {
        if let Some(tx) = BACKEND_TX.get() {
            let _ = tx.blocking_send(BackendCommand::Player(command));
        } else if let Some(tx) = PLAYER_TX.get() {
            let _ = tx.blocking_send(command);
        }
    }

    fn volume_state(&self) -> VolumeState {
        VolumeState {
            output_percent: self.output_volume.clamp(0, 100) as u8,
            monitor_percent: self.monitor_volume.clamp(0, 100) as u8,
            output_muted: self.output_muted,
            monitor_muted: self.monitor_muted,
        }
    }

    fn interruption_mode_enabled(&self) -> bool {
        crate::config::load_config()
            .map(|config| config.audio.interruption_mode == "interrupt")
            .unwrap_or(false)
    }

    fn apply_volume_state(&mut self, state: VolumeState) {
        self.output_volume = state.output_percent as i32;
        self.monitor_volume = state.monitor_percent as i32;
        self.output_muted = state.output_muted;
        self.monitor_muted = state.monitor_muted;
    }

    fn push_volumes(&self) {
        if let Some(tx) = BACKEND_TX.get() {
            if tx
                .blocking_send(BackendCommand::ApplyVolumes(self.volume_state()))
                .is_err()
            {
                tracing::warn!("backend volume channel closed, dropping volume update");
            }
        }
    }

    fn persist_volumes(&self) {
        let output_volume = self.output_volume.clamp(0, 100) as u8;
        let monitor_volume = self.monitor_volume.clamp(0, 100) as u8;
        let mic_volume = self.mic_volume.clamp(0, 100) as u8;
        let output_muted = self.output_muted;
        let monitor_muted = self.monitor_muted;
        let mic_muted = self.mic_muted;
        std::thread::spawn(move || {
            let mut config = crate::config::load_config().unwrap_or_default();
            config.audio.output_volume = output_volume;
            config.audio.monitor_volume = monitor_volume;
            config.audio.mic_volume = mic_volume;
            config.audio.output_muted = output_muted;
            config.audio.monitor_muted = monitor_muted;
            config.audio.mic_muted = mic_muted;
            let _ = crate::config::save_config(&config);
        });
    }

    fn push_mic_volume(&self) {
        if let Some(tx) = BACKEND_TX.get() {
            let _ = tx.blocking_send(BackendCommand::SetMicVolume {
                percent: self.mic_volume.clamp(0, 100) as u8,
                muted: self.mic_muted,
            });
        }
    }

    pub fn replace_tabs(&mut self, tabs: Vec<Tab>, current_path: Option<&str>) {
        self.tabs = tabs;
        self.tab_count = self.tabs.len() as i32;
        if self.tabs.is_empty() {
            self.current_tab_index = 0;
            self.current_tab_name = QString::from("");
            return;
        }
        let current = current_path.unwrap_or_default();
        if let Some(tab) = TabsRepository::resolve_current_tab(&self.tabs, current, &self.tabs_root)
        {
            if let Some(index) = self.tabs.iter().position(|t| t.path == tab.path) {
                self.current_tab_index = index as i32;
            }
        } else if self.current_tab_index as usize >= self.tabs.len() {
            self.current_tab_index = 0;
        }
        self.current_tab_name =
            QString::from(self.active_tab().map(|t| t.display_name()).unwrap_or(""));
        self.tab_version += 1;
    }

    fn rescan_tabs(&mut self, config: &Config, current_path: Option<&str>) {
        match TabsRepository::scan(config) {
            Ok(tabs) => {
                tracing::debug!("rescanned {} tab(s)", tabs.len());
                self.replace_tabs(tabs, current_path);
            }
            Err(err) => tracing::warn!("tab rescan failed: {err:#}"),
        }
        self.set_tab_warning(&Self::collect_tab_warnings(config));
    }

    fn request_tab_watch_restart() {
        if let Some(tx) = BACKEND_TX.get() {
            let _ = tx.blocking_send(BackendCommand::RestartTabWatch);
        }
    }

    fn finish_tab_mutation(
        &mut self,
        config: &Config,
        select_path: Option<&Path>,
        watch_restart: bool,
    ) -> bool {
        if let Err(err) = config::save_config(config) {
            tracing::warn!("failed to save tab config: {err:#}");
            return false;
        }
        self.tabs_root = config.paths.tabs_root.clone();
        let select = select_path
            .map(|path| path.to_string_lossy().into_owned())
            .or_else(|| {
                self.active_tab()
                    .map(|tab| tab.path.to_string_lossy().into_owned())
            });
        match TabsRepository::scan(config) {
            Ok(tabs) => {
                self.replace_tabs(tabs, select.as_deref());
                self.set_tab_warning(&Self::collect_tab_warnings(config));
            }
            Err(err) => {
                tracing::warn!("tab rescan failed: {err:#}");
                return false;
            }
        }
        if watch_restart {
            Self::request_tab_watch_restart();
        }
        true
    }

    fn config_index_for_tab_path(config: &Config, path: &Path) -> Option<usize> {
        config.tabs.iter().position(|entry| entry.path == path)
    }

    fn refresh_active_tab_slots(&mut self) {
        let config = config::load_config().unwrap_or_default();
        let current = self
            .active_tab()
            .map(|tab| tab.path.to_string_lossy().into_owned());
        self.rescan_tabs(&config, current.as_deref());
    }

    fn set_tab_warning(&mut self, messages: &[String]) {
        let warning = if messages.is_empty() {
            String::new()
        } else {
            messages.join("\n")
        };
        self.tab_warning = QString::from(warning.as_str());
    }

    fn collect_tab_warnings(config: &Config) -> Vec<String> {
        let mut warnings = Vec::new();
        let tab_paths = if !config.tabs.is_empty() {
            config
                .tabs
                .iter()
                .filter(|entry| entry.path.is_dir())
                .map(|entry| entry.path.clone())
                .collect::<Vec<_>>()
        } else if config.paths.tabs_root.is_dir() {
            TabsRepository::tab_paths(config).unwrap_or_default()
        } else {
            Vec::new()
        };
        for path in tab_paths {
            match TabsRepository::scan_tab_dir_with_warnings(&path) {
                Ok((_, mut tab_warnings)) => warnings.append(&mut tab_warnings),
                Err(err) => warnings.push(format!("Failed to scan {}: {err:#}", path.display())),
            }
        }
        warnings
    }

    fn slot_path_internal(&self, slot: i32) -> Option<PathBuf> {
        let tab = self.active_tab()?;
        let index = normalize_slot(slot)?;
        tab.slot(index).cloned()
    }

    fn refresh_mic_source_count(&mut self) {
        self.mic_source_count = MIC_SOURCES
            .get()
            .and_then(|sources| sources.lock().ok())
            .map(|sources| sources.len() as i32)
            .unwrap_or(0);
        self.mic_sources_version += 1;
    }

    fn mic_source_at(&self, index: i32) -> Option<MicSource> {
        MIC_SOURCES.get().and_then(|store| {
            store
                .lock()
                .ok()
                .and_then(|sources| sources.get(index as usize).cloned())
        })
    }

    fn refresh_audio_sink_count(&mut self) {
        self.audio_sink_count = AUDIO_SINKS
            .get()
            .and_then(|sinks| sinks.lock().ok())
            .map(|sinks| sinks.len() as i32)
            .unwrap_or(0);
        self.audio_sinks_version += 1;
    }

    fn audio_sink_at(&self, index: i32) -> Option<AudioSink> {
        AUDIO_SINKS.get().and_then(|store| {
            store
                .lock()
                .ok()
                .and_then(|sinks| sinks.get(index as usize).cloned())
        })
    }

    fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.current_tab_index as usize)
    }

    fn apply_tab_index(&mut self, index: i32) {
        if self.tabs.is_empty() || index < 0 || index as usize >= self.tabs.len() {
            return;
        }
        self.current_tab_index = index;
        self.current_tab_name =
            QString::from(self.active_tab().map(|t| t.display_name()).unwrap_or(""));
        self.persist_current_tab();
        self.tab_version += 1;
    }

    fn persist_current_tab(&self) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let path = self.state_path.clone();
        let current_tab = tab.path.to_string_lossy().into_owned();
        std::thread::spawn(move || {
            let mut state = State::load(&path).unwrap_or_default();
            state.current_tab = current_tab;
            if let Err(err) = state.save(&path) {
                tracing::warn!("failed to save state: {err:#}");
            }
        });
    }

    fn persist_window_geometry(&self, geometry: WindowGeometry) {
        let path = self.state_path.clone();
        std::thread::spawn(move || {
            let mut state = State::load(&path).unwrap_or_default();
            state.window_geometry = Some(geometry);
            if let Err(err) = state.save(&path) {
                tracing::warn!("failed to save window geometry: {err:#}");
            }
        });
    }

    fn persist_session_on_quit(&self, geometry: WindowGeometry) {
        let path = self.state_path.clone();
        std::thread::spawn(move || {
            let mut state = State::load(&path).unwrap_or_default();
            state.window_geometry = Some(geometry);
            state.last_session = Some(State::utc_now_rfc3339());
            if let Err(err) = state.save(&path) {
                tracing::warn!("failed to save session on quit: {err:#}");
            }
        });
    }

    fn toggle_output_mute_internal(&mut self) {
        self.output_muted = !self.output_muted;
        self.persist_volumes();
        self.push_volumes();
    }

    fn toggle_monitor_mute_internal(&mut self) {
        self.monitor_muted = !self.monitor_muted;
        self.persist_volumes();
        self.push_volumes();
    }

    fn toggle_mic_mute_internal(&mut self) {
        self.mic_muted = !self.mic_muted;
        self.persist_volumes();
        self.push_mic_volume();
    }

    fn handle_key_event_internal(
        &mut self,
        key: i32,
        modifiers: i32,
        native_scan_code: u32,
    ) -> KeyEventResult {
        if let Some(trigger) = trigger_from_qt(key, modifiers, native_scan_code) {
            if let Some(id) = self.shortcut_id_for_trigger(trigger.as_str()) {
                let result = self.handle_shortcut_id(&id);
                return KeyEventResult {
                    handled: true,
                    tab_changed: result.tab_changed,
                    playback_changed: !result.tab_changed && !result.mute_changed,
                    mute_changed: result.mute_changed,
                };
            }
        }

        if let Some(slot) = play_slot_from_qt_key(key, modifiers, native_scan_code) {
            let id = format!("play_{slot}");
            if !accept_shortcut(&id) {
                return KeyEventResult::default();
            }
            self.play_slot_internal(slot);
            return KeyEventResult {
                handled: true,
                tab_changed: false,
                playback_changed: true,
                mute_changed: false,
            };
        }

        KeyEventResult::default()
    }

    fn duration_for_path(&mut self, path: &Path) -> u64 {
        if let Some(duration_ms) = self.duration_cache.get(path).copied() {
            return duration_ms;
        }
        // Avoid blocking the UI thread on ffprobe; tab scan already caches durations.
        5000
    }

    fn shortcut_id_for_trigger(&self, trigger: &str) -> Option<String> {
        SHORTCUT_BINDINGS.get().and_then(|store| {
            store.lock().ok().and_then(|bindings| {
                bindings
                    .iter()
                    .find(|def| def.trigger == trigger)
                    .map(|def| def.id.clone())
            })
        })
    }

    fn shortcut_for_slot(&self, slot: i32) -> Option<String> {
        let id = match slot {
            0 | 10 => "play_10".to_string(),
            1..=9 => format!("play_{slot}"),
            _ => return None,
        };
        SHORTCUT_BINDINGS
            .get()
            .and_then(|store| store.lock().ok())
            .and_then(|bindings| {
                bindings
                    .iter()
                    .find(|def| def.id == id)
                    .map(|def| def.trigger.clone())
            })
    }

    fn tick_progress(&mut self) -> bool {
        if self.active_playbacks.is_empty() {
            return false;
        }
        self.progress_version += 1;
        true
    }

    pub fn mark_playback_ended(&mut self, tab_index: i32, slot: i32) {
        let key = SessionKey { tab_index, slot };
        self.active_playbacks.remove(&key);
    }

    fn stop_session_internal(&mut self, tab_index: i32, slot: i32) {
        let key = SessionKey { tab_index, slot };
        self.active_playbacks.remove(&key);
        self.send_player_command(PlayerCommand::StopSession { tab_index, slot });
        self.bump_playing_version();
    }

    fn bump_playing_version(&mut self) {
        self.playing_version += 1;
    }

    fn play_slot_internal(&mut self, slot: i32) {
        let tab_index = self.current_tab_index;
        let session_key = SessionKey { tab_index, slot };

        if self.active_playbacks.contains_key(&session_key) {
            if let Some(playback) = self.active_playbacks.get(&session_key) {
                if playback.started.elapsed() < Duration::from_millis(MIN_PLAY_BEFORE_TOGGLE_MS) {
                    return;
                }
            }
            self.stop_session_internal(tab_index, slot);
            self.play_coalesce = None;
            return;
        }

        if let Some(coalesce) = self.play_coalesce {
            if coalesce.tab_index == tab_index
                && coalesce.slot == slot
                && coalesce.at.elapsed() < Duration::from_millis(KEY_DEDUPE_MS)
            {
                return;
            }
        }

        if self.interruption_mode_enabled() && !self.active_playbacks.is_empty() {
            self.active_playbacks.clear();
            self.play_coalesce = None;
            self.bump_playing_version();
        }

        let Some(index) = normalize_slot(slot) else {
            return;
        };
        let (path, cached_duration) = {
            let Some(tab) = self.active_tab() else {
                return;
            };
            let Some(path) = tab.slot(index).cloned() else {
                return;
            };
            let cached = tab
                .sound_at_slot(index)
                .map(|sound| sound.duration_ms)
                .unwrap_or(0);
            (path, cached)
        };
        let duration_ms = if cached_duration > 0 {
            cached_duration
        } else {
            self.duration_for_path(&path)
        };
        self.send_player_command(PlayerCommand::Play {
            path: path.clone(),
            tab_index,
            slot,
            volumes: self.volume_state(),
        });
        self.active_playbacks.insert(
            session_key,
            ActivePlayback {
                started: Instant::now(),
                duration_ms,
            },
        );
        self.play_coalesce = Some(PlayCoalesce {
            tab_index,
            slot,
            at: Instant::now(),
        });
        self.bump_playing_version();
    }

    fn next_tab_internal(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let count = self.tabs.len() as i32;
        self.apply_tab_index((self.current_tab_index + 1) % count);
    }

    fn prev_tab_internal(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let count = self.tabs.len() as i32;
        self.apply_tab_index((self.current_tab_index + count - 1) % count);
    }

    fn select_tab_internal(&mut self, index: i32) {
        self.apply_tab_index(index);
    }

    fn stop_all_internal(&mut self) {
        self.send_player_command(PlayerCommand::StopAll);
        self.active_playbacks.clear();
        self.play_coalesce = None;
        self.bump_playing_version();
    }

    fn handle_shortcut_id(&mut self, id: &str) -> ShortcutHandleResult {
        // Collapse "<action>_nonum" companion ids (NumLock-OFF variants registered
        // by ShortcutsManager::resolve_bindings_for_registration) back to the
        // canonical action BEFORE dedup, so a single physical press is never
        // counted twice across the two registered keysyms.
        let id = id.strip_suffix("_nonum").unwrap_or(id);
        if !accept_shortcut(id) {
            return ShortcutHandleResult::default();
        }
        match id {
            s if s.starts_with("play_") => {
                if let Ok(slot) = s.trim_start_matches("play_").parse::<i32>() {
                    self.play_slot_internal(slot);
                }
                ShortcutHandleResult::default()
            }
            "tab_next" => {
                self.next_tab_internal();
                ShortcutHandleResult {
                    tab_changed: true,
                    ..Default::default()
                }
            }
            "tab_prev" => {
                self.prev_tab_internal();
                ShortcutHandleResult {
                    tab_changed: true,
                    ..Default::default()
                }
            }
            "stop_all" => {
                self.stop_all_internal();
                ShortcutHandleResult::default()
            }
            "mute_output" => {
                self.toggle_output_mute_internal();
                ShortcutHandleResult {
                    mute_changed: true,
                    ..Default::default()
                }
            }
            "mute_monitor" => {
                self.toggle_monitor_mute_internal();
                ShortcutHandleResult {
                    mute_changed: true,
                    ..Default::default()
                }
            }
            _ => ShortcutHandleResult::default(),
        }
    }

    fn reload_shortcut_bindings() {
        let config = crate::config::load_config().unwrap_or_default();
        SoundboardControllerRust::sync_shortcut_bindings(
            &ShortcutsManager::resolve_bindings_for_registration(&config.shortcuts),
        );
    }

    pub fn sync_shortcut_bindings(bindings: &[ShortcutDef]) {
        if let Some(store) = SHORTCUT_BINDINGS.get() {
            if let Ok(mut guard) = store.lock() {
                *guard = bindings.to_vec();
            }
        }
    }

    fn bump_shortcut_version(&mut self) {
        self.shortcut_version += 1;
    }
}

impl qobject::SoundboardController {
    pub fn sync_global_shortcuts_status(self: Pin<&mut Self>) {
        Self::refresh_global_shortcuts_status(self);
    }

    pub fn refresh_global_shortcuts_status(mut self: Pin<&mut Self>) {
        let label = format_global_shortcut_status();
        let status = QString::from(label.as_str());
        self.as_mut().rust_mut().global_shortcuts_status = status.clone();
        self.as_mut().set_global_shortcuts_status(status);
    }

    pub fn needs_global_shortcut_apply(&self) -> bool {
        let config = crate::config::load_config().unwrap_or_default();
        if !ShortcutsManager::uses_global_binding(&config.shortcuts.mode) {
            return false;
        }
        if config.ui.global_shortcuts_prompt_dismissed {
            return false;
        }
        // Only prompt when binding has actually FAILED. While status is Inactive
        // (e.g. the in-flight bind during startup), say no — otherwise the QML
        // dialog races the bind and pops every launch.
        matches!(
            global_shortcut_status(),
            GlobalShortcutStatus::Failed { .. }
        )
    }

    pub fn dismiss_global_shortcuts_prompt(self: Pin<&mut Self>) {
        std::thread::spawn(|| {
            let mut config = crate::config::load_config().unwrap_or_default();
            config.ui.global_shortcuts_prompt_dismissed = true;
            let _ = crate::config::save_config(&config);
        });
    }

    pub fn needs_close_action_prompt(&self) -> bool {
        let config = crate::config::load_config().unwrap_or_default();
        !config.ui.close_action_prompt_dismissed
    }

    pub fn apply_close_action_choice(self: Pin<&mut Self>, minimize_to_tray: bool, remember: bool) {
        std::thread::spawn(move || {
            let mut config = crate::config::load_config().unwrap_or_default();
            config.ui.minimize_to_tray = minimize_to_tray;
            if remember {
                config.ui.close_action_prompt_dismissed = true;
            }
            let _ = crate::config::save_config(&config);
        });
    }

    pub fn has_saved_window_geometry(&self) -> bool {
        self.rust().window_geometry.is_some()
    }

    pub fn saved_window_x(&self) -> i32 {
        self.rust()
            .window_geometry
            .map(|geometry| geometry.x)
            .unwrap_or(0)
    }

    pub fn saved_window_y(&self) -> i32 {
        self.rust()
            .window_geometry
            .map(|geometry| geometry.y)
            .unwrap_or(0)
    }

    pub fn saved_window_width(&self) -> i32 {
        self.rust()
            .window_geometry
            .map(|geometry| geometry.width)
            .unwrap_or(800)
    }

    pub fn saved_window_height(&self) -> i32 {
        self.rust()
            .window_geometry
            .map(|geometry| geometry.height)
            .unwrap_or(600)
    }

    pub fn save_window_geometry(mut self: Pin<&mut Self>, x: i32, y: i32, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }
        let geometry = WindowGeometry {
            x,
            y,
            width,
            height,
        };
        self.as_mut().rust_mut().window_geometry = Some(geometry);
        self.as_mut().rust_mut().persist_window_geometry(geometry);
    }

    pub fn save_session_on_quit(mut self: Pin<&mut Self>, x: i32, y: i32, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }
        let geometry = WindowGeometry {
            x,
            y,
            width,
            height,
        };
        self.as_mut().rust_mut().window_geometry = Some(geometry);
        self.as_mut().rust_mut().persist_session_on_quit(geometry);
    }

    pub fn shutdown_backend(self: Pin<&mut Self>) {
        if let Some(tx) = BACKEND_TX.get() {
            let _ = tx.blocking_send(BackendCommand::Shutdown);
        }
    }

    pub fn play_slot(mut self: Pin<&mut Self>, slot: i32) {
        self.as_mut().rust_mut().play_slot_internal(slot);
        self.as_mut().playing_state_changed();
    }

    pub fn next_tab(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().next_tab_internal();
        properties::sync_tab_properties(self.as_mut());
        self.as_mut().current_tab_changed();
        self.as_mut().playing_state_changed();
    }

    pub fn prev_tab(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().prev_tab_internal();
        properties::sync_tab_properties(self.as_mut());
        self.as_mut().current_tab_changed();
        self.as_mut().playing_state_changed();
    }

    pub fn select_tab(mut self: Pin<&mut Self>, index: i32) {
        self.as_mut().rust_mut().select_tab_internal(index);
        properties::sync_tab_properties(self.as_mut());
        self.as_mut().current_tab_changed();
        self.as_mut().playing_state_changed();
    }

    pub fn stop_all(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().stop_all_internal();
        self.as_mut().playing_state_changed();
    }

    pub fn invoke_shortcut(mut self: Pin<&mut Self>, id: QString) {
        let id = String::from(id);
        let result = self.as_mut().rust_mut().handle_shortcut_id(id.as_str());
        if result.mute_changed {
            properties::sync_volume_properties(self.as_mut());
        }
        if result.tab_changed {
            properties::sync_tab_properties(self.as_mut());
            self.as_mut().current_tab_changed();
        } else if !result.mute_changed {
            self.as_mut().rust_mut().bump_playing_version();
        }
        self.as_mut().playing_state_changed();
    }

    pub fn handle_key_event(
        mut self: Pin<&mut Self>,
        key: i32,
        modifiers: i32,
        native_scan_code: u32,
    ) -> bool {
        let result =
            self.as_mut()
                .rust_mut()
                .handle_key_event_internal(key, modifiers, native_scan_code);
        if !result.handled {
            return false;
        }
        if result.mute_changed {
            properties::sync_volume_properties(self.as_mut());
        }
        if result.tab_changed {
            properties::sync_tab_properties(self.as_mut());
            self.as_mut().current_tab_changed();
        } else if result.playback_changed {
            self.as_mut().rust_mut().bump_playing_version();
        }
        self.as_mut().playing_state_changed();
        true
    }

    pub fn set_playback_keys_enabled(_self: Pin<&mut Self>, _enabled: bool) {}

    pub fn set_window_active(_self: Pin<&mut Self>, active: bool) {
        WINDOW_ACTIVE.store(active, Ordering::Relaxed);
    }

    pub fn refresh_portal_parent_window(_self: Pin<&mut Self>) {
        unsafe {
            sound_spring_refresh_portal_parent_window();
        }
    }

    pub fn bind_global_shortcuts(_self: Pin<&mut Self>) {
        Self::refresh_portal_parent_window(_self);
        if let Some(tx) = BACKEND_TX.get() {
            let _ = tx.blocking_send(BackendCommand::BindShortcuts);
        }
    }

    pub fn configure_global_shortcuts(_self: Pin<&mut Self>) {
        Self::refresh_portal_parent_window(_self);
        if let Some(tx) = BACKEND_TX.get() {
            let _ = tx.blocking_send(BackendCommand::ConfigurePortalShortcuts);
        }
    }

    pub fn refresh_shortcut_bindings(mut self: Pin<&mut Self>) {
        SoundboardControllerRust::reload_shortcut_bindings();
        self.as_mut().rust_mut().bump_shortcut_version();
    }

    pub fn update_output_volume(mut self: Pin<&mut Self>, volume: i32) {
        let volume = volume.clamp(0, 100);
        {
            let mut rust = self.as_mut().rust_mut();
            rust.output_volume = volume;
            if volume > 0 {
                rust.output_muted = false;
            }
            rust.persist_volumes();
            rust.push_volumes();
        }
        properties::sync_volume_properties(self.as_mut());
    }

    pub fn update_monitor_volume(mut self: Pin<&mut Self>, volume: i32) {
        let volume = volume.clamp(0, 100);
        {
            let mut rust = self.as_mut().rust_mut();
            rust.monitor_volume = volume;
            if volume > 0 {
                rust.monitor_muted = false;
            }
            rust.persist_volumes();
            rust.push_volumes();
        }
        properties::sync_volume_properties(self.as_mut());
    }

    pub fn toggle_output_mute(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().toggle_output_mute_internal();
        properties::sync_volume_properties(self.as_mut());
    }

    pub fn toggle_monitor_mute(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().toggle_monitor_mute_internal();
        properties::sync_volume_properties(self.as_mut());
    }

    pub fn update_mic_volume(mut self: Pin<&mut Self>, volume: i32) {
        let volume = volume.clamp(0, 100);
        {
            let mut rust = self.as_mut().rust_mut();
            rust.mic_volume = volume;
            if volume > 0 {
                rust.mic_muted = false;
            }
            rust.persist_volumes();
            rust.push_mic_volume();
        }
        properties::sync_volume_properties(self.as_mut());
    }

    pub fn toggle_mic_mute(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().toggle_mic_mute_internal();
        properties::sync_volume_properties(self.as_mut());
    }

    pub fn note_first_paint(self: Pin<&mut Self>) {
        if let Some(start) = crate::PROCESS_START.get() {
            tracing::info!("startup: first frame in {} ms", start.elapsed().as_millis());
        }
    }

    pub fn process_events(mut self: Pin<&mut Self>) {
        let progress_dirty = self.as_mut().rust_mut().tick_progress();

        let events: Vec<BackendEvent> = {
            let Some(rx) = BACKEND_EVENT_RX.get() else {
                return;
            };
            let rx = match rx.lock() {
                Ok(rx) => rx,
                Err(_) => return,
            };
            rx.try_iter().collect()
        };

        let mut playback_changed = progress_dirty;
        let mut tab_changed = false;

        if events.is_empty() {
            if playback_changed {
                self.as_mut().rust_mut().bump_playing_version();
                self.as_mut().playing_state_changed();
            }
            return;
        }

        for event in events {
            match event {
                BackendEvent::PlaybackEnded { tab_index, slot } => {
                    self.as_mut()
                        .rust_mut()
                        .mark_playback_ended(tab_index, slot);
                    playback_changed = true;
                }
                BackendEvent::ShortcutTriggered { id } => {
                    let result = self.as_mut().rust_mut().handle_shortcut_id(id.as_str());
                    if result.mute_changed {
                        properties::sync_volume_properties(self.as_mut());
                    }
                    if result.tab_changed {
                        tab_changed = true;
                    } else if !result.mute_changed {
                        playback_changed = true;
                    }
                }
                BackendEvent::ConfigApplied => {
                    let config = crate::config::load_config().unwrap_or_default();
                    let state_path = crate::config::state_path(&config);
                    let saved = State::load(&state_path).unwrap_or_default();
                    let mut rust = self.as_mut().rust_mut();
                    rust.tabs_root = config.paths.tabs_root.clone();
                    rust.state_path = state_path;
                    rust.window_geometry = saved.window_geometry;
                    rust.apply_volume_state(VolumeState {
                        output_percent: config.audio.output_volume,
                        monitor_percent: config.audio.monitor_volume,
                        output_muted: config.audio.output_muted,
                        monitor_muted: config.audio.monitor_muted,
                    });
                    rust.mic_volume = config.audio.mic_volume as i32;
                    rust.mic_muted = config.audio.mic_muted;
                    let tabs = TabsRepository::scan(&config).unwrap_or_default();
                    rust.replace_tabs(tabs, Some(&saved.current_tab));
                    rust.set_tab_warning(&SoundboardControllerRust::collect_tab_warnings(&config));
                    rust.refresh_mic_source_count();
                    rust.refresh_audio_sink_count();
                    SoundboardControllerRust::reload_shortcut_bindings();
                    self.as_mut().rust_mut().bump_shortcut_version();
                    properties::sync_volume_properties(self.as_mut());
                    playback_changed = true;
                    tab_changed = true;
                }
                BackendEvent::TabsRescanned { tabs } => {
                    let config = crate::config::load_config().unwrap_or_default();
                    let current = self
                        .as_ref()
                        .rust()
                        .active_tab()
                        .map(|tab| tab.path.to_string_lossy().into_owned());
                    self.as_mut().rust_mut().tabs_root = config.paths.tabs_root.clone();
                    self.as_mut()
                        .rust_mut()
                        .replace_tabs(tabs, current.as_deref());
                    self.as_mut()
                        .rust_mut()
                        .set_tab_warning(&SoundboardControllerRust::collect_tab_warnings(&config));
                    tab_changed = true;
                }
                BackendEvent::MicSourcesUpdated => {
                    self.as_mut().rust_mut().refresh_mic_source_count();
                    properties::sync_mic_properties(self.as_mut());
                }
                BackendEvent::AudioSinksUpdated => {
                    self.as_mut().rust_mut().refresh_audio_sink_count();
                    properties::sync_audio_sink_properties(self.as_mut());
                }
                BackendEvent::GlobalShortcutStatusChanged => {
                    Self::refresh_global_shortcuts_status(self.as_mut());
                }
                BackendEvent::VoiceCaptureStatus { active, error } => {
                    crate::services::voice::voice_shared().set_capture_status(active, &error);
                }
            }
        }

        if tab_changed || playback_changed {
            if playback_changed {
                self.as_mut().rust_mut().bump_playing_version();
            }
            if tab_changed {
                properties::sync_tab_properties(self.as_mut());
                self.as_mut().tabs_changed();
                self.as_mut().current_tab_changed();
            }
            self.as_mut().playing_state_changed();
        }
    }

    pub fn reload_from_config(mut self: Pin<&mut Self>) {
        let config = crate::config::load_config().unwrap_or_default();
        let state_path = crate::config::state_path(&config);
        let saved = State::load(&state_path).unwrap_or_default();
        let mut rust = self.as_mut().rust_mut();
        rust.tabs_root = config.paths.tabs_root.clone();
        rust.state_path = state_path;
        rust.window_geometry = saved.window_geometry;
        let tabs = TabsRepository::scan(&config).unwrap_or_default();
        rust.replace_tabs(tabs, Some(&saved.current_tab));
        rust.set_tab_warning(&SoundboardControllerRust::collect_tab_warnings(&config));
        rust.refresh_mic_source_count();
        rust.refresh_audio_sink_count();
        properties::sync_tab_properties(self.as_mut());
        properties::sync_mic_properties(self.as_mut());
        properties::sync_audio_sink_properties(self.as_mut());
        self.as_mut().tabs_changed();
        self.as_mut().current_tab_changed();
        self.as_mut().playing_state_changed();
    }

    pub fn tab_name_at(&self, index: i32) -> QString {
        QString::from(
            self.rust()
                .tabs
                .get(index as usize)
                .map(|tab| tab.display_name())
                .unwrap_or(""),
        )
    }

    pub fn tab_path_at(&self, index: i32) -> QString {
        QString::from(
            self.rust()
                .tabs
                .get(index as usize)
                .map(|tab| tab.path.to_string_lossy().into_owned())
                .unwrap_or_default()
                .as_str(),
        )
    }

    pub fn tab_uses_custom_list(&self) -> bool {
        config::load_config()
            .map(|config| uses_custom_tabs(&config))
            .unwrap_or(false)
    }

    pub fn add_tab(mut self: Pin<&mut Self>, path: QString, name: QString) -> bool {
        let path_text = String::from(path).trim().to_string();
        let name_text = String::from(name).trim().to_string();
        let display_name = if name_text.is_empty() {
            "New Tab".to_string()
        } else {
            name_text
        };

        let mut config = config::load_config().unwrap_or_default();
        let new_path = if path_text.is_empty() {
            match TabsRepository::create_tab_dir(&config.paths.tabs_root, &display_name) {
                Ok(path) => path,
                Err(err) => {
                    tracing::warn!("failed to create tab directory: {err:#}");
                    return false;
                }
            }
        } else {
            let path = PathBuf::from(&path_text);
            if !path.is_dir() {
                tracing::warn!("add tab: {} is not a directory", path.display());
                return false;
            }
            path
        };

        let mut watch_restart = false;
        if path_text.is_empty() {
            if uses_custom_tabs(&config) {
                config.tabs.push(TabEntry {
                    path: new_path.clone(),
                    name: None,
                });
                watch_restart = true;
            }
        } else if uses_custom_tabs(&config) || !new_path.starts_with(&config.paths.tabs_root) {
            if config.tabs.iter().any(|entry| entry.path == new_path) {
                tracing::warn!("add tab: {} is already registered", new_path.display());
                return false;
            }
            let entry_name = if display_name == tab_name_from_path(&new_path) {
                None
            } else {
                Some(display_name)
            };
            config.tabs.push(TabEntry {
                path: new_path.clone(),
                name: entry_name,
            });
            watch_restart = true;
        }

        if !self
            .as_mut()
            .rust_mut()
            .finish_tab_mutation(&config, Some(&new_path), watch_restart)
        {
            return false;
        }
        properties::sync_tab_properties(self.as_mut());
        self.as_mut().tabs_changed();
        self.as_mut().current_tab_changed();
        true
    }

    pub fn rename_tab(mut self: Pin<&mut Self>, index: i32, name: QString) -> bool {
        let name_text = String::from(name).trim().to_string();
        if name_text.is_empty() {
            return false;
        }
        let Some(tab_path) = self
            .rust()
            .tabs
            .get(index as usize)
            .map(|tab| tab.path.clone())
        else {
            return false;
        };

        let mut config = config::load_config().unwrap_or_default();
        let select_path = if uses_custom_tabs(&config) {
            let Some(cfg_index) =
                SoundboardControllerRust::config_index_for_tab_path(&config, &tab_path)
            else {
                return false;
            };
            config.tabs[cfg_index].name = Some(name_text);
            Some(tab_path)
        } else {
            match TabsRepository::rename_tab_dir(&tab_path, &name_text) {
                Ok(path) => Some(path),
                Err(err) => {
                    tracing::warn!("failed to rename tab: {err:#}");
                    return false;
                }
            }
        };

        let Some(select_path) = select_path else {
            return false;
        };
        if !self
            .as_mut()
            .rust_mut()
            .finish_tab_mutation(&config, Some(&select_path), false)
        {
            return false;
        }
        properties::sync_tab_properties(self.as_mut());
        self.as_mut().tabs_changed();
        self.as_mut().current_tab_changed();
        true
    }

    pub fn move_tab(mut self: Pin<&mut Self>, from_index: i32, to_index: i32) -> bool {
        let from = from_index as usize;
        let to = to_index as usize;
        if from == to {
            return false;
        }
        let ordered_paths: Vec<PathBuf> = self
            .rust()
            .tabs
            .iter()
            .map(|tab| tab.path.clone())
            .collect();
        if from >= ordered_paths.len() || to >= ordered_paths.len() {
            return false;
        }

        let mut config = config::load_config().unwrap_or_default();
        let select_path = if uses_custom_tabs(&config) {
            let from_path = ordered_paths[from].clone();
            let to_path = ordered_paths[to].clone();
            let Some(cfg_from) =
                SoundboardControllerRust::config_index_for_tab_path(&config, &from_path)
            else {
                return false;
            };
            let Some(cfg_to) =
                SoundboardControllerRust::config_index_for_tab_path(&config, &to_path)
            else {
                return false;
            };
            TabsRepository::reorder_custom_tabs(&mut config.tabs, cfg_from, cfg_to);
            Some(from_path)
        } else {
            let mut reordered = ordered_paths.clone();
            let moved = reordered.remove(from);
            reordered.insert(to, moved);
            match TabsRepository::reorder_tabs_root(&config.paths.tabs_root, &reordered) {
                Ok(paths) => paths.get(to).cloned(),
                Err(err) => {
                    tracing::warn!("failed to reorder tabs: {err:#}");
                    return false;
                }
            }
        };

        let Some(select_path) = select_path else {
            return false;
        };
        if !self
            .as_mut()
            .rust_mut()
            .finish_tab_mutation(&config, Some(&select_path), false)
        {
            return false;
        }
        properties::sync_tab_properties(self.as_mut());
        self.as_mut().tabs_changed();
        self.as_mut().current_tab_changed();
        true
    }

    pub fn remove_tab(mut self: Pin<&mut Self>, index: i32) -> bool {
        if self.rust().tabs.len() <= 1 {
            tracing::warn!("remove tab: at least one tab must remain");
            return false;
        }
        let Some(tab_path) = self
            .rust()
            .tabs
            .get(index as usize)
            .map(|tab| tab.path.clone())
        else {
            return false;
        };

        let mut config = config::load_config().unwrap_or_default();
        if uses_custom_tabs(&config) {
            let Some(cfg_index) =
                SoundboardControllerRust::config_index_for_tab_path(&config, &tab_path)
            else {
                return false;
            };
            config.tabs.remove(cfg_index);
        }

        let delete_dir = tab_path.starts_with(&config.paths.tabs_root);
        if delete_dir {
            if let Err(err) = TabsRepository::remove_tab_dir(&tab_path) {
                tracing::warn!(
                    "failed to remove tab directory {}: {err:#}",
                    tab_path.display()
                );
                return false;
            }
        }

        let select_path = self
            .rust()
            .tabs
            .iter()
            .enumerate()
            .find(|(i, _)| *i != index as usize)
            .map(|(_, tab)| tab.path.clone());

        if !self
            .as_mut()
            .rust_mut()
            .finish_tab_mutation(&config, select_path.as_deref(), true)
        {
            return false;
        }
        properties::sync_tab_properties(self.as_mut());
        self.as_mut().tabs_changed();
        self.as_mut().current_tab_changed();
        true
    }

    pub fn slot_label(&self, slot: i32) -> QString {
        let Some(tab) = self.rust().active_tab() else {
            return QString::from("");
        };
        let Some(index) = normalize_slot(slot) else {
            return QString::from("");
        };
        QString::from(
            tab.sound_at_slot(index)
                .map(|sound| sound.name.as_str())
                .unwrap_or(""),
        )
    }

    pub fn slot_path_at(&self, slot: i32) -> QString {
        QString::from(
            self.rust()
                .slot_path_internal(slot)
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default()
                .as_str(),
        )
    }

    pub fn slot_empty(&self, slot: i32) -> bool {
        let Some(tab) = self.rust().active_tab() else {
            return true;
        };
        normalize_slot(slot)
            .and_then(|index| tab.slot(index))
            .is_none()
    }

    pub fn replace_slot(mut self: Pin<&mut Self>, slot: i32, path: QString) -> bool {
        let Some(index) = normalize_slot(slot) else {
            return false;
        };
        let Some(tab_dir) = self.rust().active_tab().map(|tab| tab.path.clone()) else {
            return false;
        };
        let source = PathBuf::from(String::from(path));
        match TabsRepository::replace_slot_file(&tab_dir, index, &source) {
            Ok(_) => {
                self.as_mut().rust_mut().refresh_active_tab_slots();
                properties::sync_tab_properties(self.as_mut());
                self.as_mut().tabs_changed();
                self.as_mut().playing_state_changed();
                true
            }
            Err(err) => {
                tracing::warn!("replace slot {slot}: {err:#}");
                false
            }
        }
    }

    pub fn remove_slot(mut self: Pin<&mut Self>, slot: i32) -> bool {
        let Some(index) = normalize_slot(slot) else {
            return false;
        };
        let Some(tab_dir) = self.rust().active_tab().map(|tab| tab.path.clone()) else {
            return false;
        };
        let tab_index = self.rust().current_tab_index;
        match TabsRepository::remove_slot_file(&tab_dir, index) {
            Ok(()) => {
                self.as_mut()
                    .rust_mut()
                    .stop_session_internal(tab_index, slot);
                self.as_mut().rust_mut().refresh_active_tab_slots();
                properties::sync_tab_properties(self.as_mut());
                self.as_mut().tabs_changed();
                self.as_mut().playing_state_changed();
                true
            }
            Err(err) => {
                tracing::warn!("remove slot {slot}: {err:#}");
                false
            }
        }
    }

    pub fn rename_slot(mut self: Pin<&mut Self>, slot: i32, name: QString) -> bool {
        let Some(index) = normalize_slot(slot) else {
            return false;
        };
        let Some(tab_dir) = self.rust().active_tab().map(|tab| tab.path.clone()) else {
            return false;
        };
        let name = String::from(name);
        match TabsRepository::rename_slot_file(&tab_dir, index, &name) {
            Ok(_) => {
                self.as_mut().rust_mut().refresh_active_tab_slots();
                properties::sync_tab_properties(self.as_mut());
                self.as_mut().tabs_changed();
                self.as_mut().playing_state_changed();
                true
            }
            Err(err) => {
                tracing::warn!("rename slot {slot}: {err:#}");
                false
            }
        }
    }

    pub fn move_slot(mut self: Pin<&mut Self>, from_slot: i32, to_slot: i32) -> bool {
        let Some(from_index) = normalize_slot(from_slot) else {
            return false;
        };
        let Some(to_index) = normalize_slot(to_slot) else {
            return false;
        };
        let Some(tab_dir) = self.rust().active_tab().map(|tab| tab.path.clone()) else {
            return false;
        };
        let tab_index = self.rust().current_tab_index;
        match TabsRepository::move_slot_file(&tab_dir, from_index, to_index) {
            Ok(()) => {
                self.as_mut()
                    .rust_mut()
                    .stop_session_internal(tab_index, from_slot);
                if from_slot != to_slot {
                    self.as_mut()
                        .rust_mut()
                        .stop_session_internal(tab_index, to_slot);
                }
                self.as_mut().rust_mut().refresh_active_tab_slots();
                properties::sync_tab_properties(self.as_mut());
                self.as_mut().tabs_changed();
                self.as_mut().playing_state_changed();
                true
            }
            Err(err) => {
                tracing::warn!("move slot {from_slot} -> {to_slot}: {err:#}");
                false
            }
        }
    }

    pub fn open_tab_folder(self: Pin<&mut Self>) {
        let Some(tab_dir) = self.rust().active_tab().map(|tab| tab.path.clone()) else {
            return;
        };
        std::thread::spawn(move || {
            if let Err(err) = std::process::Command::new("xdg-open").arg(tab_dir).spawn() {
                tracing::warn!("open tab folder failed: {err:#}");
            }
        });
    }

    pub fn slot_playing(&self, slot: i32) -> bool {
        let key = SessionKey {
            tab_index: self.rust().current_tab_index,
            slot,
        };
        self.rust().active_playbacks.contains_key(&key)
    }

    pub fn slot_shortcut_label(&self, slot: i32) -> QString {
        QString::from(
            self.rust()
                .shortcut_for_slot(slot)
                .map(|trigger| trigger_display(&trigger))
                .unwrap_or_default()
                .as_str(),
        )
    }

    pub fn slot_progress(&self, slot: i32) -> f64 {
        let key = SessionKey {
            tab_index: self.rust().current_tab_index,
            slot,
        };
        let Some(playback) = self.rust().active_playbacks.get(&key) else {
            return 0.0;
        };
        if playback.duration_ms == 0 {
            return 0.0;
        }
        let elapsed = playback.started.elapsed().as_millis() as f64;
        (elapsed / playback.duration_ms as f64).clamp(0.0, 1.0)
    }

    pub fn shortcut_sequence(&self, id: QString) -> QString {
        let id = String::from(id);
        let trigger = SHORTCUT_BINDINGS
            .get()
            .and_then(|store| store.lock().ok())
            .and_then(|bindings| {
                bindings
                    .iter()
                    .find(|def| def.id == id)
                    .map(|def| def.trigger.clone())
            })
            .unwrap_or_default();
        QString::from(qt_shortcut_sequence(&trigger).as_str())
    }

    pub fn mic_source_name_at(&self, index: i32) -> QString {
        QString::from(
            self.mic_source_at(index)
                .map(|source| source.description)
                .unwrap_or_default()
                .as_str(),
        )
    }

    pub fn mic_source_id_at(&self, index: i32) -> QString {
        QString::from(
            self.mic_source_at(index)
                .map(|source| source.name)
                .unwrap_or_default()
                .as_str(),
        )
    }

    pub fn mic_source_description_at(&self, index: i32) -> QString {
        self.mic_source_name_at(index)
    }

    pub fn refresh_mic_sources(self: Pin<&mut Self>) {
        self.refresh_audio_devices();
    }

    pub fn audio_sink_id_at(&self, index: i32) -> QString {
        QString::from(
            self.audio_sink_at(index)
                .map(|sink| sink.name)
                .unwrap_or_default()
                .as_str(),
        )
    }

    pub fn audio_sink_description_at(&self, index: i32) -> QString {
        QString::from(
            self.audio_sink_at(index)
                .map(|sink| sink.description)
                .unwrap_or_default()
                .as_str(),
        )
    }

    pub fn refresh_audio_devices(self: Pin<&mut Self>) {
        if let Some(tx) = BACKEND_TX.get() {
            let _ = tx.blocking_send(BackendCommand::RefreshMicSources);
            let _ = tx.blocking_send(BackendCommand::RefreshAudioSinks);
        }
    }
}

impl Constructor<()> for qobject::SoundboardController {
    type NewArguments = ();
    type BaseArguments = ();
    type InitializeArguments = ();

    fn route_arguments(
        (): (),
    ) -> (
        Self::NewArguments,
        Self::BaseArguments,
        Self::InitializeArguments,
    ) {
        ((), (), ())
    }

    fn new((): ()) -> SoundboardControllerRust {
        SoundboardControllerRust::default()
    }

    fn initialize(mut self: Pin<&mut Self>, (): ()) {
        SHORTCUT_BINDINGS.set(Mutex::new(Vec::new())).ok();
        SoundboardControllerRust::reload_shortcut_bindings();

        let config = crate::config::load_config().unwrap_or_default();
        let state_path = crate::config::state_path(&config);
        let saved = crate::state::State::load(&state_path).unwrap_or_default();
        let mut rust = self.as_mut().rust_mut();
        rust.tabs_root = config.paths.tabs_root.clone();
        rust.state_path = state_path;
        rust.window_geometry = saved.window_geometry;
        rust.apply_volume_state(VolumeState {
            output_percent: config.audio.output_volume,
            monitor_percent: config.audio.monitor_volume,
            output_muted: config.audio.output_muted,
            monitor_muted: config.audio.monitor_muted,
        });
        rust.mic_volume = config.audio.mic_volume as i32;
        rust.mic_muted = config.audio.mic_muted;
        rust.push_volumes();
        rust.push_mic_volume();
        let tabs = TabsRepository::scan(&config).unwrap_or_default();
        rust.replace_tabs(tabs, Some(&saved.current_tab));
        rust.set_tab_warning(&SoundboardControllerRust::collect_tab_warnings(&config));
        rust.refresh_mic_source_count();
        rust.refresh_audio_sink_count();
        properties::sync_tab_properties(self.as_mut());
        properties::sync_mic_properties(self.as_mut());
        properties::sync_audio_sink_properties(self.as_mut());
        properties::sync_volume_properties(self.as_mut());
        Self::refresh_global_shortcuts_status(self.as_mut());
    }
}

/// Qt property sync helpers — see `docs/cxx-qt-qml.md`.
pub(crate) mod properties {
    use core::pin::Pin;

    use cxx_qt::CxxQtType;

    use super::qobject::SoundboardController;

    pub fn next_ui_version(current: i32) -> i32 {
        current + 1
    }

    pub fn bump_ui_version(mut controller: Pin<&mut SoundboardController>) {
        let next = next_ui_version(controller.as_ref().rust().ui_version);
        controller.as_mut().set_ui_version(next);
    }

    pub fn sync_tab_properties(mut controller: Pin<&mut SoundboardController>) {
        let index = controller.as_ref().rust().current_tab_index;
        let tab_count = controller.as_ref().rust().tab_count;
        let tab_version = controller.as_ref().rust().tab_version;
        let name = controller.as_ref().rust().current_tab_name.clone();
        let tab_warning = controller.as_ref().rust().tab_warning.clone();
        controller.as_mut().set_current_tab_index(index);
        controller.as_mut().set_current_tab_name(name);
        controller.as_mut().set_tab_count(tab_count);
        controller.as_mut().set_tab_version(tab_version);
        controller.as_mut().set_tab_warning(tab_warning);
        bump_ui_version(controller);
    }

    pub fn sync_mic_properties(mut controller: Pin<&mut SoundboardController>) {
        let count = controller.as_ref().rust().mic_source_count;
        let version = controller.as_ref().rust().mic_sources_version;
        controller.as_mut().set_mic_source_count(count);
        controller.as_mut().set_mic_sources_version(version);
    }

    pub fn sync_audio_sink_properties(mut controller: Pin<&mut SoundboardController>) {
        let count = controller.as_ref().rust().audio_sink_count;
        let version = controller.as_ref().rust().audio_sinks_version;
        controller.as_mut().set_audio_sink_count(count);
        controller.as_mut().set_audio_sinks_version(version);
    }

    pub fn sync_volume_properties(mut controller: Pin<&mut SoundboardController>) {
        let output_volume = controller.as_ref().rust().output_volume;
        let monitor_volume = controller.as_ref().rust().monitor_volume;
        let mic_volume = controller.as_ref().rust().mic_volume;
        let output_muted = controller.as_ref().rust().output_muted;
        let monitor_muted = controller.as_ref().rust().monitor_muted;
        let mic_muted = controller.as_ref().rust().mic_muted;
        controller.as_mut().set_output_volume(output_volume);
        controller.as_mut().set_monitor_volume(monitor_volume);
        controller.as_mut().set_mic_volume(mic_volume);
        controller.as_mut().set_output_muted(output_muted);
        controller.as_mut().set_monitor_muted(monitor_muted);
        controller.as_mut().set_mic_muted(mic_muted);
        bump_ui_version(controller);
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;

    #[test]
    fn stop_session_only_clears_matching_tab_slot() {
        let mut controller = SoundboardControllerRust::default();
        let music = SessionKey {
            tab_index: 0,
            slot: 1,
        };
        let effects = SessionKey {
            tab_index: 1,
            slot: 1,
        };
        controller.active_playbacks.insert(
            music,
            ActivePlayback {
                started: Instant::now(),
                duration_ms: 1000,
            },
        );
        controller.active_playbacks.insert(
            effects,
            ActivePlayback {
                started: Instant::now(),
                duration_ms: 1000,
            },
        );

        controller.stop_session_internal(1, 1);

        assert!(controller.active_playbacks.contains_key(&music));
        assert!(!controller.active_playbacks.contains_key(&effects));
    }

    #[test]
    fn ui_version_bump_advances_monotonically() {
        use super::properties::next_ui_version;
        assert_eq!(next_ui_version(0), 1);
        assert_eq!(next_ui_version(3), 4);
    }
}
