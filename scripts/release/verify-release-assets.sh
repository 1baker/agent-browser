#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: $0 <asset-directory> <version> <write-checksums|verify-checksums>" >&2
}

if [ "$#" -ne 3 ]; then
  usage
  exit 2
fi

ASSET_DIR=$1
VERSION=$2
MODE=$3
MIN_SIZE=${RELEASE_MIN_BINARY_SIZE:-1000000}
CHECKSUM_FILE="$ASSET_DIR/SHA256SUMS"

case "$MODE" in
  write-checksums|verify-checksums)
    ;;
  *)
    usage
    exit 2
    ;;
esac

if [ ! -d "$ASSET_DIR" ]; then
  echo "Error: asset directory does not exist: $ASSET_DIR" >&2
  exit 1
fi

if ! [[ "$MIN_SIZE" =~ ^[1-9][0-9]*$ ]]; then
  echo "Error: RELEASE_MIN_BINARY_SIZE must be a positive integer" >&2
  exit 1
fi

EXPECTED_BINARIES=(
  "agent-browser-darwin-arm64"
  "agent-browser-darwin-x64"
  "agent-browser-linux-arm64"
  "agent-browser-linux-musl-arm64"
  "agent-browser-linux-musl-x64"
  "agent-browser-linux-x64"
  "agent-browser-win32-x64.exe"
)

expected_files=$(printf '%s\n' "${EXPECTED_BINARIES[@]}")
if [ "$MODE" = "verify-checksums" ]; then
  expected_files=$(printf '%s\n%s\n' "$expected_files" "SHA256SUMS" | sort)
fi

actual_files=$(
  find "$ASSET_DIR" -mindepth 1 -maxdepth 1 -type f -printf '%f\n' | sort
)

if [ "$actual_files" != "$expected_files" ]; then
  echo "Error: release asset inventory does not match the expected files" >&2
  echo "Expected:" >&2
  printf '%s\n' "$expected_files" >&2
  echo "Actual:" >&2
  printf '%s\n' "$actual_files" >&2
  exit 1
fi

VERSION_MARKER="agent-browser $VERSION"
for binary in "${EXPECTED_BINARIES[@]}"; do
  path="$ASSET_DIR/$binary"
  size=$(stat -c%s "$path")
  if [ "$size" -lt "$MIN_SIZE" ]; then
    echo "Error: $binary is $size bytes; expected at least $MIN_SIZE bytes" >&2
    exit 1
  fi

  if ! LC_ALL=C strings "$path" | grep -Fqx "$VERSION_MARKER"; then
    echo "Error: $binary does not contain version marker '$VERSION_MARKER'" >&2
    exit 1
  fi

  echo "Verified $binary ($size bytes, version $VERSION)"
done

linux_version=$("$ASSET_DIR/agent-browser-linux-x64" --version)
if [ "$linux_version" != "$VERSION_MARKER" ]; then
  echo "Error: Linux x64 artifact reported '$linux_version'; expected '$VERSION_MARKER'" >&2
  exit 1
fi
echo "Executed Linux x64 artifact: $linux_version"

if [ "$MODE" = "write-checksums" ]; then
  (
    cd "$ASSET_DIR"
    sha256sum "${EXPECTED_BINARIES[@]}" > SHA256SUMS
  )
  echo "Wrote SHA256SUMS for ${#EXPECTED_BINARIES[@]} binaries"
else
  manifest_files=$(awk '{print $2}' "$CHECKSUM_FILE" | sed 's/^\*//' | sort)
  if [ "$manifest_files" != "$(printf '%s\n' "${EXPECTED_BINARIES[@]}")" ]; then
    echo "Error: SHA256SUMS does not describe the exact binary inventory" >&2
    exit 1
  fi
  (
    cd "$ASSET_DIR"
    sha256sum --check --strict SHA256SUMS
  )
  echo "Verified SHA256SUMS for ${#EXPECTED_BINARIES[@]} binaries"
fi
