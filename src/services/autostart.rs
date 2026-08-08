use anyhow::{Context, Result};
use directories::BaseDirs;
use std::fs;
use std::path::PathBuf;

const AUTOSTART_DESKTOP_NAME: &str = "sound-spring.desktop";

pub fn autostart_desktop_path() -> Option<PathBuf> {
    BaseDirs::new().map(|dirs| {
        dirs.config_dir()
            .join("autostart")
            .join(AUTOSTART_DESKTOP_NAME)
    })
}

/// Resolve a stable Exec= path for autostart (prefer AppImage / install path
/// over ephemeral `cargo run` / target/debug binaries).
pub fn resolve_autostart_exec() -> String {
    if let Ok(appimage) = std::env::var("APPIMAGE") {
        let path = PathBuf::from(&appimage);
        if path.is_file() {
            return appimage;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let exe_str = exe.to_string_lossy();
        let looks_ephemeral = exe_str.contains("/target/debug/")
            || exe_str.contains("/target/release/")
            || exe_str.contains("/.cache/");
        if looks_ephemeral {
            for candidate in ["/usr/bin/sound-spring", "/usr/local/bin/sound-spring"] {
                if PathBuf::from(candidate).is_file() {
                    return candidate.to_string();
                }
            }
        }
        return exe_str.into_owned();
    }
    "sound-spring".into()
}

/// Writes or removes `~/.config/autostart/sound-spring.desktop`.
pub fn sync_launch_at_login(enabled: bool) -> Result<()> {
    let path = autostart_desktop_path().context("resolve XDG autostart path")?;
    if enabled {
        let exec = resolve_autostart_exec();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create autostart dir {}", parent.display()))?;
        }
        fs::write(&path, render_autostart_desktop(&exec))
            .with_context(|| format!("write autostart entry {}", path.display()))?;
    } else if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("remove autostart entry {}", path.display()))?;
    }
    Ok(())
}

fn render_autostart_desktop(exec: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Sound Spring\n\
         GenericName=Soundboard\n\
         Comment=PipeWire soundboard\n\
         Exec={}\n\
         Icon=io.github.benwhite1987.SoundSpring\n\
         Terminal=false\n\
         StartupWMClass=sound-spring\n\
         Hidden=false\n\
         X-GNOME-Autostart-enabled=true\n",
        format_desktop_exec(exec)
    )
}

fn format_desktop_exec(exec: &str) -> String {
    if exec.contains(' ') || exec.contains('"') || exec.contains('\t') {
        format!("\"{}\"", exec.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        exec.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_exec_quotes_paths_with_spaces() {
        assert_eq!(
            format_desktop_exec("/opt/Sound Spring/bin/sound-spring"),
            "\"/opt/Sound Spring/bin/sound-spring\""
        );
    }

    #[test]
    fn autostart_entry_contains_exec() {
        let text = render_autostart_desktop("/usr/bin/sound-spring");
        assert!(text.contains("Exec=/usr/bin/sound-spring\n"));
        assert!(text.contains("Icon=io.github.benwhite1987.SoundSpring\n"));
        assert!(text.contains("StartupWMClass=sound-spring"));
    }

    #[test]
    fn resolve_autostart_prefers_appimage_env() {
        std::env::set_var("APPIMAGE", "/tmp/does-not-exist-sound-spring.AppImage");
        // Missing file falls through; just ensure it does not panic.
        let _ = resolve_autostart_exec();
        std::env::remove_var("APPIMAGE");
    }
}
