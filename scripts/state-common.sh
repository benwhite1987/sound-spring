#!/usr/bin/env bash
# Shared state helpers for Sound Spring state.json

SOUNDBOARD_LIB="${SOUNDBOARD_LIB:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
# shellcheck source=config-common.sh
source "$SOUNDBOARD_LIB/config-common.sh"

_init_state_paths() {
    SOUNDBOARD_CACHE_DIR="$(read_state_dir)"
    SOUNDBOARD_STATE_FILE="$SOUNDBOARD_CACHE_DIR/state.json"
    LEGACY_CURRENT_TAB_FILE="$SOUNDBOARD_CACHE_DIR/current_tab"
}

_state_json_value() {
    local key="$1"
    local file="$2"
    [[ -f "$file" ]] || return 1
    local line
    line="$(grep -E "\"${key}\"" "$file" | tail -n 1 || true)"
    [[ -n "$line" ]] || return 1
    if [[ "$line" =~ \"${key}\"[[:space:]]*:[[:space:]]*\"([^\"]*)\" ]]; then
        printf '%s' "${BASH_REMATCH[1]}"
        return 0
    fi
    return 1
}

_json_escape() {
    local value="$1"
    value="${value//\\/\\\\}"
    value="${value//\"/\\\"}"
    printf '%s' "$value"
}

_migrate_legacy_current_tab() {
    _init_state_paths
    [[ -f "$LEGACY_CURRENT_TAB_FILE" ]] || return 1
    local tab=""
    tab="$(tr -d '\n' < "$LEGACY_CURRENT_TAB_FILE")"
    [[ -n "$tab" ]] || return 1
    write_current_tab "$tab"
    rm -f "$LEGACY_CURRENT_TAB_FILE"
}

read_current_tab() {
    _init_state_paths
    mkdir -p "$SOUNDBOARD_CACHE_DIR"
    local tab=""

    if tab="$(_state_json_value current_tab "$SOUNDBOARD_STATE_FILE" 2>/dev/null)"; then
        printf '%s' "$tab"
        return 0
    fi

    if [[ -f "$LEGACY_CURRENT_TAB_FILE" ]]; then
        _migrate_legacy_current_tab
        if tab="$(_state_json_value current_tab "$SOUNDBOARD_STATE_FILE" 2>/dev/null)"; then
            printf '%s' "$tab"
            return 0
        fi
    fi

    return 1
}

write_current_tab() {
    local tab="$1"
    _init_state_paths
    mkdir -p "$SOUNDBOARD_CACHE_DIR"
    printf '{\n  "current_tab": "%s"\n}\n' "$(_json_escape "$tab")" > "$SOUNDBOARD_STATE_FILE"
}

ensure_current_tab_path() {
    # shellcheck source=tabs-common.sh
    source "$SOUNDBOARD_LIB/tabs-common.sh"

    local -a tabs=()
    local current="" resolved=""

    mapfile -t tabs < <(list_tab_paths)
    [[ ${#tabs[@]} -gt 0 ]] || return 1

    if current="$(read_current_tab 2>/dev/null)" && [[ -n "$current" ]]; then
        if resolved="$(resolve_tab_reference "$current" "${tabs[@]}")"; then
            write_current_tab "$resolved"
            printf '%s' "$resolved"
            return 0
        fi
    fi

    write_current_tab "${tabs[0]}"
    printf '%s' "${tabs[0]}"
}
