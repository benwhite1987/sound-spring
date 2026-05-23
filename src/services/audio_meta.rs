use std::path::Path;
use std::process::Command;
use std::time::Duration;

pub fn probe_duration_ms(path: &Path) -> Option<u64> {
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
    let seconds: f64 = String::from_utf8_lossy(&output.stdout).trim().parse().ok()?;
    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }
    Some((seconds * 1000.0) as u64)
}

pub fn fallback_duration(path: &Path) -> Duration {
    probe_duration_ms(path)
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_secs(5))
}
