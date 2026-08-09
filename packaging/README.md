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

Qt runtime packages are in **universe** on Ubuntu. If `apt` reports packages as not installable, enable it:

```bash
sudo add-apt-repository universe
sudo apt update
```

Install the `.deb` with `apt` (not `dpkg -i` alone) so dependencies resolve automatically:

```bash
sudo apt install ./sound-spring_*_amd64.deb
gtk-launch sound-spring
```

The package declares real binary dependencies (`libqt6core6t64`, `qml6-module-qtquick`, etc.). Do **not** use source names like `qt6-base` or `qt6-declarative` — those are build-time source packages, not installable runtime packages on Ubuntu 24.04.

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

Requires PipeWire, `pactl`, `paplay`, `pw-cat`, and `ffmpeg` on the host. The AppImage bundles Qt (including QML modules, **Fusion** Quick Controls, Wayland plugins, and `offscreen` for headless smoke). UI chrome uses the app’s dark **SoundSpringTheme** + Fusion — not the host Plasma/Breeze theme — so look-and-feel matches across distros. On Wayland sessions the AppRun wrapper prefers `xcb` (XWayland) because Ubuntu Qt 6.4 Wayland-EGL often paints a black window on Plasma 6; override with `QT_QPA_PLATFORM=wayland` if you want to force native Wayland (may need a newer Qt build for reliable GL). For global shortcuts, launch from KRunner or the app menu — not from a terminal inside Cursor/VS Code/Chromium. See [docs/global-shortcuts.md](../docs/global-shortcuts.md).

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

## Local package install smoke tests (QEMU)

Ephemeral Ubuntu 24.04 and Fedora cloud VMs install the `.deb` / `.rpm` / AppImage from `dist/` and smoke-test offscreen launch. See [vm-tests/README.md](vm-tests/README.md).

```bash
# Host deps (CachyOS / Arch)
sudo pacman -S qemu-system-x86 qemu-img cloud-utils openssh edk2-ovmf

make package              # .deb + .rpm + AppImage → dist/
make test-packages        # package + VM matrix
make test-packages-quick  # reuse existing dist/
```

On CachyOS/Arch (glibc newer than Ubuntu 24.04), host-built binaries will fail guest launch with `GLIBC_2.43 not found`. Build packages in Docker instead:

```bash
make package-ubuntu       # Ubuntu 24.04 container (matches release.yml)
make test-packages-quick
```

---

## Building packages locally

On Ubuntu 24.04:

```bash
sudo apt install -y qt6-base-dev qt6-declarative-dev qt6-tools-dev \
  libqt6svg6-dev libpulse-dev pkg-config curl rpm
curl -fsSL -o /tmp/nfpm.tgz \
  https://github.com/goreleaser/nfpm/releases/download/v2.47.0/nfpm_2.47.0_Linux_x86_64.tar.gz
sudo tar -C /usr/local/bin -xzf /tmp/nfpm.tgz nfpm

QMAKE=/usr/bin/qmake6 make package
# or step-by-step:
# QMAKE=/usr/bin/qmake6 make build
# eval "$(./scripts/release-version.sh v0.1.0)"
# rm -rf staging && DESTDIR=staging PREFIX=/usr make install
# mkdir -p dist
# nfpm package -f packaging/nfpm/nfpm.yaml -p deb --target dist
# nfpm package -f packaging/nfpm/nfpm.yaml -p rpm --target dist
# VERSION="$VERSION" packaging/appimage/build-appimage.sh
```

---

## CI

Tag push (`v*`) runs [`.github/workflows/release.yml`](../.github/workflows/release.yml).

`Cargo.toml` `version` must match the tag (`v0.1.0` ↔ `0.1.0`).

---

## Flatpak

**Experimental.** Separate from GitHub Releases — see [flatpak/io.github.benwhite1987.SoundSpring.yml](flatpak/io.github.benwhite1987.SoundSpring.yml). The manifest does not yet grant the PipeWire / host-tool permissions Sound Spring needs for virtual mics and playback; use `.deb` or AppImage for real use.
