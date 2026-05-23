#!/usr/bin/env bash
# Play one audio file to a PipeWire/Pulse sink. Invoked by sb-play.
set -euo pipefail

SINK="${1:-}"
FILE="${2:-}"

if [[ -z "$SINK" || -z "$FILE" || ! -f "$FILE" ]]; then
    exit 1
fi

ext="${FILE##*.}"
ext="${ext,,}"

case "$ext" in
    mp3|m4a|aac)
        if command -v ffmpeg &>/dev/null; then
            ffmpeg -nostdin -loglevel quiet -i "$FILE" -f wav - | paplay --device="$SINK"
            exit 0
        fi
        ;;
esac

paplay --device="$SINK" "$FILE"
