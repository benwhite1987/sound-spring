use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::process::Command;
use std::sync::mpsc::Sender as StdSender;
use std::sync::{Mutex, OnceLock};
use tracing::{debug, info, warn};
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};
use zbus::{proxy, Connection};

use super::trigger::{portal_trigger, trigger_from_portal};
use super::{ShortcutDef, ShortcutEvent};

const PORTAL_SUCCESS: u32 = 0;
const PARENT_WINDOW_MAX: usize = 256;

/// xdg-desktop-portal app id; used in the System Settings KCM URL.
const PORTAL_COMPONENT: &str = "sound-spring";
const LEGACY_PORTAL_COMPONENT: &str = "sound_spring";

extern "C" {
    fn sound_spring_portal_parent_window(out: *mut std::ffi::c_char, out_len: usize);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundShortcut {
    pub id: String,
    pub description: String,
    pub trigger_description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortalBindResult {
    pub session_path: String,
    pub bound_shortcuts: Vec<BoundShortcut>,
    pub requested_count: usize,
    pub assigned_count: usize,
}

fn portal_parent_window() -> String {
    let mut buf = vec![0_u8; PARENT_WINDOW_MAX];
    unsafe {
        sound_spring_portal_parent_window(buf.as_mut_ptr().cast(), buf.len());
    }
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

#[proxy(
    interface = "org.freedesktop.portal.GlobalShortcuts",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait GlobalShortcuts {
    fn create_session(&self, options: HashMap<&str, Value<'_>>) -> zbus::Result<OwnedObjectPath>;

    fn bind_shortcuts(
        &self,
        session_handle: &ObjectPath<'_>,
        shortcuts: &[(String, HashMap<&str, Value<'_>>)],
        parent_window: &str,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;

    fn configure_shortcuts(
        &self,
        session_handle: &ObjectPath<'_>,
        parent_window: &str,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<()>;

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
    fn response(&self, response: u32, results: HashMap<String, OwnedValue>) -> zbus::Result<()>;
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

fn parse_bound_shortcuts(results: &HashMap<String, OwnedValue>) -> Vec<BoundShortcut> {
    let Some(raw) = results.get("shortcuts") else {
        warn!(
            "portal BindShortcuts response missing shortcuts key (keys: {:?})",
            results.keys().collect::<Vec<_>>()
        );
        return Vec::new();
    };

    let entries: Vec<(String, HashMap<String, OwnedValue>)> = match raw.clone().try_into() {
        Ok(entries) => entries,
        Err(err) => {
            warn!("failed to parse portal shortcuts array: {err}");
            return Vec::new();
        }
    };

    entries
        .into_iter()
        .map(|(id, attrs)| BoundShortcut {
            description: attrs
                .get("description")
                .and_then(owned_value_to_string)
                .unwrap_or_default(),
            trigger_description: attrs
                .get("trigger_description")
                .and_then(owned_value_to_string)
                .unwrap_or_default(),
            id,
        })
        .collect()
}

fn log_bound_shortcuts(requested: &[ShortcutDef], bound: &[BoundShortcut]) {
    for def in requested {
        let preferred = portal_trigger(&def.trigger);
        debug!(
            "portal bind request: id={} preferred_trigger={preferred}",
            def.id
        );
    }

    if bound.is_empty() {
        warn!("portal BindShortcuts returned zero bound shortcuts");
        return;
    }

    let assigned = bound
        .iter()
        .filter(|s| !s.trigger_description.is_empty())
        .count();
    if assigned == 0 {
        warn!(
            "portal bound {} shortcuts but none have key assignments; \
             open KDE System Settings → Shortcuts → Sound Spring to assign keys",
            bound.len()
        );
    } else if assigned < bound.len() {
        warn!(
            "portal assigned keys for {assigned}/{} shortcuts",
            bound.len()
        );
    }

    for shortcut in bound {
        if shortcut.trigger_description.is_empty() {
            warn!(
                "portal shortcut '{}' ({}) has empty trigger_description",
                shortcut.id, shortcut.description
            );
        } else {
            let internal = trigger_from_portal(&shortcut.trigger_description)
                .unwrap_or_else(|| shortcut.trigger_description.clone());
            debug!(
                "portal shortcut bound: id={} trigger={} internal={internal}",
                shortcut.id, shortcut.trigger_description
            );
        }
    }
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

    if let Some(signal) = stream.next().await {
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

pub async fn bind_with_options(
    shortcuts: &[ShortcutDef],
    event_tx: StdSender<ShortcutEvent>,
    use_parent_window: bool,
) -> Result<PortalBindResult> {
    let requested_count = shortcuts.len();
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

    let parent_window = if use_parent_window {
        portal_parent_window()
    } else {
        String::new()
    };
    info!(
        "binding portal shortcuts (parent_window={})",
        if parent_window.is_empty() {
            "<none>"
        } else {
            parent_window.as_str()
        }
    );

    let bind_started = std::time::Instant::now();
    let bind_request = proxy
        .bind_shortcuts(
            &ObjectPath::try_from(session_handle.as_str())?,
            &shortcut_defs,
            parent_window.as_str(),
            bind_options,
        )
        .await
        .context("GlobalShortcuts.BindShortcuts")?;
    let bind_results = wait_for_request(&connection, bind_request).await?;
    let bind_elapsed = bind_started.elapsed();
    let bound_shortcuts = parse_bound_shortcuts(&bind_results);
    log_bound_shortcuts(shortcuts, &bound_shortcuts);
    let assigned_count = bound_shortcuts
        .iter()
        .filter(|s| !s.trigger_description.is_empty())
        .count();
    info!(
        "portal bound {}/{} shortcuts ({assigned_count} with keys)",
        bound_shortcuts.len(),
        requested_count
    );
    // A sub-100 ms bind with zero assigned keys is the classic
    // "portal-kde silently dismissed the dialog" signature (typically caused
    // by a stale parent-cgroup app_id — see docs/global-shortcuts.md). A fast
    // bind that returns assigned keys is normal: it means the portal reused
    // a previously-stored binding for this app_id, which is the desired
    // behaviour on every launch after the first.
    if bind_elapsed.as_millis() < 100 && assigned_count == 0 {
        warn!(
            "portal BindShortcuts completed in {}ms with zero assigned keys — \
             KDE likely skipped the config dialog. Verify the journal shows \
             app_id: \"sound-spring\"; if it shows another app_id, the binary \
             is in the wrong cgroup scope (see docs/global-shortcuts.md).",
            bind_elapsed.as_millis()
        );
    }

    if bound_shortcuts.len() < requested_count {
        warn!(
            "portal bound {}/{} requested shortcuts",
            bound_shortcuts.len(),
            requested_count
        );
    }

    let session_path = session_handle.clone();
    let session_path_for_task = OwnedObjectPath::try_from(session_handle.as_str())?;
    let mut stream = proxy
        .receive_activated()
        .await
        .context("subscribe to GlobalShortcuts.Activated")?;

    info!("portal listening for Activated on session {session_path_for_task}");

    cancel_activated_listener();
    let listener = tokio::spawn(async move {
        while let Some(signal) = stream.next().await {
            let Ok(args) = signal.args() else {
                warn!("portal Activated signal had unparseable args");
                continue;
            };
            if args.session_handle().as_str() != session_path_for_task.as_str() {
                continue;
            }
            let id = args.shortcut_id().to_string();
            debug!(
                "portal shortcut activated: id={id} session={} timestamp={}",
                session_path_for_task.as_str(),
                args.timestamp()
            );
            if event_tx.send(ShortcutEvent::Triggered(id)).is_err() {
                warn!("portal shortcut event channel closed");
                break;
            }
        }
        warn!(
            "portal Activated stream ended for session {}",
            session_path_for_task.as_str()
        );
    });
    if let Ok(mut guard) = ACTIVATED_LISTENER.get_or_init(|| Mutex::new(None)).lock() {
        *guard = Some(listener);
    }

    store_portal_session(session_path.clone());

    Ok(PortalBindResult {
        session_path,
        bound_shortcuts,
        requested_count,
        assigned_count,
    })
}

pub async fn available() -> bool {
    match Connection::session().await {
        Ok(connection) => connection
            .call_method(
                Some("org.freedesktop.portal.Desktop"),
                "/org/freedesktop/portal/desktop",
                Some("org.freedesktop.DBus.Peer"),
                "Ping",
                &(),
            )
            .await
            .is_ok(),
        Err(_) => false,
    }
}

static PORTAL_SESSION: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static ACTIVATED_LISTENER: OnceLock<Mutex<Option<tokio::task::JoinHandle<()>>>> = OnceLock::new();

fn cancel_activated_listener() {
    let store = ACTIVATED_LISTENER.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = store.lock() {
        if let Some(handle) = guard.take() {
            handle.abort();
        }
    }
}

fn portal_session_store() -> &'static Mutex<Option<String>> {
    PORTAL_SESSION.get_or_init(|| Mutex::new(None))
}

pub fn store_portal_session(path: String) {
    if let Ok(mut guard) = portal_session_store().lock() {
        *guard = Some(path);
    }
}

pub fn portal_session_path() -> Option<String> {
    portal_session_store()
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
}

pub async fn configure_active_session() -> Result<()> {
    if let Some(session_path) = portal_session_path() {
        unsafe {
            extern "C" {
                fn sound_spring_refresh_portal_parent_window();
            }
            sound_spring_refresh_portal_parent_window();
        }
        let connection = Connection::session()
            .await
            .context("connect to session bus for portal shortcuts")?;
        let proxy = GlobalShortcutsProxy::new(&connection)
            .await
            .context("create GlobalShortcuts proxy")?;
        let parent_window = portal_parent_window();
        if proxy
            .configure_shortcuts(
                &ObjectPath::try_from(session_path.as_str())?,
                parent_window.as_str(),
                HashMap::new(),
            )
            .await
            .is_ok()
        {
            return Ok(());
        }
        warn!("portal ConfigureShortcuts failed; opening System Settings directly");
    }
    open_system_settings_shortcuts();
    Ok(())
}

/// Open KDE System Settings on the Sound Spring shortcut component.
pub fn open_system_settings_shortcuts() {
    let urls = [
        format!("systemsettings://kcm_keys/{PORTAL_COMPONENT}"),
        format!("systemsettings://kcm_keys/{LEGACY_PORTAL_COMPONENT}"),
        "systemsettings://kcm_keys".to_string(),
    ];
    for url in urls {
        for (bin, args) in [
            ("kde-open", vec![url.as_str()]),
            ("xdg-open", vec![url.as_str()]),
        ] {
            if Command::new(bin).args(args).spawn().is_ok() {
                info!("opened KDE shortcut settings via {bin} ({url})");
                return;
            }
        }
    }
    warn!("failed to open KDE shortcut settings");
}
