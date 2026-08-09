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

# QML imports and native Wayland are not inferred reliably from the binary alone.
export QML_SOURCES_PATHS="$ROOT/qml"
export EXTRA_QT_MODULES="QtQuick;QtQuick.Controls;QtQuick.Dialogs;QtQuick.Layouts;waylandcompositor"
# Qt 6.4+ (Ubuntu 24.04) names the Wayland platform plugins with -egl/-generic suffixes;
# older trees used a single libqwayland.so. Prefer whichever exists on this host.
# Always try to ship offscreen for headless smoke tests / CI.
PLATFORM_PLUGINS=()
QT_PLUGINS="$("$QMAKE" -query QT_INSTALL_PLUGINS 2>/dev/null || true)"
find_platform_plugin() {
  local candidate="$1"
  if [[ -n "$QT_PLUGINS" && -f "$QT_PLUGINS/platforms/$candidate" ]]; then
    echo "$candidate"
    return 0
  fi
  for base in /usr/lib/x86_64-linux-gnu/qt6/plugins /usr/lib/qt6/plugins; do
    if [[ -f "$base/platforms/$candidate" ]]; then
      echo "$candidate"
      return 0
    fi
  done
  return 1
}
for candidate in libqwayland.so libqwayland-generic.so libqwayland-egl.so libqoffscreen.so; do
  if plugin=$(find_platform_plugin "$candidate"); then
    PLATFORM_PLUGINS+=("$plugin")
  fi
done
if [[ ${#PLATFORM_PLUGINS[@]} -eq 0 ]]; then
  echo "no Wayland/offscreen Qt platform plugins found" >&2
  exit 1
fi
# linuxdeploy-plugin-qt joins EXTRA_PLATFORM_PLUGINS with ';'
export EXTRA_PLATFORM_PLUGINS
EXTRA_PLATFORM_PLUGINS="$(IFS=';'; echo "${PLATFORM_PLUGINS[*]}")"
echo "Extra platform plugins: $EXTRA_PLATFORM_PLUGINS"

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

for required in \
  "$APPDIR/usr/qml/QtQuick/libqtquick2plugin.so" \
  "$APPDIR/usr/qml/QtQuick/Controls/libqtquickcontrols2plugin.so" \
  "$APPDIR/usr/qml/QtQuick/Controls/Fusion/libqtquickcontrols2fusionstyleplugin.so"; do
  if [[ ! -f "$required" ]]; then
    # Fusion may ship as a directory of QML without a single well-known .so name
    # across Qt versions; accept either the plugin or the Fusion style tree.
    if [[ "$required" == *Fusion* && -d "$APPDIR/usr/qml/QtQuick/Controls/Fusion" ]]; then
      continue
    fi
    echo "AppImage bundle missing required Qt file: $required" >&2
    exit 1
  fi
done
if [[ ! -d "$APPDIR/usr/qml/QtQuick/Controls/Fusion" ]]; then
  echo "AppImage bundle missing QtQuick.Controls.Fusion style" >&2
  exit 1
fi

# Optional plugins that improve icons/dialogs without pulling Plasma Breeze.
copy_qt_plugin() {
  local subdir="$1"
  local name="$2"
  local dest_dir="$APPDIR/usr/plugins/$subdir"
  mkdir -p "$dest_dir"
  if [[ -n "$QT_PLUGINS" && -f "$QT_PLUGINS/$subdir/$name" ]]; then
    cp -n "$QT_PLUGINS/$subdir/$name" "$dest_dir/" 2>/dev/null || true
    return 0
  fi
  for base in /usr/lib/x86_64-linux-gnu/qt6/plugins /usr/lib/qt6/plugins; do
    if [[ -f "$base/$subdir/$name" ]]; then
      cp -n "$base/$subdir/$name" "$dest_dir/" 2>/dev/null || true
      return 0
    fi
  done
  return 1
}
copy_qt_plugin iconengines libqsvgicon.so || true
copy_qt_plugin platformthemes libqxdgdesktopportal.so || true

wayland_bundled=0
offscreen_bundled=0
for candidate in libqwayland.so libqwayland-generic.so libqwayland-egl.so; do
  if [[ -f "$APPDIR/usr/plugins/platforms/$candidate" ]]; then
    wayland_bundled=1
    break
  fi
done
if [[ -f "$APPDIR/usr/plugins/platforms/libqoffscreen.so" ]]; then
  offscreen_bundled=1
fi
if [[ "$wayland_bundled" -ne 1 ]]; then
  echo "AppImage bundle missing Wayland platform plugin (libqwayland*.so)" >&2
  exit 1
fi
if [[ "$offscreen_bundled" -ne 1 ]]; then
  echo "AppImage bundle missing offscreen platform plugin (libqoffscreen.so)" >&2
  exit 1
fi

# appimagetool rejects non-X-prefixed extension keys (DesktopNames is KDE-specific).
DESKTOP_SRC="$APPDIR/usr/share/applications/sound-spring.desktop"
if [[ -f "$DESKTOP_SRC" ]]; then
  sed -i '/^DesktopNames=/d' "$DESKTOP_SRC"
fi
rm -f "$APPDIR/sound-spring.desktop"

# linuxdeploy leaves AppRun as a symlink to the binary. Replace it with a
# wrapper: Ubuntu Qt 6.4 Wayland-EGL paints a black window on Plasma 6/Mesa.
# Prefer XWayland (xcb) unless the user already set QT_QPA_PLATFORM.
# Pin Fusion + bundled plugin/QML paths so host Plasma themes cannot leak in.
rm -f "$APPDIR/AppRun"
cat >"$APPDIR/AppRun" <<'EOF'
#!/bin/bash
HERE="$(dirname "$(readlink -f "$0")")"
export QT_PLUGIN_PATH="${QT_PLUGIN_PATH:-$HERE/usr/plugins}"
export QML2_IMPORT_PATH="${QML2_IMPORT_PATH:-$HERE/usr/qml}"
export QT_QUICK_CONTROLS_STYLE="${QT_QUICK_CONTROLS_STYLE:-Fusion}"
if [[ -z "${QT_QPA_PLATFORM:-}" && "${XDG_SESSION_TYPE:-}" == "wayland" ]]; then
  export QT_QPA_PLATFORM=xcb
fi
exec "$HERE/usr/bin/sound-spring" "$@"
EOF
chmod +x "$APPDIR/AppRun"

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
