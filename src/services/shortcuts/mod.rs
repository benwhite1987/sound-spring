mod kglobalaccel;
mod portal;
mod trigger;

use anyhow::{Context, Result};
use std::sync::mpsc::Sender as StdSender;
use tracing::{info, warn};

pub use trigger::{portal_trigger, qt_key_sequence};

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
                trigger: format!("Meta+{slot}"),
            });
        }
        defs.push(ShortcutDef {
            id: "play_10".into(),
            description: "Play slot 10".into(),
            trigger: "Meta+0".into(),
        });
        defs.push(ShortcutDef {
            id: "tab_next".into(),
            description: "Next tab".into(),
            trigger: "Meta+Bracket Right".into(),
        });
        defs.push(ShortcutDef {
            id: "tab_prev".into(),
            description: "Previous tab".into(),
            trigger: "Meta+Bracket Left".into(),
        });
        defs.push(ShortcutDef {
            id: "stop_all".into(),
            description: "Stop all".into(),
            trigger: "Meta+Escape".into(),
        });
        defs
    }

    pub async fn bind(
        shortcuts: &[ShortcutDef],
        mode: &str,
        event_tx: StdSender<ShortcutEvent>,
    ) -> Result<()> {
        match mode {
            "portal" => Self::bind_portal(shortcuts, event_tx).await,
            "kglobalaccel" => Self::bind_kglobalaccel(shortcuts, event_tx).await,
            "auto" => Self::bind_auto(shortcuts, event_tx).await,
            other => {
                warn!("unknown shortcut mode '{other}', using auto");
                Self::bind_auto(shortcuts, event_tx).await
            }
        }
    }

    async fn bind_auto(
        shortcuts: &[ShortcutDef],
        event_tx: StdSender<ShortcutEvent>,
    ) -> Result<()> {
        match Self::bind_portal(shortcuts, event_tx.clone()).await {
            Ok(()) => {
                info!("using xdg-desktop-portal global shortcuts");
                Ok(())
            }
            Err(portal_err) => {
                warn!("portal shortcuts unavailable: {portal_err:#}");
                if kglobalaccel::available().await {
                    info!("falling back to KGlobalAccel");
                    Self::bind_kglobalaccel(shortcuts, event_tx).await
                } else {
                    Err(portal_err).context("portal failed and KGlobalAccel unavailable")
                }
            }
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

    async fn bind_kglobalaccel(
        shortcuts: &[ShortcutDef],
        event_tx: StdSender<ShortcutEvent>,
    ) -> Result<()> {
        kglobalaccel::bind(shortcuts, event_tx)
            .await
            .context("bind KGlobalAccel shortcuts")
    }
}
