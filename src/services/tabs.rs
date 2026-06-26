use crate::config::{Config, TabEntry};
use crate::services::audio_meta;
use anyhow::{Context, Result};
use notify::event::{Event, EventKind, ModifyKind};
use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::fs;
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
        self.sounds.iter().find_map(|sound| {
            slot_number_from_path(&sound.path)
                .filter(|number| *number == slot)
                .map(|_| &sound.path)
        })
    }

    pub fn sound_at_slot(&self, slot: usize) -> Option<&SoundFile> {
        if slot == 0 || slot > MAX_SLOTS {
            return None;
        }
        self.sounds.iter().find(|sound| {
            slot_number_from_path(&sound.path).map(|number| number == slot) == Some(true)
        })
    }

    pub fn display_name(&self) -> &str {
        &self.name
    }
}

pub struct TabsRepository;

impl TabsRepository {
    pub fn scan(config: &Config) -> Result<Vec<Tab>> {
        if !config.tabs.is_empty() {
            let mut tabs = Vec::new();
            for entry in &config.tabs {
                if !entry.path.is_dir() {
                    continue;
                }
                match Self::scan_tab_dir(&entry.path) {
                    Ok(mut tab) => {
                        if let Some(name) = entry.name.as_deref().filter(|name| !name.is_empty()) {
                            tab.name = name.to_string();
                        }
                        tabs.push(tab);
                    }
                    Err(err) => warn!("skip tab {}: {err:#}", entry.path.display()),
                }
            }
            return Ok(tabs);
        }

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
        let (tab, warnings) = Self::scan_tab_dir_with_warnings(path)?;
        for message in &warnings {
            warn!("{message}");
        }
        Ok(tab)
    }

    pub fn scan_tab_dir_with_warnings(path: &Path) -> Result<(Tab, Vec<String>)> {
        let name = tab_name_from_path(path);
        let mut warnings = Vec::new();

        let mut files = Vec::new();
        for entry in std::fs::read_dir(path).with_context(|| format!("read {}", path.display()))? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let file_path = entry.path();
            if !is_audio_file(&file_path) {
                continue;
            }
            files.push(file_path);
        }
        files.sort();

        let mut by_slot: [Option<PathBuf>; MAX_SLOTS] = std::array::from_fn(|_| None);
        let mut overflow = Vec::new();

        for file_path in files {
            match slot_number_from_path(&file_path) {
                Some(number) if (1..=MAX_SLOTS).contains(&number) => {
                    let index = number - 1;
                    if by_slot[index].is_some() {
                        overflow.push((number, file_path));
                    } else {
                        by_slot[index] = Some(file_path);
                    }
                }
                Some(number) => overflow.push((number, file_path)),
                None => overflow.push((usize::MAX, file_path)),
            }
        }

        overflow.sort_by(|(key_a, path_a), (key_b, path_b)| {
            key_a.cmp(key_b).then_with(|| path_a.cmp(path_b))
        });

        let total_files = by_slot.iter().filter(|slot| slot.is_some()).count() + overflow.len();
        if total_files > MAX_SLOTS {
            warnings.push(format!(
                "Ignoring {} excess audio file(s) in {} (only {MAX_SLOTS} slots available)",
                total_files - MAX_SLOTS,
                path.display()
            ));
        }
        for slot in &mut by_slot {
            if slot.is_some() || overflow.is_empty() {
                continue;
            }
            let (_, file_path) = overflow.remove(0);
            *slot = Some(file_path);
        }

        let assigned: Vec<(PathBuf, usize)> = by_slot
            .into_iter()
            .enumerate()
            .filter_map(|(index, file_path)| file_path.map(|path| (path, index + 1)))
            .collect();

        let sources: Vec<PathBuf> = assigned.iter().map(|(src, _)| src.clone()).collect();
        let sounds = match normalize_assigned_files(path, &assigned, &sources) {
            Ok(paths) => paths
                .into_iter()
                .map(|file_path| sound_file_from_path(&file_path))
                .collect(),
            Err(err) => {
                warnings.push(format!(
                    "Failed to normalize files in {}: {err:#}",
                    path.display()
                ));
                Vec::new()
            }
        };

        Ok((
            Tab {
                path: path.to_path_buf(),
                name,
                sounds,
            },
            warnings,
        ))
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

    pub fn create_tab_dir(root: &Path, display_name: &str) -> Result<PathBuf> {
        fs::create_dir_all(root).with_context(|| format!("create tabs root {}", root.display()))?;
        let segment = sanitize_tab_segment(display_name);
        let prefix = next_order_prefix(root);
        let path = root.join(format!("{prefix}-{segment}"));
        fs::create_dir_all(&path).with_context(|| format!("create tab dir {}", path.display()))?;
        Ok(path)
    }

    pub fn rename_tab_dir(path: &Path, new_display_name: &str) -> Result<PathBuf> {
        let parent = path.parent().context("tab directory has no parent path")?;
        let prefix = order_prefix_from_path(path).unwrap_or_else(|| "99".to_string());
        let segment = sanitize_tab_segment(new_display_name);
        let new_path = parent.join(format!("{prefix}-{segment}"));
        if new_path != path {
            fs::rename(path, &new_path).with_context(|| {
                format!("rename tab {} -> {}", path.display(), new_path.display())
            })?;
        }
        Ok(new_path)
    }

    /// Renumber `NN-name` folders under `tabs_root` to match `ordered_paths`.
    pub fn reorder_tabs_root(root: &Path, ordered_paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
        if ordered_paths.is_empty() {
            return Ok(Vec::new());
        }
        let mut temps = Vec::with_capacity(ordered_paths.len());
        for (index, path) in ordered_paths.iter().enumerate() {
            let temp = root.join(format!(".sound_spring_reorder_{index}"));
            if path.exists() {
                fs::rename(path, &temp).with_context(|| {
                    format!("stage tab reorder {} -> {}", path.display(), temp.display())
                })?;
            }
            temps.push(temp);
        }

        let mut final_paths = Vec::with_capacity(ordered_paths.len());
        for (index, temp) in temps.iter().enumerate() {
            let display = tab_name_from_path(&ordered_paths[index]);
            let final_path = root.join(format!(
                "{:02}-{}",
                index + 1,
                sanitize_tab_segment(&display)
            ));
            if temp.exists() {
                fs::rename(temp, &final_path).with_context(|| {
                    format!(
                        "finalize tab reorder {} -> {}",
                        temp.display(),
                        final_path.display()
                    )
                })?;
            }
            final_paths.push(final_path);
        }
        Ok(final_paths)
    }

    pub fn reorder_custom_tabs(tabs: &mut Vec<TabEntry>, from: usize, to: usize) {
        if from >= tabs.len() || to >= tabs.len() || from == to {
            return;
        }
        let entry = tabs.remove(from);
        tabs.insert(to, entry);
    }

    pub fn replace_slot_file(tab_dir: &Path, slot: usize, source: &Path) -> Result<PathBuf> {
        if !is_audio_file(source) {
            anyhow::bail!("{} is not a supported audio file", source.display());
        }
        let tab = Self::scan_tab_dir(tab_dir)?;
        if let Some(existing) = tab.slot(slot) {
            fs::copy(source, existing)
                .with_context(|| format!("replace {}", existing.display()))?;
            return Ok(existing.clone());
        }
        let dest = destination_for_empty_slot(tab_dir, slot, source)?;
        fs::copy(source, &dest).with_context(|| format!("copy into {}", dest.display()))?;
        Ok(dest)
    }

    pub fn remove_slot_file(tab_dir: &Path, slot: usize) -> Result<()> {
        let tab = Self::scan_tab_dir(tab_dir)?;
        let path = tab.slot(slot).context("slot is empty")?.clone();
        fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))
    }

    pub fn rename_slot_file(tab_dir: &Path, slot: usize, new_name: &str) -> Result<PathBuf> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            anyhow::bail!("new name is empty");
        }
        let tab = Self::scan_tab_dir(tab_dir)?;
        let existing = tab.slot(slot).context("slot is empty")?.clone();
        let ext = existing
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("ogg");
        let prefix = format!("{slot:02}");
        let segment = new_name.trim();
        if segment.contains('/') || segment.contains('\\') {
            anyhow::bail!("new name contains invalid path characters");
        }
        let new_path = tab_dir.join(format!("{prefix}-{segment}.{ext}"));
        if new_path != existing {
            fs::rename(&existing, &new_path).with_context(|| {
                format!("rename {} -> {}", existing.display(), new_path.display())
            })?;
        }
        Ok(new_path)
    }

    pub fn move_slot_file(tab_dir: &Path, from_slot: usize, to_slot: usize) -> Result<()> {
        if from_slot == 0
            || from_slot > MAX_SLOTS
            || to_slot == 0
            || to_slot > MAX_SLOTS
            || from_slot == to_slot
        {
            anyhow::bail!("invalid slot move {from_slot} -> {to_slot}");
        }
        let tab = Self::scan_tab_dir(tab_dir)?;
        let from_path = tab.slot(from_slot).context("source slot is empty")?.clone();
        match tab.slot(to_slot) {
            None => {
                let dest = path_with_slot_prefix(&from_path, tab_dir, to_slot)?;
                fs::rename(&from_path, &dest).with_context(|| {
                    format!("move {} -> {}", from_path.display(), dest.display())
                })?;
            }
            Some(to_path) => swap_slot_files(tab_dir, &from_path, to_path)?,
        }
        Ok(())
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
                ModifyKind::Data(_) | ModifyKind::Name(_) | ModifyKind::Metadata(_)
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
    parse_slot_prefix_from_stem(name)
        .map(|(_, label)| label.to_string())
        .unwrap_or_else(|| name.to_string())
}

/// Parses `01-name`, `01 name`, and similar slot prefixes from a file or tab stem.
pub fn parse_slot_prefix_from_stem(stem: &str) -> Option<(usize, &str)> {
    let stem = stem.trim();
    if stem.is_empty() {
        return None;
    }
    let digit_len: usize = stem
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .map(|c| c.len_utf8())
        .sum();
    if digit_len == 0 {
        return None;
    }
    let number = stem[..digit_len].parse::<usize>().ok()?;
    let rest = &stem[digit_len..];
    if rest.is_empty() {
        return None;
    }
    let separator = rest.chars().next()?;
    if separator != '-' && !separator.is_whitespace() {
        return None;
    }
    let label = rest[separator.len_utf8()..].trim();
    if label.is_empty() {
        return None;
    }
    Some((number, label))
}

pub fn sanitize_tab_segment(name: &str) -> String {
    let sanitized: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "tab".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn order_prefix_from_path(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let (prefix, rest) = name.split_once('-')?;
    if prefix.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
        Some(prefix.to_string())
    } else {
        None
    }
}

pub fn next_order_prefix(root: &Path) -> String {
    let mut max = 0u32;
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            if let Some(prefix) = order_prefix_from_path(&entry.path()) {
                if let Ok(value) = prefix.parse::<u32>() {
                    max = max.max(value);
                }
            }
        }
    }
    format!("{:02}", max.saturating_add(1))
}

pub fn uses_custom_tabs(config: &Config) -> bool {
    !config.tabs.is_empty()
}

fn destination_for_empty_slot(tab_dir: &Path, slot: usize, source: &Path) -> Result<PathBuf> {
    let ext = source
        .extension()
        .and_then(|value| value.to_str())
        .context("audio file has no extension")?;
    let label = label_from_path(source);
    unique_canonical_path(tab_dir, slot, &label, ext, &[])
}

pub fn slot_number_from_path(path: &Path) -> Option<usize> {
    path.file_stem()
        .and_then(|value| value.to_str())
        .and_then(|stem| parse_slot_prefix_from_stem(stem).map(|(number, _)| number))
}

fn label_from_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("sound");
    parse_slot_prefix_from_stem(stem)
        .map(|(_, label)| label.to_string())
        .unwrap_or_else(|| stem.to_string())
}

fn canonical_sound_filename(slot: usize, label: &str, ext: &str) -> String {
    format!("{slot:02}-{label}.{ext}")
}

fn canonical_path_for_slot(
    tab_dir: &Path,
    src: &Path,
    slot: usize,
    moving_sources: &[PathBuf],
) -> Result<PathBuf> {
    let ext = src
        .extension()
        .and_then(|value| value.to_str())
        .context("audio file has no extension")?;
    let label = label_from_path(src);
    unique_canonical_path(tab_dir, slot, &label, ext, moving_sources)
}

fn unique_canonical_path(
    tab_dir: &Path,
    slot: usize,
    label: &str,
    ext: &str,
    moving_sources: &[PathBuf],
) -> Result<PathBuf> {
    let mut dest = tab_dir.join(canonical_sound_filename(slot, label, ext));
    let mut suffix = 1u32;
    while dest.exists() && !moving_sources.iter().any(|path| path == &dest) {
        dest = tab_dir.join(format!("{slot:02}-{label}-{suffix}.{ext}"));
        suffix += 1;
    }
    Ok(dest)
}

fn normalize_assigned_files(
    tab_dir: &Path,
    assigned: &[(PathBuf, usize)],
    sources: &[PathBuf],
) -> Result<Vec<PathBuf>> {
    let plans: Vec<(PathBuf, PathBuf)> = assigned
        .iter()
        .map(|(src, slot)| {
            let dest = canonical_path_for_slot(tab_dir, src, *slot, sources)?;
            Ok((src.clone(), dest))
        })
        .collect::<Result<_>>()?;

    let mut staged = Vec::new();
    for (index, (src, dest)) in plans.iter().enumerate() {
        if src == dest {
            continue;
        }
        let temp = tab_dir.join(format!(".sound_spring_norm_{index}"));
        fs::rename(src, &temp)
            .with_context(|| format!("stage normalize {} -> {}", src.display(), temp.display()))?;
        staged.push((temp, dest.clone()));
    }

    for (temp, dest) in staged {
        fs::rename(&temp, &dest).with_context(|| {
            format!(
                "finalize normalize {} -> {}",
                temp.display(),
                dest.display()
            )
        })?;
    }

    Ok(plans.into_iter().map(|(_, dest)| dest).collect())
}

fn display_name_for_path(path: &Path) -> String {
    label_from_path(path)
}

fn sound_file_from_path(path: &Path) -> SoundFile {
    SoundFile {
        name: display_name_for_path(path),
        path: path.to_path_buf(),
        duration_ms: audio_meta::probe_duration_ms(path).unwrap_or(0),
    }
}

fn path_with_slot_prefix(source: &Path, tab_dir: &Path, slot: usize) -> Result<PathBuf> {
    canonical_path_for_slot(tab_dir, source, slot, &[source.to_path_buf()])
}

fn swap_slot_files(tab_dir: &Path, path_a: &Path, path_b: &Path) -> Result<()> {
    let slot_a = slot_number_from_path(path_a).context("source file has no slot prefix")?;
    let slot_b = slot_number_from_path(path_b).context("target file has no slot prefix")?;
    let sources = vec![path_a.to_path_buf(), path_b.to_path_buf()];
    let dest_b = canonical_path_for_slot(tab_dir, path_a, slot_b, &sources)?;
    let dest_a = canonical_path_for_slot(tab_dir, path_b, slot_a, &sources)?;
    let temp_a = tab_dir.join(".sound_spring_swap_a");
    let temp_b = tab_dir.join(".sound_spring_swap_b");
    fs::rename(path_a, &temp_a)
        .with_context(|| format!("stage swap {} -> {}", path_a.display(), temp_a.display()))?;
    fs::rename(path_b, &temp_b)
        .with_context(|| format!("stage swap {} -> {}", path_b.display(), temp_b.display()))?;
    fs::rename(&temp_a, &dest_b)
        .with_context(|| format!("finalize swap {} -> {}", temp_a.display(), dest_b.display()))?;
    fs::rename(&temp_b, &dest_a)
        .with_context(|| format!("finalize swap {} -> {}", temp_b.display(), dest_a.display()))?;
    Ok(())
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_prefix() {
        assert_eq!(strip_order_prefix("01-memes"), "memes");
        assert_eq!(strip_order_prefix("02-Music"), "Music");
        assert_eq!(strip_order_prefix("01 memes"), "memes");
        assert_eq!(strip_order_prefix("memes"), "memes");
    }

    #[test]
    fn slot_number_accepts_space_separated_prefix() {
        assert_eq!(
            slot_number_from_path(&PathBuf::from("/tmp/01 airhorn.ogg")),
            Some(1)
        );
        assert_eq!(
            slot_number_from_path(&PathBuf::from("/tmp/03-third.ogg")),
            Some(3)
        );
        assert_eq!(
            slot_number_from_path(&PathBuf::from("/tmp/airhorn.ogg")),
            None
        );
    }

    #[test]
    fn scan_renames_space_separated_prefixes() {
        let dir = std::env::temp_dir().join("sound_spring_scan_space_prefix");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("01 airhorn.ogg"), b"fake").unwrap();
        fs::write(dir.join("02 bruh.ogg"), b"fake").unwrap();
        let (tab, warnings) = TabsRepository::scan_tab_dir_with_warnings(&dir).unwrap();
        assert!(warnings.is_empty());
        assert!(dir.join("01-airhorn.ogg").exists());
        assert!(dir.join("02-bruh.ogg").exists());
        assert_eq!(
            tab.slot(1).unwrap().file_name().and_then(|s| s.to_str()),
            Some("01-airhorn.ogg")
        );
        assert_eq!(
            tab.slot(2).unwrap().file_name().and_then(|s| s.to_str()),
            Some("02-bruh.ogg")
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_preserves_spaces_in_sound_label() {
        let dir = std::env::temp_dir().join("sound_spring_scan_label_spaces");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("01 my cool sound.ogg"), b"fake").unwrap();
        let (tab, warnings) = TabsRepository::scan_tab_dir_with_warnings(&dir).unwrap();
        assert!(warnings.is_empty());
        assert!(dir.join("01-my cool sound.ogg").exists());
        assert_eq!(
            tab.slot(1).unwrap().file_name().and_then(|s| s.to_str()),
            Some("01-my cool sound.ogg")
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_renames_unnumbered_files_into_free_slots() {
        let dir = std::env::temp_dir().join("sound_spring_scan_unnumbered");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("01-first.ogg"), b"fake").unwrap();
        fs::write(dir.join("second.ogg"), b"fake").unwrap();
        let (tab, warnings) = TabsRepository::scan_tab_dir_with_warnings(&dir).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(
            tab.slot(1).unwrap().file_name().and_then(|s| s.to_str()),
            Some("01-first.ogg")
        );
        assert_eq!(
            tab.slot(2).unwrap().file_name().and_then(|s| s.to_str()),
            Some("02-second.ogg")
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_remaps_high_prefixes_when_ten_or_fewer_files() {
        let dir = std::env::temp_dir().join("sound_spring_scan_high_prefix");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("01-one.ogg"), b"fake").unwrap();
        fs::write(dir.join("15-fifteen.ogg"), b"fake").unwrap();
        let (tab, warnings) = TabsRepository::scan_tab_dir_with_warnings(&dir).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(tab.sounds.len(), 2);
        assert_eq!(
            tab.slot(1).unwrap().file_name().and_then(|s| s.to_str()),
            Some("01-one.ogg")
        );
        assert_eq!(
            tab.slot(2).unwrap().file_name().and_then(|s| s.to_str()),
            Some("02-fifteen.ogg")
        );
        assert!(!dir.join("15-fifteen.ogg").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_warns_when_more_than_ten_files() {
        let dir = std::env::temp_dir().join("sound_spring_scan_eleven");
        fs::create_dir_all(&dir).unwrap();
        for index in 1..=11 {
            fs::write(dir.join(format!("{index:02}-file.ogg")), b"fake").unwrap();
        }
        let (tab, warnings) = TabsRepository::scan_tab_dir_with_warnings(&dir).unwrap();
        assert_eq!(tab.sounds.len(), 10);
        assert!(warnings.iter().any(|msg| msg.contains("Ignoring 1 excess")));
        assert!(dir.join("11-file.ogg").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_preserves_empty_slots_after_deletion() {
        let dir = std::env::temp_dir().join("sound_spring_scan_empty_slot");
        fs::create_dir_all(&dir).unwrap();
        for index in 1..=8 {
            fs::write(dir.join(format!("{index:02}-file.ogg")), b"fake").unwrap();
        }
        fs::write(dir.join("10-tenth.ogg"), b"fake").unwrap();
        let (tab, warnings) = TabsRepository::scan_tab_dir_with_warnings(&dir).unwrap();
        assert!(warnings.is_empty());
        assert!(tab.slot(9).is_none());
        assert_eq!(
            tab.slot(10).unwrap().file_name().and_then(|s| s.to_str()),
            Some("10-tenth.ogg")
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_fills_empty_slots_from_overflow_when_more_than_ten_files() {
        let dir = std::env::temp_dir().join("sound_spring_scan_overflow_fill");
        fs::create_dir_all(&dir).unwrap();
        for index in 1..=9 {
            fs::write(dir.join(format!("{index:02}-file.ogg")), b"fake").unwrap();
        }
        fs::write(dir.join("11-eleventh.ogg"), b"fake").unwrap();
        fs::write(dir.join("12-twelfth.ogg"), b"fake").unwrap();
        let (tab, warnings) = TabsRepository::scan_tab_dir_with_warnings(&dir).unwrap();
        assert_eq!(tab.sounds.len(), 10);
        assert!(tab.slot(9).is_some());
        assert_eq!(
            tab.slot(10).unwrap().file_name().and_then(|s| s.to_str()),
            Some("10-eleventh.ogg")
        );
        assert!(warnings.iter().any(|msg| msg.contains("Ignoring 1 excess")));
        assert!(dir.join("12-twelfth.ogg").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn tab_slot_mapping_uses_prefix_numbers() {
        let tab = Tab {
            path: PathBuf::from("/tmp/t"),
            name: "t".into(),
            sounds: vec![
                SoundFile {
                    path: PathBuf::from("/tmp/t/03-third.ogg"),
                    name: "third".into(),
                    duration_ms: 0,
                },
                SoundFile {
                    path: PathBuf::from("/tmp/t/01-first.ogg"),
                    name: "first".into(),
                    duration_ms: 0,
                },
            ],
        };
        assert_eq!(tab.slot(1).unwrap(), &PathBuf::from("/tmp/t/01-first.ogg"));
        assert!(tab.slot(2).is_none());
        assert_eq!(tab.slot(3).unwrap(), &PathBuf::from("/tmp/t/03-third.ogg"));
        assert!(tab.slot(10).is_none());
    }

    #[test]
    fn move_slot_file_swaps_occupied_targets() {
        let dir = std::env::temp_dir().join("sound_spring_move_swap");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("01-a.ogg"), b"a").unwrap();
        fs::write(dir.join("02-b.ogg"), b"b").unwrap();
        TabsRepository::move_slot_file(&dir, 1, 2).unwrap();
        assert!(dir.join("01-b.ogg").exists());
        assert!(dir.join("02-a.ogg").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn destination_for_empty_slot_strips_import_prefix() {
        let dir = std::env::temp_dir().join("sound_spring_slot_dest_strip");
        fs::create_dir_all(&dir).unwrap();
        let source = dir.join("15-oldname.ogg");
        fs::write(&source, b"fake").unwrap();
        let dest = destination_for_empty_slot(&dir, 10, &source).unwrap();
        assert_eq!(
            dest.file_name().and_then(|s| s.to_str()),
            Some("10-oldname.ogg")
        );
        let _ = fs::remove_dir_all(&dir);
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
        assert!(is_tab_content_change(&EventKind::Create(
            notify::event::CreateKind::File
        )));
    }

    #[test]
    fn sanitize_tab_segment_replaces_invalid_chars() {
        assert_eq!(sanitize_tab_segment("  My Tab!  "), "My-Tab");
        assert_eq!(sanitize_tab_segment("---"), "tab");
    }

    #[test]
    fn next_order_prefix_increments_existing_tabs() {
        let root = std::env::temp_dir().join("sound_spring_next_prefix");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(root.join("01-memes")).unwrap();
        fs::create_dir_all(root.join("02-music")).unwrap();
        assert_eq!(next_order_prefix(&root), "03");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_preserves_custom_tab_order_and_names() {
        let first = std::env::temp_dir().join("sound_spring_tab_a");
        let second = std::env::temp_dir().join("sound_spring_tab_b");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        let config = Config {
            tabs: vec![
                TabEntry {
                    path: second.clone(),
                    name: Some("Second".into()),
                },
                TabEntry {
                    path: first.clone(),
                    name: Some("First".into()),
                },
            ],
            ..Default::default()
        };
        let tabs = TabsRepository::scan(&config).unwrap();
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].name, "Second");
        assert_eq!(tabs[1].name, "First");
        let _ = fs::remove_dir_all(&first);
        let _ = fs::remove_dir_all(&second);
    }

    #[test]
    fn destination_for_empty_slot_uses_slot_prefix() {
        let dir = std::env::temp_dir().join("sound_spring_slot_dest");
        fs::create_dir_all(&dir).unwrap();
        let source = dir.join("clip.ogg");
        fs::write(&source, b"fake").unwrap();
        let dest = destination_for_empty_slot(&dir, 3, &source).unwrap();
        assert_eq!(
            dest.file_name().and_then(|s| s.to_str()),
            Some("03-clip.ogg")
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rename_slot_file_preserves_order_prefix() {
        let dir = std::env::temp_dir().join("sound_spring_slot_rename");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("01-airhorn.ogg");
        fs::write(&file, b"fake").unwrap();
        let renamed = TabsRepository::rename_slot_file(&dir, 1, "bruh").unwrap();
        assert_eq!(
            renamed.file_name().and_then(|s| s.to_str()),
            Some("01-bruh.ogg")
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
