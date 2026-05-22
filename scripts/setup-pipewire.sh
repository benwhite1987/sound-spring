#!/usr/bin/env bash
set -euo pipefail

SOUNDBOARD_LIB="${SOUNDBOARD_LIB:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
# shellcheck source=pipewire-common.sh
source "$SOUNDBOARD_LIB/pipewire-common.sh"
# shellcheck source=config-common.sh
source "$SOUNDBOARD_LIB/config-common.sh"

mic_source=""
latency_ms="$DEFAULT_LATENCY_MS"

if mic_source="$(read_audio_mic_source 2>/dev/null)"; then
    :
else
    mic_source=""
fi
latency_ms="$(read_audio_latency_ms)"

setup_soundboard_pipewire "$mic_source" "$latency_ms"
