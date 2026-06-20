use crate::config::Config;
use crate::services::audio_meta;
use anyhow::{Context, Result};
use notify::event::{CreateKind, Event, EventKind, ModifyKind};
use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc as std_mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender as TokioSender;
use tracing::warn;

pub const MAX_SLOTS: usize = 10;
pub const TAB_WATCH_DEBOUNCE_MS: u64 = 300;

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
        Ok(Tab {
            path: path.to_path_buf(),
            name,
            sounds,
        })
    }

    pub fn resolve_current_tab<'a>(
        tabs: &'a [Tab],
        current: &str,
        tabs_root: &Path,
    ) -> Option<&'a Tab> {
        if current.is_empty() {
            return tabs.first();
        }
        if current.starts_with('/') {
            return tabs.iter().find(|tab| tab.path == Path::new(current));
        }
        tabs.iter()
            .find(|tab| {
                tab.name == current
                    || tab.path.file_name().and_then(|s| s.to_str()) == Some(current)
            })
            .or_else(|| {
                let candidate = tabs_root.join(current);
                tabs.iter().find(|tab| tab.path == candidate)
            })
    }
}

/// Paths that should be watched for tab content changes.
pub fn watch_paths(config: &Config) -> Vec<PathBuf> {
    if !config.tabs.is_empty() {
        return config
            .tabs
            .iter()
            .map(|entry| entry.path.clone())
            .filter(|path| path.is_dir())
            .collect();
    }
    let root = config.paths.tabs_root.clone();
    if root.is_dir() {
        vec![root]
    } else {
        Vec::new()
    }
}

/// Owns a background thread with a debounced `notify` watcher.
pub struct TabFilesystemWatch {
    shutdown_tx: Option<std_mpsc::Sender<()>>,
    join: Option<JoinHandle<()>>,
}

impl TabFilesystemWatch {
    pub fn new() -> Self {
        Self {
            shutdown_tx: None,
            join: None,
        }
    }

    pub fn restart(&mut self, paths: Vec<PathBuf>, notify_tx: TokioSender<()>) {
        self.stop();
        if paths.is_empty() {
            return;
        }
        let (shutdown_tx, shutdown_rx) = std_mpsc::channel();
        let join = thread::spawn(move || run_tab_filesystem_watch(paths, notify_tx, shutdown_rx));
        self.shutdown_tx = Some(shutdown_tx);
        self.join = Some(join);
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for TabFilesystemWatch {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_tab_filesystem_watch(
    paths: Vec<PathBuf>,
    notify_tx: TokioSender<()>,
    shutdown_rx: std_mpsc::Receiver<()>,
) {
    let (event_tx, event_rx) = std_mpsc::channel();
    let mut watcher = match RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if is_tab_content_change(&event.kind) {
                    let _ = event_tx.send(());
                }
            }
        },
        NotifyConfig::default(),
    ) {
        Ok(watcher) => watcher,
        Err(err) => {
            warn!("tab filesystem watcher failed to start: {err:#}");
            return;
        }
    };

    for path in &paths {
        if let Err(err) = watcher.watch(path, RecursiveMode::Recursive) {
            warn!("watch {}: {err:#}", path.display());
        }
    }

    let debounce = Duration::from_millis(TAB_WATCH_DEBOUNCE_MS);
    let poll = Duration::from_millis(50);
    let mut deadline: Option<Instant> = None;

    loop {
        while event_rx.try_recv().is_ok() {
            deadline = Some(Instant::now() + debounce);
        }
        if shutdown_rx.try_recv().is_ok() {
            break;
        }
        if let Some(until) = deadline {
            if Instant::now() >= until {
                let _ = notify_tx.blocking_send(());
                deadline = None;
            }
        }
        thread::sleep(poll);
    }
}

fn is_tab_content_change(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_)
            | EventKind::Remove(_)
            | EventKind::Modify(
                ModifyKind::Data(_)
                    | ModifyKind::Name(_)
                    | ModifyKind::Metadata(_)
            )
    )
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

    #[test]
    fn watch_paths_uses_tabs_root_when_no_custom_tabs() {
        let dir = std::env::temp_dir().join("sound_spring_watch_paths_root");
        std::fs::create_dir_all(&dir).unwrap();
        let config = Config {
            paths: crate::config::PathsConfig {
                tabs_root: dir.clone(),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(watch_paths(&config), vec![dir.clone()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn watch_paths_uses_custom_tab_dirs() {
        let dir = std::env::temp_dir().join("sound_spring_watch_paths_custom");
        std::fs::create_dir_all(&dir).unwrap();
        let config = Config {
            tabs: vec![
                crate::config::TabEntry {
                    path: dir.clone(),
                    name: Some("Memes".into()),
                },
                crate::config::TabEntry {
                    path: PathBuf::from("/missing"),
                    name: None,
                },
            ],
            ..Default::default()
        };
        assert_eq!(watch_paths(&config), vec![dir.clone()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn is_tab_content_change_filters_access_events() {
        assert!(!is_tab_content_change(&EventKind::Access(
            notify::event::AccessKind::Read
        )));
        assert!(is_tab_content_change(&EventKind::Create(CreateKind::File)));
    }
}
