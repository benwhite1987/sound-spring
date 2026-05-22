#!/usr/bin/env bash
# Shared PipeWire helpers for install/uninstall.

SOUNDBOARD_SFX_SINK="soundboard_sfx"
SOUNDBOARD_VIRTMIC_SINK="soundboard_virtmic"
SOUNDBOARD_VIRTUAL_MIC_SOURCE="sound_spring_virtual_mic"
DEFAULT_LATENCY_MS=20

# pactl property values cannot contain spaces; hyphens read as "Sound Spring" in UI.
SOUND_SPRING_DISPLAY_MIC="Sound-Spring-Virtual-Microphone"
SOUND_SPRING_DISPLAY_EFFECTS="Sound-Spring-Effects"
SOUND_SPRING_DISPLAY_MIX="Sound-Spring-Mix"

# shellcheck source=audio-common.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/audio-common.sh"

soundboard_module_ids() {
    pactl list short modules | while read -r id _ rest; do
        case "$rest" in
            *"sink_name=${SOUNDBOARD_SFX_SINK}"*|*"sink_name=${SOUNDBOARD_VIRTMIC_SINK}"*)
                echo "$id"
                ;;
            *"source_name=${SOUNDBOARD_VIRTUAL_MIC_SOURCE}"*)
                echo "$id"
                ;;
            *module-loopback*)
                if [[ "$rest" == *"$SOUNDBOARD_SFX_SINK"* ]] || [[ "$rest" == *"$SOUNDBOARD_VIRTMIC_SINK"* ]]; then
                    echo "$id"
                fi
                ;;
            *module-remap-source*)
                if [[ "$rest" == *"$SOUNDBOARD_VIRTMIC_SINK"* ]] || [[ "$rest" == *"$SOUNDBOARD_VIRTUAL_MIC_SOURCE"* ]]; then
                    echo "$id"
                fi
                ;;
            # Legacy names from older installs / PROJECT.md sketch
            *"sink_name=virtmic"*|*"sink_name=soundboard"*)
                if [[ "$rest" != *"$SOUNDBOARD_SFX_SINK"* ]] && [[ "$rest" != *"$SOUNDBOARD_VIRTMIC_SINK"* ]]; then
                    echo "$id"
                fi
                ;;
        esac
    done
}

unload_soundboard_modules() {
    mapfile -t module_ids < <(soundboard_module_ids | sort -rn | uniq)
    for module_id in "${module_ids[@]}"; do
        [[ -n "$module_id" ]] && pactl unload-module "$module_id" 2>/dev/null || true
    done
}

sink_exists() {
    pactl list short sinks | awk '{print $2}' | grep -qx "$1"
}

source_exists() {
    pactl list short sources | awk '{print $2}' | grep -qx "$1"
}

setup_soundboard_pipewire() {
    local mic_source="${1:-}"
    local latency_ms="${2:-$DEFAULT_LATENCY_MS}"

    unload_soundboard_modules

    pactl load-module module-null-sink \
        sink_name="$SOUNDBOARD_VIRTMIC_SINK" \
        sink_properties="device.description=${SOUND_SPRING_DISPLAY_MIX}"

    pactl load-module module-null-sink \
        sink_name="$SOUNDBOARD_SFX_SINK" \
        sink_properties="device.description=${SOUND_SPRING_DISPLAY_EFFECTS}"

    sleep 0.3

    if ! sink_exists "$SOUNDBOARD_SFX_SINK"; then
        echo "❌ ${SOUNDBOARD_SFX_SINK} sink not found. Aborting." >&2
        return 1
    fi
    if ! sink_exists "$SOUNDBOARD_VIRTMIC_SINK"; then
        echo "❌ ${SOUNDBOARD_VIRTMIC_SINK} sink not found. Aborting." >&2
        return 1
    fi
    if ! source_exists "${SOUNDBOARD_SFX_SINK}.monitor"; then
        echo "❌ ${SOUNDBOARD_SFX_SINK}.monitor source not found. Aborting." >&2
        return 1
    fi

    pactl load-module module-loopback \
        source="${SOUNDBOARD_SFX_SINK}.monitor" \
        sink="$SOUNDBOARD_VIRTMIC_SINK" \
        latency_msec="$latency_ms"

    if [[ -n "$mic_source" ]]; then
        pactl load-module module-loopback \
            source="$mic_source" \
            sink="$SOUNDBOARD_VIRTMIC_SINK" \
            latency_msec="$latency_ms"
    fi

    pactl load-module module-remap-source \
        master="${SOUNDBOARD_VIRTMIC_SINK}.monitor" \
        source_name="$SOUNDBOARD_VIRTUAL_MIC_SOURCE" \
        source_properties="device.description=${SOUND_SPRING_DISPLAY_MIC}"

    if ! source_exists "$SOUNDBOARD_VIRTUAL_MIC_SOURCE"; then
        echo "❌ ${SOUNDBOARD_VIRTUAL_MIC_SOURCE} source not found. Aborting." >&2
        return 1
    fi
}

list_input_sources() {
    pactl list sources short | awk '
        $2 !~ /\.monitor$/ &&
        $2 !~ /^soundboard_/ &&
        $2 !~ /^soundboard\./ &&
        $2 !~ /^sound_spring_/ {
            print $2
        }'
}
