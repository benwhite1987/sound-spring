use std::sync::Mutex;
use tracing::{info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobalShortcutStatus {
    Inactive,
    Bound {
        bound_count: usize,
        assigned_count: usize,
        requested_count: usize,
    },
    Failed {
        reason: String,
    },
}

static STATUS: Mutex<GlobalShortcutStatus> = Mutex::new(GlobalShortcutStatus::Inactive);

pub fn set_global_shortcut_status(status: GlobalShortcutStatus) {
    match &status {
        GlobalShortcutStatus::Inactive => info!("global shortcuts inactive"),
        GlobalShortcutStatus::Bound {
            bound_count,
            assigned_count,
            requested_count,
        } => info!(
            "global shortcuts active: assigned={assigned_count}/{requested_count} registered={bound_count}"
        ),
        GlobalShortcutStatus::Failed { reason } => {
            warn!("global shortcuts failed: {reason}")
        }
    }
    if let Ok(mut guard) = STATUS.lock() {
        *guard = status;
    }
}

pub fn global_shortcut_status() -> GlobalShortcutStatus {
    STATUS
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or(GlobalShortcutStatus::Inactive)
}

pub fn global_shortcuts_active() -> bool {
    matches!(
        global_shortcut_status(),
        GlobalShortcutStatus::Bound {
            assigned_count,
            ..
        } if assigned_count > 0
    )
}

pub fn format_global_shortcut_status() -> String {
    match global_shortcut_status() {
        GlobalShortcutStatus::Inactive => {
            "Global shortcuts inactive — open Settings, Shortcuts tab, and click Apply.".into()
        }
        GlobalShortcutStatus::Bound {
            assigned_count,
            requested_count,
            ..
        } if assigned_count >= requested_count => {
            format!("Global shortcuts active: {assigned_count}/{requested_count} keys assigned")
        }
        GlobalShortcutStatus::Bound {
            assigned_count,
            requested_count,
            ..
        } => format!(
            "Global shortcuts partially active: {assigned_count}/{requested_count} keys assigned"
        ),
        GlobalShortcutStatus::Failed { reason } => {
            format!("Global shortcuts failed: {reason}")
        }
    }
}
