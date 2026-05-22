#!/usr/bin/env bash
# Shared config helpers for ~/.config/soundboard/config.toml

SOUNDBOARD_CONFIG_DIR="${SOUNDBOARD_CONFIG_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/soundboard}"
SOUNDBOARD_CONFIG_FILE="${SOUNDBOARD_CONFIG_FILE:-$SOUNDBOARD_CONFIG_DIR/config.toml}"
DEFAULT_LATENCY_MS="${DEFAULT_LATENCY_MS:-20}"
DEFAULT_TABS_ROOT="${SOUNDBOARD_CONFIG_DIR}/tabs"
DEFAULT_STATE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/soundboard"

_config_toml_value() {
    local key="$1"
    local file="$2"
    [[ -f "$file" ]] || return 1
    local line
    line="$(grep -E "^[[:space:]]*${key}[[:space:]]*=" "$file" | tail -n 1 || true)"
    [[ -n "$line" ]] || return 1
    if [[ "$line" =~ =[[:space:]]*\"(.*)\"[[:space:]]*$ ]]; then
        printf '%s' "${BASH_REMATCH[1]}"
        return 0
    fi
    if [[ "$line" =~ =[[:space:]]*([0-9]+)[[:space:]]*$ ]]; then
        printf '%s' "${BASH_REMATCH[1]}"
        return 0
    fi
    return 1
}

_config_toml_section_value() {
    local section="$1"
    local key="$2"
    local file="$3"
    [[ -f "$file" ]] || return 1
    awk -v section="$section" -v key="$key" '
        $0 ~ ("^\\[" section "\\]$") { in_section = 1; next }
        /^\[/ { in_section = 0 }
        in_section && $0 ~ ("^[[:space:]]*" key "[[:space:]]*=") {
            if (match($0, /"([^"]*)"/, m)) { print m[1]; exit }
        }
    ' "$file"
}

read_tabs_root() {
    local value=""
    if value="$(_config_toml_section_value paths tabs_root "$SOUNDBOARD_CONFIG_FILE" 2>/dev/null)"; then
        printf '%s' "$value"
        return 0
    fi
    printf '%s' "$DEFAULT_TABS_ROOT"
}

read_state_dir() {
    local value=""
    if value="$(_config_toml_section_value paths state_dir "$SOUNDBOARD_CONFIG_FILE" 2>/dev/null)"; then
        [[ -n "$value" ]] && printf '%s' "$value" && return 0
    fi
    printf '%s' "$DEFAULT_STATE_DIR"
}

read_audio_mic_source() {
    local value=""
    if value="$(_config_toml_value mic_source "$SOUNDBOARD_CONFIG_FILE" 2>/dev/null)"; then
        printf '%s' "$value"
        return 0
    fi
    return 1
}

read_audio_latency_ms() {
    local value=""
    if value="$(_config_toml_value latency_ms "$SOUNDBOARD_CONFIG_FILE" 2>/dev/null)"; then
        printf '%s' "$value"
        return 0
    fi
    printf '%s' "$DEFAULT_LATENCY_MS"
}

_preserve_paths_and_tabs_block() {
    [[ -f "$SOUNDBOARD_CONFIG_FILE" ]] || return 0
    awk '
        /^\[\[tabs\]\]/ { in_tabs = 1 }
        /^\[/ && $0 !~ /^\[\[tabs\]\]/ { if (in_tabs) in_tabs = 0 }
        in_tabs { print; next }
        /^\[paths\]/ { in_paths = 1; print; next }
        in_paths && /^\[/ { in_paths = 0 }
        in_paths { print }
    ' "$SOUNDBOARD_CONFIG_FILE"
}

_default_paths_block() {
    local tabs_root="${1:-$DEFAULT_TABS_ROOT}"
    cat <<EOF
[paths]
tabs_root = "$tabs_root"
state_dir = "$DEFAULT_STATE_DIR"
EOF
}

write_audio_config() {
    local mic_source="${1:-}"
    local latency_ms="${2:-$DEFAULT_LATENCY_MS}"
    local paths_block=""

    if paths_block="$(_preserve_paths_and_tabs_block)"; then
        :
    fi
    if [[ -z "$paths_block" ]]; then
        paths_block="$(_default_paths_block "$DEFAULT_TABS_ROOT")"
    fi

    mkdir -p "$SOUNDBOARD_CONFIG_DIR"
    cat > "$SOUNDBOARD_CONFIG_FILE" <<EOF
[audio]
mic_source = "$mic_source"
latency_ms = $latency_ms
auto_teardown = true

[shortcuts]
mode = "portal"

[ui]
minimize_to_tray = true
launch_at_login = false

$paths_block
EOF
}

config_exists() {
    [[ -f "$SOUNDBOARD_CONFIG_FILE" ]]
}

list_external_tab_paths() {
    [[ -f "$SOUNDBOARD_CONFIG_FILE" ]] || return 0
    local tabs_root path
    tabs_root="$(read_tabs_root)"
    while IFS= read -r path; do
        [[ -z "$path" ]] && continue
        case "$path" in
            "$tabs_root"/*|"$tabs_root") continue ;;
        esac
        printf '%s\n' "$path"
    done < <(grep -E '^path = "' "$SOUNDBOARD_CONFIG_FILE" 2>/dev/null | sed 's/^path = "\(.*\)"/\1/' || true)
}
