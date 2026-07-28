#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "sync:rdp-guac-existing-user-route-pool is a compatibility alias." >&2
echo "The installed XRDP runtime disproved same-user color-depth isolation; using route-specific users." >&2

exec bash "$SCRIPT_DIR/sync-rdp-guac-route-specific-user-pool.sh" "$@"
