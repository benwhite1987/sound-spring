# shellcheck shell=bash
# Shared helpers for packaging/vm-tests/run.sh

vm_log() { printf '[vm-tests] %s\n' "$*"; }
vm_die() { printf '[vm-tests] ERROR: %s\n' "$*" >&2; exit 1; }

vm_activate_local_tools() {
  # Prefer an extracted Arch-style prefix under .cache/vm-tools/root when
  # system packages are not installed (no root required on the host).
  local prefix="${VM_TOOLS_PREFIX:-}"
  if [[ -z "$prefix" && -x "${ROOT:-}/.cache/vm-tools/root/usr/bin/qemu-system-x86_64" ]]; then
    prefix="${ROOT}/.cache/vm-tools/root"
  fi
  if [[ -z "$prefix" || ! -d "$prefix" ]]; then
    return 0
  fi
  export PATH="${prefix}/usr/bin:${PATH}"
  if [[ -d "${prefix}/usr/lib" ]]; then
    export LD_LIBRARY_PATH="${prefix}/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  fi
  # cloud-localds looks for genisoimage; Arch cdrtools ships mkisofs.
  if [[ ! -x "${prefix}/usr/bin/genisoimage" && -x "${prefix}/usr/bin/mkisofs" ]]; then
    ln -sfn mkisofs "${prefix}/usr/bin/genisoimage"
  fi
  export VM_TOOLS_PREFIX="$prefix"
  vm_log "using local tools prefix: $prefix"
}

vm_require_host_tools() {
  vm_activate_local_tools
  local missing=()
  local cmd
  for cmd in qemu-system-x86_64 qemu-img cloud-localds ssh ssh-keygen curl; do
    command -v "$cmd" >/dev/null 2>&1 || missing+=("$cmd")
  done
  if ((${#missing[@]})); then
    vm_die "missing host tools: ${missing[*]} (see packaging/vm-tests/README.md)"
  fi
}

vm_find_ovmf_pair() {
  # Prints: CODE_PATH<TAB>VARS_PATH  (matching 4M / non-4M pair)
  local prefix="${VM_TOOLS_PREFIX:-}"
  local pairs=(
    "${prefix}/usr/share/edk2/x64/OVMF_CODE.4m.fd|${prefix}/usr/share/edk2/x64/OVMF_VARS.4m.fd"
    "${prefix}/usr/share/edk2/x64/OVMF_CODE.fd|${prefix}/usr/share/edk2/x64/OVMF_VARS.fd"
    "/usr/share/edk2/x64/OVMF_CODE.4m.fd|/usr/share/edk2/x64/OVMF_VARS.4m.fd"
    "/usr/share/edk2/x64/OVMF_CODE.fd|/usr/share/edk2/x64/OVMF_VARS.fd"
    "/usr/share/edk2-ovmf/x64/OVMF_CODE.4m.fd|/usr/share/edk2-ovmf/x64/OVMF_VARS.4m.fd"
    "/usr/share/edk2-ovmf/x64/OVMF_CODE.fd|/usr/share/edk2-ovmf/x64/OVMF_VARS.fd"
    "/usr/share/OVMF/OVMF_CODE_4M.fd|/usr/share/OVMF/OVMF_VARS_4M.fd"
    "/usr/share/OVMF/OVMF_CODE.fd|/usr/share/OVMF/OVMF_VARS.fd"
    "/usr/share/qemu/OVMF_CODE.fd|/usr/share/qemu/OVMF_VARS.fd"
  )
  local entry code vars
  for entry in "${pairs[@]}"; do
    code="${entry%%|*}"
    vars="${entry##*|}"
    if [[ -n "$code" && -n "$vars" && -f "$code" && -f "$vars" ]]; then
      printf '%s\t%s\n' "$code" "$vars"
      return 0
    fi
  done
  return 1
}

vm_download_image() {
  local url="$1"
  local dest="$2"
  if [[ -f "$dest" ]]; then
    vm_log "using cached image $(basename "$dest")"
    return 0
  fi
  mkdir -p "$(dirname "$dest")"
  local tmp="${dest}.partial"
  vm_log "downloading $(basename "$dest") ..."
  curl -fL --retry 3 --retry-delay 2 -o "$tmp" "$url"
  mv -f "$tmp" "$dest"
}

vm_pick_free_port() {
  # Prefer Python; fall back to a high ephemeral range scan.
  if command -v python3 >/dev/null 2>&1; then
    python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
    return 0
  fi
  local port
  for port in $(seq 22022 22122); do
    if ! ss -ltn "sport = :$port" 2>/dev/null | grep -q ":$port"; then
      printf '%s\n' "$port"
      return 0
    fi
  done
  vm_die "could not find a free SSH forward port"
}

vm_write_cloud_init() {
  local work="$1"
  local user="$2"
  local pubkey="$3"
  local hostname="$4"

  cat >"$work/user-data" <<EOF
#cloud-config
hostname: ${hostname}
manage_etc_hosts: true
users:
  - default
  - name: ${user}
    lock_passwd: true
    shell: /bin/bash
    sudo: ALL=(ALL) NOPASSWD:ALL
    ssh_authorized_keys:
      - $(cat "$pubkey")
ssh_pwauth: false
package_update: false
growpart:
  mode: auto
  devices: ["/"]
  ignore_growroot_disabled: false
EOF

  cat >"$work/meta-data" <<EOF
instance-id: sound-spring-${hostname}
local-hostname: ${hostname}
EOF

  cloud-localds "$work/seed.iso" "$work/user-data" "$work/meta-data"
  [[ -f "$work/seed.iso" ]] || vm_die "cloud-localds failed to create $work/seed.iso (need genisoimage/mkisofs)"
}

vm_ssh() {
  local port="$1"
  local user="$2"
  local key="$3"
  shift 3
  ssh \
    -i "$key" \
    -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    -o GlobalKnownHostsFile=/dev/null \
    -o LogLevel=ERROR \
    -o ConnectTimeout=5 \
    -o BatchMode=yes \
    -p "$port" \
    "${user}@127.0.0.1" \
    "$@"
}

vm_scp_to() {
  local port="$1"
  local user="$2"
  local key="$3"
  local src="$4"
  local dest="$5"
  scp \
    -i "$key" \
    -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    -o GlobalKnownHostsFile=/dev/null \
    -o LogLevel=ERROR \
    -o BatchMode=yes \
    -P "$port" \
    "$src" \
    "${user}@127.0.0.1:${dest}"
}

vm_scp_from() {
  local port="$1"
  local user="$2"
  local key="$3"
  local src="$4"
  local dest="$5"
  scp \
    -i "$key" \
    -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    -o GlobalKnownHostsFile=/dev/null \
    -o LogLevel=ERROR \
    -o BatchMode=yes \
    -P "$port" \
    "${user}@127.0.0.1:${src}" \
    "$dest"
}

vm_wait_for_ssh() {
  local port="$1"
  local user="$2"
  local key="$3"
  local timeout="${4:-180}"
  local start now
  start=$(date +%s)
  vm_log "waiting for SSH on 127.0.0.1:${port} (up to ${timeout}s) ..."
  while true; do
    if vm_ssh "$port" "$user" "$key" 'true' 2>/dev/null; then
      vm_log "SSH is up"
      return 0
    fi
    now=$(date +%s)
    if (( now - start >= timeout )); then
      return 1
    fi
    sleep 2
  done
}

vm_start_qemu() {
  local overlay="$1"
  local seed="$2"
  local vars="$3"
  local code="$4"
  local serial_log="$5"
  local port="$6"
  local mem_mb="$7"
  local cpus="$8"
  local pidfile="$9"

  local accel=() cpu=()
  if [[ -r /dev/kvm ]]; then
    accel=(-accel kvm)
    # Fedora cloud + OVMF has page-faulted with -cpu host on some hosts; max is safer.
    # la57 (5-level paging) also trips #PF with some OVMF/kernel combos — disable it.
    if [[ "${VM_CPU_MODEL:-}" == "host" ]]; then
      cpu=(-cpu host,la57=off)
    else
      cpu=(-cpu "${VM_CPU_MODEL:-max}",la57=off)
    fi
  else
    accel=(-accel tcg)
    cpu=(-cpu max,la57=off)
    vm_log "WARNING: /dev/kvm not available; using TCG (slow)"
  fi

  local machine="q35"
  if [[ -n "${VM_MACHINE_OPTS:-}" ]]; then
    machine="q35,${VM_MACHINE_OPTS}"
  fi

  qemu-system-x86_64 \
    -name "sound-spring-vm" \
    -machine "$machine" \
    "${accel[@]}" \
    "${cpu[@]}" \
    -m "$mem_mb" \
    -smp "$cpus" \
    -drive "if=pflash,format=raw,readonly=on,file=${code}" \
    -drive "if=pflash,format=raw,file=${vars}" \
    -drive "file=${overlay},if=virtio,format=qcow2,discard=unmap" \
    -cdrom "$seed" \
    -netdev "user,id=net0,hostfwd=tcp:127.0.0.1:${port}-:22" \
    -device virtio-net-pci,netdev=net0 \
    -vga std \
    -display none \
    -serial "file:${serial_log}" \
    -pidfile "$pidfile" \
    -daemonize
}

vm_stop_qemu() {
  local port="$1"
  local user="$2"
  local key="$3"
  local pidfile="$4"
  local timeout=60

  if vm_ssh "$port" "$user" "$key" 'sudo poweroff' 2>/dev/null; then
    :
  elif [[ -f "$pidfile" ]]; then
    local pid
    pid=$(cat "$pidfile")
    kill -TERM "$pid" 2>/dev/null || true
  fi

  local start now
  start=$(date +%s)
  while [[ -f "$pidfile" ]]; do
    local pid
    pid=$(cat "$pidfile" 2>/dev/null || true)
    if [[ -z "${pid:-}" ]] || ! kill -0 "$pid" 2>/dev/null; then
      rm -f "$pidfile"
      return 0
    fi
    now=$(date +%s)
    if (( now - start >= timeout )); then
      kill -KILL "$pid" 2>/dev/null || true
      rm -f "$pidfile"
      return 0
    fi
    sleep 1
  done
}

vm_load_guest_conf() {
  local conf="$1"
  # shellcheck disable=SC1090
  source "$conf"
  : "${GUEST_NAME:?GUEST_NAME missing in $conf}"
  : "${GUEST_USER:?GUEST_USER missing in $conf}"
  : "${IMAGE_URL:?IMAGE_URL missing in $conf}"
  : "${IMAGE_FILE:?IMAGE_FILE missing in $conf}"
  : "${PKG_FORMATS:?PKG_FORMATS missing in $conf}"
}

vm_resolve_artifact() {
  local dist="$1"
  local kind="$2"
  local pattern match
  case "$kind" in
    deb) pattern='sound-spring_*_amd64.deb' ;;
    rpm) pattern='sound-spring-*.x86_64.rpm' ;;
    appimage) pattern='sound-spring-*-x86_64.AppImage' ;;
    *) vm_die "unknown artifact kind: $kind" ;;
  esac
  # Prefer the newest match if several exist.
  match=$(ls -1t "$dist"/$pattern 2>/dev/null | head -1 || true)
  if [[ -z "$match" || ! -f "$match" ]]; then
    return 1
  fi
  printf '%s\n' "$match"
}
