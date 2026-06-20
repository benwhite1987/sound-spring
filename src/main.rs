mod config;
mod qobjects;
mod services;
mod state;

use anyhow::{Context, Result};
use config::Config;
use config::SFX_SINK;
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};
use qobjects::controller::{
    BackendCommand, BackendEvent, AUDIO_SINKS, BACKEND_EVENT_RX, BACKEND_TX, MIC_SOURCES,
};
use services::pipewire::{Modules, PipewireManager};
use services::player::{Player, VolumeState};
use services::shortcuts::{set_global_shortcut_status, GlobalShortcutStatus, ShortcutsManager};
use services::tabs::TabsRepository;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

extern "C" {
    fn sound_spring_register_key_forwarder();
    fn sound_spring_init_app_identity();
    fn sound_spring_refresh_portal_parent_window();
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("sound_spring=info".parse()?))
        .init();

    let mut config = config::load_config().unwrap_or_default();
    config::ensure_default_layout(&mut config).context("ensure config layout")?;

    MIC_SOURCES
        .set(Mutex::new(Vec::new()))
        .map_err(|_| anyhow::anyhow!("MIC_SOURCES already set"))?;
    AUDIO_SINKS
        .set(Mutex::new(Vec::new()))
        .map_err(|_| anyhow::anyhow!("AUDIO_SINKS already set"))?;

    let (backend_cmd_tx, backend_cmd_rx) = mpsc::channel(256);
    let (backend_event_tx, backend_event_rx) = std::sync::mpsc::channel();
    BACKEND_TX
        .set(backend_cmd_tx)
        .map_err(|_| anyhow::anyhow!("BACKEND_TX already set"))?;
    BACKEND_EVENT_RX
        .set(Mutex::new(backend_event_rx))
        .map_err(|_| anyhow::anyhow!("BACKEND_EVENT_RX already set"))?;

    let worker_config = config.clone();
    thread::spawn(move || {
        if let Err(err) = run_backend(worker_config, backend_cmd_rx, backend_event_tx) {
            error!("backend thread exited: {err:#}");
        }
    });

    let mut app = QGuiApplication::new();

    unsafe {
        sound_spring_init_app_identity();
        sound_spring_register_key_forwarder();
    }

    let mut engine = QQmlApplicationEngine::new();

    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(
            "qrc:/qt/qml/com/benkahn/soundboard/qml/Main.qml",
        ));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }

    if config.audio.auto_teardown {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(PipewireManager::unload_stale_modules())?;
    }

    Ok(())
}

async fn publish_mic_sources(event_tx: &std::sync::mpsc::Sender<BackendEvent>) {
    match PipewireManager::available_sources().await {
        Ok(sources) => {
            info!("detected {} microphone source(s)", sources.len());
            if let Some(store) = MIC_SOURCES.get() {
                if let Ok(mut guard) = store.lock() {
                    *guard = sources;
                }
            }
            let _ = event_tx.send(BackendEvent::MicSourcesUpdated);
        }
        Err(err) => warn!("failed to list mic sources: {err:#}"),
    }
}

async fn publish_audio_sinks(event_tx: &std::sync::mpsc::Sender<BackendEvent>) {
    match PipewireManager::available_sinks().await {
        Ok(sinks) => {
            info!("detected {} output device(s)", sinks.len());
            if let Some(store) = AUDIO_SINKS.get() {
                if let Ok(mut guard) = store.lock() {
                    *guard = sinks;
                }
            }
            let _ = event_tx.send(BackendEvent::AudioSinksUpdated);
        }
        Err(err) => warn!("failed to list output devices: {err:#}"),
    }
}

async fn bind_shortcuts(
    config: &config::Config,
    use_parent_window: bool,
    event_tx: &std::sync::mpsc::Sender<BackendEvent>,
) {
    if !ShortcutsManager::uses_global_binding(&config.shortcuts.mode) {
        set_global_shortcut_status(GlobalShortcutStatus::Inactive);
        let _ = event_tx.send(BackendEvent::GlobalShortcutStatusChanged);
        return;
    }

    if use_parent_window {
        unsafe {
            sound_spring_refresh_portal_parent_window();
        }
    }

    let bindings = ShortcutsManager::resolve_bindings_for_registration(&config.shortcuts);
    match ShortcutsManager::bind_global(&bindings, use_parent_window).await {
        Ok(Some(result)) => {
            let bound_count = result.bound_shortcuts.len();
            let assigned_count = result.assigned_count;

            if bound_count == 0 {
                set_global_shortcut_status(GlobalShortcutStatus::Failed {
                    reason: "no global shortcuts registered".into(),
                });
            } else if assigned_count == 0 {
                // The portal bind round-tripped in ~10 ms with 15 empty entries.
                // This means xdg-desktop-portal resolved app_id from the parent
                // cgroup scope (Cursor / VS Code / Chromium / Electron) and reused
                // that app's portal session. See README → "Testing global shortcuts".
                set_global_shortcut_status(GlobalShortcutStatus::Failed {
                    reason: "no keys assigned — portal-kde resolved app_id from the \
                             parent cgroup (likely Cursor / VS Code / Chromium / \
                             Electron). Relaunch outside that terminal, via the \
                             .desktop entry, or with `systemd-run --user --scope \
                             sound-spring`. In-window keys still work while focused."
                        .into(),
                });
            } else {
                set_global_shortcut_status(GlobalShortcutStatus::Bound {
                    bound_count,
                    assigned_count,
                    requested_count: result.requested_count,
                });
            }
        }
        Ok(None) => set_global_shortcut_status(GlobalShortcutStatus::Inactive),
        Err(err) => {
            set_global_shortcut_status(GlobalShortcutStatus::Failed {
                reason: format!("{err:#}"),
            });
            warn!("global shortcut bind failed: {err:#}; using in-window keys only");
        }
    }
    let _ = event_tx.send(BackendEvent::GlobalShortcutStatusChanged);
}

async fn apply_volumes(volumes: VolumeState) {
    if let Err(err) =
        PipewireManager::set_sink_volume(SFX_SINK, volumes.output_percent, volumes.output_muted)
            .await
    {
        warn!("failed to set output sink volume: {err:#}");
    }
}

async fn apply_runtime_config(
    config: &Config,
    event_tx: &std::sync::mpsc::Sender<BackendEvent>,
    modules: &mut Modules,
    previous: Option<&Config>,
) {
    let initial = previous.is_none();
    let audio_routing_changed = initial
        || previous.is_some_and(|prev| {
            prev.audio.mic_source != config.audio.mic_source
                || prev.audio.latency_ms != config.audio.latency_ms
        });
    let volume_changed = initial
        || previous.is_some_and(|prev| {
            prev.audio.output_volume != config.audio.output_volume
                || prev.audio.monitor_volume != config.audio.monitor_volume
                || prev.audio.output_muted != config.audio.output_muted
                || prev.audio.monitor_muted != config.audio.monitor_muted
        });

    if audio_routing_changed {
        if !initial {
            let _ = PipewireManager::teardown(modules).await;
        }
        match PipewireManager::setup(&config.audio.mic_source, config.audio.latency_ms).await {
            Ok(new_modules) => *modules = new_modules,
            Err(err) => warn!("PipeWire setup failed: {err:#}"),
        }
        publish_mic_sources(event_tx).await;
    }

    // Output devices don't depend on audio routing; refresh on every apply so
    // the monitor-device picker reflects the current sink list.
    publish_audio_sinks(event_tx).await;

    if volume_changed {
        apply_volumes(VolumeState {
            output_percent: config.audio.output_volume,
            monitor_percent: config.audio.monitor_volume,
            output_muted: config.audio.output_muted,
            monitor_muted: config.audio.monitor_muted,
        })
        .await;
    }

    if ShortcutsManager::uses_global_binding(&config.shortcuts.mode) {
        // On startup, the main window is not visible yet, so don't bother fetching
        // a parent window handle. Subsequent re-binds (Apply, mode change) do use it
        // so the portal can parent its assignment dialog if it has to show one.
        bind_shortcuts(config, !initial, event_tx).await;
    } else if !initial
        && previous
            .is_some_and(|prev| ShortcutsManager::uses_global_binding(&prev.shortcuts.mode))
    {
        set_global_shortcut_status(GlobalShortcutStatus::Inactive);
        let _ = event_tx.send(BackendEvent::GlobalShortcutStatusChanged);
    }

    let tabs = TabsRepository::scan(config).unwrap_or_default();
    info!("loaded {} tabs", tabs.len());
    let _ = event_tx.send(BackendEvent::ConfigApplied);
}

fn run_backend(
    config: config::Config,
    mut backend_cmd_rx: mpsc::Receiver<BackendCommand>,
    backend_event_tx: std::sync::mpsc::Sender<BackendEvent>,
) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .context("build tokio runtime")?;

    rt.block_on(async move {
        let (shortcut_tx, shortcut_rx) = std::sync::mpsc::channel();
        services::shortcuts::set_shortcut_event_tx(shortcut_tx);
        let bridge_tx = backend_event_tx.clone();
        std::thread::spawn(move || {
            for event in shortcut_rx {
                let services::shortcuts::ShortcutEvent::Triggered(id) = event;
                let _ = bridge_tx.send(BackendEvent::ShortcutTriggered { id });
            }
        });

        let mut modules = Modules::default();
        let mut active_config = config.clone();
        apply_runtime_config(&active_config, &backend_event_tx, &mut modules, None).await;

        let (source_watch_tx, mut source_watch_rx) = tokio::sync::mpsc::channel(8);
        PipewireManager::spawn_source_watch(source_watch_tx);

        let mut player = Player::default_sink();
        player.set_volumes(VolumeState {
            output_percent: active_config.audio.output_volume,
            monitor_percent: active_config.audio.monitor_volume,
            output_muted: active_config.audio.output_muted,
            monitor_muted: active_config.audio.monitor_muted,
        });
        player.set_monitor_sink(&active_config.audio.monitor_sink);
        loop {
            tokio::select! {
                command = backend_cmd_rx.recv() => {
                    match command {
                        Some(BackendCommand::ApplyConfig(new_config)) => {
                            let previous = active_config.clone();
                            apply_runtime_config(
                                &new_config,
                                &backend_event_tx,
                                &mut modules,
                                Some(&previous),
                            )
                            .await;
                            player.set_volumes(VolumeState {
                                output_percent: new_config.audio.output_volume,
                                monitor_percent: new_config.audio.monitor_volume,
                                output_muted: new_config.audio.output_muted,
                                monitor_muted: new_config.audio.monitor_muted,
                            });
                            player.set_monitor_sink(&new_config.audio.monitor_sink);
                            active_config = new_config;
                        }
                        Some(BackendCommand::BindShortcuts) => {
                            bind_shortcuts(&active_config, true, &backend_event_tx).await;
                        }
                        Some(BackendCommand::ConfigurePortalShortcuts) => {
                            unsafe {
                                sound_spring_refresh_portal_parent_window();
                            }
                            ShortcutsManager::configure_global_shortcuts().await;
                        }
                        Some(BackendCommand::RefreshMicSources) => {
                            publish_mic_sources(&backend_event_tx).await;
                        }
                        Some(BackendCommand::RefreshAudioSinks) => {
                            publish_audio_sinks(&backend_event_tx).await;
                        }
                        Some(BackendCommand::ApplyVolumes(volumes)) => {
                            if let Err(err) = player
                                .handle_command(services::player::PlayerCommand::SetVolumes(volumes))
                                .await
                            {
                                warn!("failed to apply live stream volumes: {err:#}");
                            }
                            apply_volumes(volumes).await;
                        }
                        Some(BackendCommand::Player(cmd)) => {
                            if let Err(err) = player.handle_command(cmd).await {
                                warn!("player command failed: {err:#}");
                            }
                        }
                        None => break,
                    }
                }
                _ = source_watch_rx.recv() => {
                    publish_mic_sources(&backend_event_tx).await;
                    publish_audio_sinks(&backend_event_tx).await;
                }
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    for (tab_index, slot) in player.reap_finished().await {
                        let _ = backend_event_tx.send(BackendEvent::PlaybackEnded {
                            tab_index,
                            slot,
                        });
                    }
                }
            }
        }
    });

    Ok(())
}
