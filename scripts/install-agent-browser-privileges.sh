#!/usr/bin/env bash
set -euo pipefail

APPLY=0
WITH_WORKSTATION_DEPS=0
GROUP_NAME="${AGENT_BROWSER_PRIVILEGED_GROUP:-agent-browser}"
OPERATOR_USER="${AGENT_BROWSER_PRIVILEGED_USER:-${SUDO_USER:-${USER:-}}}"
HELPER_SOURCE="${AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE:-scripts/libexec/agent-browser-privileged-helper}"
HELPER_DIR="${AGENT_BROWSER_PRIVILEGED_HELPER_DIR:-/usr/local/libexec/agent-browser}"
HELPER_PATH="${AGENT_BROWSER_PRIVILEGED_HELPER:-$HELPER_DIR/agent-browser-privileged-helper}"
EXPECTED_HELPER_SHA256="${AGENT_BROWSER_PRIVILEGED_HELPER_SHA256:-}"
SUDOERS_PATH="${AGENT_BROWSER_PRIVILEGED_SUDOERS:-/etc/sudoers.d/agent-browser}"

usage() {
  cat <<'EOF'
Usage: bash scripts/install-agent-browser-privileges.sh [--dry-run|--apply] [--with-workstation-deps]

Installs the narrow root-owned helper used by agent-browser RDP/Guacamole setup.
The helper is protected by a sudoers rule for the agent-browser group so later
route-user and display-access maintenance can run without repeated prompts.
The optional workstation dependency phase is Ubuntu 24.04 amd64 only and
installs Docker, Compose, XRDP, XorgXRDP, Openbox, and required host tools.
EOF
}

for arg in "$@"; do
  case "$arg" in
    --)
      ;;
    --apply)
      APPLY=1
      ;;
    --dry-run)
      APPLY=0
      ;;
    --with-workstation-deps)
      WITH_WORKSTATION_DEPS=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$OPERATOR_USER" || "$OPERATOR_USER" == "root" ]]; then
  echo "Set AGENT_BROWSER_PRIVILEGED_USER to the non-root user that runs agent-browser." >&2
  exit 2
fi

if [[ ! "$OPERATOR_USER" =~ ^[a-z_][a-z0-9_-]{0,31}$ ]]; then
  echo "Operator user must be a local username." >&2
  exit 2
fi

if ! getent passwd "$OPERATOR_USER" >/dev/null; then
  echo "Operator user does not exist: $OPERATOR_USER" >&2
  exit 2
fi

if [[ ! "$GROUP_NAME" =~ ^[a-z_][a-z0-9_-]{0,31}$ ]]; then
  echo "Privileged group must be a local group name." >&2
  exit 2
fi

if [[ "$HELPER_PATH" != /* ]]; then
  echo "Installed helper path must be absolute." >&2
  exit 2
fi

if [[ ! -f "$HELPER_SOURCE" ]]; then
  echo "Missing helper source: $HELPER_SOURCE" >&2
  exit 1
fi
if [[ -z "$EXPECTED_HELPER_SHA256" ]]; then
  EXPECTED_HELPER_SHA256="$(sha256sum "$HELPER_SOURCE" | awk '{print $1}')"
fi
if [[ ! "$EXPECTED_HELPER_SHA256" =~ ^[a-f0-9]{64}$ ]]; then
  echo "Expected helper SHA-256 must be 64 lowercase hexadecimal characters." >&2
  exit 2
fi
if [[ "$(sha256sum "$HELPER_SOURCE" | awk '{print $1}')" != "$EXPECTED_HELPER_SHA256" ]]; then
  echo "Helper source SHA-256 does not match the embedded installer manifest." >&2
  exit 1
fi

expected_sudoers_content() {
  cat <<EOF
# agent-browser narrow privileged helper
%$GROUP_NAME ALL=(root) NOPASSWD: $HELPER_PATH
EOF
}

workstation_packages() {
  printf '%s\n' \
    docker.io \
    docker-compose-v2 \
    xrdp \
    xorgxrdp \
    openbox \
    x11-utils \
    x11-xserver-utils \
    imagemagick \
    tesseract-ocr \
    curl \
    python3 \
    nodejs \
    util-linux \
    ca-certificates \
    ssl-cert \
    freerdp2-x11 \
    xvfb \
    xauth \
    dbus-x11 \
    iproute2 \
    libxcb-shm0 \
    libx11-xcb1 \
    libx11-6 \
    libxcb1 \
    libxext6 \
    libxrandr2 \
    libxcomposite1 \
    libxcursor1 \
    libxdamage1 \
    libxfixes3 \
    libxi6 \
    libgtk-3-0t64 \
    libpangocairo-1.0-0 \
    libpango-1.0-0 \
    libatk1.0-0t64 \
    libcairo-gobject2 \
    libcairo2 \
    libgdk-pixbuf-2.0-0 \
    libxrender1 \
    libasound2t64 \
    libfreetype6 \
    libfontconfig1 \
    libdbus-1-3 \
    libnss3 \
    libnspr4 \
    libatk-bridge2.0-0t64 \
    libdrm2 \
    libxkbcommon0 \
    libatspi2.0-0t64 \
    libcups2t64 \
    libxshmfence1 \
    libgbm1 \
    fonts-noto-color-emoji \
    fonts-noto-cjk \
    fonts-freefont-ttf
}

workstation_deps_ready() {
  [[ "$(uname -m)" == "x86_64" ]] || return 1
  command -v apt-get >/dev/null 2>&1 || return 1
  command -v docker >/dev/null 2>&1 || return 1
  docker compose version >/dev/null 2>&1 || return 1
  command -v xrdp >/dev/null 2>&1 || [[ -x /usr/sbin/xrdp ]] || return 1
  command -v openbox-session >/dev/null 2>&1 || return 1
  command -v xhost >/dev/null 2>&1 || return 1
  command -v flock >/dev/null 2>&1 || return 1
  getent group docker >/dev/null 2>&1 || return 1
  id -nG "$OPERATOR_USER" 2>/dev/null | tr ' ' '\n' | grep -Fx docker >/dev/null || return 1
}

current_install_ready() {
  getent group "$GROUP_NAME" >/dev/null 2>&1 || return 1
  id -nG "$OPERATOR_USER" 2>/dev/null | tr ' ' '\n' | grep -Fx "$GROUP_NAME" >/dev/null || return 1
  [[ -x "$HELPER_PATH" ]] || return 1
  cmp -s "$HELPER_SOURCE" "$HELPER_PATH" || return 1
  sudo -n "$HELPER_PATH" verify-install \
    --group "$GROUP_NAME" \
    --sudoers "$SUDOERS_PATH" \
    --sha256 "$EXPECTED_HELPER_SHA256" >/dev/null 2>&1 || return 1
  if [[ "$WITH_WORKSTATION_DEPS" == "1" ]]; then
    workstation_deps_ready || return 1
  fi
}

print_install_status() {
  echo "Current readiness:"
  if getent group "$GROUP_NAME" >/dev/null 2>&1; then
    echo "  group: ready"
  else
    echo "  group: missing"
  fi

  if id -nG "$OPERATOR_USER" 2>/dev/null | tr ' ' '\n' | grep -Fx "$GROUP_NAME" >/dev/null; then
    echo "  membership: ready"
  else
    echo "  membership: $OPERATOR_USER is not in $GROUP_NAME"
  fi

  if [[ -x "$HELPER_PATH" ]]; then
    if cmp -s "$HELPER_SOURCE" "$HELPER_PATH"; then
      echo "  helper: ready"
    else
      echo "  helper: installed helper differs from bundled helper and must be refreshed"
    fi
  elif [[ -e "$HELPER_PATH" ]]; then
    echo "  helper: present but not executable"
  else
    echo "  helper: missing"
  fi

  if [[ -r "$SUDOERS_PATH" ]] && expected_sudoers_content | diff -q - "$SUDOERS_PATH" >/dev/null 2>&1; then
    echo "  sudoers: ready"
  elif [[ -e "$SUDOERS_PATH" ]]; then
    echo "  sudoers: protected; helper verification required"
  else
    echo "  sudoers: protected or missing; helper verification required"
  fi

  if [[ -x "$HELPER_PATH" ]] && sudo -n "$HELPER_PATH" verify-install \
    --group "$GROUP_NAME" \
    --sudoers "$SUDOERS_PATH" \
    --sha256 "$EXPECTED_HELPER_SHA256" >/dev/null 2>&1; then
    echo "  sudo helper install verification: ready"
  else
    echo "  sudo helper install verification: not ready"
  fi

  if [[ "$WITH_WORKSTATION_DEPS" == "1" ]]; then
    if workstation_deps_ready; then
      echo "  workstation dependencies: ready"
    else
      echo "  workstation dependencies: missing or operator docker membership is stale"
    fi
  fi
}

if [[ "$WITH_WORKSTATION_DEPS" == "1" ]]; then
  if [[ "$(uname -m)" != "x86_64" ]]; then
    echo "Workstation installation currently supports Linux x86_64 only." >&2
    exit 1
  fi
  if [[ ! -r /etc/os-release ]]; then
    echo "Unable to verify Ubuntu release from /etc/os-release." >&2
    exit 1
  fi
  # shellcheck disable=SC1091
  . /etc/os-release
  if [[ "${ID:-}" != "ubuntu" || "${VERSION_ID:-}" != "24.04" ]]; then
    echo "Workstation installation currently supports Ubuntu 24.04 only." >&2
    exit 1
  fi
  if ! command -v apt-get >/dev/null 2>&1 || ! command -v apt-cache >/dev/null 2>&1; then
    echo "apt-get and apt-cache are required for workstation dependency installation." >&2
    exit 1
  fi
fi

if [[ "$APPLY" != "1" ]]; then
  cat <<EOF
agent-browser privileged helper install dry run

Group: $GROUP_NAME
Operator user: $OPERATOR_USER
Helper source: $HELPER_SOURCE
Installed helper: $HELPER_PATH
Sudoers file: $SUDOERS_PATH

Would run with one privileged authorization:
  sudo install -d -o root -g root -m 0755 $HELPER_DIR
  sudo install -o root -g root -m 0755 $HELPER_SOURCE $HELPER_PATH
  sudo groupadd --force $GROUP_NAME
  sudo usermod -aG $GROUP_NAME $OPERATOR_USER
  sudo install validated sudoers policy at $SUDOERS_PATH
EOF
  if [[ "$WITH_WORKSTATION_DEPS" == "1" ]]; then
    echo "  sudo apt-get update"
    echo "  sudo apt-get install after a no-removal simulation:"
    workstation_packages | sed 's/^/    /'
    echo "  sudo usermod -aG docker $OPERATOR_USER"
    echo "  sudo systemctl enable --now docker xrdp"
  fi
  cat <<EOF

After applying, log out and back in or reboot so group membership is active.
EOF
  print_install_status
  exit 0
fi

if current_install_ready; then
  echo "agent-browser privileged helper is already ready."
  echo "No privileged changes were needed."
  exit 0
fi

if ! command -v visudo >/dev/null 2>&1; then
  echo "visudo is required to validate the sudoers policy." >&2
  exit 1
fi

print_install_status
sudo -v

if [[ "$WITH_WORKSTATION_DEPS" == "1" ]]; then
  mapfile -t WORKSTATION_PACKAGES < <(workstation_packages)
  sudo -n apt-get update
  for package_name in "${WORKSTATION_PACKAGES[@]}"; do
    if ! apt-cache show "$package_name" >/dev/null 2>&1; then
      echo "Required workstation package has no apt candidate after updating indexes: $package_name" >&2
      exit 1
    fi
  done
  SIMULATION_OUTPUT="$(sudo -n apt-get install --simulate "${WORKSTATION_PACKAGES[@]}" 2>&1)" || {
    printf '%s\n' "$SIMULATION_OUTPUT" >&2
    echo "Workstation dependency simulation failed." >&2
    exit 1
  }
  if grep -q '^Remv ' <<<"$SIMULATION_OUTPUT"; then
    printf '%s\n' "$SIMULATION_OUTPUT" >&2
    echo "Workstation dependency installation would remove packages." >&2
    exit 1
  fi
  sudo -n env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends --no-remove \
    "${WORKSTATION_PACKAGES[@]}"
  sudo -n groupadd --force docker
  sudo -n usermod -aG docker "$OPERATOR_USER"
  sudo -n loginctl enable-linger "$OPERATOR_USER"
  sudo -n systemctl enable --now docker xrdp xrdp-sesman
  sudo -n docker info >/dev/null
  sudo -n docker compose version >/dev/null
  sudo -n systemctl is-active --quiet docker xrdp xrdp-sesman
  sudo -n ss -ltn | grep -Eq '(^|[[:space:]])[^[:space:]]*:3389[[:space:]]'
fi

SUDOERS_TMP="$(mktemp)"
trap 'rm -f "$SUDOERS_TMP"' EXIT
expected_sudoers_content >"$SUDOERS_TMP"

sudo -n visudo -cf "$SUDOERS_TMP" >/dev/null
sudo -n install -d -o root -g root -m 0755 "$HELPER_DIR"
sudo -n install -o root -g root -m 0755 "$HELPER_SOURCE" "$HELPER_PATH"
sudo -n groupadd --force "$GROUP_NAME"
sudo -n usermod -aG "$GROUP_NAME" "$OPERATOR_USER"
sudo -n install -o root -g root -m 0440 "$SUDOERS_TMP" "$SUDOERS_PATH"
sudo -n visudo -cf "$SUDOERS_PATH" >/dev/null
sudo -n test "$(stat -c '%U:%G:%a' "$HELPER_PATH")" = "root:root:755"
INSTALLED_HELPER_SHA256="$(sudo -n sha256sum "$HELPER_PATH" | awk '{print $1}')"
if [[ "$INSTALLED_HELPER_SHA256" != "$EXPECTED_HELPER_SHA256" ]]; then
  echo "Installed helper SHA-256 does not match the embedded installer manifest." >&2
  exit 1
fi

echo "Installed agent-browser privileged helper at $HELPER_PATH."
echo "Added $OPERATOR_USER to group $GROUP_NAME."
echo "Installed sudoers policy at $SUDOERS_PATH."
if [[ "$WITH_WORKSTATION_DEPS" == "1" ]]; then
  echo "Added $OPERATOR_USER to group docker."
fi
echo "Log out and back in or reboot so group membership is active."
