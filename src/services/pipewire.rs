use anyhow::{anyhow, Context, Result};
use std::process::Stdio;
use tokio::process::Command;
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

pub struct PipewireManager;

impl PipewireManager {
    pub async fn available_sources() -> Result<Vec<MicSource>> {
        let output = Command::new("pactl")
            .args(["list", "sources", "short"])
            .output()
            .await
            .context("pactl list sources")?;
        if !output.status.success() {
            return Err(anyhow!(
                "pactl list sources failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let mut sources = Vec::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let mut parts = line.split_whitespace();
            let _index = parts.next();
            let name = parts.next().unwrap_or_default();
            if name.is_empty()
                || name.ends_with(".monitor")
                || name.starts_with("soundboard_")
                || name.starts_with("sound_spring_")
            {
                continue;
            }
            sources.push(MicSource {
                name: name.to_string(),
                description: name.to_string(),
            });
        }
        Ok(sources)
    }

    pub async fn setup(mic_source: &str, latency_ms: u32) -> Result<Modules> {
        Self::teardown(&Modules::default()).await.ok();
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
            let _ = Command::new("pactl")
                .args(["unload-module", &id.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
        }
        Ok(())
    }

    pub async fn sink_exists(name: &str) -> bool {
        Self::list_short("sinks")
            .await
            .map(|lines| lines.iter().any(|line| line.contains(name)))
            .unwrap_or(false)
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

    async fn load_remap_source(virtmic_sink: &str, source_name: &str, description: &str) -> Result<u32> {
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
        let id = String::from_utf8_lossy(&output.stdout).trim().parse::<u32>()?;
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
        if lines.iter().any(|line| line.split_whitespace().nth(1) == Some(name)) {
            Ok(())
        } else {
            Err(anyhow!("source not found: {name}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_sources_does_not_panic() {
        let _ = PipewireManager::available_sources().await;
    }
}
