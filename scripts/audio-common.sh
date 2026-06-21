#!/usr/bin/env bash
# Shared audio format and playback helpers.

# Extensions matched case-insensitively when scanning tab folders.
SOUND_SPRING_AUDIO_EXTENSIONS=(ogg oga opus wav flac mp3 m4a aac)

SOUNDBOARD_LIB="${SOUNDBOARD_LIB:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
PLAY_FILE_SCRIPT="${PLAY_FILE_SCRIPT:-$SOUNDBOARD_LIB/play-file.sh}"

_audio_find_predicate() {
    local predicate="" ext
    for ext in "${SOUND_SPRING_AUDIO_EXTENSIONS[@]}"; do
        if [[ -n "$predicate" ]]; then
            predicate+=" -o "
        fi
        predicate+="-iname *.$ext"
    done
    printf '%s' "$predicate"
}

list_tab_audio_files() {
    local tab_dir="$1"
    find "$tab_dir" -maxdepth 1 -type f \( $(_audio_find_predicate) \) | sort
}

_audio_file_stem() {
    local path="$1"
    local base="${path##*/}"
    printf '%s' "${base%.*}"
}

# Matches Rust parse_slot_prefix_from_stem: "01-name", "01 name", etc.
parse_slot_number_from_file() {
    local path="$1"
    local stem
    stem="$(_audio_file_stem "$path")"
    if [[ "$stem" =~ ^[[:space:]]*([0-9]+)([-[:space:]])(.+)$ ]]; then
        local label="${BASH_REMATCH[3]}"
        label="${label#"${label%%[![:space:]]*}"}"
        label="${label%"${label##*[![:space:]]}"}"
        if [[ -n "$label" ]]; then
            echo "${BASH_REMATCH[1]}"
            return 0
        fi
    fi
    return 1
}

# Resolve the audio file bound to slot 1-10 using the same rules as the GUI scanner.
resolve_tab_slot_file() {
    local tab_dir="$1"
    local want_slot="$2"
    local -A by_slot=()
    local -a overflow_lines=()
    local file num assigned total line slot_idx sort_key

    if [[ ! "$want_slot" =~ ^[0-9]+$ ]] || [[ "$want_slot" -lt 1 || "$want_slot" -gt 10 ]]; then
        return 1
    fi

    while IFS= read -r file; do
        [[ -z "$file" ]] && continue
        if num="$(parse_slot_number_from_file "$file")"; then
            num=$((10#$num))
            if [[ "$num" -ge 1 && "$num" -le 10 ]]; then
                if [[ -n "${by_slot[$num]:-}" ]]; then
                    overflow_lines+=("$(printf '%010d|%s' "$num" "$file")")
                else
                    by_slot[$num]="$file"
                fi
            else
                overflow_lines+=("$(printf '%010d|%s' "$num" "$file")")
            fi
        else
            overflow_lines+=("$(printf '%010d|%s' "9999999999" "$file")")
        fi
    done < <(list_tab_audio_files "$tab_dir")

    if [[ ${#overflow_lines[@]} -gt 0 ]]; then
        local -a sorted_overflow=()
        mapfile -t sorted_overflow < <(printf '%s\n' "${overflow_lines[@]}" | sort)
        overflow_lines=("${sorted_overflow[@]}")
    fi

    assigned=${#by_slot[@]}
    total=$((assigned + ${#overflow_lines[@]}))

    for slot_idx in $(seq 1 10); do
        [[ -n "${by_slot[$slot_idx]:-}" ]] && continue
        [[ ${#overflow_lines[@]} -eq 0 ]] && break
        line="${overflow_lines[0]}"
        overflow_lines=("${overflow_lines[@]:1}")
        by_slot[$slot_idx]="${line#*|}"
    done

    if [[ -n "${by_slot[$want_slot]:-}" ]]; then
        printf '%s' "${by_slot[$want_slot]}"
        return 0
    fi
    return 1
}

stop_sound_spring_playback() {
    pkill -f "$PLAY_FILE_SCRIPT" 2>/dev/null || true
    pkill -f 'paplay --device=soundboard_sfx' 2>/dev/null || true
}

play_audio_file_async() {
    local sink="$1"
    local file="$2"

    if [[ ! -x "$PLAY_FILE_SCRIPT" ]]; then
        return 1
    fi

    "$PLAY_FILE_SCRIPT" "$sink" "$file" &
    disown
}

audio_playback_ready() {
    [[ -x "$PLAY_FILE_SCRIPT" ]] && command -v paplay &>/dev/null
}

mp3_playback_ready() {
    command -v paplay &>/dev/null || command -v ffmpeg &>/dev/null
}
