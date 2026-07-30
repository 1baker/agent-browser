#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

DRY_RUN=0
for arg in "$@"; do
  case "$arg" in
    --)
      ;;
    --dry-run)
      DRY_RUN=1
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      echo "Usage: bash scripts/sync-rdp-guac-route-specific-user-pool.sh [--dry-run]" >&2
      exit 2
      ;;
  esac
done

GUAC_DIR="${AGENT_BROWSER_GUACAMOLE_DIR:-$HOME/.agent-browser/guacamole}"
SECRET_FILE="${AGENT_BROWSER_GUACAMOLE_SECRET_FILE:-$HOME/.agent-browser/secrets/guacamole.env}"
HOSTNAME="${AGENT_BROWSER_RDP_TARGET_HOST:-host.docker.internal}"
PORT="${AGENT_BROWSER_RDP_TARGET_PORT:-3389}"
CONNECTION_A="${AGENT_BROWSER_RDP_ROUTE_A_CONNECTION_NAME:-Agent Browser RDP Route A}"
CONNECTION_B="${AGENT_BROWSER_RDP_ROUTE_B_CONNECTION_NAME:-Agent Browser RDP Route B}"
LEGACY_CONNECTION_A="${AGENT_BROWSER_RDP_ROUTE_A_LEGACY_CONNECTION_NAME:-Agent Browser RDP Existing User Route A}"
LEGACY_CONNECTION_B="${AGENT_BROWSER_RDP_ROUTE_B_LEGACY_CONNECTION_NAME:-Agent Browser RDP Existing User Route B}"

if [[ ! -d "$GUAC_DIR" ]]; then
  echo "Missing Guacamole compose directory: $GUAC_DIR" >&2
  exit 1
fi
if [[ ! -r "$SECRET_FILE" ]]; then
  echo "Missing readable Guacamole secret file: $SECRET_FILE" >&2
  exit 1
fi
if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required" >&2
  exit 1
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required" >&2
  exit 1
fi

compose_env_args=()
if [[ -r "$GUAC_DIR/.env" ]]; then
  compose_env_args+=(--env-file "$GUAC_DIR/.env")
fi
compose_env_args+=(--env-file "$SECRET_FILE")

compose() {
  (
    cd "$GUAC_DIR"
    docker compose "${compose_env_args[@]}" "$@"
  )
}

ensure_guacamole_postgres() {
  bash "$SCRIPT_DIR/ensure-rdp-guac-postgres.sh" --apply
}

read_secret() {
  local key="$1"
  python3 - "$SECRET_FILE" "$key" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
key = sys.argv[2]
for line in path.read_text().splitlines():
    if not line or line.strip().startswith("#") or "=" not in line:
        continue
    name, value = line.split("=", 1)
    if name.strip() != key:
        continue
    value = value.strip()
    if (value.startswith('"') and value.endswith('"')) or (value.startswith("'") and value.endswith("'")):
        value = value[1:-1]
    print(value)
    raise SystemExit(0)
raise SystemExit(1)
PY
}

USER_A="${AGENT_BROWSER_RDP_ROUTE_A_USERNAME:-$(read_secret XRDP_AGENT_BROWSER_ROUTE_A_USERNAME || true)}"
PASS_A="${AGENT_BROWSER_RDP_ROUTE_A_PASSWORD:-$(read_secret XRDP_AGENT_BROWSER_ROUTE_A_PASSWORD || true)}"
USER_B="${AGENT_BROWSER_RDP_ROUTE_B_USERNAME:-$(read_secret XRDP_AGENT_BROWSER_ROUTE_B_USERNAME || true)}"
PASS_B="${AGENT_BROWSER_RDP_ROUTE_B_PASSWORD:-$(read_secret XRDP_AGENT_BROWSER_ROUTE_B_PASSWORD || true)}"

for required in \
  XRDP_AGENT_BROWSER_ROUTE_A_USERNAME \
  XRDP_AGENT_BROWSER_ROUTE_A_PASSWORD \
  XRDP_AGENT_BROWSER_ROUTE_B_USERNAME \
  XRDP_AGENT_BROWSER_ROUTE_B_PASSWORD; do
  case "$required" in
    XRDP_AGENT_BROWSER_ROUTE_A_USERNAME) value="$USER_A" ;;
    XRDP_AGENT_BROWSER_ROUTE_A_PASSWORD) value="$PASS_A" ;;
    XRDP_AGENT_BROWSER_ROUTE_B_USERNAME) value="$USER_B" ;;
    XRDP_AGENT_BROWSER_ROUTE_B_PASSWORD) value="$PASS_B" ;;
  esac
  if [[ -z "$value" ]]; then
    echo "Missing $required in $SECRET_FILE" >&2
    exit 1
  fi
done

if [[ "$USER_A" == "$USER_B" ]]; then
  echo "Route A and B usernames must be distinct." >&2
  exit 1
fi
if ! getent passwd "$USER_A" >/dev/null; then
  echo "Route A Linux user does not exist: $USER_A" >&2
  exit 1
fi
if ! getent passwd "$USER_B" >/dev/null; then
  echo "Route B Linux user does not exist: $USER_B" >&2
  exit 1
fi

if [[ "$DRY_RUN" == "1" ]]; then
  cat <<EOF
agent-browser route-specific Guacamole route-pool sync dry run

Guacamole compose directory: $GUAC_DIR
Secret file: $SECRET_FILE
RDP target: $HOSTNAME:$PORT
Route A user: $USER_A
Route A connection: $CONNECTION_A
Route B user: $USER_B
Route B connection: $CONNECTION_B

No Guacamole records were changed.
This command does not create Linux users, rotate passwords, change XRDP policy,
restart XRDP, or require sudo.
EOF
  exit 0
fi

ensure_guacamole_postgres

SQL="$(python3 - \
  "$CONNECTION_A" "$LEGACY_CONNECTION_A" "$USER_A" "$PASS_A" \
  "$CONNECTION_B" "$LEGACY_CONNECTION_B" "$USER_B" "$PASS_B" \
  "$HOSTNAME" "$PORT" <<'PY'
import sys

(
    canonical_a,
    legacy_a,
    user_a,
    pass_a,
    canonical_b,
    legacy_b,
    user_b,
    pass_b,
    hostname,
    port,
) = sys.argv[1:]

def quote(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"

def route_block(
    label: str,
    canonical: str,
    legacy: str,
    username: str,
    password: str,
    id_variable: str,
) -> str:
    params = {
        "hostname": hostname,
        "port": port,
        "username": username,
        "password": password,
        "security": "any",
        "ignore-cert": "true",
        "resize-method": "display-update",
        "enable-audio-input": "false",
        "enable-drive": "false",
        "enable-theming": "false",
        "enable-wallpaper": "false",
    }
    values = ",\n".join(
        f"    ({id_variable}, {quote(name)}, {quote(value)})"
        for name, value in params.items()
    )
    return f"""
  SELECT count(*) FILTER (WHERE connection_name = {quote(canonical)}),
         count(*) FILTER (WHERE connection_name = {quote(legacy)})
  INTO canonical_count, legacy_count
  FROM guacamole_connection
  WHERE parent_id IS NULL
    AND connection_name IN ({quote(canonical)}, {quote(legacy)});

  IF canonical_count + legacy_count > 1 THEN
    RAISE EXCEPTION 'ambiguous managed route {label}: canonical count %, legacy count %',
      canonical_count, legacy_count;
  END IF;

  IF legacy_count = 1 THEN
    UPDATE guacamole_connection
    SET connection_name = {quote(canonical)},
        protocol = 'rdp',
        max_connections = 4,
        max_connections_per_user = 2
    WHERE parent_id IS NULL
      AND connection_name = {quote(legacy)}
    RETURNING connection_id INTO {id_variable};
  ELSIF canonical_count = 1 THEN
    UPDATE guacamole_connection
    SET protocol = 'rdp',
        max_connections = 4,
        max_connections_per_user = 2
    WHERE parent_id IS NULL
      AND connection_name = {quote(canonical)}
    RETURNING connection_id INTO {id_variable};
  ELSE
    INSERT INTO guacamole_connection (
      connection_name,
      protocol,
      max_connections,
      max_connections_per_user
    )
    VALUES ({quote(canonical)}, 'rdp', 4, 2)
    RETURNING connection_id INTO {id_variable};
  END IF;

  DELETE FROM guacamole_connection_parameter
  WHERE connection_id = {id_variable}
    AND parameter_name = 'color-depth';

  INSERT INTO guacamole_connection_parameter (
    connection_id,
    parameter_name,
    parameter_value
  )
  VALUES
{values}
  ON CONFLICT (connection_id, parameter_name) DO UPDATE
  SET parameter_value = EXCLUDED.parameter_value;

  INSERT INTO guacamole_connection_permission (entity_id, connection_id, permission)
  SELECT entity.entity_id, {id_variable}, 'READ'::guacamole_object_permission_type
  FROM guacamole_entity entity
  WHERE entity.type = 'USER'
  ON CONFLICT DO NOTHING;
""".rstrip()

print(f"""BEGIN;

DO $$
DECLARE
  canonical_count integer;
  legacy_count integer;
  route_a_id integer;
  route_b_id integer;
  final_canonical_count integer;
  final_legacy_count integer;
  distinct_username_count integer;
BEGIN
{route_block("A", canonical_a, legacy_a, user_a, pass_a, "route_a_id")}

{route_block("B", canonical_b, legacy_b, user_b, pass_b, "route_b_id")}

  SELECT count(*)
  INTO final_canonical_count
  FROM guacamole_connection
  WHERE parent_id IS NULL
    AND connection_name IN ({quote(canonical_a)}, {quote(canonical_b)});

  SELECT count(*)
  INTO final_legacy_count
  FROM guacamole_connection
  WHERE parent_id IS NULL
    AND connection_name IN ({quote(legacy_a)}, {quote(legacy_b)});

  SELECT count(DISTINCT parameter_value)
  INTO distinct_username_count
  FROM guacamole_connection_parameter
  WHERE connection_id IN (route_a_id, route_b_id)
    AND parameter_name = 'username';

  IF final_canonical_count <> 2
     OR final_legacy_count <> 0
     OR route_a_id = route_b_id
     OR distinct_username_count <> 2 THEN
    RAISE EXCEPTION
      'route-specific migration postcondition failed: canonical %, legacy %, distinct usernames %',
      final_canonical_count, final_legacy_count, distinct_username_count;
  END IF;
END $$;

COMMIT;""")
PY
)"

printf '%s\n' "$SQL" |
  compose exec -T postgres psql -U guacamole_user -d guacamole_db -v ON_ERROR_STOP=1
compose exec -T postgres psql -U guacamole_user -d guacamole_db \
  -v ON_ERROR_STOP=1 -c "CHECKPOINT;" >/dev/null

echo "Configured two canonical Guacamole RDP routes with distinct route-specific users."
echo "Guacamole Postgres route writes checkpoint completed."
echo "Next: open both routes in Guacamole, then run node scripts/inspect-rdp-route-displays.js."
