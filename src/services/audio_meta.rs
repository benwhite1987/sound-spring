use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

type DurationCache = HashMap<PathBuf, (u64, u64, u64)>;

static DURATION_CACHE: Mutex<Option<DurationCache>> = Mutex::new(None);

fn cache_key(path: &Path) -> Option<(PathBuf, u64, u64)> {
    let meta = fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Some((path.to_path_buf(), mtime, meta.len()))
}

pub fn probe_duration_ms(path: &Path) -> Option<u64> {
    let (path_key, mtime, size) = cache_key(path)?;
    if let Ok(mut guard) = DURATION_CACHE.lock() {
        let cache = guard.get_or_insert_with(HashMap::new);
        if let Some(&(cached_mtime, cached_size, duration)) = cache.get(&path_key) {
            if cached_mtime == mtime && cached_size == size {
                return Some(duration);
            }
        }
        if let Some(duration) = probe_duration_ms_uncached(path) {
            cache.insert(path_key, (mtime, size, duration));
            return Some(duration);
        }
    }
    probe_duration_ms_uncached(path)
}

fn probe_duration_ms_uncached(path: &Path) -> Option<u64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            &path.to_string_lossy(),
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let seconds: f64 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .ok()?;
    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }
    Some((seconds * 1000.0) as u64)
}
