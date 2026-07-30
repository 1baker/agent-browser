#!/usr/bin/env bash
set -euo pipefail

readonly GUACAMOLE_IMAGE='guacamole/guacamole:1.5.5@sha256:0f62f6d17ab379e46aa66874b2ff564dab856a6ef5e754a69cbb34c32d3e588a'
readonly EXPECTED_SHA256='b62286f085c2dacb65ac3910473f5d9e986e22ced87325aecf8739535804b657'

if [[ "$#" -gt 1 ]]; then
  echo "Usage: $0 [output-path]" >&2
  exit 2
fi

output_path="${1:-001-initdb.sql}"
output_dir="$(dirname "$output_path")"
mkdir -p "$output_dir"

temporary_path="$(mktemp "$output_dir/.001-initdb.sql.XXXXXX")"
cleanup() {
  rm -f "$temporary_path"
}
trap cleanup EXIT

docker run --rm "$GUACAMOLE_IMAGE" \
  /opt/guacamole/bin/initdb.sh --postgresql >"$temporary_path"
sed -i 's/[[:space:]]*$//' "$temporary_path"

actual_sha256="$(sha256sum "$temporary_path" | awk '{print $1}')"
if [[ "$actual_sha256" != "$EXPECTED_SHA256" ]]; then
  echo "Generated Guacamole schema hash mismatch." >&2
  echo "Expected: $EXPECTED_SHA256" >&2
  echo "Actual:   $actual_sha256" >&2
  exit 1
fi

chmod 0644 "$temporary_path"
mv "$temporary_path" "$output_path"
trap - EXIT
echo "$EXPECTED_SHA256  $output_path"
