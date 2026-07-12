# Package install smoke tests (QEMU)

Boots ephemeral Ubuntu 24.04 and Fedora cloud VMs, installs Sound Spring
artifacts from `dist/`, and smoke-tests offscreen launch plus host audio CLIs.

## Host dependencies (CachyOS / Arch)

```bash
sudo pacman -S qemu-system-x86 qemu-img cloud-utils openssh edk2-ovmf
```

Required tools: `qemu-system-x86_64`, `qemu-img`, `cloud-localds`, `ssh`,
`ssh-keygen`, and OVMF firmware (`edk2-ovmf`).

## Artifacts

Place (or build) these under `dist/`:

- `sound-spring_*_amd64.deb`
- `sound-spring-*.x86_64.rpm`
- `sound-spring-*-x86_64.AppImage`

```bash
make package              # build all three into dist/
make test-packages        # package + run VM matrix
make test-packages-quick  # reuse existing dist/
```

Or:

```bash
packaging/vm-tests/run.sh
```

## What is checked

| Guest | Formats | Checks |
|-------|---------|--------|
| Ubuntu 24.04 | `.deb`, AppImage | apt/dnf install succeeds; `ldd` clean (native packages); `pactl`/`paplay`/`pw-cat`/`ffmpeg` present; `QT_QPA_PLATFORM=offscreen` launch sees `startup: QML engine loaded` or stays alive ~2s |
| Fedora 42 | `.rpm`, AppImage | same |

AppImages use `APPIMAGE_EXTRACT_AND_RUN=1` (no FUSE required).

## Cache and resources

- Images: `.cache/vm-images/` (~0.5–1 GiB each, kept across runs)
- Work/logs: `.cache/vm-work/` (overlays discarded after each guest)

Each guest uses ~2 GiB RAM and 2 vCPUs. First run downloads cloud images
(~5–15 min depending on network); cached runs are typically a few minutes.

## Environment overrides

| Variable | Default | Meaning |
|----------|---------|---------|
| `DIST_DIR` | `$ROOT/dist` | Artifact directory |
| `VM_GUESTS` | `ubuntu-24.04 fedora` | Space-separated guest config names |
| `VM_MEM_MB` | `2048` | Guest RAM |
| `VM_CPUS` | `2` | Guest vCPUs |
| `VM_SSH_TIMEOUT` | `180` | Seconds to wait for SSH |
| `VM_KEEP_WORK` | unset | If `1`, keep overlays/logs under `.cache/vm-work/` |
