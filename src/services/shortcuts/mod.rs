mod dedupe;
mod portal;
mod status;
mod trigger;

use anyhow::{Context, Result};
use std::sync::mpsc::Sender as StdSender;
use std::sync::{Mutex, OnceLock};
use tracing::warn;

pub use dedupe::accept_shortcut;
pub use portal::PortalBindResult;
pub use status::{
    format_global_shortcut_status, global_shortcuts_active, set_global_shortcut_status,
    GlobalShortcutStatus,
};
#[allow(unused_imports)]
pub use trigger::{
    play_slot_from_qt_key, qt_shortcut_sequence, trigger_display, trigger_from_portal,
    trigger_from_qt,
};

#[derive(Debug, Clone)]
pub struct ShortcutDef {
    pub id: String,
    pub description: String,
    pub trigger: String,
}

#[derive(Debug, Clone)]
pub enum ShortcutEvent {
    Triggered(String),
}

static EVENT_TX: OnceLock<Mutex<Option<StdSender<ShortcutEvent>>>> = OnceLock::new();

pub fn set_shortcut_event_tx(tx: StdSender<ShortcutEvent>) {
    let store = EVENT_TX.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = store.lock() {
        *guard = Some(tx);
    }
}

fn shortcut_event_tx() -> Result<StdSender<ShortcutEvent>> {
    EVENT_TX
        .get()
        .and_then(|store| store.lock().ok().and_then(|guard| guard.clone()))
        .context("shortcut event channel not initialized")
}

pub struct ShortcutsManager;

impl ShortcutsManager {
    pub fn default_bindings() -> Vec<ShortcutDef> {
        let mut defs = Vec::new();
        for slot in 1..=9 {
            defs.push(ShortcutDef {
                id: format!("play_{slot}"),
                description: format!("Play slot {slot}"),
                trigger: format!("KP_{slot}"),
            });
        }
        defs.push(ShortcutDef {
            id: "play_10".into(),
            description: "Play slot 10".into(),
            trigger: "KP_0".into(),
        });
        defs.push(ShortcutDef {
            id: "tab_next".into(),
            description: "Next tab".into(),
            trigger: "Ctrl+KP_Add".into(),
        });
        defs.push(ShortcutDef {
            id: "tab_prev".into(),
            description: "Previous tab".into(),
            trigger: "Ctrl+KP_Subtract".into(),
        });
        defs.push(ShortcutDef {
            id: "stop_all".into(),
            description: "Stop all".into(),
            trigger: "Ctrl+KP_Decimal".into(),
        });
        defs.push(ShortcutDef {
            id: "mute_output".into(),
            description: "Mute output (remote)".into(),
            trigger: "Alt+KP_Add".into(),
        });
        defs.push(ShortcutDef {
            id: "mute_monitor".into(),
            description: "Mute monitor".into(),
            trigger: "Alt+KP_Subtract".into(),
        });
        defs
    }

    pub async fn bind_global(
        shortcuts: &[ShortcutDef],
        use_parent_window: bool,
    ) -> Result<Option<PortalBindResult>> {
        if !portal::available().await {
            warn!("xdg-desktop-portal GlobalShortcuts unavailable; global shortcuts disabled");
            return Ok(None);
        }
        let event_tx = shortcut_event_tx()?;
        portal::bind_with_options(shortcuts, event_tx, use_parent_window)
            .await
            .map(Some)
            .context("register global shortcuts via portal")
    }

    pub fn effective_mode(mode: &str) -> &'static str {
        match mode {
            "local" => "local",
            "portal" | "auto" | "kglobalaccel" => "global",
            _ => "global",
        }
    }

    pub fn uses_global_binding(mode: &str) -> bool {
        Self::effective_mode(mode) == "global"
    }

    pub fn resolve_bindings(config: &crate::config::ShortcutsConfig) -> Vec<ShortcutDef> {
        Self::default_bindings()
            .into_iter()
            .map(|mut def| {
                if let Some(trigger) = config.bindings.get(&def.id) {
                    if !trigger.trim().is_empty() {
                        def.trigger = trigger.trim().to_string();
                    }
                }
                def
            })
            .collect()
    }

    pub async fn configure_global_shortcuts() {
        if let Err(err) = portal::configure_active_session().await {
            warn!("failed to open global shortcut settings: {err:#}");
        }
    }
}
