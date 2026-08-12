#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
source "$ROOT/scripts/lib/wsl-windows-chromium-profile-paths.sh"

FIXTURE_ROOT="$(mktemp -d /tmp/agent-browser-wsl-windows-paths-XXXXXX)"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

USERS_ROOT="$FIXTURE_ROOT/Users"
BAKER_LOCAL="$USERS_ROOT/Baker/AppData/Local"
DEFAULT_LOCAL="$USERS_ROOT/Default/AppData/Local"
INSTALL_ROOT="$BAKER_LOCAL/chromium-stealthcdp"
CHROME_PATH="$INSTALL_ROOT/current/chrome.exe"
PROFILE_ROOT="$BAKER_LOCAL/Temp"

mkdir -p "$INSTALL_ROOT/current" "$PROFILE_ROOT" "$DEFAULT_LOCAL/chromium-stealthcdp/current"
touch "$CHROME_PATH"
chmod +x "$CHROME_PATH"
printf '%s\n' '{"executable":{"relativePath":"chrome.exe"}}' >"$INSTALL_ROOT/current/manifest.json"
printf '%s\n' '{"executable":{"relativePath":"wrong.exe"}}' >"$DEFAULT_LOCAL/chromium-stealthcdp/current/manifest.json"

resolved="$(
  unset AGENT_BROWSER_WINDOWS_CHROMIUM_PATH
  unset AGENT_BROWSER_STEALTHCDP_CHROMIUM_MANIFEST_PATH
  unset AGENT_BROWSER_STEALTHCDP_CHROMIUM_INSTALL_ROOT
  resolve_wsl_windows_chromium_path "$USERS_ROOT"
)"
[[ "$resolved" == "$CHROME_PATH" ]]

resolved="$(
  AGENT_BROWSER_STEALTHCDP_CHROMIUM_INSTALL_ROOT="$INSTALL_ROOT" \
    resolve_wsl_windows_chromium_path "$FIXTURE_ROOT/missing-users"
)"
[[ "$resolved" == "$CHROME_PATH" ]]

resolved="$(
  AGENT_BROWSER_STEALTHCDP_CHROMIUM_MANIFEST_PATH="$INSTALL_ROOT/current/manifest.json" \
    resolve_wsl_windows_chromium_path "$FIXTURE_ROOT/missing-users"
)"
[[ "$resolved" == "$CHROME_PATH" ]]

resolved="$(
  AGENT_BROWSER_WINDOWS_CHROMIUM_PATH="$FIXTURE_ROOT/explicit/chrome.exe" \
    resolve_wsl_windows_chromium_path "$USERS_ROOT"
)"
[[ "$resolved" == "$FIXTURE_ROOT/explicit/chrome.exe" ]]

resolved="$(
  unset AGENT_BROWSER_WINDOWS_PROFILE_SMOKE_ROOT
  resolve_wsl_windows_profile_root "$CHROME_PATH" "$USERS_ROOT"
)"
[[ "$resolved" == "$PROFILE_ROOT" ]]

resolved="$(
  AGENT_BROWSER_WINDOWS_PROFILE_SMOKE_ROOT="$FIXTURE_ROOT/explicit-temp" \
    resolve_wsl_windows_profile_root "$CHROME_PATH" "$USERS_ROOT"
)"
[[ "$resolved" == "$FIXTURE_ROOT/explicit-temp" ]]

is_wsl_windows_smoke_profile_path \
  "/mnt/c/Users/Baker/AppData/Local/Temp/agent-browser-wsl-windows-profile-aB123"
if is_wsl_windows_smoke_profile_path \
  "/mnt/c/Users/Baker/AppData/Local/Temp/not-an-agent-browser-profile"; then
  echo "unsafe Windows smoke profile path was accepted" >&2
  exit 1
fi
if is_wsl_windows_smoke_profile_path "/tmp/agent-browser-wsl-windows-profile-aB123"; then
  echo "non-Windows smoke profile path was accepted" >&2
  exit 1
fi

echo "WSL Windows Chromium path resolution tests passed"
