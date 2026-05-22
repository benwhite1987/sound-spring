#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOUNDBOARD_SHARE="${XDG_DATA_HOME:-$HOME/.local/share}/soundspring"
SYSTEMD_USER_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
SYSTEMD_UNIT="soundboard-pipewire.service"

# shellcheck source=scripts/pipewire-common.sh
source "$SCRIPT_DIR/scripts/pipewire-common.sh"
# shellcheck source=scripts/config-common.sh
source "$SCRIPT_DIR/scripts/config-common.sh"

usage() {
    cat <<EOF
usage: ./install.sh [options]

Options:
  --mic <source>   Set microphone source (pactl source name)
  --skip-mic       Sound Spring audio only (no mic loopback)
  -h, --help       Show this help

Environment:
  SOUNDBOARD_MIC   Same as --mic
EOF
}

SELECTED_MIC=""
SKIP_MIC=0
NONINTERACTIVE=0
USE_SAVED_MIC=""
LATENCY_MS="$DEFAULT_LATENCY_MS"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --mic)
            [[ $# -ge 2 ]] || { echo "❌ --mic requires a source name" >&2; exit 1; }
            SELECTED_MIC="$2"
            NONINTERACTIVE=1
            shift 2
            ;;
        --skip-mic)
            SELECTED_MIC=""
            SKIP_MIC=1
            NONINTERACTIVE=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "❌ Unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [[ -n "${SOUNDBOARD_MIC:-}" ]]; then
    SELECTED_MIC="$SOUNDBOARD_MIC"
    NONINTERACTIVE=1
fi

echo "Setting up Sound Spring..."

install_package() {
    local pkg="$1"
    if command -v apt &> /dev/null; then
        sudo apt install -y "$pkg"
    elif command -v dnf &> /dev/null; then
        sudo dnf install -y "$pkg"
    elif command -v pacman &> /dev/null; then
        sudo pacman -S --noconfirm "$pkg"
    else
        echo "❌ Cannot auto-install $pkg. Please install it manually." >&2
        exit 1
    fi
}

echo "Checking for required dependencies..."

if ! command -v pactl &> /dev/null || ! command -v paplay &> /dev/null; then
    echo "Installing PipeWire..."
    if command -v apt &> /dev/null; then
        install_package pipewire
        install_package pipewire-audio
    elif command -v pacman &> /dev/null; then
        install_package pipewire
        install_package pipewire-pulse
        install_package pipewire-alsa
    elif command -v dnf &> /dev/null; then
        install_package pipewire
        install_package pipewire-pulseaudio
    fi
fi

if ! command -v notify-send &> /dev/null; then
    echo "Installing libnotify..."
    if command -v apt &> /dev/null; then
        install_package libnotify-bin
    elif command -v dnf &> /dev/null || command -v pacman &> /dev/null; then
        install_package libnotify
    fi
fi

if ! command -v ffmpeg &> /dev/null; then
    echo "⚠️  ffmpeg not installed — MP3/M4A/AAC playback needs ffmpeg or paplay with MP3 support."
    echo "   Install ffmpeg for reliable MP3 support (e.g. pacman -S ffmpeg)."
fi

mkdir -p "$SOUNDBOARD_CONFIG_DIR"
mkdir -p "$DEFAULT_TABS_ROOT"
mkdir -p "${HOME}/.local/bin"
mkdir -p "$DEFAULT_STATE_DIR"
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/sxhkd"
mkdir -p "$SOUNDBOARD_SHARE"
mkdir -p "$SYSTEMD_USER_DIR"

LOCAL_BIN="${HOME}/.local/bin"

install -m 755 "$SCRIPT_DIR/scripts/sb-play" "$LOCAL_BIN/sb-play"
install -m 755 "$SCRIPT_DIR/scripts/sb-tab" "$LOCAL_BIN/sb-tab"
install -m 755 "$SCRIPT_DIR/scripts/sb-stop" "$LOCAL_BIN/sb-stop"
install -m 644 "$SCRIPT_DIR/scripts/sxhkdrc.fragment" "${XDG_CONFIG_HOME:-$HOME/.config}/sxhkd/soundboard.conf"

install -m 644 "$SCRIPT_DIR/scripts/pipewire-common.sh" "$SOUNDBOARD_SHARE/pipewire-common.sh"
install -m 644 "$SCRIPT_DIR/scripts/config-common.sh" "$SOUNDBOARD_SHARE/config-common.sh"
install -m 644 "$SCRIPT_DIR/scripts/state-common.sh" "$SOUNDBOARD_SHARE/state-common.sh"
install -m 644 "$SCRIPT_DIR/scripts/tabs-common.sh" "$SOUNDBOARD_SHARE/tabs-common.sh"
install -m 644 "$SCRIPT_DIR/scripts/audio-common.sh" "$SOUNDBOARD_SHARE/audio-common.sh"
install -m 755 "$SCRIPT_DIR/scripts/play-file.sh" "$SOUNDBOARD_SHARE/play-file.sh"
install -m 755 "$SCRIPT_DIR/scripts/setup-pipewire.sh" "$SOUNDBOARD_SHARE/setup-pipewire.sh"

install -m 644 "$SCRIPT_DIR/scripts/soundboard-pipewire.service" "$SYSTEMD_USER_DIR/$SYSTEMD_UNIT"
echo "✓ Installed systemd user unit: $SYSTEMD_USER_DIR/$SYSTEMD_UNIT"

TABS_ROOT="$DEFAULT_TABS_ROOT"
if config_exists; then
    TABS_ROOT="$(read_tabs_root)"
fi
mkdir -p "$TABS_ROOT/01-memes"
mkdir -p "$TABS_ROOT/02-music"
mkdir -p "$TABS_ROOT/03-effects"

SXHKDRC="$HOME/.config/sxhkd/sxhkdrc"
INCLUDE_LINE="include ~/.config/sxhkd/soundboard.conf"
if [[ -f "$SXHKDRC" ]]; then
    if ! grep -Fq "$INCLUDE_LINE" "$SXHKDRC"; then
        printf '\n%s\n' "$INCLUDE_LINE" >> "$SXHKDRC"
        echo "✓ Added soundboard include to existing sxhkdrc"
    fi
else
    printf '%s\n' "$INCLUDE_LINE" > "$SXHKDRC"
    echo "✓ Created sxhkdrc with soundboard include"
fi

if command -v sxhkd &> /dev/null; then
    if pgrep -x sxhkd > /dev/null; then
        pkill -USR1 sxhkd || true
    else
        echo "⚠️  sxhkd is installed but not running. Start it with: sxhkd &"
    fi
else
    echo "⚠️  sxhkd not installed (X11 only). On KDE Wayland, bind sb-play/sb-tab/sb-stop in System Settings → Shortcuts."
fi

if config_exists; then
    LATENCY_MS="$(read_audio_latency_ms)"
fi

echo ""
echo "🔧 Setting up PipeWire virtual microphone and soundboard..."

if [[ "$NONINTERACTIVE" -eq 0 && "$SKIP_MIC" -eq 0 ]]; then
    saved_mic=""
    if saved_mic="$(read_audio_mic_source 2>/dev/null)"; then
        if [[ -n "$saved_mic" ]]; then
            echo "Saved microphone: $saved_mic"
            read -r -p "Use saved mic? [Y/n/change]: " mic_choice
            case "${mic_choice:-Y}" in
                Y|y|"")
                    SELECTED_MIC="$saved_mic"
                    USE_SAVED_MIC=1
                    ;;
                c|C|change|Change)
                    USE_SAVED_MIC=0
                    ;;
                n|N)
                    SELECTED_MIC=""
                    USE_SAVED_MIC=1
                    ;;
                *)
                    echo "Invalid choice. Using saved mic."
                    SELECTED_MIC="$saved_mic"
                    USE_SAVED_MIC=1
                    ;;
            esac
        else
            read -r -p "Saved config has no mic. Set up mic loopback? [y/N]: " mic_choice
            case "${mic_choice:-N}" in
                y|Y)
                    USE_SAVED_MIC=0
                    ;;
                *)
                    SELECTED_MIC=""
                    USE_SAVED_MIC=1
                    ;;
            esac
        fi
    fi
fi

if [[ "$SKIP_MIC" -eq 1 ]]; then
    SELECTED_MIC=""
elif [[ -z "$SELECTED_MIC" && "$USE_SAVED_MIC" != "1" ]]; then
    mapfile -t INPUT_SOURCES < <(list_input_sources)

    if [[ ${#INPUT_SOURCES[@]} -eq 0 ]]; then
        echo "⚠️  No hardware microphones detected. Skipping mic loopback."
    else
        echo "Found the following input sources:"
        echo ""
        for i in "${!INPUT_SOURCES[@]}"; do
            echo "  $((i + 1)): ${INPUT_SOURCES[$i]}"
        done
        echo "  $((${#INPUT_SOURCES[@]} + 1)): Skip"
        echo ""

        PS3="Select your microphone by number: "
        select _ in "${INPUT_SOURCES[@]}" "Skip"; do
            if [[ -n "${REPLY:-}" && "$REPLY" =~ ^[0-9]+$ ]]; then
                if [[ "$REPLY" -eq $((${#INPUT_SOURCES[@]} + 1)) ]]; then
                    echo "Skipping microphone setup."
                    break
                elif [[ "$REPLY" -ge 1 && "$REPLY" -le ${#INPUT_SOURCES[@]} ]]; then
                    SELECTED_MIC="${INPUT_SOURCES[$((REPLY - 1))]}"
                    echo "Selected: $SELECTED_MIC"
                    break
                fi
            fi
            echo "Invalid selection. Please try again."
        done
    fi
fi

write_audio_config "$SELECTED_MIC" "$LATENCY_MS"
echo "✓ Wrote $SOUNDBOARD_CONFIG_FILE"

setup_soundboard_pipewire "$SELECTED_MIC" "$LATENCY_MS"

if command -v systemctl &> /dev/null; then
    systemctl --user daemon-reload
    systemctl --user enable --now "$SYSTEMD_UNIT"
    echo "✓ Enabled systemd user service: $SYSTEMD_UNIT"
    echo "  Check status: systemctl --user status $SYSTEMD_UNIT"
fi

echo ""
echo "✅ PipeWire setup complete!"
echo "Your virtual microphone captures:"
echo "  - Sounds played through Sound Spring Effects"
if [[ -n "$SELECTED_MIC" ]]; then
    echo "  - Your microphone: $SELECTED_MIC"
else
    echo "  - No hardware mic looped in (Sound Spring audio only)"
fi
echo ""
echo "In Discord, Zoom, or OBS:"
echo "  Microphone → Sound-Spring-Virtual-Microphone"
echo "  (Sound-Spring-Effects and Sound-Spring-Mix under Speakers are internal — do not select them as mic)"

echo ""
echo "✅ Sound Spring installation complete!"
echo ""
echo "1. Add audio files to tab directories, e.g.:"
echo "   $TABS_ROOT/01-memes/"
echo "   $TABS_ROOT/02-music/"
echo "   $TABS_ROOT/03-effects/"
echo ""
echo "   Or add custom folders in config.toml ([[tabs]] path = \"...\")."
echo ""
echo "2. On KDE Wayland, create custom shortcuts for:"
echo "   sb-play 1 … sb-play 0, sb-tab next, sb-tab prev, sb-stop"
echo ""
echo "3. Re-run ./install.sh anytime to reset PipeWire routing."
echo "   Non-interactive: ./install.sh --mic <source> or ./install.sh --skip-mic"
