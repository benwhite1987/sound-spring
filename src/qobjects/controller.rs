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
        #[qproperty(i32, tab_version)]
        #[qproperty(i32, progress_version)]
        #[qproperty(i32, shortcut_version)]
        #[qproperty(i32, ui_version)]
        #[qproperty(i32, output_volume)]
        #[qproperty(i32, monitor_volume)]
        #[qproperty(bool, output_muted)]
        #[qproperty(bool, monitor_muted)]
        #[qproperty(QString, global_shortcuts_status)]
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
        fn tab_name_at(self: &SoundboardController, index: i32) -> QString;

        #[qinvokable]
        fn slot_label(self: &SoundboardController, slot: i32) -> QString;

        #[qinvokable]
        fn slot_empty(self: &SoundboardController, slot: i32) -> bool;

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
        fn reload_from_config(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn sync_global_shortcuts_status(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn refresh_global_shortcuts_status(self: Pin<&mut SoundboardController>);

        #[qinvokable]
        fn needs_global_shortcut_apply(self: &SoundboardController) -> bool;

        #[qinvokable]
        fn dismiss_global_shortcuts_prompt(self: Pin<&mut SoundboardController>);

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

use crate::config::Config;
use crate::services::pipewire::MicSource;
use crate::services::player::{PlayerCommand, VolumeState};
use crate::services::shortcuts::{
    accept_shortcut, format_global_shortcut_status, global_shortcuts_active, play_slot_from_qt_key,
    qt_shortcut_sequence, trigger_display, trigger_from_qt, ShortcutDef, ShortcutsManager,
};
use crate::services::tabs::{normalize_slot, Tab, TabsRepository};
use crate::state::State;

#[derive(Debug)]
pub enum BackendCommand {
    ApplyConfig(Config),
    BindShortcuts,
    ConfigurePortalShortcuts,
    Player(PlayerCommand),
    RefreshMicSources,
    ApplyVolumes(VolumeState),
}

#[derive(Debug, Clone)]
pub enum BackendEvent {
    PlaybackEnded { tab_index: i32, slot: i32 },
    ShortcutTriggered { id: String },
    GlobalShortcutStatusChanged,
    ConfigApplied,
    MicSourcesUpdated,
}

pub static BACKEND_TX: OnceLock<TokioSender<BackendCommand>> = OnceLock::new();
pub static BACKEND_EVENT_RX: OnceLock<Mutex<StdReceiver<BackendEvent>>> = OnceLock::new();

pub static MIC_SOURCES: OnceLock<Mutex<Vec<MicSource>>> = OnceLock::new();
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
    tab_version: i32,
    progress_version: i32,
    shortcut_version: i32,
    ui_version: i32,
    output_volume: i32,
    monitor_volume: i32,
    output_muted: bool,
    monitor_muted: bool,
    global_shortcuts_status: QString,
    tabs: Vec<Tab>,
    active_playbacks: HashMap<SessionKey, ActivePlayback>,
    play_coalesce: Option<PlayCoalesce>,
    duration_cache: HashMap<PathBuf, u64>,
    tabs_root: PathBuf,
    state_path: PathBuf,
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
        let output_muted = self.output_muted;
        let monitor_muted = self.monitor_muted;
        std::thread::spawn(move || {
            let mut config = crate::config::load_config().unwrap_or_default();
            config.audio.output_volume = output_volume;
            config.audio.monitor_volume = monitor_volume;
            config.audio.output_muted = output_muted;
            config.audio.monitor_muted = monitor_muted;
            let _ = crate::config::save_config(&config);
        });
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
                .sounds
                .get(index - 1)
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
        SoundboardControllerRust::sync_shortcut_bindings(&ShortcutsManager::resolve_bindings(
            &config.shortcuts,
        ));
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
        !global_shortcuts_active()
    }

    pub fn dismiss_global_shortcuts_prompt(self: Pin<&mut Self>) {
        std::thread::spawn(|| {
            let mut config = crate::config::load_config().unwrap_or_default();
            config.ui.global_shortcuts_prompt_dismissed = true;
            let _ = crate::config::save_config(&config);
        });
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
                    let tabs = TabsRepository::scan(&config).unwrap_or_default();
                    rust.replace_tabs(tabs, Some(&saved.current_tab));
                    rust.refresh_mic_source_count();
                    SoundboardControllerRust::reload_shortcut_bindings();
                    self.as_mut().rust_mut().bump_shortcut_version();
                    playback_changed = true;
                    tab_changed = true;
                }
                BackendEvent::MicSourcesUpdated => {
                    self.as_mut().rust_mut().refresh_mic_source_count();
                    properties::sync_mic_properties(self.as_mut());
                }
                BackendEvent::GlobalShortcutStatusChanged => {
                    Self::refresh_global_shortcuts_status(self.as_mut());
                }
            }
        }

        if tab_changed || playback_changed {
            if playback_changed {
                self.as_mut().rust_mut().bump_playing_version();
            }
            if tab_changed {
                properties::sync_tab_properties(self.as_mut());
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
        let tabs = TabsRepository::scan(&config).unwrap_or_default();
        rust.replace_tabs(tabs, Some(&saved.current_tab));
        rust.refresh_mic_source_count();
        properties::sync_tab_properties(self.as_mut());
        properties::sync_mic_properties(self.as_mut());
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

    pub fn slot_label(&self, slot: i32) -> QString {
        let Some(tab) = self.rust().active_tab() else {
            return QString::from("");
        };
        let Some(index) = normalize_slot(slot) else {
            return QString::from("");
        };
        QString::from(
            tab.sounds
                .get(index - 1)
                .map(|sound| sound.name.as_str())
                .unwrap_or(""),
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
        if let Some(tx) = BACKEND_TX.get() {
            let _ = tx.blocking_send(BackendCommand::RefreshMicSources);
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
        rust.apply_volume_state(VolumeState {
            output_percent: config.audio.output_volume,
            monitor_percent: config.audio.monitor_volume,
            output_muted: config.audio.output_muted,
            monitor_muted: config.audio.monitor_muted,
        });
        rust.push_volumes();
        let tabs = TabsRepository::scan(&config).unwrap_or_default();
        rust.replace_tabs(tabs, Some(&saved.current_tab));
        rust.refresh_mic_source_count();
        properties::sync_tab_properties(self.as_mut());
        properties::sync_mic_properties(self.as_mut());
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
        controller.as_mut().set_current_tab_index(index);
        controller.as_mut().set_current_tab_name(name);
        controller.as_mut().set_tab_count(tab_count);
        controller.as_mut().set_tab_version(tab_version);
        bump_ui_version(controller);
    }

    pub fn sync_mic_properties(mut controller: Pin<&mut SoundboardController>) {
        let count = controller.as_ref().rust().mic_source_count;
        let version = controller.as_ref().rust().mic_sources_version;
        controller.as_mut().set_mic_source_count(count);
        controller.as_mut().set_mic_sources_version(version);
    }

    pub fn sync_volume_properties(mut controller: Pin<&mut SoundboardController>) {
        let output_volume = controller.as_ref().rust().output_volume;
        let monitor_volume = controller.as_ref().rust().monitor_volume;
        let output_muted = controller.as_ref().rust().output_muted;
        let monitor_muted = controller.as_ref().rust().monitor_muted;
        controller.as_mut().set_output_volume(output_volume);
        controller.as_mut().set_monitor_volume(monitor_volume);
        controller.as_mut().set_output_muted(output_muted);
        controller.as_mut().set_monitor_muted(monitor_muted);
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
