use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::warn;

use crate::config::SFX_SINK;

const MONITOR_SINK: &str = "@DEFAULT_SINK@";

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
        slot: i32,
        volumes: VolumeState,
    },
    StopSlot(i32),
    StopAll,
    SetVolumes(VolumeState),
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Started { id: u64, slot: Option<i32> },
    Ended { id: u64 },
}

struct PlaySession {
    remote: Child,
    monitor: Child,
    slot: i32,
    remote_tag: String,
    monitor_tag: String,
}

pub struct Player {
    sink: String,
    next_id: u64,
    volumes: VolumeState,
    children: Arc<Mutex<HashMap<u64, PlaySession>>>,
}

impl Player {
    pub fn new(sink: impl Into<String>) -> Self {
        Self {
            sink: sink.into(),
            next_id: 1,
            volumes: VolumeState::default(),
            children: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn default_sink() -> Self {
        Self::new(SFX_SINK)
    }

    pub fn set_volumes(&mut self, volumes: VolumeState) {
        self.volumes = volumes;
    }

    pub async fn handle_command(&mut self, command: PlayerCommand) -> Result<Option<PlayerEvent>> {
        match command {
            PlayerCommand::Play { path, slot, volumes } => {
                self.volumes = volumes;
                let id = self.play(path, slot).await?;
                Ok(Some(PlayerEvent::Started { id, slot: Some(slot) }))
            }
            PlayerCommand::StopSlot(slot) => {
                self.stop_slot(slot).await;
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

    pub async fn play(&mut self, file: PathBuf, slot: i32) -> Result<u64> {
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
            MONITOR_SINK,
            &file,
            &monitor_tag,
            self.volumes.monitor_paplay_volume(),
        )
        .await?;

        self.children.lock().await.insert(
            id,
            PlaySession {
                remote,
                monitor,
                slot,
                remote_tag,
                monitor_tag,
            },
        );
        self.apply_volume_to_active().await;
        Ok(id)
    }

    pub async fn stop_slot(&mut self, slot: i32) {
        let mut children = self.children.lock().await;
        let ids: Vec<u64> = children
            .iter()
            .filter(|(_, session)| session.slot == slot)
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            if let Some(mut session) = children.remove(&id) {
                let _ = session.remote.kill().await;
                let _ = session.monitor.kill().await;
            }
        }
    }

    pub async fn stop_all(&mut self) {
        let mut children = self.children.lock().await;
        for (_, mut session) in children.drain() {
            let _ = session.remote.kill().await;
            let _ = session.monitor.kill().await;
        }
    }

    pub async fn reap_finished(&mut self) -> Vec<i32> {
        let mut finished_slots = Vec::new();
        let mut children = self.children.lock().await;
        children.retain(|id, session| {
            let remote_done = matches!(session.remote.try_wait(), Ok(Some(_)) | Err(_));
            let monitor_done = matches!(session.monitor.try_wait(), Ok(Some(_)) | Err(_));
            if remote_done || monitor_done {
                if !remote_done {
                    let _ = session.remote.start_kill();
                }
                if !monitor_done {
                    let _ = session.monitor.start_kill();
                }
                finished_slots.push(session.slot);
                false
            } else {
                true
            }
        });
        finished_slots
    }

    async fn apply_volume_to_active(&self) {
        let children = self.children.lock().await;
        for session in children.values() {
            if let Err(err) = Self::set_stream_volume_by_media_name(
                &session.remote_tag,
                self.volumes.output_paplay_volume(),
            )
            .await
            {
                warn!(
                    "failed to set remote stream volume for {}: {err:#}",
                    session.remote_tag
                );
            }
            if let Err(err) = Self::set_stream_volume_by_media_name(
                &session.monitor_tag,
                self.volumes.monitor_paplay_volume(),
            )
            .await
            {
                warn!(
                    "failed to set monitor stream volume for {}: {err:#}",
                    session.monitor_tag
                );
            }
        }
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

        let marker = format!("media.name = \"{name}\"");
        let text = String::from_utf8_lossy(&output.stdout);
        let mut current_index: Option<String> = None;

        for line in text.lines() {
            if let Some(index) = line.strip_prefix("Sink Input #") {
                current_index = Some(index.trim().to_string());
                continue;
            }
            if line.trim() == marker {
                if let Some(index) = current_index {
                    let percent = ((volume as f64 / 65535.0) * 100.0).round() as u32;
                    let status = Command::new("pactl")
                        .args([
                            "set-sink-input-volume",
                            &index,
                            &format!("{percent}%"),
                        ])
                        .status()
                        .await
                        .context("pactl set-sink-input-volume")?;
                    if !status.success() {
                        anyhow::bail!("pactl set-sink-input-volume failed for index {index}");
                    }
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    async fn spawn_playback(
        sink: &str,
        file: &PathBuf,
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
        file: &PathBuf,
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
        file: &PathBuf,
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
}
