#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
source "$ROOT/scripts/lib/wsl-windows-chromium-profile-paths.sh"

if ! CHROME_PATH="$(resolve_wsl_windows_chromium_path)"; then
  echo "Error: could not resolve a Windows Chromium executable from the stealthcdp manifest; set AGENT_BROWSER_WINDOWS_CHROMIUM_PATH" >&2
  exit 1
fi
if ! PROFILE_ROOT="$(resolve_wsl_windows_profile_root "$CHROME_PATH")"; then
  echo "Error: could not resolve a Windows temporary profile root; set AGENT_BROWSER_WINDOWS_PROFILE_SMOKE_ROOT" >&2
  exit 1
fi
MODE="${AGENT_BROWSER_WINDOWS_PROFILE_SMOKE_MODE:-headed}"
KEEP_PROFILE="${AGENT_BROWSER_WINDOWS_PROFILE_SMOKE_KEEP_PROFILE:-0}"
SESSION="wsl-windows-profile-$(date +%s%N)"
SOCKET_DIR="$(mktemp -d /tmp/agent-browser-wsl-windows-profile-socket-XXXXXX)"
PROFILE_DIR="$(mktemp -u "$PROFILE_ROOT/agent-browser-wsl-windows-profile-XXXXXX")"
TMP_WORK="$(mktemp -d /tmp/agent-browser-wsl-windows-profile-work-XXXXXX)"
RESULT_JSON="$TMP_WORK/result.json"
STDERR_MATCHES="$TMP_WORK/stderr-matches.txt"

cleanup() {
  local original_status=$?
  local cleanup_failed=0
  local attempt

  trap - EXIT
  AGENT_BROWSER_SOCKET_DIR="$SOCKET_DIR" \
    cargo run --quiet --manifest-path "$ROOT/cli/Cargo.toml" -- \
    --session "$SESSION" close --all --json >/dev/null 2>&1 || true
  if [[ "$KEEP_PROFILE" != "1" ]]; then
    ensure_wsl_windows_chromium_profile_stopped "$PROFILE_DIR" >/dev/null 2>&1 || true
    for attempt in {1..20}; do
      rm -rf "$PROFILE_DIR" 2>/dev/null || true
      [[ ! -e "$PROFILE_DIR" ]] && break
      sleep 0.25
    done
    if [[ -e "$PROFILE_DIR" ]]; then
      echo "Error: could not remove Windows Chromium smoke profile: $PROFILE_DIR" >&2
      cleanup_failed=1
    fi
  fi
  rm -rf "$SOCKET_DIR"
  rm -rf "$TMP_WORK"

  if [[ "$original_status" -eq 0 && "$cleanup_failed" -ne 0 ]]; then
    original_status=1
  fi
  exit "$original_status"
}
trap cleanup EXIT

if [[ ! -x "$CHROME_PATH" ]]; then
  echo "Error: Windows Chromium executable not found or not executable: $CHROME_PATH" >&2
  exit 1
fi
if [[ ! -d "$PROFILE_ROOT" || ! -w "$PROFILE_ROOT" ]]; then
  echo "Error: Windows profile root is missing or not writable: $PROFILE_ROOT" >&2
  exit 1
fi

case "$MODE" in
  headed)
    mode_flag=(--headed)
    ;;
  headless)
    mode_flag=(--headless)
    ;;
  *)
    echo "Error: AGENT_BROWSER_WINDOWS_PROFILE_SMOKE_MODE must be headed or headless" >&2
    exit 1
    ;;
esac

before_latest="$(find "$HOME/.agent-browser/tmp/chrome-launches" -type f -printf '%T@ %p\n' 2>/dev/null | sort -nr | head -n 1 | cut -d' ' -f2- || true)"

TMPDIR="$PROFILE_ROOT" \
AGENT_BROWSER_SOCKET_DIR="$SOCKET_DIR" \
AGENT_BROWSER_EXECUTABLE_PATH="$CHROME_PATH" \
  cargo run --quiet --manifest-path "$ROOT/cli/Cargo.toml" -- \
    --session "$SESSION" \
    --profile "$PROFILE_DIR" \
    open about:blank \
    "${mode_flag[@]}" \
    --timeout 20000 \
    --json >"$RESULT_JSON" || {
      cat "$RESULT_JSON" >&2
      exit 1
    }

RESULT_JSON="$RESULT_JSON" python3 - <<'PY'
import json
import os
from pathlib import Path

payload = json.loads(Path(os.environ['RESULT_JSON']).read_text())
if not payload.get('success'):
    raise SystemExit(f"agent-browser open failed: {payload}")
if payload.get('data', {}).get('url') != 'about:blank':
    raise SystemExit(f"unexpected smoke URL: {payload}")
PY

for required in DevToolsActivePort "Local State"; do
  if [[ ! -f "$PROFILE_DIR/$required" ]]; then
    echo "Error: expected profile file missing: $PROFILE_DIR/$required" >&2
    exit 1
  fi
done

latest="$(find "$HOME/.agent-browser/tmp/chrome-launches" -type f -printf '%T@ %p\n' 2>/dev/null | sort -nr | head -n 1 | cut -d' ' -f2- || true)"
if [[ -z "$latest" || "$latest" == "$before_latest" ]]; then
  echo "Error: no new Chrome stderr log was captured" >&2
  exit 1
fi

if grep -E "(/mnt/[a-zA-Z]/|failed.*write|Failed.*write|cannot.*write|Cannot.*write)" "$latest" >"$STDERR_MATCHES"; then
  echo "Error: Chrome stderr contains path/write failure evidence:" >&2
  cat "$STDERR_MATCHES" >&2
  exit 1
fi

echo "WSL Windows Chromium profile smoke passed"
echo "mode=$MODE"
echo "profile=$PROFILE_DIR"
echo "stderrLog=$latest"
