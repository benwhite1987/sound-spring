use tracing::info;
use zbus::{proxy, Connection};

/// Must match xdg-desktop-portal app id / KDE KGlobalAccel component name.
pub const PORTAL_COMPONENT: &str = "sound-spring";

#[proxy(
    interface = "org.kde.KGlobalAccel",
    default_service = "org.kde.kglobalaccel",
    default_path = "/kglobalaccel"
)]
trait KGlobalAccel {
    #[zbus(name = "unregister")]
    fn unregister(&self, component: &str, action: &str) -> zbus::Result<bool>;

    fn get_component(&self, component: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}

async fn count_assigned_bindings(connection: &Connection) -> usize {
    let component_path = format!("/component/{}", PORTAL_COMPONENT.replace('-', "_"));
    let Ok(reply) = connection
        .call_method(
            Some("org.kde.kglobalaccel"),
            component_path.as_str(),
            Some("org.kde.kglobalaccel.Component"),
            "allShortcutInfos",
            &(),
        )
        .await
    else {
        return 0;
    };
    let body = reply.body();
    let infos: Vec<(String, String, String, String, String, String, Vec<i32>, Vec<i32>)> =
        match body.deserialize() {
            Ok(infos) => infos,
            Err(_) => return 0,
        };
    infos.iter().filter(|info| !info.7.is_empty()).count()
}

async fn component_exists(connection: &Connection) -> bool {
    let kg = match KGlobalAccelProxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(_) => return false,
    };
    kg.get_component(PORTAL_COMPONENT).await.is_ok()
}

/// True when KGlobalAccel still has our shortcuts registered with no keys assigned.
pub async fn has_stale_empty_bindings() -> bool {
    let Ok(connection) = Connection::session().await else {
        return false;
    };
    if !component_exists(&connection).await {
        return false;
    };
    let assigned = count_assigned_bindings(&connection).await;
    if assigned == 0 {
        let component_path = format!("/component/{}", PORTAL_COMPONENT.replace('-', "_"));
        let Ok(reply) = connection
            .call_method(
                Some("org.kde.kglobalaccel"),
                component_path.as_str(),
                Some("org.kde.kglobalaccel.Component"),
                "allShortcutInfos",
                &(),
            )
            .await
        else {
            return false;
        };
        let body = reply.body();
        let infos: Vec<(String, String, String, String, String, String, Vec<i32>, Vec<i32>)> =
            match body.deserialize() {
                Ok(infos) => infos,
                Err(_) => return false,
            };
        return !infos.is_empty();
    }
    false
}

pub async fn unregister_component(shortcut_ids: &[&str]) {
    let Ok(connection) = Connection::session().await else {
        return;
    };
    let Ok(proxy) = KGlobalAccelProxy::new(&connection).await else {
        return;
    };
    let mut cleared = 0usize;
    for id in shortcut_ids {
        if proxy.unregister(PORTAL_COMPONENT, id).await.unwrap_or(false) {
            cleared += 1;
            info!("cleared KGlobalAccel shortcut {PORTAL_COMPONENT}/{id}");
        }
    }
    if cleared > 0 {
        info!("cleared {cleared} KGlobalAccel shortcut(s) for {PORTAL_COMPONENT}");
    }
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
