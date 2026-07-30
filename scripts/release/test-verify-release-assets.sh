#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
VERIFY_SCRIPT="$SCRIPT_DIR/verify-release-assets.sh"
EXTRACT_NOTES_SCRIPT="$SCRIPT_DIR/extract-release-notes.js"
FIXTURE_ROOT=$(mktemp -d)
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

VERSION=9.8.7
export RELEASE_MIN_BINARY_SIZE=128

EXPECTED_BINARIES=(
  "agent-browser-darwin-arm64"
  "agent-browser-darwin-x64"
  "agent-browser-linux-arm64"
  "agent-browser-linux-musl-arm64"
  "agent-browser-linux-musl-x64"
  "agent-browser-linux-x64"
  "agent-browser-win32-x64.exe"
)

create_fixture() {
  destination=$1
  mkdir -p "$destination"

  for binary in "${EXPECTED_BINARIES[@]}"; do
    if [ "$binary" = "agent-browser-linux-x64" ]; then
      {
        printf '%s\n' '#!/usr/bin/env bash'
        printf '%s\n' 'if [ "${1:-}" = "--version" ]; then'
        printf '  printf '\''%%s\\n'\'' '\''agent-browser %s'\''\n' "$VERSION"
        printf '%s\n' '  exit 0'
        printf '%s\n' 'fi'
        printf '%s\n' 'exit 1'
        printf 'agent-browser %s\n' "$VERSION"
        printf '%s\n' '# padding padding padding padding padding padding padding padding'
      } > "$destination/$binary"
      chmod +x "$destination/$binary"
    else
      {
        printf 'agent-browser %s\n' "$VERSION"
        printf '%160s\n' "release fixture $binary"
      } > "$destination/$binary"
    fi
  done
}

valid_dir="$FIXTURE_ROOT/valid"
create_fixture "$valid_dir"
"$VERIFY_SCRIPT" "$valid_dir" "$VERSION" write-checksums
"$VERIFY_SCRIPT" "$valid_dir" "$VERSION" verify-checksums

extra_dir="$FIXTURE_ROOT/extra"
create_fixture "$extra_dir"
touch "$extra_dir/unexpected-asset"
if "$VERIFY_SCRIPT" "$extra_dir" "$VERSION" write-checksums >/dev/null 2>&1; then
  echo "Error: verifier accepted an unexpected release asset" >&2
  exit 1
fi

stale_dir="$FIXTURE_ROOT/stale"
create_fixture "$stale_dir"
if "$VERIFY_SCRIPT" "$stale_dir" "9.8.6" write-checksums >/dev/null 2>&1; then
  echo "Error: verifier accepted stale version markers" >&2
  exit 1
fi

printf 'corruption\n' >> "$valid_dir/agent-browser-darwin-arm64"
if "$VERIFY_SCRIPT" "$valid_dir" "$VERSION" verify-checksums >/dev/null 2>&1; then
  echo "Error: verifier accepted a checksum mismatch" >&2
  exit 1
fi

valid_changelog="$FIXTURE_ROOT/CHANGELOG-valid.md"
cat > "$valid_changelog" <<'EOF'
# Changelog

## Unreleased

## 9.8.7

<!-- release:start -->
### Bug Fixes

- Fixed the fixture release.
<!-- release:end -->

## 9.8.6
EOF
node "$EXTRACT_NOTES_SCRIPT" \
  "$valid_changelog" "$VERSION" "$FIXTURE_ROOT/release-notes.md"
grep -q '^### Bug Fixes$' "$FIXTURE_ROOT/release-notes.md"

stale_changelog="$FIXTURE_ROOT/CHANGELOG-stale.md"
sed 's/## 9.8.7/## 9.8.6/' "$valid_changelog" > "$stale_changelog"
if node "$EXTRACT_NOTES_SCRIPT" \
  "$stale_changelog" "$VERSION" "$FIXTURE_ROOT/stale-notes.md" >/dev/null 2>&1; then
  echo "Error: release-note extractor accepted markers under a stale version" >&2
  exit 1
fi

echo "Release asset verifier fixture passed"
