#!/usr/bin/env bash

manifest_executable_path() {
  local manifest_path="$1"

  MANIFEST_PATH="$manifest_path" python3 - <<'PY'
import json
import os
from pathlib import Path

manifest_path = Path(os.environ["MANIFEST_PATH"])
try:
    payload = json.loads(manifest_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError):
    raise SystemExit(1)

relative_path = payload.get("executable", {}).get("relativePath")
if not isinstance(relative_path, str) or not relative_path.strip():
    raise SystemExit(1)

print(manifest_path.parent / relative_path)
PY
}

resolve_wsl_windows_chromium_path() {
  local users_root="${1:-/mnt/c/Users}"
  local candidate
  local manifest_path
  local user_name

  if [[ -n "${AGENT_BROWSER_WINDOWS_CHROMIUM_PATH:-}" ]]; then
    printf '%s\n' "$AGENT_BROWSER_WINDOWS_CHROMIUM_PATH"
    return 0
  fi

  if [[ -n "${AGENT_BROWSER_STEALTHCDP_CHROMIUM_MANIFEST_PATH:-}" ]]; then
    manifest_path="$AGENT_BROWSER_STEALTHCDP_CHROMIUM_MANIFEST_PATH"
    if [[ -f "$manifest_path" ]]; then
      manifest_executable_path "$manifest_path"
      return
    fi
  fi

  if [[ -n "${AGENT_BROWSER_STEALTHCDP_CHROMIUM_INSTALL_ROOT:-}" ]]; then
    manifest_path="$AGENT_BROWSER_STEALTHCDP_CHROMIUM_INSTALL_ROOT/current/manifest.json"
    if [[ -f "$manifest_path" ]]; then
      manifest_executable_path "$manifest_path"
      return
    fi
  fi

  for candidate in "$users_root"/*; do
    [[ -d "$candidate" ]] || continue
    user_name="${candidate##*/}"
    case "$user_name" in
      "All Users"|"Default"|"Default User"|"Public")
        continue
        ;;
    esac
    manifest_path="$candidate/AppData/Local/chromium-stealthcdp/current/manifest.json"
    if [[ -f "$manifest_path" ]]; then
      manifest_executable_path "$manifest_path"
      return
    fi
  done

  return 1
}

resolve_wsl_windows_profile_root() {
  local chrome_path="$1"
  local users_root="${2:-/mnt/c/Users}"
  local candidate
  local user_name

  if [[ -n "${AGENT_BROWSER_WINDOWS_PROFILE_SMOKE_ROOT:-}" ]]; then
    printf '%s\n' "$AGENT_BROWSER_WINDOWS_PROFILE_SMOKE_ROOT"
    return 0
  fi

  case "$chrome_path" in
    */AppData/Local/*)
      printf '%s\n' "${chrome_path%%/AppData/Local/*}/AppData/Local/Temp"
      return 0
      ;;
  esac

  for candidate in "$users_root"/*; do
    [[ -d "$candidate" ]] || continue
    user_name="${candidate##*/}"
    case "$user_name" in
      "All Users"|"Default"|"Default User"|"Public")
        continue
        ;;
    esac
    if [[ -d "$candidate/AppData/Local/Temp" ]]; then
      printf '%s\n' "$candidate/AppData/Local/Temp"
      return 0
    fi
  done

  return 1
}

is_wsl_windows_smoke_profile_path() {
  local profile_dir="$1"

  [[ "$profile_dir" =~ ^/mnt/[[:alpha:]]/.+/agent-browser-wsl-windows-profile-[[:alnum:]]+$ ]]
}

ensure_wsl_windows_chromium_profile_stopped() {
  local profile_dir="$1"
  local windows_profile

  is_wsl_windows_smoke_profile_path "$profile_dir" || return 2
  command -v powershell.exe >/dev/null 2>&1 || return 2
  command -v wslpath >/dev/null 2>&1 || return 2
  windows_profile="$(wslpath -w "$profile_dir")"

  printf '%s' "$windows_profile" | powershell.exe -NoProfile -NonInteractive -Command '
$profile = [Console]::In.ReadToEnd()
if (
  [string]::IsNullOrWhiteSpace($profile) -or
  $profile -notmatch "\\agent-browser-wsl-windows-profile-[A-Za-z0-9]+$"
) {
  exit 2
}

function Get-ProfileChromiumProcesses {
  @(
    Get-CimInstance Win32_Process |
      Where-Object {
        $_.Name -eq "chrome.exe" -and
        $_.CommandLine -and
        $_.CommandLine.Contains($profile)
      }
  )
}

$gracefulDeadline = (Get-Date).AddSeconds(3)
do {
  $remaining = Get-ProfileChromiumProcesses
  if ($remaining.Count -eq 0) {
    exit 0
  }
  Start-Sleep -Milliseconds 100
} while ((Get-Date) -lt $gracefulDeadline)

$remaining |
  ForEach-Object {
    Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
  }

$forcedDeadline = (Get-Date).AddSeconds(5)
do {
  Start-Sleep -Milliseconds 100
  $remaining = Get-ProfileChromiumProcesses
} while ($remaining.Count -gt 0 -and (Get-Date) -lt $forcedDeadline)

if ($remaining.Count -gt 0) {
  exit 3
}
'
}
