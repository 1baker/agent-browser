#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

FAKE_BIN="$WORKDIR/bin"
STATE="$WORKDIR/state"
HELPER_DIR="$WORKDIR/usr/local/libexec/agent-browser"
HELPER_PATH="$HELPER_DIR/agent-browser-privileged-helper"
SUDOERS_PATH="$WORKDIR/etc/sudoers.d/agent-browser"
LOG="$WORKDIR/sudo.log"
OPERATOR_USER="${USER:-}"
GROUP_NAME="ab-workstation-fixture"

if [[ -z "$OPERATOR_USER" || "$OPERATOR_USER" == "root" ]]; then
  echo "This fixture needs a non-root USER environment value." >&2
  exit 2
fi

mkdir -p "$FAKE_BIN" "$STATE" "$(dirname "$SUDOERS_PATH")"
: >"$LOG"

cat >"$FAKE_BIN/getent" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  passwd)
    exec /usr/bin/getent "$@"
    ;;
  group)
    group="${2:-}"
    if [[ -f "$AGENT_BROWSER_FIXTURE_STATE/group-$group" ]]; then
      printf '%s:x:9001:%s\n' "$group" "$AGENT_BROWSER_FIXTURE_OPERATOR_USER"
      exit 0
    fi
    exit 2
    ;;
  *)
    exec /usr/bin/getent "$@"
    ;;
esac
EOF

cat >"$FAKE_BIN/id" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "-u" && "${AGENT_BROWSER_FAKE_ROOT:-0}" == "1" ]]; then
  echo 0
  exit 0
fi
if [[ "${1:-}" == "-nG" ]]; then
  user="${2:-$AGENT_BROWSER_FIXTURE_OPERATOR_USER}"
  groups=("$user")
  for group in "$AGENT_BROWSER_FIXTURE_GROUP" docker; do
    if [[ -f "$AGENT_BROWSER_FIXTURE_STATE/member-$user-$group" ]]; then
      groups+=("$group")
    fi
  done
  printf '%s\n' "${groups[*]}"
  exit 0
fi
exec /usr/bin/id "$@"
EOF

cat >"$FAKE_BIN/visudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "-cf" && -f "${2:-}" ]]
EOF

cat >"$FAKE_BIN/apt-get" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  update)
    exit 0
    ;;
  install)
    if [[ " $* " == *" --simulate "* ]]; then
      echo "0 upgraded, 9 newly installed, 0 to remove"
      exit 0
    fi
    touch "$AGENT_BROWSER_FIXTURE_STATE/deps-installed"
    exit 0
    ;;
esac
exit 2
EOF

cat >"$FAKE_BIN/docker" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "compose" && "${2:-}" == "version" && -f "$AGENT_BROWSER_FIXTURE_STATE/deps-installed" ]]; then
  echo "Docker Compose version fixture"
  exit 0
fi
if [[ "${1:-}" == "info" && -f "$AGENT_BROWSER_FIXTURE_STATE/deps-installed" ]]; then
  exit 0
fi
exit 1
EOF

for command_name in xrdp openbox-session xhost flock; do
  cat >"$FAKE_BIN/$command_name" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
done

cat >"$FAKE_BIN/ss" <<'EOF'
#!/usr/bin/env bash
echo "LISTEN 0 128 0.0.0.0:3389 0.0.0.0:*"
EOF

cat >"$FAKE_BIN/sudo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf 'SUDO' >>"$AGENT_BROWSER_FIXTURE_LOG"
for arg in "$@"; do
  printf ' %q' "$arg" >>"$AGENT_BROWSER_FIXTURE_LOG"
done
printf '\n' >>"$AGENT_BROWSER_FIXTURE_LOG"

if [[ "${1:-}" == "-v" ]]; then
  exit 0
fi
if [[ "${1:-}" == "-n" ]]; then
  shift
fi
cmd="${1:-}"
shift || true
case "$cmd" in
  install)
    args=()
    while [[ $# -gt 0 ]]; do
      case "$1" in
        -o|-g)
          shift 2
          ;;
        *)
          args+=("$1")
          shift
          ;;
      esac
    done
    exec /usr/bin/install "${args[@]}"
    ;;
  groupadd)
    touch "$AGENT_BROWSER_FIXTURE_STATE/group-${*: -1}"
    ;;
  usermod)
    [[ "${1:-}" == "-aG" ]]
    touch "$AGENT_BROWSER_FIXTURE_STATE/member-${3:-}-${2:-}"
    ;;
  visudo)
    exec visudo "$@"
    ;;
  apt-get)
    exec apt-get "$@"
    ;;
  env)
    exec env "$@"
    ;;
  systemctl)
    exit 0
    ;;
  loginctl)
    exit 0
    ;;
  test)
    exit 0
    ;;
  *)
    AGENT_BROWSER_FAKE_ROOT=1 exec "$cmd" "$@"
    ;;
esac
EOF

chmod +x "$FAKE_BIN"/*

run_installer() {
  PATH="$FAKE_BIN:$PATH" \
    AGENT_BROWSER_FIXTURE_LOG="$LOG" \
    AGENT_BROWSER_FIXTURE_STATE="$STATE" \
    AGENT_BROWSER_FIXTURE_GROUP="$GROUP_NAME" \
    AGENT_BROWSER_FIXTURE_OPERATOR_USER="$OPERATOR_USER" \
    AGENT_BROWSER_PRIVILEGED_GROUP="$GROUP_NAME" \
    AGENT_BROWSER_PRIVILEGED_USER="$OPERATOR_USER" \
    AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE="$ROOT/scripts/libexec/agent-browser-privileged-helper" \
    AGENT_BROWSER_PRIVILEGED_HELPER_DIR="$HELPER_DIR" \
    AGENT_BROWSER_PRIVILEGED_HELPER="$HELPER_PATH" \
    AGENT_BROWSER_PRIVILEGED_SUDOERS="$SUDOERS_PATH" \
    bash "$ROOT/scripts/install-agent-browser-privileges.sh" \
      --apply \
      --with-workstation-deps
}

run_installer >"$WORKDIR/first.out"

if [[ "$(grep -c '^SUDO -v$' "$LOG" || true)" != "1" ]]; then
  echo "Expected exactly one sudo authorization on first apply." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$(grep -c '^SUDO -n apt-get update$' "$LOG" || true)" != "1" ]]; then
  echo "Expected one fail-closed apt update." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$(grep -c '^SUDO -n apt-get install --simulate ' "$LOG" || true)" != "1" ]]; then
  echo "Expected one dependency simulation." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$(grep -c '^SUDO -n env DEBIAN_FRONTEND=noninteractive apt-get install ' "$LOG" || true)" != "1" ]]; then
  echo "Expected one dependency install." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ ! -f "$STATE/member-$OPERATOR_USER-docker" || ! -f "$STATE/deps-installed" ]]; then
  echo "Expected Docker membership and installed dependency state." >&2
  exit 1
fi

first_command_count="$(wc -l <"$LOG" | tr -d ' ')"
run_installer >"$WORKDIR/second.out"
second_command_count="$(wc -l <"$LOG" | tr -d ' ')"

if [[ "$(grep -c '^SUDO -v$' "$LOG" || true)" != "1" ]]; then
  echo "Idempotent rerun added another sudo authorization." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$second_command_count" != "$((first_command_count + 1))" ]]; then
  echo "Idempotent rerun should add only the noninteractive helper readiness check." >&2
  cat "$LOG" >&2
  exit 1
fi
if [[ "$(tail -n 1 "$LOG")" != SUDO\ -n\ "$HELPER_PATH"\ check ]]; then
  echo "Unexpected idempotent rerun command." >&2
  tail -n 3 "$LOG" >&2
  exit 1
fi

echo "Workstation host-provision fixture passed"
