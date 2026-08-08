#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BIN="$ROOT/target/debug/sound-spring"

echo "== cargo build =="
cargo build --quiet

if [[ ! -x "$BIN" ]]; then
  echo "FAIL: missing binary $BIN"
  exit 1
fi

echo "== QML binding pattern =="
if command -v qml6 >/dev/null; then
  QT_QPA_PLATFORM=offscreen qml6 "$ROOT/ci/test-chrome-bindings.qml"
else
  echo "skip: qml6 not found"
fi

echo "== offscreen app launch =="
QT_QPA_PLATFORM=offscreen timeout 4 "$BIN" &
APP_PID=$!
sleep 2
if ! kill -0 "$APP_PID" 2>/dev/null; then
  echo "FAIL: app exited early"
  exit 1
fi
kill "$APP_PID" 2>/dev/null || true
wait "$APP_PID" 2>/dev/null || true

echo "PASS: chrome binding checks"
