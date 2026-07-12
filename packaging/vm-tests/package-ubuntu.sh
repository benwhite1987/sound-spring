#!/usr/bin/env bash
# Build .deb / .rpm / AppImage inside Ubuntu 24.04 (matches release.yml glibc).
# Required when the host glibc is newer than Ubuntu/Fedora guests (e.g. CachyOS).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="${VERSION:-$(sed -nE 's/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$ROOT/Cargo.toml" | head -1)}"
IMAGE="${PACKAGE_UBUNTU_IMAGE:-ubuntu:24.04}"
CARGO_CACHE="${CARGO_CACHE:-$HOME/.cargo}"

if ! command -v docker >/dev/null 2>&1; then
  echo "docker not found; install Docker or build on Ubuntu 24.04" >&2
  exit 1
fi

mkdir -p "$ROOT/dist" "$CARGO_CACHE"

echo "[package-ubuntu] building VERSION=$VERSION in $IMAGE"

docker run --rm \
  --network=host \
  -e VERSION="$VERSION" \
  -e DEBIAN_FRONTEND=noninteractive \
  -e CARGO_HOME=/cargo \
  -e RUSTUP_HOME=/cargo/rustup \
  -e RUSTUP_INIT_SKIP_PATH_CHECK=yes \
  -e CARGO_TARGET_DIR=/src/target-ubuntu \
  -e HOST_UID="$(id -u)" \
  -e HOST_GID="$(id -g)" \
  -v "$ROOT:/src:rw" \
  -v "$CARGO_CACHE:/cargo:rw" \
  -w /src \
  "$IMAGE" \
  bash -lc '
set -euo pipefail
apt-get update -qq
apt-get install -y -qq \
  build-essential curl pkg-config file rpm libfuse2 libssl-dev \
  qt6-base-dev qt6-base-dev-tools qt6-declarative-dev qt6-tools-dev \
  qt6-wayland qt6-wayland-dev libqt6svg6-dev libpulse-dev ca-certificates \
  qml6-module-qtquick qml6-module-qtquick-window qml6-module-qtquick-templates \
  qml6-module-qtquick-controls qml6-module-qtquick-dialogs \
  qml6-module-qtquick-layouts qml6-module-qtqml-workerscript
curl -fsSL https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
# shellcheck disable=SC1091
if [[ -f /cargo/env ]]; then
  source /cargo/env
elif [[ -f "$HOME/.cargo/env" ]]; then
  source "$HOME/.cargo/env"
fi
export PATH="/cargo/bin:${HOME}/.cargo/bin:${PATH}"
export CARGO_TARGET_DIR=/src/target-ubuntu
rustup default stable >/dev/null
rustc --version
cargo --version
curl -fsSL -o /tmp/nfpm.tgz \
  https://github.com/goreleaser/nfpm/releases/download/v2.47.0/nfpm_2.47.0_Linux_x86_64.tar.gz
tar -C /usr/local/bin -xzf /tmp/nfpm.tgz nfpm
nfpm --version

QMAKE=/usr/bin/qmake6 cargo build --release
test -x "$CARGO_TARGET_DIR/release/sound-spring"

# Stage the Ubuntu-built binary where make install / AppImage expect it,
# without re-running a host-oriented cargo build.
mkdir -p target/release
cp -a "$CARGO_TARGET_DIR/release/sound-spring" target/release/sound-spring

# Skip the `build` prerequisite so we keep the Ubuntu binary.
cat > /usr/local/bin/make <<'"'"'EOF'"'"'
#!/bin/bash
exec /usr/bin/make -o build "$@"
EOF
chmod +x /usr/local/bin/make

rm -rf staging AppDir
DESTDIR=staging PREFIX=/usr QMAKE=/usr/bin/qmake6 make install
mkdir -p dist
VERSION="$VERSION" nfpm package -f packaging/nfpm/nfpm.yaml -p deb --target dist
VERSION="$VERSION" nfpm package -f packaging/nfpm/nfpm.yaml -p rpm --target dist
ls -lh dist/
# Docker runs as root; hand ownership back even if a later step fails.
trap 'if [[ -n "${HOST_UID:-}" && -n "${HOST_GID:-}" ]]; then
  chown -R "${HOST_UID}:${HOST_GID}" \
    staging AppDir dist target/release/sound-spring target-ubuntu \
    .cache/appimage-tools 2>/dev/null || true
fi' EXIT
VERSION="$VERSION" QMAKE=/usr/bin/qmake6 packaging/appimage/build-appimage.sh
echo "=== ldd (should not require GLIBC > 2.39) ==="
ldd target/release/sound-spring | head -20 || true
objdump -T target/release/sound-spring 2>/dev/null | grep -oE "GLIBC_[0-9.]+" | sort -u | tail -5 || true
ls -lh dist/
'

echo "[package-ubuntu] done — artifacts in $ROOT/dist/"
