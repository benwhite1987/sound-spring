mod dedupe;
mod kglobalaccel;
mod portal;
mod status;
mod trigger;

use anyhow::{Context, Result};
use std::sync::{Mutex, OnceLock};
use std::sync::mpsc::Sender as StdSender;
use tracing::{info, warn};

pub use dedupe::accept_shortcut;
pub use kglobalaccel::PORTAL_COMPONENT;
pub use portal::{PortalBindResult, configure_active_session};
pub use status::{
    global_shortcut_status, global_shortcuts_active, set_global_shortcut_status,
    GlobalShortcutStatus,
};
pub use trigger::{
    play_slot_from_qt_key, qt_key_sequence, qt_shortcut_sequence, trigger_display, trigger_from_qt,
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

    pub async fn bind_global(shortcuts: &[ShortcutDef]) -> Result<Option<PortalBindResult>> {
        if !portal::available().await {
            warn!("xdg-desktop-portal GlobalShortcuts unavailable; global shortcuts disabled");
            return Ok(None);
        }
        let event_tx = shortcut_event_tx()?;
        portal::bind(shortcuts, event_tx)
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

    pub fn open_system_shortcuts_settings() {
        portal::open_system_settings_shortcuts();
    }

    pub async fn configure_global_shortcuts() {
        if let Err(err) = configure_active_session().await {
            warn!("failed to open global shortcut settings: {err:#}");
        }
    }

    pub async fn has_stale_empty_kglobalaccel_bindings() -> bool {
        kglobalaccel::has_stale_empty_bindings().await
    }

    pub async fn cleanup_kglobalaccel_component() {
        let owned: Vec<String> = Self::default_bindings()
            .into_iter()
            .map(|def| def.id)
            .collect();
        let id_refs: Vec<&str> = owned.iter().map(String::as_str).collect();
        kglobalaccel::unregister_component(&id_refs).await;
    }

    pub async fn unregister_changed_bindings(previous: &crate::config::Config, config: &crate::config::Config) {
        let old = Self::resolve_bindings(&previous.shortcuts);
        let new = Self::resolve_bindings(&config.shortcuts);
        let changed: Vec<&str> = old
            .iter()
            .zip(new.iter())
            .filter(|(o, n)| o.id == n.id && o.trigger != n.trigger)
            .map(|(o, _)| o.id.as_str())
            .collect();
        if !changed.is_empty() {
            info!(
                "unregistering {} KGlobalAccel shortcut(s) with changed bindings",
                changed.len()
            );
            kglobalaccel::unregister_component(&changed).await;
        }
    }
}
