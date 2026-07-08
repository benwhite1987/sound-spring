# Sound Spring — release packages

Pre-built artifacts are published on [GitHub Releases](https://github.com/benwhite1987/sound-spring/releases) for each `v*` tag.

| Format | Target |
|--------|--------|
| `.deb` | Ubuntu 24.04+, Debian-based |
| `.rpm` | Fedora 40+, RHEL-like (Qt6 package names) |
| `.AppImage` | Portable x86_64 Linux (bundles Qt) |

Host audio tools (**PipeWire**, **paplay**, **ffmpeg**) are not bundled in any format.

---

## Ubuntu / Debian (.deb)

```bash
sudo apt update
sudo apt install -y pipewire pulseaudio-utils qt6-base qt6-declarative qt6-wayland ffmpeg
sudo apt install ./sound-spring_*_amd64.deb
gtk-launch sound-spring
```

Uninstall: `sudo apt remove sound-spring`

---

## Fedora (.rpm)

```bash
sudo dnf install -y pipewire pulseaudio-utils qt6-qtbase qt6-qtdeclarative qt6-qtwayland ffmpeg
sudo dnf install ./sound-spring-*.x86_64.rpm
sound-spring
```

Uninstall: `sudo dnf remove sound-spring`

---

## AppImage

```bash
chmod +x sound-spring-*-x86_64.AppImage
./sound-spring-*-x86_64.AppImage
```

Requires PipeWire, `pactl`, `paplay`, `pw-cat`, and `ffmpeg` on the host. The AppImage bundles Qt (including QML modules and native Wayland support). For global shortcuts, launch from KRunner or the app menu — not from a terminal inside Cursor/VS Code/Chromium. See [docs/global-shortcuts.md](../docs/global-shortcuts.md).

On Arch/CachyOS install host audio tools if missing:

```bash
sudo pacman -S pipewire wireplumber pipewire-pulse pipewire-audio ffmpeg
```

If Settings shows no microphones or only "Default output device", verify the host sees devices:

```bash
pactl list short sources
pactl list short sinks
RUST_LOG=sound_spring=debug ./sound-spring-*-x86_64.AppImage 2>&1 | grep -E 'listed|pactl|mic source'
```

If an older AppImage fails on Wayland with missing `wayland` or QML plugins, rebuild with the current `packaging/appimage/build-appimage.sh` or download a newer GitHub Release.

---

## Building packages locally

On Ubuntu 24.04:

```bash
sudo apt install -y qt6-base-dev qt6-declarative-dev qt6-tools-dev \
  libpulse-dev pkg-config curl rpm
curl -fsSL -o /tmp/nfpm.tgz \
  https://github.com/goreleaser/nfpm/releases/download/v2.47.0/nfpm_2.47.0_Linux_x86_64.tar.gz
sudo tar -C /usr/local/bin -xzf /tmp/nfpm.tgz nfpm

QMAKE=/usr/bin/qmake6 make build
eval "$(./scripts/release-version.sh v0.1.0)"
rm -rf staging && DESTDIR=staging PREFIX=/usr make install
mkdir -p dist
nfpm package -f packaging/nfpm/nfpm.yaml -p deb --target dist
nfpm package -f packaging/nfpm/nfpm.yaml -p rpm --target dist
VERSION="$VERSION" packaging/appimage/build-appimage.sh
```

---

## CI

Tag push (`v*`) runs [`.github/workflows/release.yml`](../.github/workflows/release.yml).

`Cargo.toml` `version` must match the tag (`v0.1.0` ↔ `0.1.0`).

---

## Flatpak

Separate from GitHub Releases — see [flatpak/io.github.benwhite1987.SoundSpring.yml](flatpak/io.github.benwhite1987.SoundSpring.yml).
