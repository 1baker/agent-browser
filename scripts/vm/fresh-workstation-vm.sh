#!/usr/bin/env bash
set -euo pipefail

COMMAND="${1:-}"
if [[ "$#" -gt 0 ]]; then
  shift
fi

STATE_DIR="${AGENT_BROWSER_VM_STATE_DIR:-}"
BASE_IMAGE="${AGENT_BROWSER_VM_BASE_IMAGE:-}"
CANDIDATE="${AGENT_BROWSER_VM_CANDIDATE:-}"
SSH_PORT="${AGENT_BROWSER_VM_SSH_PORT:-22222}"
DASHBOARD_PORT="${AGENT_BROWSER_VM_DASHBOARD_PORT:-14848}"
GUACAMOLE_PORT="${AGENT_BROWSER_VM_GUACAMOLE_PORT:-18092}"
VM_MEMORY_MB="${AGENT_BROWSER_VM_MEMORY_MB:-8192}"
VM_CPUS="${AGENT_BROWSER_VM_CPUS:-4}"
VM_PASSWORD="${AGENT_BROWSER_VM_PASSWORD:-agent-browser-fresh-install}"
QEMU_SYSTEM_BIN="${QEMU_SYSTEM_BIN:-qemu-system-x86_64}"
QEMU_IMG_BIN="${QEMU_IMG_BIN:-qemu-img}"
QEMU_FIRMWARE_DIR="${AGENT_BROWSER_VM_QEMU_FIRMWARE_DIR:-}"
QEMU_BIOS_PATH="${AGENT_BROWSER_VM_QEMU_BIOS_PATH:-}"
CLOUD_LOCALDS_BIN="${CLOUD_LOCALDS_BIN:-cloud-localds}"

usage() {
  cat <<'EOF'
Usage:
  AGENT_BROWSER_VM_STATE_DIR=<path> \
  AGENT_BROWSER_VM_BASE_IMAGE=<ubuntu-noble-cloud-image> \
  scripts/vm/fresh-workstation-vm.sh <prepare|start|wait|stage|status|stop|reset>

Commands:
  prepare  Create the SSH key, cloud-init seed, and clean qcow2 overlay.
  start    Start the prepared VM with loopback-only forwarded ports.
  wait     Wait up to five minutes for SSH and print the boot ID.
  stage    Copy AGENT_BROWSER_VM_CANDIDATE to the source-free guest.
  status   Show the QEMU process, guest boot ID, and candidate version.
  stop     Terminate only the QEMU process recorded in the scoped state dir.
  reset    Recreate only the scoped overlay from the immutable base image.

The guest user is agent. The disposable sudo password comes from
AGENT_BROWSER_VM_PASSWORD. Run the candidate install command in a visible SSH
TTY so the initial sudo prompt can be counted and recorded.
Set AGENT_BROWSER_VM_QEMU_FIRMWARE_DIR or AGENT_BROWSER_VM_QEMU_BIOS_PATH when
an extracted QEMU build cannot discover its firmware from system paths.

Inside the guest:
  /home/agent/agent-browser-candidate install workstation --dry-run --json
  /home/agent/agent-browser-candidate install workstation --apply --json
EOF
}

require_state_dir() {
  if [[ -z "$STATE_DIR" || "$STATE_DIR" == "/" || "$STATE_DIR" == "$HOME" ]]; then
    echo "AGENT_BROWSER_VM_STATE_DIR must be an explicit narrow directory." >&2
    exit 2
  fi
}

require_base_image() {
  if [[ -z "$BASE_IMAGE" || ! -f "$BASE_IMAGE" ]]; then
    echo "AGENT_BROWSER_VM_BASE_IMAGE must name a readable Ubuntu cloud image." >&2
    exit 2
  fi
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Required command is unavailable: $1" >&2
    exit 2
  fi
}

pid_is_live() {
  [[ -r "$STATE_DIR/qemu.pid" ]] || return 1
  local pid
  pid="$(cat "$STATE_DIR/qemu.pid")"
  [[ "$pid" =~ ^[1-9][0-9]*$ ]] || return 1
  kill -0 "$pid" 2>/dev/null
}

prepare() {
  require_state_dir
  require_base_image
  require_command "$QEMU_IMG_BIN"
  require_command "$CLOUD_LOCALDS_BIN"
  require_command ssh-keygen
  if [[ ! "$VM_PASSWORD" =~ ^[A-Za-z0-9._-]+$ ]]; then
    echo "AGENT_BROWSER_VM_PASSWORD must use only safe disposable password characters." >&2
    exit 2
  fi
  mkdir -p "$STATE_DIR"
  chmod 700 "$STATE_DIR"
  if [[ ! -f "$STATE_DIR/id_ed25519" ]]; then
    ssh-keygen -q -t ed25519 -N '' -f "$STATE_DIR/id_ed25519"
  fi
  chmod 600 "$STATE_DIR/id_ed25519"
  local public_key
  public_key="$(cat "$STATE_DIR/id_ed25519.pub")"
  cat > "$STATE_DIR/user-data" <<EOF
#cloud-config
users:
  - name: agent
    groups: [adm, sudo]
    shell: /bin/bash
    lock_passwd: false
    plain_text_passwd: ${VM_PASSWORD}
    ssh_authorized_keys:
      - ${public_key}
ssh_pwauth: false
package_update: false
EOF
  cat > "$STATE_DIR/meta-data" <<'EOF'
instance-id: agent-browser-fresh-install
local-hostname: agent-browser-fresh-install
EOF
  "$CLOUD_LOCALDS_BIN" "$STATE_DIR/seed.img" \
    "$STATE_DIR/user-data" "$STATE_DIR/meta-data"
  recreate_overlay
  echo "Prepared fresh workstation VM state in $STATE_DIR"
}

recreate_overlay() {
  require_state_dir
  require_base_image
  require_command "$QEMU_IMG_BIN"
  if pid_is_live; then
    echo "Refusing to reset a running VM." >&2
    exit 1
  fi
  rm -f "$STATE_DIR/overlay.qcow2"
  "$QEMU_IMG_BIN" create -f qcow2 -F qcow2 -b "$BASE_IMAGE" \
    "$STATE_DIR/overlay.qcow2"
}

start_vm() {
  require_state_dir
  require_command "$QEMU_SYSTEM_BIN"
  for file in overlay.qcow2 seed.img id_ed25519; do
    if [[ ! -f "$STATE_DIR/$file" ]]; then
      echo "Missing prepared VM file: $STATE_DIR/$file" >&2
      exit 2
    fi
  done
  if pid_is_live; then
    echo "VM is already running with PID $(cat "$STATE_DIR/qemu.pid")." >&2
    exit 1
  fi
  local -a firmware_args
  firmware_args=()
  if [[ -n "$QEMU_FIRMWARE_DIR" ]]; then
    if [[ ! -d "$QEMU_FIRMWARE_DIR" ]]; then
      echo "AGENT_BROWSER_VM_QEMU_FIRMWARE_DIR must name a readable directory." >&2
      exit 2
    fi
    firmware_args+=(-L "$QEMU_FIRMWARE_DIR")
  fi
  if [[ -n "$QEMU_BIOS_PATH" ]]; then
    if [[ ! -f "$QEMU_BIOS_PATH" ]]; then
      echo "AGENT_BROWSER_VM_QEMU_BIOS_PATH must name a readable BIOS image." >&2
      exit 2
    fi
    firmware_args+=(-bios "$QEMU_BIOS_PATH")
  fi
  "$QEMU_SYSTEM_BIN" \
    "${firmware_args[@]}" \
    -accel tcg,thread=multi \
    -cpu max \
    -smp "$VM_CPUS" \
    -m "$VM_MEMORY_MB" \
    -drive "file=$STATE_DIR/overlay.qcow2,if=virtio,format=qcow2" \
    -drive "file=$STATE_DIR/seed.img,if=virtio,format=raw,readonly=on" \
    -netdev "user,id=net0,hostfwd=tcp:127.0.0.1:$SSH_PORT-:22,hostfwd=tcp:127.0.0.1:$DASHBOARD_PORT-:4848,hostfwd=tcp:127.0.0.1:$GUACAMOLE_PORT-:8092" \
    -device virtio-net-pci,netdev=net0 \
    -display none \
    -serial "file:$STATE_DIR/serial.log" \
    -daemonize \
    -pidfile "$STATE_DIR/qemu.pid"
  echo "Started fresh workstation VM with PID $(cat "$STATE_DIR/qemu.pid")."
}

ssh_args() {
  printf '%s\0' \
    -i "$STATE_DIR/id_ed25519" \
    -p "$SSH_PORT" \
    -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    -o ConnectTimeout=5
}

scp_args() {
  printf '%s\0' \
    -i "$STATE_DIR/id_ed25519" \
    -P "$SSH_PORT" \
    -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    -o ConnectTimeout=5
}

wait_for_vm() {
  require_state_dir
  local -a args
  mapfile -d '' -t args < <(ssh_args)
  for _attempt in $(seq 1 60); do
    if ssh "${args[@]}" agent@127.0.0.1 \
      'cat /proc/sys/kernel/random/boot_id' 2>/dev/null; then
      return 0
    fi
    sleep 5
  done
  echo "VM did not become reachable within five minutes." >&2
  exit 1
}

stage_candidate() {
  require_state_dir
  if [[ -z "$CANDIDATE" || ! -x "$CANDIDATE" ]]; then
    echo "AGENT_BROWSER_VM_CANDIDATE must name an executable release binary." >&2
    exit 2
  fi
  local -a copy_args shell_args
  mapfile -d '' -t copy_args < <(scp_args)
  mapfile -d '' -t shell_args < <(ssh_args)
  scp "${copy_args[@]}" "$CANDIDATE" \
    agent@127.0.0.1:/home/agent/agent-browser-candidate
  ssh "${shell_args[@]}" agent@127.0.0.1 \
    'chmod 0755 /home/agent/agent-browser-candidate && /home/agent/agent-browser-candidate --version'
}

status_vm() {
  require_state_dir
  if pid_is_live; then
    ps -p "$(cat "$STATE_DIR/qemu.pid")" -o pid,etime,stat,%cpu,%mem,cmd
  else
    echo "VM is not running."
    return 1
  fi
  local -a args
  mapfile -d '' -t args < <(ssh_args)
  ssh "${args[@]}" agent@127.0.0.1 \
    'printf "boot_id="; cat /proc/sys/kernel/random/boot_id; test ! -x /home/agent/agent-browser-candidate || /home/agent/agent-browser-candidate --version'
}

stop_vm() {
  require_state_dir
  if ! pid_is_live; then
    echo "VM is not running."
    return 0
  fi
  local pid
  pid="$(cat "$STATE_DIR/qemu.pid")"
  kill "$pid"
  for _attempt in $(seq 1 30); do
    if ! kill -0 "$pid" 2>/dev/null; then
      rm -f "$STATE_DIR/qemu.pid"
      echo "Stopped fresh workstation VM."
      return 0
    fi
    sleep 1
  done
  echo "VM process did not stop after SIGTERM: $pid" >&2
  exit 1
}

case "$COMMAND" in
  prepare)
    prepare
    ;;
  start)
    start_vm
    ;;
  wait)
    wait_for_vm
    ;;
  stage)
    stage_candidate
    ;;
  status)
    status_vm
    ;;
  stop)
    stop_vm
    ;;
  reset)
    recreate_overlay
    echo "Reset fresh workstation overlay in $STATE_DIR"
    ;;
  -h | --help | help)
    usage
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
