mod kglobalaccel;
mod portal;
mod trigger;

use anyhow::{Context, Result};
use std::sync::mpsc::Sender as StdSender;
use tracing::{info, warn};

pub use trigger::{portal_trigger, play_slot_from_qt_key, qt_key_sequence, qt_shortcut_sequence, trigger_display, trigger_from_qt};

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
        defs
    }

    pub async fn bind(
        shortcuts: &[ShortcutDef],
        mode: &str,
        event_tx: StdSender<ShortcutEvent>,
    ) -> Result<()> {
        match Self::effective_mode(mode) {
            "local" => {
                info!("using in-window keyboard shortcuts only");
                Ok(())
            }
            "portal" => Self::bind_portal(shortcuts, event_tx).await,
            other => {
                warn!("unknown shortcut mode '{other}', using portal");
                Self::bind_portal(shortcuts, event_tx).await
            }
        }
    }

    /// KGlobalAccel registration can destabilize Plasma 6; route legacy modes to portal.
    fn effective_mode(mode: &str) -> &'static str {
        match mode {
            "local" => "local",
            "portal" => "portal",
            "auto" | "kglobalaccel" => {
                warn!(
                    "shortcut mode '{mode}' is routed to portal for desktop stability"
                );
                "portal"
            }
            _ => "portal",
        }
    }

    async fn bind_portal(
        shortcuts: &[ShortcutDef],
        event_tx: StdSender<ShortcutEvent>,
    ) -> Result<()> {
        portal::bind(shortcuts, event_tx)
            .await
            .context("bind portal shortcuts")
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

    pub async fn cleanup_kglobalaccel_component() {
        kglobalaccel::unregister_component().await;
    }
}
