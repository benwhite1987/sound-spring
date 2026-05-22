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
        #[qproperty(i32, latency_ms)]
        #[qproperty(bool, auto_teardown)]
        #[qproperty(QString, tabs_root)]
        #[qproperty(QString, state_dir)]
        #[qproperty(QString, shortcut_mode)]
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
        fn shortcut_description_at(self: &Settings, index: i32) -> QString;

        #[qinvokable]
        fn shortcut_trigger_at(self: &Settings, index: i32) -> QString;
    }

    impl cxx_qt::Constructor<()> for Settings {}
}

use core::pin::Pin;
use cxx_qt::{Constructor, CxxQtType};
use cxx_qt_lib::QString;
use std::path::PathBuf;

use crate::config::{self, Config, TabEntry};
use crate::qobjects::controller::{BackendCommand, BACKEND_TX};
use crate::services::ShortcutsManager;

#[derive(Default)]
pub struct SettingsRust {
    mic_source: QString,
    latency_ms: i32,
    auto_teardown: bool,
    tabs_root: QString,
    state_dir: QString,
    shortcut_mode: QString,
    minimize_to_tray: bool,
    launch_at_login: bool,
    custom_tab_count: i32,
    status_message: QString,
    shortcut_count: i32,
    custom_tabs: Vec<TabEntry>,
}

impl SettingsRust {
    fn set_status(&mut self, message: &str) {
        self.status_message = QString::from(message);
    }

    fn build_config(&self) -> Config {
        Config {
            audio: config::AudioConfig {
                mic_source: String::from(self.mic_source.clone()),
                latency_ms: self.latency_ms.max(10) as u32,
                auto_teardown: self.auto_teardown,
            },
            shortcuts: config::ShortcutsConfig {
                mode: String::from(self.shortcut_mode.clone()),
            },
            ui: config::UiConfig {
                minimize_to_tray: self.minimize_to_tray,
                launch_at_login: self.launch_at_login,
            },
            paths: config::PathsConfig {
                tabs_root: PathBuf::from(String::from(self.tabs_root.clone())),
                state_dir: PathBuf::from(String::from(self.state_dir.clone())),
            },
            tabs: self.custom_tabs.clone(),
        }
    }

    fn apply_config_to_fields(&mut self, config: &Config) {
        self.mic_source = QString::from(config.audio.mic_source.as_str());
        self.latency_ms = config.audio.latency_ms as i32;
        self.auto_teardown = config.audio.auto_teardown;
        self.tabs_root = QString::from(config.paths.tabs_root.to_string_lossy().as_ref());
        self.state_dir = QString::from(config.paths.state_dir.to_string_lossy().as_ref());
        self.shortcut_mode = QString::from(config.shortcuts.mode.as_str());
        self.minimize_to_tray = config.ui.minimize_to_tray;
        self.launch_at_login = config.ui.launch_at_login;
        self.custom_tabs = config.tabs.clone();
        self.custom_tab_count = self.custom_tabs.len() as i32;
        self.shortcut_count = ShortcutsManager::default_bindings().len() as i32;
    }
}

impl qobject::Settings {
    pub fn load_from_config(mut self: Pin<&mut Self>) {
        let config = config::load_config().unwrap_or_default();
        self.as_mut().rust_mut().apply_config_to_fields(&config);
        self.as_mut()
            .rust_mut()
            .set_status("Settings loaded.");
    }

    pub fn apply(mut self: Pin<&mut Self>) {
        let mut config = self.rust().build_config();
        match config::ensure_default_layout(&mut config).and_then(|_| config::save_config(&config)) {
            Ok(()) => {
                if let Some(tx) = BACKEND_TX.get() {
                    let _ = tx.blocking_send(BackendCommand::ApplyConfig(config));
                }
                self.as_mut()
                    .rust_mut()
                    .set_status("Settings saved. Audio and shortcuts will reload.");
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
        self.as_mut().rust_mut().custom_tab_count =
            self.rust().custom_tabs.len() as i32;
    }

    pub fn remove_custom_tab(mut self: Pin<&mut Self>, index: i32) {
        let idx = index as usize;
        if idx < self.rust().custom_tabs.len() {
            self.as_mut().rust_mut().custom_tabs.remove(idx);
            self.as_mut().rust_mut().custom_tab_count =
                self.rust().custom_tabs.len() as i32;
        }
    }

    pub fn shortcut_description_at(&self, index: i32) -> QString {
        QString::from(
            ShortcutsManager::default_bindings()
                .get(index as usize)
                .map(|def| def.description.as_str())
                .unwrap_or(""),
        )
    }

    pub fn shortcut_trigger_at(&self, index: i32) -> QString {
        QString::from(
            ShortcutsManager::default_bindings()
                .get(index as usize)
                .map(|def| def.trigger.as_str())
                .unwrap_or(""),
        )
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
