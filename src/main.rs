mod config;
mod qobjects;
mod services;
mod state;

use anyhow::{Context, Result};
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};
use qobjects::controller::{BackendCommand, BackendEvent, BACKEND_EVENT_RX, BACKEND_TX, MIC_SOURCES};
use services::pipewire::PipewireManager;
use services::{Player, ShortcutEvent, ShortcutsManager, TabsRepository};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("sound_spring=info".parse()?))
        .init();

    let mut config = config::load_config().unwrap_or_default();
    config::ensure_default_layout(&mut config).context("ensure config layout")?;

    MIC_SOURCES
        .set(Mutex::new(Vec::new()))
        .map_err(|_| anyhow::anyhow!("MIC_SOURCES already set"))?;

    let (backend_cmd_tx, backend_cmd_rx) = mpsc::channel(64);
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
        rt.block_on(PipewireManager::teardown(&Default::default()))?;
    }

    Ok(())
}

async fn publish_mic_sources(event_tx: &std::sync::mpsc::Sender<BackendEvent>) {
    match PipewireManager::available_sources().await {
        Ok(sources) => {
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

async fn bind_shortcuts(mode: &str, event_tx: &std::sync::mpsc::Sender<BackendEvent>) {
    let (shortcut_tx, shortcut_rx) = std::sync::mpsc::channel();
    match ShortcutsManager::bind(&ShortcutsManager::default_bindings(), mode, shortcut_tx).await {
        Ok(()) => {
            let bridge_tx = event_tx.clone();
            std::thread::spawn(move || {
                for event in shortcut_rx {
                    if let ShortcutEvent::Triggered(id) = event {
                        let _ = bridge_tx.send(BackendEvent::ShortcutTriggered { id });
                    }
                }
            });
        }
        Err(err) => warn!("shortcut bind failed: {err:#}"),
    }
}

async fn apply_runtime_config(
    config: &config::Config,
    event_tx: &std::sync::mpsc::Sender<BackendEvent>,
) {
    if let Err(err) =
        PipewireManager::setup(&config.audio.mic_source, config.audio.latency_ms).await
    {
        warn!("PipeWire setup failed: {err:#}");
    }

    publish_mic_sources(event_tx).await;
    let _ = bind_shortcuts(&config.shortcuts.mode, event_tx).await;

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
        apply_runtime_config(&config, &backend_event_tx).await;

        let mut player = Player::default_sink();
        loop {
            tokio::select! {
                command = backend_cmd_rx.recv() => {
                    match command {
                        Some(BackendCommand::ApplyConfig(new_config)) => {
                            apply_runtime_config(&new_config, &backend_event_tx).await;
                        }
                        Some(BackendCommand::Player(cmd)) => {
                            if let Err(err) = player.handle_command(cmd).await {
                                warn!("player command failed: {err:#}");
                            }
                        }
                        None => break,
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    for slot in player.reap_finished().await {
                        let _ = backend_event_tx.send(BackendEvent::PlaybackEnded { slot });
                    }
                }
            }
        }
    });

    Ok(())
}
