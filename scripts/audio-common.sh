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
