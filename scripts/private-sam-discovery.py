"""Trusted in-process LitScout/Slack adapter. No CLI or browser execution.

The service supplies its own ServiceContext and approved plan identity. This
module never constructs/approves plans, chooses another DB, or outputs secrets.
Not wired into an installed service until private end-to-end acceptance passes.
"""
import importlib.util
import os
from pathlib import Path
import pwd

_SPEC = importlib.util.spec_from_file_location(
    "_sam_slack_discovery", Path(__file__).with_name("private-slack-discovery.py"))
_DISCOVERY = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_DISCOVERY)


def _connection():
    setup = _DISCOVERY._SETUP
    root = Path(pwd.getpwuid(os.geteuid()).pw_dir) / ".config/agent-browser/private-slack"
    directory = setup.open_directory(root, create=False)
    try:
        fd = os.open(setup.CONFIG, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK,
                     dir_fd=directory)
        with os.fdopen(fd, "rb") as handle:
            setup.check_file(handle.fileno())
            data = handle.read(setup.LIMIT + 1)
            setup.check_file(handle.fileno())
        if len(data) > setup.LIMIT:
            raise ValueError("private_sam_connection_invalid")
        value = setup.decode_json(data)
        setup.validate_connection(value)
        return value
    finally:
        os.close(directory)


def _resolve(ctx, plan_id, digest, retained):
    """Connect durable LitScout admission to the actual private Slack collector.

    The real connection is opened only inside the admitted collector. No secrets
    enter callback results or database receipts. Neither extraction nor browser
    dispatch is enabled here; resolved references require private revalidation.
    """
    try:
        from litscout.app.credential_discovery import run_credential_discovery
        _DISCOVERY._SETUP.harden()

        def collect(manifest):
            scope = manifest["authentication"]["discovery"]
            authority = _DISCOVERY.DiscoveryAuthority(
                team_id=scope["team_id"], channel_id=scope["channel_id"],
                account_email=manifest["account_email"].lower(),
                oldest_ts=scope["oldest_ts"], latest_ts=scope["latest_ts"],
                max_messages=scope["max_messages"], max_pages=scope["max_pages"])
            connection = _connection()
            result = _DISCOVERY.discover(connection, authority)
            if retained is not None:
                retained.append((connection, authority, result))
            return {
                "credential_source": {"channel_id": result.credential_source.channel_id,
                                      "message_ts": result.credential_source.message_ts},
                "backup_code_source": {"channel_id": result.backup_code_source.channel_id,
                                       "message_ts": result.backup_code_source.message_ts},
                "scanned_messages": result.scanned_messages,
                "scanned_pages": result.scanned_pages,
            }

        return run_credential_discovery(ctx, plan_id, digest, collect)
    except Exception:
        raise ValueError("private_sam_discovery_failed") from None


def resolve_approved_sources(ctx, plan_id, digest):
    """Discovery-only public metadata receipt; preserves the original behavior."""
    return _resolve(ctx, plan_id, digest, None)


def execute_approved_sources(ctx, plan_id, digest, consumer):
    """Trusted in-process private ingress, not a public callback/command surface.

    Retains genuine discovery provenance until durable publication and execution
    claim succeed. The consumer receives (detached_manifest, PrivateMaterial),
    never through a JSON/job envelope. No browser or installation is enabled by
    this adapter itself. Any downstream uncertainty leaves approval consumed.
    """
    retained = []
    try:
        from litscout.app.credential_broker import run_credential_broker
        _resolve(ctx, plan_id, digest, retained)

        def handoff(manifest):
            if len(retained) != 1:
                raise ValueError("private_sam_provenance_required")
            connection, authority, result = retained.pop()
            material = _DISCOVERY.extract(connection, authority, result)
            consumer(manifest, material)

        return run_credential_broker(ctx, plan_id, digest, handoff)
    except Exception:
        raise ValueError("private_sam_handoff_failed") from None
    finally:
        retained.clear()
