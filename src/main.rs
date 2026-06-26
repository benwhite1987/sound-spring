mod config;
mod qobjects;
mod services;
mod state;

use anyhow::{Context, Result};
use config::Config;
use config::SFX_SINK;
use cxx_qt_lib::{QQmlApplicationEngine, QUrl};
use qobjects::controller::{
    BackendCommand, BackendEvent, AUDIO_SINKS, BACKEND_EVENT_RX, BACKEND_TX, MIC_SOURCES,
};
use services::autostart;
use services::pipewire::{Modules, PipewireManager, VIRTMIC_SINK};
use services::player::{Player, PlayerCommand, VolumeState};
use services::shortcuts::{set_global_shortcut_status, GlobalShortcutStatus, ShortcutsManager};
use services::tabs::{watch_paths, TabFilesystemWatch, TabsRepository};
use services::voice::{
    spectrum_source_from_str, voice_shared, VoiceSession, SPECTRUM_SOURCE_MIXED,
};
use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

/// Set at the very start of `main` for startup timing logs.
pub static PROCESS_START: OnceLock<Instant> = OnceLock::new();

extern "C" {
    fn sound_spring_init_qt_application(argc: i32, argv: *mut *mut c_char);
    fn sound_spring_exec_qt_application() -> i32;
    fn sound_spring_register_key_forwarder();
    fn sound_spring_register_system_tray();
    fn sound_spring_init_app_identity();
    fn sound_spring_refresh_portal_parent_window();
}

fn main() -> Result<()> {
    let _ = PROCESS_START.set(Instant::now());

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("sound_spring=info".parse()?))
        .init();

    let mut config = config::load_config().unwrap_or_default();
    config::ensure_default_layout(&mut config).context("ensure config layout")?;
    if let Err(err) = autostart::sync_launch_at_login(config.ui.launch_at_login) {
        warn!("failed to sync autostart entry: {err:#}");
    }

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
    let backend_handle = thread::spawn(move || {
        if let Err(err) = run_backend(worker_config, backend_cmd_rx, backend_event_tx) {
            error!("backend thread exited: {err:#}");
        }
    });

    let qt_args: Vec<CString> = std::env::args()
        .map(|arg| CString::new(arg).unwrap_or_default())
        .collect();
    let mut qt_argv: Vec<*mut c_char> = qt_args
        .iter()
        .map(|arg| arg.as_ptr() as *mut c_char)
        .collect();
    qt_argv.push(std::ptr::null_mut());

    unsafe {
        sound_spring_init_qt_application((qt_argv.len() - 1) as i32, qt_argv.as_mut_ptr());
        sound_spring_init_app_identity();
        sound_spring_register_key_forwarder();
        sound_spring_register_system_tray();
    }

    let mut engine = QQmlApplicationEngine::new();

    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(
            "qrc:/qt/qml/io/github/benwhite1987/soundspring/qml/Main.qml",
        ));
    }
    if let Some(start) = PROCESS_START.get() {
        info!(
            "startup: QML engine loaded in {} ms",
            start.elapsed().as_millis()
        );
    }

    let _ = unsafe { sound_spring_exec_qt_application() };

    if let Some(tx) = BACKEND_TX.get() {
        let _ = tx.blocking_send(BackendCommand::Shutdown);
    }
    if let Err(err) = backend_handle.join() {
        error!("backend thread join failed: {err:?}");
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(restore_mic_after_playback(&config));
    if config.audio.auto_teardown {
        rt.block_on(PipewireManager::unload_stale_modules())?;
    }

    Ok(())
}

async fn restore_mic_after_playback(config: &Config) {
    sync_mic_mute_for_playback(config, 0).await;
}

async fn sync_mic_mute_for_playback(config: &Config, active_sessions: usize) {
    if !config.audio.mute_mic_during_playback || config.audio.mic_source.is_empty() {
        return;
    }
    let muted = active_sessions > 0;
    if let Err(err) = PipewireManager::set_source_mute(&config.audio.mic_source, muted).await {
        warn!(
            "failed to {} mic for playback: {err:#}",
            if muted { "mute" } else { "unmute" }
        );
    }
}

fn sync_sfx_mix_for_playback(active_sessions: usize) {
    let shared = voice_shared();
    let want_mix = shared.capturing.load(Ordering::Relaxed)
        && (active_sessions > 0 || shared.spectrum_source() == SPECTRUM_SOURCE_MIXED);
    shared.set_sfx_mix_enabled(want_mix);
}

fn sync_voice_gate_config(config: &Config) {
    let shared = voice_shared();
    shared.set_gate_hangover_ms(config.voice.gate_hangover_ms);
    shared.set_gate_release_ms(config.voice.gate_release_ms);
    shared.set_verification_warmup_enabled(config.voice.verification_warmup);
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

async fn publish_audio_sinks(event_tx: &std::sync::mpsc::Sender<BackendEvent>) {
    match PipewireManager::available_sinks().await {
        Ok(sinks) => {
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

async fn ensure_playback_routing(
    config: &Config,
    modules: &mut Modules,
    event_tx: &std::sync::mpsc::Sender<BackendEvent>,
) -> bool {
    match PipewireManager::ensure_routing_for_playback(
        &config.audio.mic_source,
        config.audio.latency_ms,
        modules,
    )
    .await
    {
        Ok(()) => {
            apply_volumes(VolumeState {
                output_percent: config.audio.output_volume,
                monitor_percent: config.audio.monitor_volume,
                output_muted: config.audio.output_muted,
                monitor_muted: config.audio.monitor_muted,
            })
            .await;
            publish_mic_sources(event_tx).await;
            true
        }
        Err(err) => {
            warn!("PipeWire setup before play failed: {err:#}");
            false
        }
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

    if previous.is_some_and(|prev| {
        prev.audio.mute_mic_during_playback && !config.audio.mute_mic_during_playback
    }) && !config.audio.mic_source.is_empty()
    {
        if let Err(err) = PipewireManager::set_source_mute(&config.audio.mic_source, false).await {
            warn!("failed to restore mic after setting change: {err:#}");
        }
    }

    let mic_mute_changed = initial
        || previous.is_some_and(|prev| {
            prev.audio.mic_muted != config.audio.mic_muted
                || prev.audio.mic_volume != config.audio.mic_volume
                || prev.audio.mic_source != config.audio.mic_source
        });
    if mic_mute_changed && !config.audio.mic_source.is_empty() {
        if let Err(err) = PipewireManager::set_source_volume(
            &config.audio.mic_source,
            config.audio.mic_volume,
            config.audio.mic_muted,
        )
        .await
        {
            warn!("failed to apply mic volume: {err:#}");
        }
    }

    if ShortcutsManager::uses_global_binding(&config.shortcuts.mode) {
        // On startup, the main window is not visible yet, so don't bother fetching
        // a parent window handle. Subsequent re-binds (Apply, mode change) do use it
        // so the portal can parent its assignment dialog if it has to show one.
        bind_shortcuts(config, !initial, event_tx).await;
    } else if !initial
        && previous.is_some_and(|prev| ShortcutsManager::uses_global_binding(&prev.shortcuts.mode))
    {
        set_global_shortcut_status(GlobalShortcutStatus::Inactive);
        let _ = event_tx.send(BackendEvent::GlobalShortcutStatusChanged);
    }

    let tabs = TabsRepository::scan(config).unwrap_or_default();
    info!("loaded {} tabs", tabs.len());
    let _ = event_tx.send(BackendEvent::TabsRescanned { tabs });
    let _ = event_tx.send(BackendEvent::ConfigApplied);
}

/// Drop an active voice session and restore the Phase 1 raw mic loopback when
/// routing had replaced it.
async fn stop_voice_session(
    config: &Config,
    session: &mut Option<VoiceSession>,
    voice_routing: &mut bool,
    event_tx: &std::sync::mpsc::Sender<BackendEvent>,
) {
    if session.is_none() {
        return;
    }
    let was_routing = *voice_routing;
    *session = None;
    *voice_routing = false;
    let _ = event_tx.send(BackendEvent::VoiceCaptureStatus {
        active: false,
        error: String::new(),
    });
    if was_routing {
        if let Err(err) = PipewireManager::set_mic_passthrough(
            true,
            &config.audio.mic_source,
            config.audio.latency_ms,
        )
        .await
        {
            warn!("failed to restore mic loopback: {err:#}");
        }
    }
}

/// Bring the voice session in line with the desired state derived from the
/// Voice panel visibility, the verification setting, and noise suppression.
///
/// Two modes exist: visualization-only (panel showing, routing untouched) and
/// routed (verification enabled + enrolled, and/or suppression on). Routed mode
/// removes the Phase 1 raw mic loopback and feeds the processed mic into the
/// virtmic instead; leaving it restores the loopback so Phase 1 passthrough
/// resumes. Suppression denoises the routed path; verification gates it.
fn sync_spectrum_panel_visible(visible: bool) {
    voice_shared().set_spectrum_panel_visible(visible);
}

async fn reconcile_voice(
    config: &Config,
    session: &mut Option<VoiceSession>,
    current_routing: &mut bool,
    panel_visible: bool,
    event_tx: &std::sync::mpsc::Sender<BackendEvent>,
) {
    let want_verify =
        config.voice.verification_enabled && config::voiceprint_path(config).is_file();
    let want_routing = want_verify || config.voice.suppression_enabled;
    let want_on = panel_visible || want_routing;

    // Tear down when no longer wanted, or when the mode must change.
    if session.is_some() && (!want_on || *current_routing != want_routing) {
        stop_voice_session(config, session, current_routing, event_tx).await;
    }

    if !want_on || session.is_some() {
        return;
    }

    // Starting fresh. When routing, drop the raw loopback first so only the
    // processed feed reaches the virtmic.
    if want_routing {
        if let Err(err) = PipewireManager::set_mic_passthrough(
            false,
            &config.audio.mic_source,
            config.audio.latency_ms,
        )
        .await
        {
            warn!("failed to remove mic loopback for routing: {err:#}");
        }
    }

    let params = services::voice::VoiceParams {
        mic_source: config.audio.mic_source.clone(),
        vad_open: config.voice.vad_open_threshold,
        vad_close: config.voice.vad_close_threshold,
        verification_enabled: config.voice.verification_enabled,
        match_threshold: config.voice.match_threshold,
        voiceprint_path: config::voiceprint_path(config),
        gating: want_routing,
        output_sink: if want_routing {
            VIRTMIC_SINK.to_string()
        } else {
            String::new()
        },
        suppression: config.voice.suppression_enabled,
        vad_enabled: config.voice.vad_enabled,
        gate_hangover_ms: config.voice.gate_hangover_ms,
        gate_release_ms: config.voice.gate_release_ms,
        verification_warmup: config.voice.verification_warmup,
    };
    match VoiceSession::start(params) {
        Ok(s) => {
            *session = Some(s);
            *current_routing = want_routing;
            let _ = event_tx.send(BackendEvent::VoiceCaptureStatus {
                active: true,
                error: String::new(),
            });
        }
        Err(err) => {
            warn!("voice session start failed: {err:#}");
            let _ = event_tx.send(BackendEvent::VoiceCaptureStatus {
                active: false,
                error: format!("Voice capture failed: {err:#}"),
            });
            if want_routing {
                let _ = PipewireManager::set_mic_passthrough(
                    true,
                    &config.audio.mic_source,
                    config.audio.latency_ms,
                )
                .await;
            }
        }
    }
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

        let (tab_watch_tx, mut tab_watch_rx) = tokio::sync::mpsc::channel(8);
        let mut tab_watch = TabFilesystemWatch::new();
        tab_watch.restart(watch_paths(&active_config), tab_watch_tx.clone());

        let (playback_done_tx, mut playback_done_rx) = mpsc::channel(32);

        let mut player = Player::default_sink();
        player.set_playback_done_tx(playback_done_tx);
        player.set_volumes(VolumeState {
            output_percent: active_config.audio.output_volume,
            monitor_percent: active_config.audio.monitor_volume,
            output_muted: active_config.audio.output_muted,
            monitor_muted: active_config.audio.monitor_muted,
        });
        player.set_monitor_sink(&active_config.audio.monitor_sink);
        player.set_interruption_mode(&active_config.audio.interruption_mode);

        // Phase 2 voice session. Runs while the Voice panel is showing (for the
        // spectrum/VAD display) or whenever routing is active (verification
        // gating and/or noise suppression). In routed mode it replaces the
        // Phase 1 raw mic loopback with the processed feed; otherwise it is
        // visualization-only and leaves routing untouched.
        let mut voice_session: Option<VoiceSession> = None;
        let mut voice_routing = false;
        let mut voice_panel_visible = false;
        let voice = voice_shared();
        voice.set_vad_enabled(active_config.voice.vad_enabled);
        voice.set_spectrum_source(spectrum_source_from_str(
            &active_config.voice.spectrum_source,
        ));
        sync_voice_gate_config(&active_config);
        // Engage suppression/gating routing at startup if the config enables it,
        // so the processed mic is active without first opening the Voice panel.
        reconcile_voice(
            &active_config,
            &mut voice_session,
            &mut voice_routing,
            voice_panel_visible,
            &backend_event_tx,
        )
        .await;
        loop {
            tokio::select! {
                command = backend_cmd_rx.recv() => {
                    match command {
                        Some(BackendCommand::ApplyConfig(new_config)) => {
                            let new_config = *new_config;
                            let previous = active_config.clone();
                            // A routing change tears down and rebuilds all
                            // modules (re-adding the raw mic loopback), so stop
                            // any gating session first and let reconcile rebuild
                            // it afterward against the fresh routing.
                            let routing_changed = previous.audio.mic_source
                                != new_config.audio.mic_source
                                || previous.audio.latency_ms != new_config.audio.latency_ms;
                            if routing_changed {
                                stop_voice_session(
                                    &previous,
                                    &mut voice_session,
                                    &mut voice_routing,
                                    &backend_event_tx,
                                )
                                .await;
                            }
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
                            player.set_interruption_mode(&new_config.audio.interruption_mode);
                            let new_watch_paths = watch_paths(&new_config);
                            if new_watch_paths != watch_paths(&active_config) {
                                tab_watch.restart(new_watch_paths, tab_watch_tx.clone());
                            }
                            active_config = new_config;
                            sync_voice_gate_config(&active_config);
                            reconcile_voice(
                                &active_config,
                                &mut voice_session,
                                &mut voice_routing,
                                voice_panel_visible,
                                &backend_event_tx,
                            )
                            .await;
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
                        Some(BackendCommand::RestartTabWatch) => {
                            let config = config::load_config().unwrap_or_default();
                            tab_watch.restart(watch_paths(&config), tab_watch_tx.clone());
                        }
                        Some(BackendCommand::StartVoiceCapture) => {
                            voice_panel_visible = true;
                            sync_spectrum_panel_visible(true);
                            reconcile_voice(
                                &active_config,
                                &mut voice_session,
                                &mut voice_routing,
                                voice_panel_visible,
                                &backend_event_tx,
                            )
                            .await;
                            sync_sfx_mix_for_playback(player.active_session_count().await);
                        }
                        Some(BackendCommand::StopVoiceCapture) => {
                            voice_panel_visible = false;
                            sync_spectrum_panel_visible(false);
                            reconcile_voice(
                                &active_config,
                                &mut voice_session,
                                &mut voice_routing,
                                voice_panel_visible,
                                &backend_event_tx,
                            )
                            .await;
                        }
                        Some(BackendCommand::SetVoiceVerification { enabled, threshold }) => {
                            active_config.voice.verification_enabled = enabled;
                            active_config.voice.match_threshold = threshold;
                            if let Err(err) = config::save_config(&active_config) {
                                warn!("failed to persist voice verification settings: {err:#}");
                            }
                            reconcile_voice(
                                &active_config,
                                &mut voice_session,
                                &mut voice_routing,
                                voice_panel_visible,
                                &backend_event_tx,
                            )
                            .await;
                        }
                        Some(BackendCommand::SetVoiceSuppression { enabled }) => {
                            active_config.voice.suppression_enabled = enabled;
                            if let Err(err) = config::save_config(&active_config) {
                                warn!("failed to persist noise suppression setting: {err:#}");
                            }
                            reconcile_voice(
                                &active_config,
                                &mut voice_session,
                                &mut voice_routing,
                                voice_panel_visible,
                                &backend_event_tx,
                            )
                            .await;
                        }
                        Some(BackendCommand::SetVoiceVad { enabled }) => {
                            active_config.voice.vad_enabled = enabled;
                            if let Err(err) = config::save_config(&active_config) {
                                warn!("failed to persist VAD enable setting: {err:#}");
                            }
                            voice_shared().set_vad_enabled(enabled);
                        }
                        Some(BackendCommand::SetMicVolume { percent, muted }) => {
                            active_config.audio.mic_muted = muted;
                            active_config.audio.mic_volume = percent;
                            if let Err(err) = config::save_config(&active_config) {
                                warn!("failed to persist mic volume setting: {err:#}");
                            }
                            if !active_config.audio.mic_source.is_empty() {
                                if let Err(err) = PipewireManager::set_source_volume(
                                    &active_config.audio.mic_source,
                                    percent,
                                    muted,
                                )
                                .await
                                {
                                    warn!("failed to set mic volume: {err:#}");
                                }
                            }
                        }
                        Some(BackendCommand::SetSpectrumSource { source }) => {
                            active_config.voice.spectrum_source = source.clone();
                            if let Err(err) = config::save_config(&active_config) {
                                warn!("failed to persist spectrum source: {err:#}");
                            }
                            voice_shared().set_spectrum_source(spectrum_source_from_str(&source));
                            sync_sfx_mix_for_playback(player.active_session_count().await);
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
                            let play = matches!(cmd, PlayerCommand::Play { .. });
                            if play
                                && !ensure_playback_routing(
                                    &active_config,
                                    &mut modules,
                                    &backend_event_tx,
                                )
                                .await
                            {
                                continue;
                            }
                            if let Err(err) = player.handle_command(cmd).await {
                                warn!("player command failed: {err:#}");
                            }
                            sync_mic_mute_for_playback(
                                &active_config,
                                player.active_session_count().await,
                            )
                            .await;
                            sync_sfx_mix_for_playback(player.active_session_count().await);
                        }
                        Some(BackendCommand::Shutdown) => {
                            stop_voice_session(
                                &active_config,
                                &mut voice_session,
                                &mut voice_routing,
                                &backend_event_tx,
                            )
                            .await;
                            if let Err(err) = player.handle_command(PlayerCommand::StopAll).await
                            {
                                warn!("failed to stop playback on shutdown: {err:#}");
                            }
                            if let Err(err) = PipewireManager::teardown(&modules).await {
                                warn!("PipeWire teardown on shutdown failed: {err:#}");
                            }
                            break;
                        }
                        None => break,
                    }
                }
                _ = source_watch_rx.recv() => {
                    publish_mic_sources(&backend_event_tx).await;
                    publish_audio_sinks(&backend_event_tx).await;
                }
                _ = tab_watch_rx.recv() => {
                    let config = active_config.clone();
                    let event_tx = backend_event_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let tabs = TabsRepository::scan(&config).unwrap_or_default();
                        let _ = event_tx.send(BackendEvent::TabsRescanned { tabs });
                    });
                }
                Some(done) = playback_done_rx.recv() => {
                    let (tab_index, slot) = done;
                    let _ = backend_event_tx.send(BackendEvent::PlaybackEnded {
                        tab_index,
                        slot,
                    });
                    sync_mic_mute_for_playback(
                        &active_config,
                        player.active_session_count().await,
                    )
                    .await;
                    sync_sfx_mix_for_playback(player.active_session_count().await);
                }
            }
        }
    });

    Ok(())
}
