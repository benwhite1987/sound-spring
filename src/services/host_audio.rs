//! Spawn host audio CLI tools without inheriting the AppImage/Qt runtime library
//! path, which can break `pactl`, `paplay`, and `pw-cat` on some systems.

use tokio::process::Command;

const STRIPPED_ENV_VARS: &[&str] = &[
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "QT_PLUGIN_PATH",
    "QML2_IMPORT_PATH",
];

/// Build a `tokio::process::Command` for a host-provided audio binary.
pub fn host_audio_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    for var in STRIPPED_ENV_VARS {
        cmd.env_remove(var);
    }
    if let Ok(path) = std::env::var("PATH") {
        if !path.split(':').any(|entry| entry == "/usr/bin") {
            cmd.env("PATH", format!("/usr/bin:/bin:{path}"));
        }
    }
    cmd
}
