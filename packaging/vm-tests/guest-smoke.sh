#!/usr/bin/env bash
# Runs inside the guest. Smoke-tests an installed binary or AppImage path.
# Usage: guest-smoke.sh <label> <binary-or-appimage> [--skip-ldd] [--appimage]
set -euo pipefail

LABEL="${1:?label}"
BIN="${2:?binary path}"
shift 2

SKIP_LDD=0
IS_APPIMAGE=0
for arg in "$@"; do
  case "$arg" in
    --skip-ldd) SKIP_LDD=1 ;;
    --appimage) IS_APPIMAGE=1; SKIP_LDD=1 ;;
    *)
      echo "unknown flag: $arg" >&2
      exit 2
      ;;
  esac
done

RESULT_JSON="${SS_VM_RESULT_JSON:-/tmp/ss-vm-result-${LABEL}.json}"
SMOKE_LOG="${SS_VM_SMOKE_LOG:-/tmp/ss-smoke-${LABEL}.log}"

pass() { printf 'PASS: %s\n' "$1"; }
fail() { printf 'FAIL: %s\n' "$1"; FAILED=1; }
warn() { printf 'WARN: %s\n' "$1"; }

FAILED=0
CHECK_BINARY=0
CHECK_LDD=0
CHECK_AUDIO_CLIS=0
CHECK_PIPEWIRE=0
CHECK_LAUNCH=0

json_bool() {
  if [[ "$1" -eq 1 ]]; then echo true; else echo false; fi
}

# --- binary ---
if [[ -x "$BIN" ]]; then
  pass "binary executable: $BIN"
  CHECK_BINARY=1
else
  fail "binary missing or not executable: $BIN"
fi

# --- ldd (native packages only) ---
if [[ "$SKIP_LDD" -eq 0 && "$CHECK_BINARY" -eq 1 ]]; then
  if ldd "$BIN" 2>/dev/null | grep -q 'not found'; then
    fail "ldd reports missing libraries"
    ldd "$BIN" || true
  else
    pass "ldd clean"
    CHECK_LDD=1
  fi
else
  CHECK_LDD=1
  pass "ldd skipped"
fi

# --- audio CLIs ---
AUDIO_OK=1
for cmd in pactl paplay pw-cat ffmpeg; do
  if command -v "$cmd" >/dev/null 2>&1; then
    pass "found $cmd"
  else
    fail "missing $cmd"
    AUDIO_OK=0
  fi
done
CHECK_AUDIO_CLIS=$AUDIO_OK

# --- PipeWire session (best-effort) ---
if command -v systemctl >/dev/null 2>&1; then
  systemctl --user start pipewire.socket pipewire pipewire-pulse.socket pipewire-pulse wireplumber 2>/dev/null || true
fi
if command -v pactl >/dev/null 2>&1 && pactl info >/dev/null 2>&1; then
  pass "pactl info ok"
  CHECK_PIPEWIRE=1
else
  warn "pactl info failed (PipeWire session may be unavailable in this cloud image)"
  CHECK_PIPEWIRE=0
fi

# --- offscreen launch ---
if [[ "$CHECK_BINARY" -eq 1 ]]; then
  rm -f "$SMOKE_LOG"
  export RUST_LOG=sound_spring=info
  export QT_QPA_PLATFORM=offscreen
  if [[ "$IS_APPIMAGE" -eq 1 ]]; then
    export APPIMAGE_EXTRACT_AND_RUN=1
  fi

  set +e
  timeout 8 "$BIN" >"$SMOKE_LOG" 2>&1 &
  APP_PID=$!
  sleep 2
  ALIVE=0
  if kill -0 "$APP_PID" 2>/dev/null; then
    ALIVE=1
  fi
  wait "$APP_PID" 2>/dev/null
  set -e

  if grep -Eq 'startup: first frame' "$SMOKE_LOG" \
    && ! grep -Eq 'failed to load component|is not installed|plugin .* not found| is not a type|Type .* unavailable' "$SMOKE_LOG"; then
    pass "launch: first frame"
    CHECK_LAUNCH=1
  elif grep -Eq 'startup: QML engine loaded' "$SMOKE_LOG" \
    && ! grep -Eq 'failed to load component|is not installed|plugin .* not found| is not a type|Type .* unavailable' "$SMOKE_LOG"; then
    pass "launch: QML engine loaded"
    CHECK_LAUNCH=1
  elif [[ "$ALIVE" -eq 1 ]] \
    && ! grep -Eq 'failed to load component|is not installed|plugin .* not found| is not a type|Type .* unavailable|GLIBC_[0-9.]+. not found' "$SMOKE_LOG"; then
    pass "launch: process stayed alive >=2s"
    CHECK_LAUNCH=1
  else
    fail "launch: early exit without healthy QML startup"
    if grep -Eq 'GLIBC_[0-9.]+. not found' "$SMOKE_LOG"; then
      fail "glibc too old for this binary — rebuild with: make package-ubuntu"
    fi
    if grep -Eq 'is not installed|failed to load component|plugin .* not found| is not a type|Type .* unavailable' "$SMOKE_LOG"; then
      fail "QML module missing — check packaging Depends"
    fi
    echo "----- smoke log -----"
    cat "$SMOKE_LOG" || true
    echo "---------------------"
  fi
fi

cat >"$RESULT_JSON" <<EOF
{
  "label": "$(printf '%s' "$LABEL" | sed 's/"/\\"/g')",
  "binary": "$(printf '%s' "$BIN" | sed 's/"/\\"/g')",
  "checks": {
    "binary": $(json_bool "$CHECK_BINARY"),
    "ldd": $(json_bool "$CHECK_LDD"),
    "audio_clis": $(json_bool "$CHECK_AUDIO_CLIS"),
    "pipewire": $(json_bool "$CHECK_PIPEWIRE"),
    "launch": $(json_bool "$CHECK_LAUNCH")
  },
  "passed": $(json_bool $((1 - FAILED)))
}
EOF

if [[ "$FAILED" -ne 0 ]]; then
  exit 1
fi
echo "PASS: guest smoke ($LABEL)"
