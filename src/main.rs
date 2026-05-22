mod config;
mod qobjects;
mod services;
mod state;

use anyhow::{Context, Result};
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};
use qobjects::controller::PLAYER_TX;
use services::{pipewire::PipewireManager, Player, PlayerCommand, ShortcutsManager, TabsRepository};
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

    let (player_cmd_tx, player_cmd_rx) = mpsc::channel(64);
    PLAYER_TX
        .set(player_cmd_tx)
        .map_err(|_| anyhow::anyhow!("PLAYER_TX already set"))?;

    let worker_config = config.clone();
    thread::spawn(move || {
        if let Err(err) = run_backend(worker_config, player_cmd_rx) {
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

fn run_backend(config: config::Config, mut player_cmd_rx: mpsc::Receiver<PlayerCommand>) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .context("build tokio runtime")?;

    rt.block_on(async move {
        if let Err(err) =
            PipewireManager::setup(&config.audio.mic_source, config.audio.latency_ms).await
        {
            warn!("PipeWire setup failed: {err:#}");
        }

        if let Err(err) = ShortcutsManager::bind(
            &ShortcutsManager::default_bindings(),
            &config.shortcuts.mode,
        )
        .await
        {
            warn!("shortcut bind failed: {err:#}");
        }

        let tabs = TabsRepository::scan(&config).unwrap_or_default();
        info!("loaded {} tabs", tabs.len());

        let mut player = Player::default_sink();
        loop {
            tokio::select! {
                command = player_cmd_rx.recv() => {
                    match command {
                        Some(cmd) => {
                            if let Err(err) = player.handle_command(cmd).await {
                                warn!("player command failed: {err:#}");
                            }
                        }
                        None => break,
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(50)) => {
                    let _finished = player.reap_finished().await;
                }
            }
        }
    });

    Ok(())
}
