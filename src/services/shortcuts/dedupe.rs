use std::sync::Mutex;
use std::time::{Duration, Instant};

static LAST_SHORTCUT: Mutex<Option<(String, Instant)>> = Mutex::new(None);

const DEDUPE_MS: u64 = 150;

/// Returns true when this shortcut action should run (not a duplicate within the debounce window).
pub fn accept_shortcut(id: &str) -> bool {
    let now = Instant::now();
    let Ok(mut guard) = LAST_SHORTCUT.lock() else {
        return true;
    };
    if let Some((last_id, last_at)) = guard.as_ref() {
        if last_id == id && now.duration_since(*last_at) < Duration::from_millis(DEDUPE_MS) {
            return false;
        }
    }
    *guard = Some((id.to_string(), now));
    true
}
