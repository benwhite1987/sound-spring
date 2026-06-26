use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, watch, Mutex};
use tokio::task::JoinHandle;
use tracing::warn;

use crate::config::SFX_SINK;
use crate::services::voice::voice_shared;

/// PipeWire/Pulse sink that follows the system default output device.
const DEFAULT_MONITOR_SINK: &str = "@DEFAULT_SINK@";

#[derive(Debug, Clone, Copy)]
pub struct VolumeState {
    pub output_percent: u8,
    pub monitor_percent: u8,
    pub output_muted: bool,
    pub monitor_muted: bool,
}

impl Default for VolumeState {
    fn default() -> Self {
        Self {
            output_percent: 100,
            monitor_percent: 100,
            output_muted: false,
            monitor_muted: false,
        }
    }
}

impl VolumeState {
    fn paplay_volume(percent: u8, muted: bool) -> u32 {
        if muted {
            0
        } else {
            (65535 * percent as u32 / 100).min(65535)
        }
    }

    pub fn output_paplay_volume(&self) -> u32 {
        Self::paplay_volume(self.output_percent, self.output_muted)
    }

    pub fn monitor_paplay_volume(&self) -> u32 {
        Self::paplay_volume(self.monitor_percent, self.monitor_muted)
    }
}

#[derive(Debug)]
pub enum PlayerCommand {
    Play {
        path: PathBuf,
        tab_index: i32,
        slot: i32,
        volumes: VolumeState,
    },
    StopSession {
        tab_index: i32,
        slot: i32,
    },
    StopAll,
    SetVolumes(VolumeState),
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Started,
}

struct PlaySession {
    spectrum_feed: JoinHandle<()>,
    tab_index: i32,
    slot: i32,
    remote_tag: String,
    monitor_tag: String,
    remote_index: Arc<Mutex<Option<String>>>,
    monitor_index: Arc<Mutex<Option<String>>>,
    stop: watch::Sender<bool>,
    _reaper: JoinHandle<()>,
}

pub struct Player {
    sink: String,
    monitor_sink: String,
    interruption_mode: String,
    next_id: u64,
    volumes: VolumeState,
    children: Arc<Mutex<HashMap<u64, PlaySession>>>,
    done_tx: Option<mpsc::Sender<(i32, i32)>>,
}

impl Player {
    pub fn new(sink: impl Into<String>) -> Self {
        Self {
            sink: sink.into(),
            monitor_sink: DEFAULT_MONITOR_SINK.to_string(),
            interruption_mode: "overlap".into(),
            next_id: 1,
            volumes: VolumeState::default(),
            children: Arc::new(Mutex::new(HashMap::new())),
            done_tx: None,
        }
    }

    pub fn set_playback_done_tx(&mut self, tx: mpsc::Sender<(i32, i32)>) {
        self.done_tx = Some(tx);
    }

    pub fn default_sink() -> Self {
        Self::new(SFX_SINK)
    }

    pub fn set_volumes(&mut self, volumes: VolumeState) {
        self.volumes = volumes;
    }

    /// Select the local monitor output device. An empty string follows the
    /// system default output (`@DEFAULT_SINK@`).
    pub fn set_monitor_sink(&mut self, sink: &str) {
        self.monitor_sink = if sink.trim().is_empty() {
            DEFAULT_MONITOR_SINK.to_string()
        } else {
            sink.trim().to_string()
        };
    }

    pub fn set_interruption_mode(&mut self, mode: &str) {
        self.interruption_mode = mode.to_string();
    }

    fn interrupts_playback(&self) -> bool {
        self.interruption_mode == "interrupt"
    }

    pub async fn active_session_count(&self) -> usize {
        self.children.lock().await.len()
    }

    pub async fn handle_command(&mut self, command: PlayerCommand) -> Result<Option<PlayerEvent>> {
        match command {
            PlayerCommand::Play {
                path,
                tab_index,
                slot,
                volumes,
            } => {
                self.volumes = volumes;
                let id = self.play(path, tab_index, slot).await?;
                let _ = id;
                Ok(Some(PlayerEvent::Started))
            }
            PlayerCommand::StopSession { tab_index, slot } => {
                self.stop_session(tab_index, slot).await;
                Ok(None)
            }
            PlayerCommand::StopAll => {
                self.stop_all().await;
                Ok(None)
            }
            PlayerCommand::SetVolumes(volumes) => {
                self.volumes = volumes;
                self.apply_volume_to_active().await;
                Ok(None)
            }
        }
    }

    pub async fn play(&mut self, file: PathBuf, tab_index: i32, slot: i32) -> Result<u64> {
        if self.interrupts_playback() {
            self.stop_all().await;
        }

        let id = self.next_id;
        self.next_id += 1;

        let remote_tag = format!("sound-spring-remote-{id}");
        let monitor_tag = format!("sound-spring-monitor-{id}");

        let remote = Self::spawn_playback(
            &self.sink,
            &file,
            &remote_tag,
            self.volumes.output_paplay_volume(),
        )
        .await?;
        let monitor = Self::spawn_playback(
            &self.monitor_sink,
            &file,
            &monitor_tag,
            self.volumes.monitor_paplay_volume(),
        )
        .await?;
        let spectrum_feed = Self::spawn_sfx_spectrum_feed(file.clone());

        let remote_index = Arc::new(Mutex::new(None));
        let monitor_index = Arc::new(Mutex::new(None));
        Self::spawn_sink_index_resolver(remote_tag.clone(), remote_index.clone());
        Self::spawn_sink_index_resolver(monitor_tag.clone(), monitor_index.clone());

        let (stop_tx, stop_rx) = watch::channel(false);
        let done_tx = self
            .done_tx
            .clone()
            .context("playback done notifier not configured")?;
        let children = self.children.clone();
        let reaper = tokio::spawn(Self::reap_playback(
            id,
            remote,
            monitor,
            tab_index,
            slot,
            stop_rx,
            done_tx,
            children,
        ));

        self.children.lock().await.insert(
            id,
            PlaySession {
                spectrum_feed,
                tab_index,
                slot,
                remote_tag,
                monitor_tag,
                remote_index,
                monitor_index,
                stop: stop_tx,
                _reaper: reaper,
            },
        );
        self.apply_volume_to_active().await;
        Ok(id)
    }

    pub async fn stop_session(&mut self, tab_index: i32, slot: i32) {
        let mut children = self.children.lock().await;
        let ids: Vec<u64> = children
            .iter()
            .filter(|(_, session)| session.tab_index == tab_index && session.slot == slot)
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            if let Some(session) = children.remove(&id) {
                session.spectrum_feed.abort();
                let _ = session.stop.send(true);
            }
        }
    }

    pub async fn stop_all(&mut self) {
        let mut children = self.children.lock().await;
        for (_, session) in children.drain() {
            session.spectrum_feed.abort();
            let _ = session.stop.send(true);
        }
    }

    async fn reap_playback(
        id: u64,
        mut remote: Child,
        mut monitor: Child,
        tab_index: i32,
        slot: i32,
        mut stop_rx: watch::Receiver<bool>,
        done_tx: mpsc::Sender<(i32, i32)>,
        children: Arc<Mutex<HashMap<u64, PlaySession>>>,
    ) {
        let finished = tokio::select! {
            changed = stop_rx.changed() => {
                if changed.is_ok() && *stop_rx.borrow() {
                    let _ = remote.start_kill();
                    let _ = monitor.start_kill();
                    let _ = remote.wait().await;
                    let _ = monitor.wait().await;
                }
                false
            }
            _ = remote.wait() => {
                let _ = monitor.start_kill();
                let _ = monitor.wait().await;
                true
            }
            _ = monitor.wait() => {
                let _ = remote.start_kill();
                let _ = remote.wait().await;
                true
            }
        };
        children.lock().await.remove(&id);
        if finished {
            let _ = done_tx.send((tab_index, slot)).await;
        }
    }

    async fn apply_volume_to_active(&self) {
        let listing = Self::list_sink_input_indices().await.ok();
        let children = self.children.lock().await;
        for session in children.values() {
            if let Err(err) = Self::apply_volume_to_session(session, listing.as_ref(), self.volumes)
                .await
            {
                warn!("failed to apply playback volume: {err:#}");
            }
        }
    }

    async fn apply_volume_to_session(
        session: &PlaySession,
        listing: Option<&HashMap<String, String>>,
        volumes: VolumeState,
    ) -> Result<()> {
        Self::set_stream_volume_cached(
            &session.remote_index,
            &session.remote_tag,
            listing,
            volumes.output_paplay_volume(),
        )
        .await?;
        Self::set_stream_volume_cached(
            &session.monitor_index,
            &session.monitor_tag,
            listing,
            volumes.monitor_paplay_volume(),
        )
        .await?;
        Ok(())
    }

    fn spawn_sink_index_resolver(tag: String, slot: Arc<Mutex<Option<String>>>) {
        tokio::spawn(async move {
            for attempt in 0..8 {
                if attempt > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
                }
                let Ok(listing) = Self::list_sink_input_indices().await else {
                    continue;
                };
                if let Some(index) = listing.get(&tag) {
                    *slot.lock().await = Some(index.clone());
                    break;
                }
            }
        });
    }

    async fn set_stream_volume_cached(
        cached: &Mutex<Option<String>>,
        name: &str,
        listing: Option<&HashMap<String, String>>,
        volume: u32,
    ) -> Result<()> {
        if let Some(index) = cached.lock().await.clone() {
            if Self::set_sink_input_volume_by_index(&index, volume).await.is_ok() {
                return Ok(());
            }
        }
        if let Some(map) = listing {
            if let Some(index) = map.get(name) {
                Self::set_sink_input_volume_by_index(index, volume).await?;
                *cached.lock().await = Some(index.clone());
                return Ok(());
            }
        }
        if Self::set_stream_volume_by_media_name(name, volume).await.is_ok() {
            if let Ok(map) = Self::list_sink_input_indices().await {
                if let Some(index) = map.get(name) {
                    *cached.lock().await = Some(index.clone());
                }
            }
        }
        Ok(())
    }

    async fn set_sink_input_volume_by_index(index: &str, volume: u32) -> Result<()> {
        let percent = ((volume as f64 / 65535.0) * 100.0).round() as u32;
        let status = Command::new("pactl")
            .args(["set-sink-input-volume", index, &format!("{percent}%")])
            .status()
            .await
            .context("pactl set-sink-input-volume")?;
        if !status.success() {
            anyhow::bail!("pactl set-sink-input-volume failed for index {index}");
        }
        Ok(())
    }

    async fn list_sink_input_indices() -> Result<HashMap<String, String>> {
        let output = Command::new("pactl")
            .args(["list", "sink-inputs"])
            .output()
            .await
            .context("pactl list sink-inputs")?;
        if !output.status.success() {
            anyhow::bail!(
                "pactl list sink-inputs failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(parse_sink_input_indices(&String::from_utf8_lossy(&output.stdout)))
    }

    async fn set_stream_volume_by_media_name(name: &str, volume: u32) -> Result<()> {
        for attempt in 0..5 {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(40)).await;
            }
            if Self::try_set_stream_volume_by_media_name(name, volume).await? {
                return Ok(());
            }
        }
        Ok(())
    }

    async fn try_set_stream_volume_by_media_name(name: &str, volume: u32) -> Result<bool> {
        let listing = Self::list_sink_input_indices().await?;
        let Some(index) = listing.get(name) else {
            return Ok(false);
        };
        Self::set_sink_input_volume_by_index(index, volume).await?;
        Ok(true)
    }

    async fn spawn_playback(
        sink: &str,
        file: &Path,
        stream_name: &str,
        volume: u32,
    ) -> Result<Child> {
        let ext = file
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        let child = match ext.as_str() {
            "mp3" | "m4a" | "aac" => {
                Self::spawn_ffmpeg_pipe(sink, file, stream_name, volume).await?
            }
            _ => Self::spawn_paplay(sink, file, stream_name, volume).await?,
        };
        Ok(child)
    }

    async fn spawn_paplay(
        sink: &str,
        file: &Path,
        stream_name: &str,
        volume: u32,
    ) -> Result<Child> {
        if !file.is_file() {
            anyhow::bail!("audio file not found: {}", file.display());
        }

        Command::new("paplay")
            .args([
                "--device",
                sink,
                &format!("--property=media.name={stream_name}"),
                &format!("--volume={volume}"),
                &file.to_string_lossy(),
            ])
            .spawn()
            .with_context(|| format!("spawn paplay for {}", file.display()))
    }

    async fn spawn_ffmpeg_pipe(
        sink: &str,
        file: &Path,
        stream_name: &str,
        volume: u32,
    ) -> Result<Child> {
        if !file.is_file() {
            anyhow::bail!("audio file not found: {}", file.display());
        }

        let mut paplay = Command::new("paplay")
            .args([
                "--device",
                sink,
                &format!("--property=media.name={stream_name}"),
                &format!("--volume={volume}"),
            ])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("spawn paplay stdin pipe")?;

        let mut ffmpeg = Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-loglevel",
                "quiet",
                "-i",
                file.as_os_str().to_str().unwrap_or_default(),
                "-f",
                "wav",
                "-",
            ])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .context("spawn ffmpeg")?;

        if let (Some(ffmpeg_out), Some(paplay_in)) = (ffmpeg.stdout.take(), paplay.stdin.take()) {
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut reader = ffmpeg_out;
                let mut writer = paplay_in;
                let mut buf = [0u8; 8192];
                loop {
                    match reader.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            if writer.write_all(&buf[..n]).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let _ = writer.shutdown().await;
            });
        }

        Ok(paplay)
    }

    /// Decode the playing clip to mono f32 @ 48 kHz and feed the mixed-spectrum
    /// SFX leg (PipeWire monitor capture is unreliable for null-sink monitors).
    fn spawn_sfx_spectrum_feed(file: PathBuf) -> JoinHandle<()> {
        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let path = file.as_os_str().to_str().unwrap_or_default();
            let mut child = match Command::new("ffmpeg")
                .args([
                    "-nostdin",
                    "-loglevel",
                    "quiet",
                    "-re",
                    "-i",
                    path,
                    "-f",
                    "f32le",
                    "-ac",
                    "1",
                    "-ar",
                    "48000",
                    "-",
                ])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(child) => child,
                Err(_) => return,
            };
            let mut stdout = match child.stdout.take() {
                Some(stdout) => stdout,
                None => return,
            };
            let shared = voice_shared();
            let mut carry = Vec::with_capacity(4);
            let mut buf = vec![0u8; 8192];
            loop {
                let n = match stdout.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };
                push_sfx_spectrum_bytes(&buf[..n], &mut carry, &shared);
            }
        })
    }
}

/// Map `media.name` property values to PulseAudio sink-input indices.
fn parse_sink_input_indices(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut current_index: Option<String> = None;
    for line in text.lines() {
        if let Some(index) = line.strip_prefix("Sink Input #") {
            current_index = Some(index.trim().to_string());
            continue;
        }
        let trimmed = line.trim();
        let Some(name) = trimmed.strip_prefix("media.name = \"") else {
            continue;
        };
        let Some(name) = name.strip_suffix('"') else {
            continue;
        };
        if let Some(index) = current_index.clone() {
            out.insert(name.to_string(), index);
        }
    }
    out
}

fn push_sfx_spectrum_bytes(
    bytes: &[u8],
    carry: &mut Vec<u8>,
    shared: &Arc<crate::services::voice::VoiceShared>,
) {
    let mut offset = 0;
    if !carry.is_empty() {
        let need = 4 - carry.len();
        let take = need.min(bytes.len());
        carry.extend_from_slice(&bytes[..take]);
        offset = take;
        if carry.len() == 4 {
            let sample = f32::from_le_bytes([carry[0], carry[1], carry[2], carry[3]]);
            if sample.is_finite() {
                shared.push_sfx_spectrum_sample(sample);
            }
            carry.clear();
        }
    }
    let rest = &bytes[offset..];
    let full = rest.len() / 4 * 4;
    for chunk in rest[..full].chunks_exact(4) {
        let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if sample.is_finite() {
            shared.push_sfx_spectrum_sample(sample);
        }
    }
    carry.extend_from_slice(&rest[full..]);
}

#[cfg(test)]
mod tests {
    #[test]
    fn interrupt_mode_matches_config_value() {
        let mut player = super::Player::default_sink();
        player.set_interruption_mode("interrupt");
        assert!(player.interrupts_playback());
        player.set_interruption_mode("overlap");
        assert!(!player.interrupts_playback());
    }

    #[test]
    fn parse_sink_input_indices_maps_media_names() {
        let text = r#"
Sink Input #42
	media.name = "sound-spring-remote-1"
Sink Input #43
	media.name = "sound-spring-monitor-1"
"#;
        let map = super::parse_sink_input_indices(text);
        assert_eq!(map.get("sound-spring-remote-1").map(String::as_str), Some("42"));
        assert_eq!(map.get("sound-spring-monitor-1").map(String::as_str), Some("43"));
    }

    #[test]
    fn stop_session_filter_is_tab_scoped() {
        let sessions = [(0_i32, 1_i32), (1, 1), (0, 2)];
        let targets: Vec<(i32, i32)> = sessions
            .iter()
            .copied()
            .filter(|(tab_index, slot)| *tab_index == 0 && *slot == 1)
            .collect();
        assert_eq!(targets, vec![(0, 1)]);
    }
}
