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
        #[qproperty(i32, tab_version)]
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
        fn mic_source_name_at(self: &SoundboardController, index: i32) -> QString;

        #[qinvokable]
        fn reload_from_config(self: Pin<&mut SoundboardController>);

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
use cxx_qt::{CxxQtType, Constructor};
use cxx_qt_lib::QString;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{mpsc::Receiver as StdReceiver, Mutex, OnceLock};
use tokio::sync::mpsc::Sender as TokioSender;

use crate::config::Config;
use crate::services::pipewire::MicSource;
use crate::services::tabs::{normalize_slot, Tab, TabsRepository};
use crate::services::PlayerCommand;
use crate::state::State;

#[derive(Debug)]
pub enum BackendCommand {
    ApplyConfig(Config),
    Player(PlayerCommand),
}

#[derive(Debug, Clone)]
pub enum BackendEvent {
    PlaybackEnded { slot: i32 },
    ShortcutTriggered { id: String },
    ConfigApplied,
    MicSourcesUpdated,
}

pub static BACKEND_TX: OnceLock<TokioSender<BackendCommand>> = OnceLock::new();
pub static BACKEND_EVENT_RX: OnceLock<Mutex<StdReceiver<BackendEvent>>> = OnceLock::new();

pub static MIC_SOURCES: OnceLock<Mutex<Vec<MicSource>>> = OnceLock::new();

// Compatibility with existing playback wiring.
pub type PlayerBackendEvent = BackendEvent;
pub static PLAYER_TX: OnceLock<TokioSender<PlayerCommand>> = OnceLock::new();
pub static PLAYER_EVENT_RX: OnceLock<Mutex<StdReceiver<BackendEvent>>> = OnceLock::new();

#[derive(Default)]
pub struct SoundboardControllerRust {
    current_tab_index: i32,
    current_tab_name: QString,
    tab_count: i32,
    playing_version: i32,
    mic_source_count: i32,
    tab_version: i32,
    tabs: Vec<Tab>,
    playing_slots: HashSet<i32>,
    tabs_root: PathBuf,
    state_path: PathBuf,
}

impl SoundboardControllerRust {
    fn send_player_command(&self, command: PlayerCommand) {
        if let Some(tx) = BACKEND_TX.get() {
            let _ = tx.blocking_send(BackendCommand::Player(command));
        } else if let Some(tx) = PLAYER_TX.get() {
            let _ = tx.blocking_send(command);
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
        if let Some(tab) = TabsRepository::resolve_current_tab(&self.tabs, current, &self.tabs_root) {
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
        let mut state = State::load(&self.state_path).unwrap_or_default();
        state.current_tab = tab.path.to_string_lossy().into_owned();
        if let Err(err) = state.save(&self.state_path) {
            tracing::warn!("failed to save state: {err:#}");
        }
    }

    pub fn mark_playback_ended(&mut self, slot: i32) {
        self.playing_slots.remove(&slot);
    }

    fn bump_playing_version(&mut self) {
        self.playing_version += 1;
    }

    fn play_slot_internal(&mut self, slot: i32) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let Some(index) = normalize_slot(slot) else {
            return;
        };
        let Some(path) = tab.slot(index).cloned() else {
            return;
        };
        self.send_player_command(PlayerCommand::Play { path, slot });
        self.playing_slots.insert(slot);
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
        self.playing_slots.clear();
        self.bump_playing_version();
    }

    fn handle_shortcut_id(&mut self, id: &str) {
        match id {
            s if s.starts_with("play_") => {
                if let Ok(slot) = s.trim_start_matches("play_").parse::<i32>() {
                    self.play_slot_internal(slot);
                }
            }
            "tab_next" => self.next_tab_internal(),
            "tab_prev" => self.prev_tab_internal(),
            "stop_all" => self.stop_all_internal(),
            _ => {}
        }
    }
}

impl qobject::SoundboardController {
    pub fn play_slot(mut self: Pin<&mut Self>, slot: i32) {
        self.as_mut().rust_mut().play_slot_internal(slot);
        self.playing_state_changed();
    }

    pub fn next_tab(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().next_tab_internal();
        self.current_tab_changed();
    }

    pub fn prev_tab(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().prev_tab_internal();
        self.current_tab_changed();
    }

    pub fn select_tab(mut self: Pin<&mut Self>, index: i32) {
        self.as_mut().rust_mut().select_tab_internal(index);
        self.current_tab_changed();
    }

    pub fn stop_all(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().stop_all_internal();
        self.playing_state_changed();
    }

    pub fn process_events(mut self: Pin<&mut Self>) {
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

        if events.is_empty() {
            return;
        }

        let mut changed = false;

        for event in events {
            match event {
                BackendEvent::PlaybackEnded { slot } => {
                    self.as_mut().rust_mut().mark_playback_ended(slot);
                    changed = true;
                }
                BackendEvent::ShortcutTriggered { id } => {
                    self.as_mut().rust_mut().handle_shortcut_id(id.as_str());
                    changed = true;
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
                    changed = true;
                }
                BackendEvent::MicSourcesUpdated => {
                    self.as_mut().rust_mut().refresh_mic_source_count();
                    changed = true;
                }
            }
        }

        if changed {
            self.as_mut().rust_mut().bump_playing_version();
            self.playing_state_changed();
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
        self.playing_state_changed();
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
        self.rust().playing_slots.contains(&slot)
    }

    pub fn mic_source_name_at(&self, index: i32) -> QString {
        let name = MIC_SOURCES.get().and_then(|store| {
            store
                .lock()
                .ok()
                .and_then(|sources| {
                    sources
                        .get(index as usize)
                        .map(|source| source.description.clone())
                })
        }).unwrap_or_default();
        QString::from(name.as_str())
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
        let config = crate::config::load_config().unwrap_or_default();
        let state_path = crate::config::state_path(&config);
        let saved = crate::state::State::load(&state_path).unwrap_or_default();
        let mut rust = self.as_mut().rust_mut();
        rust.tabs_root = config.paths.tabs_root.clone();
        rust.state_path = state_path;
        let tabs = TabsRepository::scan(&config).unwrap_or_default();
        rust.replace_tabs(tabs, Some(&saved.current_tab));
        rust.refresh_mic_source_count();
    }
}
