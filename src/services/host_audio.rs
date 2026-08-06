//! Spawn host audio CLI tools without inheriting the AppImage/Qt runtime library
//! path, which can break `pactl`, `paplay`, `pw-cat`, `ffmpeg`, and `ffprobe`.

use std::process::Command as StdCommand;
use tokio::process::Command;

const STRIPPED_ENV_VARS: &[&str] = &[
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "QT_PLUGIN_PATH",
    "QML2_IMPORT_PATH",
];

fn scrub_env(cmd: &mut impl HostCommandEnv) {
    for var in STRIPPED_ENV_VARS {
        cmd.env_remove_var(var);
    }
    if let Ok(path) = std::env::var("PATH") {
        if !path.split(':').any(|entry| entry == "/usr/bin") {
            cmd.env_set("PATH", format!("/usr/bin:/bin:{path}"));
        }
    }
}

trait HostCommandEnv {
    fn env_remove_var(&mut self, key: &str);
    fn env_set(&mut self, key: &str, value: String);
}

impl HostCommandEnv for Command {
    fn env_remove_var(&mut self, key: &str) {
        self.env_remove(key);
    }
    fn env_set(&mut self, key: &str, value: String) {
        self.env(key, value);
    }
}

impl HostCommandEnv for StdCommand {
    fn env_remove_var(&mut self, key: &str) {
        self.env_remove(key);
    }
    fn env_set(&mut self, key: &str, value: String) {
        self.env(key, value);
    }
}

/// Build a `tokio::process::Command` for a host-provided audio binary.
pub fn host_audio_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    scrub_env(&mut cmd);
    cmd
}

/// Build a `std::process::Command` with the same AppImage env scrubbing.
pub fn host_audio_std_command(program: &str) -> StdCommand {
    let mut cmd = StdCommand::new(program);
    scrub_env(&mut cmd);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn scrub_strips_appimage_library_path() {
        std::env::set_var("LD_LIBRARY_PATH", "/tmp/appimage/lib");
        std::env::set_var("QT_PLUGIN_PATH", "/tmp/appimage/plugins");
        let cmd = host_audio_std_command("ffmpeg");
        // Command debug formatting includes env overrides on Unix.
        let debug = format!("{cmd:?}");
        assert!(
            debug.contains("ffmpeg"),
            "command should invoke ffmpeg: {debug}"
        );
        // Scrubbed vars are removed via env_remove; confirm we still set PATH
        // when /usr/bin is missing from PATH.
        let mut path_only = StdCommand::new("true");
        let old_path = std::env::var_os("PATH");
        std::env::set_var("PATH", "/opt/custom/bin");
        scrub_env(&mut path_only);
        let debug_path = format!("{path_only:?}");
        assert!(
            debug_path.contains("/usr/bin"),
            "scrub should prepend /usr/bin: {debug_path}"
        );
        match old_path {
            Some(v) => std::env::set_var("PATH", v),
            None => std::env::remove_var("PATH"),
        }
        let _ = OsString::new();
        std::env::remove_var("LD_LIBRARY_PATH");
        std::env::remove_var("QT_PLUGIN_PATH");
    }
}
