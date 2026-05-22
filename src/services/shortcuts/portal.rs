use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::mpsc::Sender as StdSender;
use tracing::{info, warn};
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};
use zbus::{proxy, Connection};

use super::trigger::portal_trigger;
use super::{ShortcutDef, ShortcutEvent};

const PORTAL_SUCCESS: u32 = 0;

#[proxy(
    interface = "org.freedesktop.portal.GlobalShortcuts",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait GlobalShortcuts {
    fn create_session(
        &self,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;

    fn bind_shortcuts(
        &self,
        session_handle: &ObjectPath<'_>,
        shortcuts: &[(String, HashMap<&str, Value<'_>>)],
        parent_window: &str,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;

    #[zbus(signal)]
    fn activated(
        &self,
        session_handle: ObjectPath<'_>,
        shortcut_id: &str,
        timestamp: u64,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.portal.Request",
    default_service = "org.freedesktop.portal.Desktop"
)]
trait PortalRequest {
    #[zbus(signal)]
    fn response(
        &self,
        response: u32,
        results: HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;
}

fn token(label: &str) -> String {
    format!(
        "{label}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

fn owned_value_to_string(value: &OwnedValue) -> Option<String> {
    value.downcast_ref::<&str>().ok().map(|s| (*s).to_string())
}

async fn wait_for_request(
    connection: &Connection,
    request_path: OwnedObjectPath,
) -> Result<HashMap<String, OwnedValue>> {
    let request = PortalRequestProxy::builder(connection)
        .path(request_path.as_str())?
        .build()
        .await
        .context("build portal request proxy")?;

    let mut stream = request
        .receive_response()
        .await
        .map_err(|err| anyhow!("subscribe to portal request response: {err}"))?;

    while let Some(signal) = stream.next().await {
        let args = signal
            .args()
            .map_err(|err| anyhow!("parse portal request response args: {err}"))?;
        if *args.response() != PORTAL_SUCCESS {
            return Err(anyhow!(
                "portal request failed with response code {}",
                args.response()
            ));
        }
        return Ok(args.results().clone());
    }

    Err(anyhow!("portal request closed without response"))
}

pub async fn bind(
    shortcuts: &[ShortcutDef],
    event_tx: StdSender<ShortcutEvent>,
) -> Result<()> {
    let connection = Connection::session()
        .await
        .context("connect to session bus for portal shortcuts")?;
    let proxy = GlobalShortcutsProxy::new(&connection)
        .await
        .context("create GlobalShortcuts proxy")?;

    let mut create_options = HashMap::new();
    create_options.insert("handle_token", Value::from(token("create")));
    create_options.insert("session_handle_token", Value::from(token("session")));

    let create_request = proxy
        .create_session(create_options)
        .await
        .context("GlobalShortcuts.CreateSession")?;
    let create_results = wait_for_request(&connection, create_request).await?;
    let session_handle = create_results
        .get("session_handle")
        .and_then(owned_value_to_string)
        .ok_or_else(|| anyhow!("portal CreateSession missing session_handle"))?;

    let shortcut_defs: Vec<(String, HashMap<&str, Value<'_>>)> = shortcuts
        .iter()
        .map(|def| {
            let mut entry = HashMap::new();
            entry.insert("description", Value::from(def.description.as_str()));
            entry.insert(
                "preferred_trigger",
                Value::from(portal_trigger(&def.trigger)),
            );
            (def.id.clone(), entry)
        })
        .collect();

    let mut bind_options = HashMap::new();
    bind_options.insert("handle_token", Value::from(token("bind")));

    let bind_request = proxy
        .bind_shortcuts(
            &ObjectPath::try_from(session_handle.as_str())?,
            &shortcut_defs,
            "",
            bind_options,
        )
        .await
        .context("GlobalShortcuts.BindShortcuts")?;
    let _bound = wait_for_request(&connection, bind_request).await?;

    let session_path = OwnedObjectPath::try_from(session_handle.as_str())?;
    let mut stream = proxy
        .receive_activated()
        .await
        .context("subscribe to GlobalShortcuts.Activated")?;

    info!("portal global shortcuts bound for session {session_path}");

    tokio::spawn(async move {
        while let Some(signal) = stream.next().await {
            let Ok(args) = signal.args() else {
                continue;
            };
            if args.session_handle().as_str() != session_path.as_str() {
                continue;
            }
            let id = args.shortcut_id().to_string();
            let _ = event_tx.send(ShortcutEvent::Triggered(id));
        }
        warn!("portal Activated stream ended");
    });

    Ok(())
}
