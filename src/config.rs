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
    pub voice: VoiceConfig,
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
    #[serde(default = "default_volume")]
    pub mic_volume: u8,
    #[serde(default)]
    pub output_muted: bool,
    #[serde(default)]
    pub monitor_muted: bool,
    /// `overlap` allows multiple sounds at once; `interrupt` stops every
    /// currently playing sound before starting a new one.
    #[serde(default = "default_interruption_mode")]
    pub interruption_mode: String,
    #[serde(default)]
    pub mute_mic_during_playback: bool,
    /// User-requested mute of the configured mic source (Voice panel).
    #[serde(default)]
    pub mic_muted: bool,
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
    #[serde(default)]
    pub close_action_prompt_dismissed: bool,
}

/// Phase 2 voice-enhancement settings. Only `spectrum_fps` is consumed in
/// Milestone 1 (the live spectrum visualization); the remaining fields are
/// persisted now for forward compatibility with later milestones (VAD,
/// speaker verification, DeepFilterNet denoise, enrollment).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    #[serde(default)]
    pub verification_enabled: bool,
    #[serde(default)]
    pub suppression_enabled: bool,
    #[serde(default = "default_suppression_model")]
    pub suppression_model: String,
    #[serde(default = "default_match_threshold")]
    pub match_threshold: f32,
    #[serde(default = "default_vad_open_threshold")]
    pub vad_open_threshold: f32,
    #[serde(default = "default_vad_close_threshold")]
    pub vad_close_threshold: f32,
    #[serde(default = "default_enrollment_path")]
    pub enrollment_path: String,
    #[serde(default = "default_spectrum_fps")]
    pub spectrum_fps: u32,
    #[serde(default = "default_true")]
    pub vad_enabled: bool,
    /// Spectrum display source: `raw`, `filtered`, or `mixed`.
    #[serde(default = "default_spectrum_source")]
    pub spectrum_source: String,
    /// Hold the output gate open this long after VAD drops (ms).
    #[serde(default = "default_gate_hangover_ms")]
    pub gate_hangover_ms: u32,
    /// Output gate fade-out time when closing (ms).
    #[serde(default = "default_gate_release_ms")]
    pub gate_release_ms: u32,
    /// Pass audio through until the first failed speaker check.
    #[serde(default = "default_true")]
    pub verification_warmup: bool,
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
            mic_volume: default_volume(),
            output_muted: false,
            monitor_muted: false,
            interruption_mode: default_interruption_mode(),
            mute_mic_during_playback: false,
            mic_muted: false,
        }
    }
}

fn default_interruption_mode() -> String {
    "overlap".into()
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
            close_action_prompt_dismissed: false,
        }
    }
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            verification_enabled: false,
            suppression_enabled: false,
            suppression_model: default_suppression_model(),
            match_threshold: default_match_threshold(),
            vad_open_threshold: default_vad_open_threshold(),
            vad_close_threshold: default_vad_close_threshold(),
            enrollment_path: default_enrollment_path(),
            spectrum_fps: default_spectrum_fps(),
            vad_enabled: true,
            spectrum_source: default_spectrum_source(),
            gate_hangover_ms: default_gate_hangover_ms(),
            gate_release_ms: default_gate_release_ms(),
            verification_warmup: true,
        }
    }
}

fn default_spectrum_source() -> String {
    "raw".into()
}

fn default_suppression_model() -> String {
    "deepfilternet3".into()
}

fn default_match_threshold() -> f32 {
    0.6
}

fn default_vad_open_threshold() -> f32 {
    0.45
}

fn default_vad_close_threshold() -> f32 {
    0.20
}

fn default_gate_hangover_ms() -> u32 {
    200
}

fn default_gate_release_ms() -> u32 {
    100
}

fn default_enrollment_path() -> String {
    "voiceprints/default.bin".into()
}

fn default_spectrum_fps() -> u32 {
    30
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

/// Config directory, falling back to `.config/soundboard` if the platform dirs
/// can't be resolved.
pub fn config_dir() -> PathBuf {
    project_dirs()
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".config/soundboard"))
}

/// Absolute path of the enrolled voiceprint, resolving `voice.enrollment_path`
/// (relative entries are taken under the config dir).
pub fn voiceprint_path(config: &Config) -> PathBuf {
    let raw = PathBuf::from(&config.voice.enrollment_path);
    if raw.is_absolute() {
        raw
    } else {
        config_dir().join(raw)
    }
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
        assert_eq!(parsed.audio.interruption_mode, "overlap");
        assert_eq!(parsed.voice.spectrum_fps, 30);
        assert_eq!(parsed.voice.suppression_model, "deepfilternet3");
        assert!(!parsed.voice.suppression_enabled);
        assert!(!parsed.voice.verification_enabled);
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
