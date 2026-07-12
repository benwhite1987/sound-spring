#!/usr/bin/env bash
# Orchestrate QEMU package install smoke tests for Sound Spring.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VMTESTS="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/common.sh
source "$VMTESTS/lib/common.sh"

DIST_DIR="${DIST_DIR:-$ROOT/dist}"
IMAGE_CACHE="${IMAGE_CACHE:-$ROOT/.cache/vm-images}"
WORK_ROOT="${WORK_ROOT:-$ROOT/.cache/vm-work}"
VM_GUESTS="${VM_GUESTS:-ubuntu-24.04 fedora}"
VM_MEM_MB="${VM_MEM_MB:-2048}"
VM_CPUS="${VM_CPUS:-2}"
VM_SSH_TIMEOUT="${VM_SSH_TIMEOUT:-180}"
VM_KEEP_WORK="${VM_KEEP_WORK:-0}"
OVERLAY_SIZE="${OVERLAY_SIZE:-20G}"

RUN_ID="$(date +%Y%m%d-%H%M%S)-$$"
RUN_DIR="$WORK_ROOT/$RUN_ID"
SUMMARY=()
FAILURES=0

cleanup_run() {
  if [[ "$VM_KEEP_WORK" == "1" ]]; then
    vm_log "keeping work dir $RUN_DIR"
    return 0
  fi
  # Overlays/seeds are large; keep only logs if present.
  if [[ -d "$RUN_DIR" ]]; then
    find "$RUN_DIR" -type f \( -name '*.qcow2' -o -name 'seed.iso' -o -name 'OVMF_VARS*.fd' -o -name 'id_ed25519*' \) -delete 2>/dev/null || true
  fi
}
trap cleanup_run EXIT

guest_install_and_smoke() {
  local port="$1" user="$2" key="$3" format="$4" artifact="$5"
  local remote_dir="/home/${user}/ss-artifacts"
  local remote_pkg remote_bin label smoke_flags=()

  vm_ssh "$port" "$user" "$key" "mkdir -p '$remote_dir'"
  vm_scp_to "$port" "$user" "$key" "$artifact" "$remote_dir/"
  vm_scp_to "$port" "$user" "$key" "$VMTESTS/guest-smoke.sh" "$remote_dir/guest-smoke.sh"
  vm_ssh "$port" "$user" "$key" "chmod +x '$remote_dir/guest-smoke.sh'"

  remote_pkg="$remote_dir/$(basename "$artifact")"
  label="${GUEST_NAME}-${format}"

  case "$format" in
    deb)
      vm_log "[$GUEST_NAME] apt install $(basename "$artifact")"
      vm_ssh "$port" "$user" "$key" "sudo apt-get update -qq"
      # Enable universe if available (idempotent).
      vm_ssh "$port" "$user" "$key" \
        "sudo apt-get install -y -qq software-properties-common >/dev/null 2>&1 || true; \
         sudo add-apt-repository -y universe >/dev/null 2>&1 || true; \
         sudo apt-get update -qq"
      vm_ssh "$port" "$user" "$key" "sudo DEBIAN_FRONTEND=noninteractive apt-get install -y '$remote_pkg'"
      # Ensure host audio tools exist even if Recommends were skipped.
      vm_ssh "$port" "$user" "$key" \
        "sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq pipewire pipewire-pulse pipewire-bin pulseaudio-utils ffmpeg wireplumber >/dev/null"
      remote_bin=/usr/bin/sound-spring
      ;;
    rpm)
      vm_log "[$GUEST_NAME] dnf install $(basename "$artifact")"
      vm_ssh "$port" "$user" "$key" "sudo dnf install -y '$remote_pkg'"
      vm_ssh "$port" "$user" "$key" \
        "sudo dnf install -y pipewire pipewire-pulseaudio pipewire-utils pulseaudio-utils ffmpeg wireplumber >/dev/null"
      remote_bin=/usr/bin/sound-spring
      ;;
    appimage)
      vm_log "[$GUEST_NAME] AppImage prepare $(basename "$artifact")"
      vm_ssh "$port" "$user" "$key" "chmod +x '$remote_pkg'"
      # Audio tools still required on the host for AppImage.
      if vm_ssh "$port" "$user" "$key" "command -v apt-get >/dev/null"; then
        vm_ssh "$port" "$user" "$key" \
          "sudo DEBIAN_FRONTEND=noninteractive apt-get update -qq; sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq pipewire pipewire-pulse pipewire-bin pulseaudio-utils ffmpeg wireplumber >/dev/null"
      else
        vm_ssh "$port" "$user" "$key" \
          "sudo dnf install -y pipewire pipewire-pulseaudio pipewire-utils pulseaudio-utils ffmpeg wireplumber >/dev/null"
      fi
      remote_bin="$remote_pkg"
      smoke_flags=(--appimage)
      ;;
    *)
      vm_die "unsupported format: $format"
      ;;
  esac

  local flags_str=""
  if ((${#smoke_flags[@]})); then
    flags_str="${smoke_flags[*]}"
  fi

  vm_log "[$GUEST_NAME] smoke ($format)"
  if vm_ssh "$port" "$user" "$key" \
    "SS_VM_RESULT_JSON=/tmp/ss-vm-result-${label}.json \
     SS_VM_SMOKE_LOG=/tmp/ss-smoke-${label}.log \
     bash '$remote_dir/guest-smoke.sh' '$label' '$remote_bin' $flags_str"; then
    SUMMARY+=("PASS  ${label}")
    vm_scp_from "$port" "$user" "$key" "/tmp/ss-vm-result-${label}.json" "$RUN_DIR/${label}.json" 2>/dev/null || true
    vm_scp_from "$port" "$user" "$key" "/tmp/ss-smoke-${label}.log" "$RUN_DIR/${label}.log" 2>/dev/null || true
    return 0
  fi

  SUMMARY+=("FAIL  ${label}")
  FAILURES=$((FAILURES + 1))
  vm_scp_from "$port" "$user" "$key" "/tmp/ss-smoke-${label}.log" "$RUN_DIR/${label}.log" 2>/dev/null || true
  vm_scp_from "$port" "$user" "$key" "/tmp/ss-vm-result-${label}.json" "$RUN_DIR/${label}.json" 2>/dev/null || true
  if [[ -f "$RUN_DIR/${label}.log" ]]; then
    vm_log "---- ${label} smoke log ----"
    sed -n '1,80p' "$RUN_DIR/${label}.log" || true
  fi
  return 1
}

run_guest() {
  local conf_name="$1"
  local conf="$VMTESTS/guests/${conf_name}.conf"
  [[ -f "$conf" ]] || vm_die "guest config not found: $conf"

  vm_load_guest_conf "$conf"

  local guest_dir="$RUN_DIR/$GUEST_NAME"
  mkdir -p "$guest_dir"

  local base_image="$IMAGE_CACHE/$IMAGE_FILE"
  vm_download_image "$IMAGE_URL" "$base_image"

  local ovmf_code ovmf_vars_template
  local ovmf_pair
  ovmf_pair=$(vm_find_ovmf_pair) || vm_die "OVMF CODE/VARS not found (install edk2-ovmf)"
  ovmf_code="${ovmf_pair%%$'\t'*}"
  ovmf_vars_template="${ovmf_pair#*$'\t'}"

  local overlay="$guest_dir/overlay.qcow2"
  local vars="$guest_dir/OVMF_VARS.fd"
  local serial_log="$guest_dir/serial.log"
  local pidfile="$guest_dir/qemu.pid"
  local pubkey="$RUN_DIR/id_ed25519.pub"
  local privkey="$RUN_DIR/id_ed25519"

  cp "$ovmf_vars_template" "$vars"
  qemu-img create -f qcow2 -b "$base_image" -F qcow2 "$overlay" "$OVERLAY_SIZE" >/dev/null
  vm_write_cloud_init "$guest_dir" "$GUEST_USER" "$pubkey" "ss-${GUEST_NAME}"

  local port
  port=$(vm_pick_free_port)

  vm_log "booting $GUEST_NAME (SSH port $port, ${VM_MEM_MB}M, ${VM_CPUS} cpus)"
  vm_start_qemu \
    "$overlay" \
    "$guest_dir/seed.iso" \
    "$vars" \
    "$ovmf_code" \
    "$serial_log" \
    "$port" \
    "$VM_MEM_MB" \
    "$VM_CPUS" \
    "$pidfile"

  if ! vm_wait_for_ssh "$port" "$GUEST_USER" "$privkey" "$VM_SSH_TIMEOUT"; then
    vm_log "serial log (tail):"
    tail -n 80 "$serial_log" 2>/dev/null || true
    vm_stop_qemu "$port" "$GUEST_USER" "$privkey" "$pidfile"
    SUMMARY+=("FAIL  ${GUEST_NAME} (SSH timeout)")
    FAILURES=$((FAILURES + 1))
    return 1
  fi

  local format artifact
  local guest_failed=0
  for format in $PKG_FORMATS; do
    if ! artifact=$(vm_resolve_artifact "$DIST_DIR" "$format"); then
      vm_log "SKIP ${GUEST_NAME}-${format}: no artifact in $DIST_DIR"
      SUMMARY+=("SKIP  ${GUEST_NAME}-${format} (missing artifact)")
      continue
    fi
    if ! guest_install_and_smoke "$port" "$GUEST_USER" "$privkey" "$format" "$artifact"; then
      guest_failed=1
      # Continue other formats on the same guest when possible.
    fi
  done

  vm_stop_qemu "$port" "$GUEST_USER" "$privkey" "$pidfile"
  return "$guest_failed"
}

main() {
  vm_require_host_tools
  mkdir -p "$IMAGE_CACHE" "$RUN_DIR"

  if [[ ! -d "$DIST_DIR" ]]; then
    vm_die "dist dir missing: $DIST_DIR (run: make package)"
  fi

  local needed_any=0
  local g conf
  for g in $VM_GUESTS; do
    conf="$VMTESTS/guests/${g}.conf"
    [[ -f "$conf" ]] || vm_die "guest config not found: $conf"
    # shellcheck disable=SC1090
    source "$conf"
    local fmt
    for fmt in $PKG_FORMATS; do
      if vm_resolve_artifact "$DIST_DIR" "$fmt" >/dev/null; then
        needed_any=1
      fi
    done
  done
  if [[ "$needed_any" -eq 0 ]]; then
    vm_die "no matching artifacts in $DIST_DIR (expected .deb / .rpm / .AppImage). Run: make package"
  fi

  ssh-keygen -t ed25519 -N '' -f "$RUN_DIR/id_ed25519" -C "sound-spring-vm-tests" >/dev/null

  vm_log "run id $RUN_ID"
  vm_log "artifacts from $DIST_DIR"
  vm_log "guests: $VM_GUESTS"

  local guest
  for guest in $VM_GUESTS; do
    run_guest "$guest" || true
  done

  echo
  vm_log "======== summary ========"
  local line
  for line in "${SUMMARY[@]+"${SUMMARY[@]}"}"; do
    printf '%s\n' "$line"
  done
  if [[ "$FAILURES" -gt 0 ]]; then
    vm_log "FAILED ($FAILURES cell(s))"
    exit 1
  fi
  if [[ ${#SUMMARY[@]} -eq 0 ]]; then
    vm_die "no test cells ran"
  fi
  vm_log "ALL PASSED"
}

main "$@"
