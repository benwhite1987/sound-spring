#!/usr/bin/env bash
# Build a portable AppImage (bundles Qt; host still needs PipeWire, paplay, ffmpeg).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

: "${VERSION:?Set VERSION (e.g. 0.1.0)}"
: "${QMAKE:=/usr/bin/qmake6}"

APPDIR="$ROOT/AppDir"
DIST="$ROOT/dist"
mkdir -p "$DIST"

rm -rf "$APPDIR"
mkdir -p "$APPDIR"

# linuxdeploy ships an old strip that cannot handle .relr.dyn on modern distros.
export NO_STRIP=1
export APPIMAGE_EXTRACT_AND_RUN=1
export LINUXDEPLOY_OUTPUT_VERSION="$VERSION"
export ARCH=x86_64

echo "== install into AppDir =="
DESTDIR="$APPDIR" PREFIX=/usr make install QMAKE="$QMAKE"

TOOLS="$ROOT/.cache/appimage-tools"
mkdir -p "$TOOLS"

LINUXDEPLOY="$TOOLS/linuxdeploy-x86_64.AppImage"
PLUGIN_QT="$TOOLS/linuxdeploy-plugin-qt-x86_64.AppImage"

if [[ ! -x "$LINUXDEPLOY" ]]; then
  curl -fsSL -o "$LINUXDEPLOY" \
    https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
  chmod +x "$LINUXDEPLOY"
fi

if [[ ! -x "$PLUGIN_QT" ]]; then
  curl -fsSL -o "$PLUGIN_QT" \
    https://github.com/linuxdeploy/linuxdeploy-plugin-qt/releases/download/continuous/linuxdeploy-plugin-qt-x86_64.AppImage
  chmod +x "$PLUGIN_QT"
fi

export QMAKE
export LINUXDEPLOY_PLUGIN_QT="$PLUGIN_QT"

ICON="$(find "$APPDIR/usr/share/icons" -name 'io.github.benwhite1987.SoundSpring.png' | head -1)"
if [[ -z "$ICON" ]]; then
  echo "application icon not found under $APPDIR/usr/share/icons" >&2
  exit 1
fi

OUTPUT="$DIST/sound-spring-${VERSION}-x86_64.AppImage"

echo "== linuxdeploy (binary, desktop, icon) =="
"$LINUXDEPLOY" --appdir "$APPDIR" \
  --executable "$APPDIR/usr/bin/sound-spring" \
  --desktop-file "$APPDIR/usr/share/applications/sound-spring.desktop" \
  --icon-file "$ICON"

echo "== linuxdeploy-plugin-qt =="
# KDE kimageformats plugins (Arch, etc.) often pull optional deps we do not need.
"$PLUGIN_QT" --appdir "$APPDIR" --exclude-library 'kimg_*'

# appimagetool rejects non-X-prefixed extension keys (DesktopNames is KDE-specific).
DESKTOP_SRC="$APPDIR/usr/share/applications/sound-spring.desktop"
if [[ -f "$DESKTOP_SRC" ]]; then
  sed -i '/^DesktopNames=/d' "$DESKTOP_SRC"
fi
rm -f "$APPDIR/sound-spring.desktop"

echo "== linuxdeploy (AppImage output) =="
cd "$ROOT"
"$LINUXDEPLOY" --appdir "$APPDIR" --output appimage

BUILT=""
for candidate in \
  "$ROOT/sound-spring-${VERSION}-x86_64.AppImage" \
  "$ROOT/Sound_Spring-${VERSION}-x86_64.AppImage"; do
  if [[ -f "$candidate" ]]; then
    BUILT="$candidate"
    break
  fi
done

if [[ -z "$BUILT" ]]; then
  BUILT="$(find "$ROOT" -maxdepth 1 -name "*-${VERSION}-x86_64.AppImage" -type f | head -1)"
fi

# Some appimagetool builds write to $HOME instead of the project directory.
if [[ -z "$BUILT" && -f "$HOME/Sound_Spring-${VERSION}-x86_64.AppImage" ]]; then
  BUILT="$HOME/Sound_Spring-${VERSION}-x86_64.AppImage"
fi
if [[ -z "$BUILT" && -f "$HOME/sound-spring-${VERSION}-x86_64.AppImage" ]]; then
  BUILT="$HOME/sound-spring-${VERSION}-x86_64.AppImage"
fi

if [[ -z "$BUILT" || ! -f "$BUILT" ]]; then
  echo "AppImage output not found under $ROOT" >&2
  exit 1
fi

mv -f "$BUILT" "$OUTPUT"
echo "Wrote $OUTPUT"
ls -lh "$OUTPUT"
