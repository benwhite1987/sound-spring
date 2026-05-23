#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== cargo test =="
/usr/bin/cargo test --quiet

echo "== QML binding pattern =="
if command -v qml6 >/dev/null; then
  QT_QPA_PLATFORM=offscreen qml6 "$ROOT/scripts/test-chrome-bindings.qml"
else
  echo "skip: qml6 not found"
fi

echo "== offscreen app launch =="
QT_QPA_PLATFORM=offscreen timeout 4 "$ROOT/target/debug/sound-spring" &
APP_PID=$!
sleep 2
if ! kill -0 "$APP_PID" 2>/dev/null; then
  echo "FAIL: app exited early"
  exit 1
fi
kill "$APP_PID" 2>/dev/null || true
wait "$APP_PID" 2>/dev/null || true

echo "PASS: chrome binding checks"
