use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::warn;

use crate::config::SFX_SINK;

#[derive(Debug)]
pub enum PlayerCommand {
    Play(PathBuf),
    Stop(u64),
    StopAll,
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Started { id: u64, slot: Option<i32> },
    Ended { id: u64 },
}

pub struct Player {
    sink: String,
    next_id: u64,
    children: Arc<Mutex<HashMap<u64, Child>>>,
}

impl Player {
    pub fn new(sink: impl Into<String>) -> Self {
        Self {
            sink: sink.into(),
            next_id: 1,
            children: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn default_sink() -> Self {
        Self::new(SFX_SINK)
    }

    pub async fn handle_command(&mut self, command: PlayerCommand) -> Result<Option<PlayerEvent>> {
        match command {
            PlayerCommand::Play(path) => {
                let id = self.play(path).await?;
                Ok(Some(PlayerEvent::Started { id, slot: None }))
            }
            PlayerCommand::Stop(id) => {
                self.stop(id).await;
                Ok(Some(PlayerEvent::Ended { id }))
            }
            PlayerCommand::StopAll => {
                self.stop_all().await;
                Ok(None)
            }
        }
    }

    pub async fn play(&mut self, file: PathBuf) -> Result<u64> {
        let id = self.next_id;
        self.next_id += 1;

        let child = Self::spawn_playback(&self.sink, &file).await?;
        self.children.lock().await.insert(id, child);
        Ok(id)
    }

    pub async fn stop(&mut self, id: u64) {
        if let Some(mut child) = self.children.lock().await.remove(&id) {
            let _ = child.kill().await;
        }
    }

    pub async fn stop_all(&mut self) {
        let mut children = self.children.lock().await;
        for (_, mut child) in children.drain() {
            let _ = child.kill().await;
        }
    }

    pub async fn reap_finished(&mut self) -> Vec<u64> {
        let mut finished = Vec::new();
        let mut children = self.children.lock().await;
        children.retain(|id, child| {
            match child.try_wait() {
                Ok(Some(_)) => {
                    finished.push(*id);
                    false
                }
                Ok(None) => true,
                Err(err) => {
                    warn!("try_wait failed for play id {id}: {err}");
                    finished.push(*id);
                    false
                }
            }
        });
        finished
    }

    async fn spawn_playback(sink: &str, file: &PathBuf) -> Result<Child> {
        let ext = file
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        let child = match ext.as_str() {
            "mp3" | "m4a" | "aac" => Self::spawn_ffmpeg_pipe(sink, file).await?,
            _ => Self::spawn_paplay(sink, file).await?,
        };
        Ok(child)
    }

    async fn spawn_paplay(sink: &str, file: &PathBuf) -> Result<Child> {
        Command::new("paplay")
            .args(["--device", sink, &file.to_string_lossy()])
            .spawn()
            .with_context(|| format!("spawn paplay for {}", file.display()))
    }

    async fn spawn_ffmpeg_pipe(sink: &str, file: &PathBuf) -> Result<Child> {
        let mut paplay = Command::new("paplay")
            .args(["--device", sink, "-"])
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
