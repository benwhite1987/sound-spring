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
        #[qproperty(QString, mic_source)]
        #[qproperty(QString, monitor_sink)]
        #[qproperty(i32, latency_ms)]
        #[qproperty(bool, auto_teardown)]
        #[qproperty(QString, interruption_mode)]
        #[qproperty(bool, mute_mic_during_playback)]
        #[qproperty(QString, tabs_root)]
        #[qproperty(QString, state_dir)]
        #[qproperty(QString, shortcut_mode)]
        #[qproperty(bool, ignore_numlock)]
        #[qproperty(bool, minimize_to_tray)]
        #[qproperty(bool, launch_at_login)]
        #[qproperty(i32, custom_tab_count)]
        #[qproperty(QString, status_message)]
        #[qproperty(i32, shortcut_count)]
        type Settings = super::SettingsRust;

        #[qinvokable]
        fn load_from_config(self: Pin<&mut Settings>);

        #[qinvokable]
        fn apply(self: Pin<&mut Settings>);

        #[qinvokable]
        fn custom_tab_path_at(self: &Settings, index: i32) -> QString;

        #[qinvokable]
        fn custom_tab_name_at(self: &Settings, index: i32) -> QString;

        #[qinvokable]
        fn add_custom_tab(self: Pin<&mut Settings>, path: QString, name: QString);

        #[qinvokable]
        fn remove_custom_tab(self: Pin<&mut Settings>, index: i32);

        #[qinvokable]
        fn shortcut_id_at(self: &Settings, index: i32) -> QString;

        #[qinvokable]
        fn shortcut_description_at(self: &Settings, index: i32) -> QString;

        #[qinvokable]
        fn shortcut_trigger_at(self: &Settings, index: i32) -> QString;

        #[qinvokable]
        fn set_shortcut_trigger_at(self: Pin<&mut Settings>, index: i32, trigger: QString);

        #[qinvokable]
        fn trigger_from_key_event(
            self: &Settings,
            key: i32,
            modifiers: i32,
            native_scan_code: u32,
        ) -> QString;

        #[qinvokable]
        fn shortcut_display_at(self: &Settings, index: i32) -> QString;
    }

    impl cxx_qt::Constructor<()> for Settings {}
}

use core::pin::Pin;
use cxx_qt::{Constructor, CxxQtType};
use cxx_qt_lib::QString;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::{self, Config, TabEntry};
use crate::qobjects::controller::{BackendCommand, SoundboardControllerRust, BACKEND_TX};
use crate::services::autostart;
use crate::services::shortcuts::{trigger_display, trigger_from_qt, ShortcutDef, ShortcutsManager};

#[derive(Default)]
pub struct SettingsRust {
    mic_source: QString,
    monitor_sink: QString,
    latency_ms: i32,
    auto_teardown: bool,
    interruption_mode: QString,
    mute_mic_during_playback: bool,
    tabs_root: QString,
    state_dir: QString,
    shortcut_mode: QString,
    ignore_numlock: bool,
    minimize_to_tray: bool,
    launch_at_login: bool,
    custom_tab_count: i32,
    status_message: QString,
    shortcut_count: i32,
    custom_tabs: Vec<TabEntry>,
    shortcuts: Vec<ShortcutDef>,
}

impl SettingsRust {
    fn set_status(&mut self, message: &str) {
        self.status_message = QString::from(message);
    }

    fn build_config(&self) -> Config {
        let mut config = config::load_config().unwrap_or_default();
        let mut bindings = HashMap::new();
        for def in &self.shortcuts {
            let default_trigger = ShortcutsManager::default_bindings()
                .into_iter()
                .find(|d| d.id == def.id)
                .map(|d| d.trigger)
                .unwrap_or_default();
            if def.trigger != default_trigger {
                bindings.insert(def.id.clone(), def.trigger.clone());
            }
        }

        config.audio.mic_source = String::from(self.mic_source.clone());
        config.audio.monitor_sink = String::from(self.monitor_sink.clone());
        config.audio.latency_ms = self.latency_ms.max(10) as u32;
        config.audio.auto_teardown = self.auto_teardown;
        config.audio.interruption_mode =
            String::from(self.interruption_mode.clone());
        if config.audio.interruption_mode != "interrupt" {
            config.audio.interruption_mode = "overlap".to_string();
        }
        config.audio.mute_mic_during_playback = self.mute_mic_during_playback;
        config.shortcuts.mode = String::from(self.shortcut_mode.clone());
        config.shortcuts.bindings = bindings;
        config.shortcuts.ignore_numlock = self.ignore_numlock;
        config.ui.minimize_to_tray = self.minimize_to_tray;
        config.ui.launch_at_login = self.launch_at_login;
        config.paths.tabs_root = PathBuf::from(String::from(self.tabs_root.clone()));
        config.paths.state_dir = PathBuf::from(String::from(self.state_dir.clone()));
        config.tabs = self.custom_tabs.clone();
        config::normalize_shortcuts_config(&mut config);
        config
    }

    fn apply_config_to_fields(&mut self, config: &Config) {
        self.mic_source = QString::from(config.audio.mic_source.as_str());
        self.monitor_sink = QString::from(config.audio.monitor_sink.as_str());
        self.latency_ms = config.audio.latency_ms as i32;
        self.auto_teardown = config.audio.auto_teardown;
        self.interruption_mode = QString::from(config.audio.interruption_mode.as_str());
        self.mute_mic_during_playback = config.audio.mute_mic_during_playback;
        self.tabs_root = QString::from(config.paths.tabs_root.to_string_lossy().as_ref());
        self.state_dir = QString::from(config.paths.state_dir.to_string_lossy().as_ref());
        self.shortcut_mode = QString::from(config.shortcuts.mode.as_str());
        self.ignore_numlock = config.shortcuts.ignore_numlock;
        self.minimize_to_tray = config.ui.minimize_to_tray;
        self.launch_at_login = config.ui.launch_at_login;
        self.custom_tabs = config.tabs.clone();
        self.custom_tab_count = self.custom_tabs.len() as i32;
        self.shortcuts = ShortcutsManager::resolve_bindings(&config.shortcuts);
        self.shortcut_count = self.shortcuts.len() as i32;
    }
}

impl qobject::Settings {
    pub fn load_from_config(mut self: Pin<&mut Self>) {
        let config = config::load_config().unwrap_or_default();
        self.as_mut().rust_mut().apply_config_to_fields(&config);
        SoundboardControllerRust::sync_shortcut_bindings(&self.rust().shortcuts);
        self.as_mut().rust_mut().set_status("Settings loaded.");
    }

    pub fn apply(mut self: Pin<&mut Self>) {
        let shortcuts = self.rust().shortcuts.clone();
        SoundboardControllerRust::sync_shortcut_bindings(&shortcuts);
        let mut config = self.rust().build_config();
        match config::ensure_default_layout(&mut config).and_then(|_| config::save_config(&config))
        {
            Ok(()) => {
                let mut status = "Settings saved. Audio and shortcuts will reload.".to_string();
                if let Err(err) = autostart::sync_launch_at_login(config.ui.launch_at_login) {
                    status = format!("Settings saved, but autostart update failed: {err:#}");
                }
                if let Some(tx) = BACKEND_TX.get() {
                    let _ = tx.blocking_send(BackendCommand::ApplyConfig(config));
                }
                self.as_mut().rust_mut().set_status(&status);
            }
            Err(err) => self
                .as_mut()
                .rust_mut()
                .set_status(&format!("Failed to save settings: {err:#}")),
        }
    }

    pub fn custom_tab_path_at(&self, index: i32) -> QString {
        QString::from(
            self.rust()
                .custom_tabs
                .get(index as usize)
                .map(|tab| tab.path.to_string_lossy().into_owned())
                .unwrap_or_default()
                .as_str(),
        )
    }

    pub fn custom_tab_name_at(&self, index: i32) -> QString {
        QString::from(
            self.rust()
                .custom_tabs
                .get(index as usize)
                .and_then(|tab| tab.name.as_deref())
                .unwrap_or(""),
        )
    }

    pub fn add_custom_tab(mut self: Pin<&mut Self>, path: QString, name: QString) {
        let path = PathBuf::from(String::from(path));
        if path.as_os_str().is_empty() {
            return;
        }
        let name = String::from(name);
        let entry = TabEntry {
            path,
            name: if name.is_empty() { None } else { Some(name) },
        };
        self.as_mut().rust_mut().custom_tabs.push(entry);
        self.as_mut().rust_mut().custom_tab_count = self.rust().custom_tabs.len() as i32;
    }

    pub fn remove_custom_tab(mut self: Pin<&mut Self>, index: i32) {
        let idx = index as usize;
        if idx < self.rust().custom_tabs.len() {
            self.as_mut().rust_mut().custom_tabs.remove(idx);
            self.as_mut().rust_mut().custom_tab_count = self.rust().custom_tabs.len() as i32;
        }
    }

    pub fn shortcut_id_at(&self, index: i32) -> QString {
        QString::from(
            self.rust()
                .shortcuts
                .get(index as usize)
                .map(|def| def.id.as_str())
                .unwrap_or(""),
        )
    }

    pub fn shortcut_description_at(&self, index: i32) -> QString {
        QString::from(
            self.rust()
                .shortcuts
                .get(index as usize)
                .map(|def| def.description.as_str())
                .unwrap_or(""),
        )
    }

    pub fn shortcut_trigger_at(&self, index: i32) -> QString {
        QString::from(
            self.rust()
                .shortcuts
                .get(index as usize)
                .map(|def| def.trigger.as_str())
                .unwrap_or(""),
        )
    }

    pub fn set_shortcut_trigger_at(mut self: Pin<&mut Self>, index: i32, trigger: QString) {
        let idx = index as usize;
        if idx < self.rust().shortcuts.len() {
            self.as_mut().rust_mut().shortcuts[idx].trigger = String::from(trigger);
            SoundboardControllerRust::sync_shortcut_bindings(&self.rust().shortcuts);
        }
    }

    pub fn trigger_from_key_event(
        &self,
        key: i32,
        modifiers: i32,
        native_scan_code: u32,
    ) -> QString {
        QString::from(
            trigger_from_qt(key, modifiers, native_scan_code)
                .unwrap_or_default()
                .as_str(),
        )
    }

    pub fn shortcut_display_at(&self, index: i32) -> QString {
        let label = self
            .rust()
            .shortcuts
            .get(index as usize)
            .map(|def| trigger_display(&def.trigger))
            .unwrap_or_default();
        QString::from(label.as_str())
    }
}

impl Constructor<()> for qobject::Settings {
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

    fn new((): ()) -> SettingsRust {
        SettingsRust::default()
    }

    fn initialize(mut self: Pin<&mut Self>, (): ()) {
        let config = config::load_config().unwrap_or_default();
        self.as_mut().rust_mut().apply_config_to_fields(&config);
    }
}
