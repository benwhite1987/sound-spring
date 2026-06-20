use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_LATENCY_MS: u32 = 20;
pub const SFX_SINK: &str = "soundboard_sfx";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub audio: AudioConfig,
    #[serde(default)]
    pub shortcuts: ShortcutsConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub paths: PathsConfig,
    #[serde(default)]
    pub tabs: Vec<TabEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    #[serde(default)]
    pub mic_source: String,
    /// Local monitor output device sink name. Empty follows the system default
    /// output device (`@DEFAULT_SINK@`).
    #[serde(default)]
    pub monitor_sink: String,
    #[serde(default = "default_latency_ms")]
    pub latency_ms: u32,
    #[serde(default = "default_true")]
    pub auto_teardown: bool,
    #[serde(default = "default_volume")]
    pub output_volume: u8,
    #[serde(default = "default_volume")]
    pub monitor_volume: u8,
    #[serde(default)]
    pub output_muted: bool,
    #[serde(default)]
    pub monitor_muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutsConfig {
    #[serde(default = "default_shortcut_mode")]
    pub mode: String,
    #[serde(default)]
    pub bindings: HashMap<String, String>,
    /// When true, every numpad-digit / numpad-decimal binding is registered
    /// in two forms: the standard NumLock-ON keysym (e.g. KP_1) and its
    /// NumLock-OFF equivalent (e.g. KP_End). Lets users trigger numpad
    /// shortcuts regardless of NumLock state at the cost of also occupying
    /// the navigation-cluster slot in KDE's shortcut table.
    #[serde(default)]
    pub ignore_numlock: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_true")]
    pub minimize_to_tray: bool,
    #[serde(default)]
    pub launch_at_login: bool,
    #[serde(default)]
    pub global_shortcuts_prompt_dismissed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub tabs_root: PathBuf,
    pub state_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabEntry {
    pub path: PathBuf,
    #[serde(default)]
    pub name: Option<String>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            mic_source: String::new(),
            monitor_sink: String::new(),
            latency_ms: DEFAULT_LATENCY_MS,
            auto_teardown: true,
            output_volume: default_volume(),
            monitor_volume: default_volume(),
            output_muted: false,
            monitor_muted: false,
        }
    }
}

fn default_volume() -> u8 {
    100
}

impl Default for ShortcutsConfig {
    fn default() -> Self {
        Self {
            mode: default_shortcut_mode(),
            bindings: HashMap::new(),
            ignore_numlock: false,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            minimize_to_tray: true,
            launch_at_login: false,
            global_shortcuts_prompt_dismissed: false,
        }
    }
}

impl Default for PathsConfig {
    fn default() -> Self {
        let dirs = project_dirs().expect("home directory");
        Self {
            tabs_root: dirs.config_dir().join("tabs"),
            state_dir: dirs.cache_dir().to_path_buf(),
        }
    }
}

fn default_latency_ms() -> u32 {
    DEFAULT_LATENCY_MS
}

fn default_true() -> bool {
    true
}

fn default_shortcut_mode() -> String {
    "portal".into()
}

pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "soundboard")
}

pub fn config_path() -> PathBuf {
    project_dirs()
        .map(|d| d.config_dir().join("config.toml"))
        .unwrap_or_else(|| PathBuf::from(".config/soundboard/config.toml"))
}

pub fn load_config() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let text =
        fs::read_to_string(&path).with_context(|| format!("read config {}", path.display()))?;
    let mut config: Config =
        toml::from_str(&text).with_context(|| format!("parse config {}", path.display()))?;
    normalize_shortcuts_config(&mut config);
    Ok(config)
}

/// Migrate legacy shortcut modes that destabilize Plasma or duplicate in-window keys.
pub fn normalize_shortcuts_config(config: &mut Config) {
    match config.shortcuts.mode.as_str() {
        "kglobalaccel" | "auto" => config.shortcuts.mode = "portal".to_string(),
        _ => {}
    }
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create config dir {}", parent.display()))?;
    }
    let text = toml::to_string_pretty(config).context("serialize config")?;
    fs::write(&path, text).with_context(|| format!("write config {}", path.display()))
}

pub fn state_path(config: &Config) -> PathBuf {
    config.paths.state_dir.join("state.json")
}

pub fn ensure_default_layout(config: &mut Config) -> Result<()> {
    fs::create_dir_all(&config.paths.tabs_root)
        .with_context(|| format!("create tabs root {}", config.paths.tabs_root.display()))?;
    fs::create_dir_all(&config.paths.state_dir)
        .with_context(|| format!("create state dir {}", config.paths.state_dir.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_roundtrip() {
        let config = Config::default();
        let text = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&text).unwrap();
        assert_eq!(parsed.audio.latency_ms, DEFAULT_LATENCY_MS);
    }

    #[test]
    fn parse_tabs_block() {
        use std::path::Path;

        let text = r#"
[audio]
mic_source = ""
latency_ms = 20

[paths]
tabs_root = "/tmp/tabs"
state_dir = "/tmp/state"

[[tabs]]
path = "/tmp/custom"
name = "Custom"
"#;
        let parsed: Config = toml::from_str(text).unwrap();
        assert_eq!(parsed.tabs.len(), 1);
        assert_eq!(parsed.tabs[0].path, Path::new("/tmp/custom"));
    }
}
