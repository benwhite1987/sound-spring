use crate::config::Config;
use crate::services::audio_meta;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::warn;

pub const MAX_SLOTS: usize = 10;

static AUDIO_EXTENSIONS: &[&str] = &["ogg", "oga", "opus", "wav", "flac", "mp3", "m4a", "aac"];

pub fn normalize_slot(slot: i32) -> Option<usize> {
    match slot {
        0 | 10 => Some(10),
        1..=9 => Some(slot as usize),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SoundFile {
    pub path: PathBuf,
    pub name: String,
    #[serde(default)]
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tab {
    pub path: PathBuf,
    pub name: String,
    pub sounds: Vec<SoundFile>,
}

impl Tab {
    pub fn slot(&self, slot: usize) -> Option<&PathBuf> {
        if slot == 0 || slot > MAX_SLOTS {
            return None;
        }
        self.sounds.get(slot - 1).map(|s| &s.path)
    }

    pub fn slot_duration_ms(&self, slot: usize) -> Option<u64> {
        if slot == 0 || slot > MAX_SLOTS {
            return None;
        }
        self.sounds.get(slot - 1).map(|s| s.duration_ms)
    }

    pub fn display_name(&self) -> &str {
        &self.name
    }
}

pub struct TabsRepository;

impl TabsRepository {
    pub fn scan(config: &Config) -> Result<Vec<Tab>> {
        let paths = Self::tab_paths(config)?;
        let mut tabs = paths
            .into_iter()
            .filter_map(|path| match Self::scan_tab_dir(&path) {
                Ok(tab) => Some(tab),
                Err(err) => {
                    warn!("skip tab {}: {err:#}", path.display());
                    None
                }
            })
            .collect::<Vec<_>>();
        tabs.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(tabs)
    }

    pub fn tab_paths(config: &Config) -> Result<Vec<PathBuf>> {
        if !config.tabs.is_empty() {
            return Ok(config
                .tabs
                .iter()
                .map(|entry| entry.path.clone())
                .filter(|path| path.is_dir())
                .collect());
        }
        let root = &config.paths.tabs_root;
        if !root.is_dir() {
            return Ok(Vec::new());
        }
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                paths.push(entry.path());
            }
        }
        paths.sort();
        Ok(paths)
    }

    pub fn scan_tab_dir(path: &Path) -> Result<Tab> {
        let name = tab_name_from_path(path);
        let mut sounds = Vec::new();
        for entry in std::fs::read_dir(path).with_context(|| format!("read {}", path.display()))? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let file_path = entry.path();
            if !is_audio_file(&file_path) {
                continue;
            }
            sounds.push(SoundFile {
                name: file_name(&file_path),
                path: file_path.clone(),
                duration_ms: audio_meta::probe_duration_ms(&file_path).unwrap_or(0),
            });
        }
        sounds.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(Tab { path: path.to_path_buf(), name, sounds })
    }

    pub fn resolve_current_tab<'a>(tabs: &'a [Tab], current: &str, tabs_root: &Path) -> Option<&'a Tab> {
        if current.is_empty() {
            return tabs.first();
        }
        if current.starts_with('/') {
            return tabs.iter().find(|tab| tab.path == Path::new(current));
        }
        tabs.iter()
            .find(|tab| tab.name == current || tab.path.file_name().and_then(|s| s.to_str()) == Some(current))
            .or_else(|| {
                let candidate = tabs_root.join(current);
                tabs.iter().find(|tab| tab.path == candidate)
            })
    }
}

pub fn tab_name_from_path(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    strip_order_prefix(&file_name)
}

pub fn strip_order_prefix(name: &str) -> String {
    let Some((prefix, rest)) = name.split_once('-') else {
        return name.to_string();
    };
    if prefix.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
        rest.to_string()
    } else {
        name.to_string()
    }
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_prefix() {
        assert_eq!(strip_order_prefix("01-memes"), "memes");
        assert_eq!(strip_order_prefix("memes"), "memes");
    }

    #[test]
    fn tab_slot_mapping() {
        let tab = Tab {
            path: PathBuf::from("/tmp/t"),
            name: "t".into(),
            sounds: (0..11)
                .map(|i| SoundFile {
                    path: PathBuf::from(format!("/tmp/{i:02}.ogg")),
                    name: format!("{i:02}.ogg"),
                    duration_ms: 0,
                })
                .collect(),
        };
        assert_eq!(tab.slot(1).unwrap(), &PathBuf::from("/tmp/00.ogg"));
        assert_eq!(tab.slot(10).unwrap(), &PathBuf::from("/tmp/09.ogg"));
        assert!(tab.slot(11).is_none());
    }
}
