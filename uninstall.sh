#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOUNDBOARD_SHARE="${XDG_DATA_HOME:-$HOME/.local/share}/soundspring"
LOCAL_BIN="${HOME}/.local/bin"
SXHKD_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/sxhkd"
SYSTEMD_UNIT="soundboard-pipewire.service"
SYSTEMD_USER_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"

SOUNDBOARD_LIB="$SCRIPT_DIR/scripts"
# shellcheck source=scripts/pipewire-common.sh
source "$SCRIPT_DIR/scripts/pipewire-common.sh"
# shellcheck source=scripts/config-common.sh
source "$SCRIPT_DIR/scripts/config-common.sh"
# shellcheck source=scripts/tabs-common.sh
source "$SCRIPT_DIR/scripts/tabs-common.sh"

CONFIG_DIR="$SOUNDBOARD_CONFIG_DIR"
TABS_ROOT="$(read_tabs_root 2>/dev/null || printf '%s' "$DEFAULT_TABS_ROOT")"
STATE_DIR="$(read_state_dir 2>/dev/null || printf '%s' "$DEFAULT_STATE_DIR")"

echo "⚠️  Sound Spring uninstall"
echo "This will remove:"
echo "  - sb-play, sb-tab, sb-stop from $LOCAL_BIN"
echo "  - shared scripts in $SOUNDBOARD_SHARE"
echo "  - systemd user service ($SYSTEMD_UNIT)"
echo "  - sxhkd bindings fragment"
echo "  - runtime state in $STATE_DIR"
echo "  - PipeWire virtual mic routing"
echo ""
echo "Tab audio folders are kept by default."
echo "  Default tabs root: $TABS_ROOT"
mapfile -t EXTERNAL_TABS < <(list_external_tab_paths 2>/dev/null || true)
if [[ ${#EXTERNAL_TABS[@]} -gt 0 ]]; then
    echo "  Custom tab folders (never deleted automatically):"
    for path in "${EXTERNAL_TABS[@]}"; do
        echo "    - $path"
    done
fi
echo ""

read -r -p "Continue? (y/N): " confirm
[[ "$confirm" == "y" || "$confirm" == "Y" ]] || { echo "Cancelled."; exit 0; }

echo ""
echo "🧹 Removing Sound Spring components..."

stop_sound_spring_playback
echo "✓ Stopped Sound Spring playback"

if command -v systemctl &> /dev/null; then
    systemctl --user disable --now "$SYSTEMD_UNIT" 2>/dev/null || true
    rm -f "$SYSTEMD_USER_DIR/$SYSTEMD_UNIT"
    systemctl --user daemon-reload 2>/dev/null || true
    systemctl --user reset-failed "$SYSTEMD_UNIT" 2>/dev/null || true
    echo "✓ Removed systemd user service: $SYSTEMD_UNIT"
fi

rm -f "$LOCAL_BIN/sb-play" "$LOCAL_BIN/sb-tab" "$LOCAL_BIN/sb-stop"
echo "✓ Removed sb-play, sb-tab, sb-stop"

rm -rf "$SOUNDBOARD_SHARE"
echo "✓ Removed shared scripts ($SOUNDBOARD_SHARE)"

rm -f "$SXHKD_DIR/soundboard.conf"
SXHKDRC="$SXHKD_DIR/sxhkdrc"
if [[ -f "$SXHKDRC" ]]; then
    grep -Fv 'include ~/.config/sxhkd/soundboard.conf' "$SXHKDRC" > "${SXHKDRC}.tmp" || true
    mv "${SXHKDRC}.tmp" "$SXHKDRC"
    echo "✓ Removed Sound Spring include from sxhkdrc"
fi

rm -rf "$STATE_DIR"
echo "✓ Removed runtime state ($STATE_DIR)"

read -r -p "Also remove config and default tab folders ($CONFIG_DIR)? (y/N): " remove_config
if [[ "$remove_config" == "y" || "$remove_config" == "Y" ]]; then
    rm -rf "$CONFIG_DIR"
    echo "✓ Removed $CONFIG_DIR"
else
    echo "✓ Kept config and tab folders"
    if [[ ${#EXTERNAL_TABS[@]} -gt 0 ]]; then
        echo "  Custom tab folders outside $CONFIG_DIR were not touched."
    fi
fi

echo ""
echo "🔧 Unloading PipeWire modules..."
unload_soundboard_modules

REMAINING=$(soundboard_module_ids | wc -l)
if [[ "$REMAINING" -eq 0 ]]; then
    echo "✓ All Sound Spring PipeWire modules removed."
else
    echo "⚠️  $REMAINING module(s) may remain. Retrying unload..."
    unload_soundboard_modules
    REMAINING=$(soundboard_module_ids | wc -l)
    if [[ "$REMAINING" -eq 0 ]]; then
        echo "✓ All Sound Spring PipeWire modules removed."
    else
        echo "⚠️  Some modules may remain. Try:"
        echo "   systemctl --user restart pipewire pipewire-pulse"
    fi
fi

if pgrep -x sxhkd > /dev/null; then
    pkill -USR1 sxhkd || true
fi

echo ""
echo "✅ Sound Spring uninstalled."
echo "Restart Discord/Zoom/OBS if they still show Sound Spring audio devices."
