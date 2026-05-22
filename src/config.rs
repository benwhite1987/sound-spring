use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_LATENCY_MS: u32 = 20;
pub const SFX_SINK: &str = "soundboard_sfx";

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default = "default_latency_ms")]
    pub latency_ms: u32,
    #[serde(default = "default_true")]
    pub auto_teardown: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutsConfig {
    #[serde(default = "default_shortcut_mode")]
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_true")]
    pub minimize_to_tray: bool,
    #[serde(default)]
    pub launch_at_login: bool,
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
            latency_ms: DEFAULT_LATENCY_MS,
            auto_teardown: true,
        }
    }
}

impl Default for ShortcutsConfig {
    fn default() -> Self {
        Self {
            mode: default_shortcut_mode(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            minimize_to_tray: true,
            launch_at_login: false,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            audio: AudioConfig::default(),
            shortcuts: ShortcutsConfig::default(),
            ui: UiConfig::default(),
            paths: PathsConfig::default(),
            tabs: Vec::new(),
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
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read config {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse config {}", path.display()))
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
