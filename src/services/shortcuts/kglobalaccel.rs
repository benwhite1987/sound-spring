use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::mpsc::Sender as StdSender;
use tracing::{info, warn};
use zbus::zvariant::ObjectPath;
use zbus::{proxy, Connection};

use super::trigger::qt_key_sequence;
use super::{ShortcutDef, ShortcutEvent};

const COMPONENT: &str = "sound_spring";
const CONTEXT: u32 = 0;

#[proxy(
    interface = "org.kde.KGlobalAccel",
    default_service = "org.kde.kglobalaccel",
    default_path = "/kglobalaccel"
)]
trait KGlobalAccel {
    fn do_register(&self, components: &[&str]) -> zbus::Result<()>;

    fn set_foreign_shortcut_keys(
        &self,
        component: &str,
        action: &str,
        shortcuts: &[Vec<i32>],
        context: u32,
    ) -> zbus::Result<()>;

    fn unregister(&self, component: &str, action: &str) -> zbus::Result<bool>;
}

#[proxy(
    interface = "org.kde.kglobalaccel.Component",
    default_service = "org.kde.kglobalaccel"
)]
trait KGlobalComponent {
    #[zbus(signal)]
    fn global_shortcut_pressed(
        &self,
        action: &str,
        shortcut: &str,
        timestamp: i64,
    ) -> zbus::Result<()>;
}

pub async fn bind(
    shortcuts: &[ShortcutDef],
    event_tx: StdSender<ShortcutEvent>,
) -> Result<()> {
    let connection = Connection::session()
        .await
        .context("connect to session bus for KGlobalAccel")?;
    let proxy = KGlobalAccelProxy::new(&connection)
        .await
        .context("create KGlobalAccel proxy")?;

    proxy
        .do_register(&[COMPONENT])
        .await
        .context("KGlobalAccel.doRegister")?;

    for def in shortcuts {
        let _ = proxy.unregister(COMPONENT, &def.id).await;
        let keys = qt_key_sequence(&def.trigger)?;
        proxy
            .set_foreign_shortcut_keys(COMPONENT, &def.id, &[keys], CONTEXT)
            .await
            .with_context(|| format!("set shortcut for {}", def.id))?;
    }

    let component_path = format!("/component/{COMPONENT}");
    let component = KGlobalComponentProxy::builder(&connection)
        .path(ObjectPath::try_from(component_path.as_str())?)?
        .build()
        .await
        .context("create KGlobalAccel component proxy")?;

    let mut stream = component
        .receive_global_shortcut_pressed()
        .await
        .context("subscribe to globalShortcutPressed")?;

    info!("KGlobalAccel shortcuts registered for component {COMPONENT}");

    tokio::spawn(async move {
        while let Some(signal) = stream.next().await {
            let Ok(args) = signal.args() else {
                continue;
            };
            let id = args.action().to_string();
            let _ = event_tx.send(ShortcutEvent::Triggered(id));
        }
        warn!("KGlobalAccel signal stream ended");
    });

    Ok(())
}

pub async fn available() -> bool {
    match Connection::session().await {
        Ok(connection) => connection
            .call_method(
                Some("org.kde.kglobalaccel"),
                "/kglobalaccel",
                Some("org.freedesktop.DBus.Peer"),
                "Ping",
                &(),
            )
            .await
            .is_ok(),
        Err(_) => false,
    }
}
