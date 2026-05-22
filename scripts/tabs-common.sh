#!/usr/bin/env bash
# Tab discovery and resolution from config.toml paths.

SOUNDBOARD_LIB="${SOUNDBOARD_LIB:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
# shellcheck source=config-common.sh
source "$SOUNDBOARD_LIB/config-common.sh"

_canonical_path() {
    local path="$1"
    if command -v realpath &>/dev/null; then
        realpath "$path" 2>/dev/null && return 0
    fi
    readlink -f "$path" 2>/dev/null && return 0
    printf '%s' "$path"
}

list_explicit_tab_paths() {
    [[ -f "$SOUNDBOARD_CONFIG_FILE" ]] || return 0
    local path
    while IFS= read -r path; do
        [[ -z "$path" ]] && continue
        [[ -d "$path" ]] || continue
        _canonical_path "$path"
    done < <(grep -E '^path = "' "$SOUNDBOARD_CONFIG_FILE" 2>/dev/null | sed 's/^path = "\(.*\)"/\1/' || true)
}

list_scanned_tab_paths() {
    local tabs_root
    tabs_root="$(read_tabs_root)"
    [[ -d "$tabs_root" ]] || return 0
    local name
    while IFS= read -r name; do
        [[ -n "$name" ]] || continue
        _canonical_path "$tabs_root/$name"
    done < <(find "$tabs_root" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' 2>/dev/null | sort)
}

list_tab_paths() {
    local -a explicit=()
    mapfile -t explicit < <(list_explicit_tab_paths | sort -u)
    if [[ ${#explicit[@]} -gt 0 ]]; then
        printf '%s\n' "${explicit[@]}"
        return 0
    fi
    list_scanned_tab_paths
}

tab_display_name() {
    basename "$1"
}

resolve_tab_reference() {
    local ref="$1"
    shift
    local -a tabs=("$@")
    local tab root candidate

    if [[ "$ref" == /* ]]; then
        candidate="$(_canonical_path "$ref")"
        for tab in "${tabs[@]}"; do
            [[ "$tab" == "$candidate" ]] && { printf '%s' "$tab"; return 0; }
        done
    fi

    for tab in "${tabs[@]}"; do
        [[ "$(basename "$tab")" == "$ref" ]] && { printf '%s' "$tab"; return 0; }
    done

    root="$(read_tabs_root)"
    if [[ -d "$root/$ref" ]]; then
        candidate="$(_canonical_path "$root/$ref")"
        for tab in "${tabs[@]}"; do
            [[ "$tab" == "$candidate" ]] && { printf '%s' "$tab"; return 0; }
        done
    fi

    return 1
}

load_tab_paths() {
    mapfile -t SOUND_SPRING_TABS < <(list_tab_paths)
}
