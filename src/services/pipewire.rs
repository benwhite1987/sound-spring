use anyhow::{anyhow, Context, Result};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::Sender as TokioSender;
use tokio::time::{sleep, sleep_until, Duration, Instant};
use tracing::{info, warn};

pub const VIRTMIC_SINK: &str = "soundboard_virtmic";
pub const SFX_SINK: &str = "soundboard_sfx";
pub const VIRTUAL_MIC_SOURCE: &str = "sound_spring_virtual_mic";

pub const DISPLAY_MIC: &str = "Sound-Spring-Virtual-Microphone";
pub const DISPLAY_EFFECTS: &str = "Sound-Spring-Effects";
pub const DISPLAY_MIX: &str = "Sound-Spring-Mix";

#[derive(Debug, Clone, Default)]
pub struct Modules {
    pub ids: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct MicSource {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct AudioSink {
    pub name: String,
    pub description: String,
}

pub struct PipewireManager;

impl PipewireManager {
    pub async fn available_sources() -> Result<Vec<MicSource>> {
        let output = Command::new("pactl")
            .args(["list", "sources"])
            .output()
            .await
            .context("pactl list sources")?;
        if !output.status.success() {
            return Err(anyhow!(
                "pactl list sources failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(parse_source_list(&String::from_utf8_lossy(&output.stdout)))
    }

    pub async fn available_sinks() -> Result<Vec<AudioSink>> {
        let output = Command::new("pactl")
            .args(["list", "sinks"])
            .output()
            .await
            .context("pactl list sinks")?;
        if !output.status.success() {
            return Err(anyhow!(
                "pactl list sinks failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(parse_sink_list(&String::from_utf8_lossy(&output.stdout)))
    }

    pub fn spawn_source_watch(notify_tx: TokioSender<()>) {
        tokio::spawn(async move {
            loop {
                if let Err(err) = Self::run_source_subscribe(&notify_tx).await {
                    warn!("pactl source subscribe ended: {err:#}");
                    sleep(Duration::from_secs(2)).await;
                }
            }
        });
    }

    async fn run_source_subscribe(notify_tx: &TokioSender<()>) -> Result<()> {
        // Subscribe to all PipeWire/Pulse events (not just sources) so device
        // hotplug for both microphones and output sinks triggers a refresh.
        let mut child = Command::new("pactl")
            .args(["subscribe"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("spawn pactl subscribe")?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("pactl subscribe missing stdout"))?;
        let mut lines = BufReader::new(stdout).lines();
        let debounce = Duration::from_millis(500);
        let mut deadline = Instant::now() + Duration::from_secs(3600);
        loop {
            tokio::select! {
                line = lines.next_line() => {
                    match line.context("read pactl subscribe")? {
                        Some(_) => deadline = Instant::now() + debounce,
                        None => return Ok(()),
                    }
                }
                _ = sleep_until(deadline) => {
                    let _ = notify_tx.send(()).await;
                    deadline = Instant::now() + Duration::from_secs(3600);
                }
            }
        }
    }

    pub async fn setup(mic_source: &str, latency_ms: u32) -> Result<Modules> {
        if Self::sinks_ready().await {
            let ids = Self::find_module_ids().await?;
            if !ids.is_empty() {
                info!(
                    "reusing {} existing soundboard PipeWire module(s)",
                    ids.len()
                );
                return Ok(Modules { ids });
            }
        }

        Self::unload_stale_modules().await?;
        sleep(Duration::from_millis(100)).await;
        let mut ids = Vec::new();

        ids.push(
            Self::load_null_sink(VIRTMIC_SINK, DISPLAY_MIX)
                .await
                .context("load virtmic sink")?,
        );
        ids.push(
            Self::load_null_sink(SFX_SINK, DISPLAY_EFFECTS)
                .await
                .context("load sfx sink")?,
        );

        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        Self::ensure_sink(SFX_SINK).await?;
        Self::ensure_sink(VIRTMIC_SINK).await?;
        Self::ensure_source(&format!("{SFX_SINK}.monitor")).await?;

        ids.push(
            Self::load_loopback(&format!("{SFX_SINK}.monitor"), VIRTMIC_SINK, latency_ms)
                .await
                .context("loop sfx to virtmic")?,
        );

        if !mic_source.is_empty() {
            ids.push(
                Self::load_loopback(mic_source, VIRTMIC_SINK, latency_ms)
                    .await
                    .with_context(|| format!("loop mic {mic_source}"))?,
            );
        }

        ids.push(
            Self::load_remap_source(VIRTMIC_SINK, VIRTUAL_MIC_SOURCE, DISPLAY_MIC)
                .await
                .context("remap virtual mic source")?,
        );

        Self::ensure_source(VIRTUAL_MIC_SOURCE).await?;

        info!("PipeWire setup complete with {} modules", ids.len());
        Ok(Modules { ids })
    }

    pub async fn teardown(modules: &Modules) -> Result<()> {
        for id in modules.ids.iter().copied().rev() {
            Self::unload_module(id).await;
        }
        Self::unload_stale_modules().await
    }

    pub async fn unload_stale_modules() -> Result<()> {
        let mut ids = Self::find_module_ids().await?;
        ids.sort_unstable_by(|a, b| b.cmp(a));
        ids.dedup();
        if ids.is_empty() {
            return Ok(());
        }
        info!(
            "unloading {} stale soundboard PipeWire module(s)",
            ids.len()
        );
        for id in ids {
            Self::unload_module(id).await;
        }
        Ok(())
    }

    pub async fn set_sink_volume(sink: &str, percent: u8, muted: bool) -> Result<()> {
        let mute_arg = if muted { "1" } else { "0" };
        let output = Command::new("pactl")
            .args(["set-sink-mute", sink, mute_arg])
            .output()
            .await
            .context("pactl set-sink-mute")?;
        if !output.status.success() {
            return Err(anyhow!(
                "pactl set-sink-mute failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        if !muted {
            let volume = (65535 * percent as u32 / 100).to_string();
            let output = Command::new("pactl")
                .args(["set-sink-volume", sink, &volume])
                .output()
                .await
                .context("pactl set-sink-volume")?;
            if !output.status.success() {
                return Err(anyhow!(
                    "pactl set-sink-volume failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
        Ok(())
    }

    async fn unload_module(id: u32) {
        let _ = Command::new("pactl")
            .args(["unload-module", &id.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
    }

    async fn find_module_ids() -> Result<Vec<u32>> {
        let lines = Self::list_short("modules").await?;
        let mut ids = Vec::new();
        for line in lines {
            let mut parts = line.split_whitespace();
            let Some(id_str) = parts.next() else {
                continue;
            };
            let Ok(id) = id_str.parse::<u32>() else {
                continue;
            };
            let rest = parts.collect::<Vec<_>>().join(" ");
            if module_matches_soundboard(&rest) {
                ids.push(id);
            }
        }
        Ok(ids)
    }

    pub async fn sink_exists(name: &str) -> bool {
        Self::list_short("sinks")
            .await
            .map(|lines| lines.iter().any(|line| line.contains(name)))
            .unwrap_or(false)
    }

    async fn sinks_ready() -> bool {
        Self::sink_exists(SFX_SINK).await
            && Self::sink_exists(VIRTMIC_SINK).await
            && Self::ensure_source(VIRTUAL_MIC_SOURCE).await.is_ok()
    }

    async fn load_null_sink(name: &str, description: &str) -> Result<u32> {
        let output = Command::new("pactl")
            .args([
                "load-module",
                "module-null-sink",
                &format!("sink_name={name}"),
                &format!("sink_properties=device.description={description}"),
            ])
            .output()
            .await?;
        Self::parse_module_id(&output)
    }

    async fn load_loopback(source: &str, sink: &str, latency_ms: u32) -> Result<u32> {
        let output = Command::new("pactl")
            .args([
                "load-module",
                "module-loopback",
                &format!("source={source}"),
                &format!("sink={sink}"),
                &format!("latency_msec={latency_ms}"),
            ])
            .output()
            .await?;
        Self::parse_module_id(&output)
    }

    async fn load_remap_source(
        virtmic_sink: &str,
        source_name: &str,
        description: &str,
    ) -> Result<u32> {
        let output = Command::new("pactl")
            .args([
                "load-module",
                "module-remap-source",
                &format!("master={virtmic_sink}.monitor"),
                &format!("source_name={source_name}"),
                &format!("source_properties=device.description={description}"),
            ])
            .output()
            .await?;
        Self::parse_module_id(&output)
    }

    fn parse_module_id(output: &std::process::Output) -> Result<u32> {
        if !output.status.success() {
            return Err(anyhow!(
                "pactl failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let id = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u32>()?;
        Ok(id)
    }

    async fn list_short(kind: &str) -> Result<Vec<String>> {
        let output = Command::new("pactl")
            .args(["list", "short", kind])
            .output()
            .await?;
        if !output.status.success() {
            return Err(anyhow!("pactl list short {kind} failed"));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_string)
            .collect())
    }

    async fn ensure_sink(name: &str) -> Result<()> {
        if Self::sink_exists(name).await {
            Ok(())
        } else {
            Err(anyhow!("sink not found: {name}"))
        }
    }

    async fn ensure_source(name: &str) -> Result<()> {
        let lines = Self::list_short("sources").await?;
        if lines
            .iter()
            .any(|line| line.split_whitespace().nth(1) == Some(name))
        {
            Ok(())
        } else {
            Err(anyhow!("source not found: {name}"))
        }
    }
}

fn parse_source_list(text: &str) -> Vec<MicSource> {
    let mut sources = Vec::new();
    let mut name = String::new();
    let mut description = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Source #") {
            if !name.is_empty() && is_user_mic_source(&name) {
                sources.push(MicSource {
                    name: name.clone(),
                    description: if description.is_empty() {
                        name.clone()
                    } else {
                        description.clone()
                    },
                });
            }
            name.clear();
            description.clear();
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("Name: ") {
            name = value.to_string();
        } else if let Some(value) = trimmed.strip_prefix("Description: ") {
            description = value.to_string();
        }
    }
    if !name.is_empty() && is_user_mic_source(&name) {
        sources.push(MicSource {
            name: name.clone(),
            description: if description.is_empty() {
                name
            } else {
                description
            },
        });
    }
    sources.sort_by(|a, b| a.description.cmp(&b.description));
    sources
}

fn is_user_mic_source(name: &str) -> bool {
    !name.is_empty()
        && !name.ends_with(".monitor")
        && !name.starts_with("soundboard_")
        && !name.starts_with("sound_spring_")
}

fn parse_sink_list(text: &str) -> Vec<AudioSink> {
    let mut sinks = Vec::new();
    let mut name = String::new();
    let mut description = String::new();
    let flush = |name: &mut String, description: &mut String, out: &mut Vec<AudioSink>| {
        if !name.is_empty() && is_user_sink(name) {
            out.push(AudioSink {
                name: name.clone(),
                description: if description.is_empty() {
                    name.clone()
                } else {
                    description.clone()
                },
            });
        }
        name.clear();
        description.clear();
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Sink #") {
            flush(&mut name, &mut description, &mut sinks);
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("Name: ") {
            name = value.to_string();
        } else if let Some(value) = trimmed.strip_prefix("Description: ") {
            description = value.to_string();
        }
    }
    flush(&mut name, &mut description, &mut sinks);
    sinks.sort_by(|a, b| a.description.cmp(&b.description));
    sinks
}

fn is_user_sink(name: &str) -> bool {
    !name.is_empty() && !name.starts_with("soundboard_") && !name.starts_with("sound_spring_")
}

fn module_matches_soundboard(rest: &str) -> bool {
    if rest.contains(&format!("sink_name={VIRTMIC_SINK}"))
        || rest.contains(&format!("sink_name={SFX_SINK}"))
    {
        return true;
    }
    if rest.contains(&format!("source_name={VIRTUAL_MIC_SOURCE}")) {
        return true;
    }
    if rest.contains("module-loopback") && (rest.contains(VIRTMIC_SINK) || rest.contains(SFX_SINK))
    {
        return true;
    }
    if rest.contains("module-remap-source")
        && (rest.contains(VIRTMIC_SINK) || rest.contains(VIRTUAL_MIC_SOURCE))
    {
        return true;
    }
    (rest.contains("sink_name=virtmic") || rest.contains("sink_name=soundboard"))
        && !rest.contains(SFX_SINK)
        && !rest.contains(VIRTMIC_SINK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sources_skips_monitors_and_virtual() {
        let text = "\
Source #0
\tName: sound_spring_virtual_mic
\tDescription: Sound-Spring-Virtual-Microphone
Source #1
\tName: alsa_input.usb_mic
\tDescription: USB Microphone
Source #2
\tName: alsa_output.speakers.monitor
\tDescription: Speakers Monitor
";
        let sources = parse_source_list(text);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "alsa_input.usb_mic");
        assert_eq!(sources[0].description, "USB Microphone");
    }

    #[test]
    fn module_matcher_finds_soundboard_modules() {
        assert!(module_matches_soundboard(
            "module-null-sink sink_name=soundboard_virtmic sink_properties=device.description=Sound-Spring-Mix"
        ));
        assert!(module_matches_soundboard(
            "module-loopback source=soundboard_sfx.monitor sink=soundboard_virtmic latency_msec=20"
        ));
        assert!(!module_matches_soundboard(
            "module-null-sink sink_name=other_sink"
        ));
    }

    #[test]
    fn parse_sinks_skips_soundboard_sinks() {
        let text = "\
Sink #10
\tName: soundboard_sfx
\tDescription: Sound-Spring-Effects
Sink #11
\tName: alsa_output.pci-0000_00.analog-stereo
\tDescription: Built-in Speakers
Sink #12
\tName: alsa_output.usb-headset
\tDescription: USB Headset
";
        let sinks = parse_sink_list(text);
        assert_eq!(sinks.len(), 2);
        assert_eq!(sinks[0].description, "Built-in Speakers");
        assert_eq!(sinks[1].name, "alsa_output.usb-headset");
    }

    #[tokio::test]
    async fn list_sources_does_not_panic() {
        let _ = PipewireManager::available_sources().await;
    }

    #[tokio::test]
    async fn list_sinks_does_not_panic() {
        let _ = PipewireManager::available_sinks().await;
    }
}
