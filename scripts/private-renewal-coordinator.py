"""Trusted in-process renewal adapter, never a public CLI or callback surface.

Reviewed templates must be independently supplied before daemon activation.
The immutable private authority file is not consent derived from incoming wire
data. Python cannot zeroize immutable strings; no private values are logged.
"""
from copy import deepcopy
from datetime import datetime, timezone
import hashlib
import hmac
import importlib.util
import json
import os
from pathlib import Path
import re
import socket
import stat
import struct
import time
from urllib.parse import urlsplit

_SPEC = importlib.util.spec_from_file_location(
    "_renewal_sam", Path(__file__).with_name("private-sam-discovery.py"))
_SAM = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_SAM)
_STORAGE = _SAM._DISCOVERY._SETUP
FAILED = "private_renewal_incomplete_inspect_before_recovery"
PASSWORD_SLOT = "PRIVATE_PASSWORD_SLOT"
CODE_SLOT = "PRIVATE-CODE-SLOT"
_FIELDS = {"schema", "planId", "manifestSha256", "recipeSha256", "endpoint",
           "sessionName", "expiresAt", "operations"}


def _canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True,
                      allow_nan=False).encode("ascii")


def _recipe_digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":"),
                                    ensure_ascii=False, allow_nan=False).encode("utf-8")).hexdigest()


def _current(ctx, plan_id, digest, state):
    from litscout.app.credential_approvals import _validate_current
    from litscout.app.jobs import _session_for
    from litscout.store.db import CredentialApproval
    with _session_for(ctx.db_path, ctx.db_url) as session:
        row = session.get(CredentialApproval, plan_id)
        if row is None or row.state != state:
            raise ValueError(FAILED)
        _validate_current(ctx, row, digest)
        return deepcopy(row.manifest_json)


def _validate(authority, manifest):
    if (type(authority) is not dict or set(authority) != _FIELDS
            or authority["schema"] != "agent-browser.private-coordinator.v1"
            or manifest["action"] != "renew"
            or authority["planId"] != manifest["request_id"]
            or authority["manifestSha256"] != hashlib.sha256(_canonical(manifest)).hexdigest()
            or authority["expiresAt"] != manifest["expires_at"]
            or datetime.fromisoformat(authority["expiresAt"]) <= datetime.now(timezone.utc)):
        raise ValueError(FAILED)
    auth = manifest["authentication"]
    endpoint = urlsplit(authority["endpoint"])
    if (endpoint.scheme not in ("ws", "wss") or not endpoint.hostname
            or endpoint.username is not None or endpoint.password is not None
            or endpoint.fragment or endpoint.query
            or any(c.isspace() or not c.isprintable() for c in authority["endpoint"])
            or len(authority["endpoint"]) > 2048
            or authority["sessionName"] != auth["session_name"]):
        raise ValueError(FAILED)
    operations = authority["operations"]
    if (type(operations) is not list or len(operations) != 4
            or _recipe_digest(operations) != authority["recipeSha256"]):
        raise ValueError(FAILED)
    names = {"profileId": "profile_id", "browserId": "browser_id",
             "sessionName": "session_name", "targetId": "target_id"}
    initial = operations[0]["service_tab_handle"]
    if (any(initial.get(wire) != auth[field] for wire, field in names.items())
            or initial.get("valid") is not True or initial.get("staleReason", False) is not None
            or initial.get("leaseState") != "exclusive"
            or initial.get("leaseHeartbeatExpected") is not True
            or initial.get("leaseId") != auth["session_name"]
            or any(not isinstance(initial.get(k), str) or not initial[k] for k in ("tabId", "leaseId"))):
        raise ValueError(FAILED)
    account = manifest["account_email"].lower()
    kinds = ("login", "backup_code", "renew", "read_key")
    fields = (
        {"kind", "email_selector", "password_selector", "submit_selector", "email", "password"},
        {"kind", "code_selector", "submit_selector", "account_selector", "account_value", "code", "source_scope"},
        {"kind", "submit_selector", "account_selector", "account_value"},
        {"kind", "selector", "account_selector", "account_value"},
    )
    for index, value in enumerate(operations):
        if set(value) != {"schema", "service_tab_handle", "expected_origin", "expected_url",
                         "consent_sha256", "account_scope", "operation"}:
            raise ValueError(FAILED)
        operation = value["operation"]
        origin = "https://secure.login.gov" if index < 2 else "https://sam.gov"
        url = urlsplit(value["expected_url"])
        expected_handle = dict(initial, url=value["expected_url"])
        if (value["schema"] != "agent-browser.private-operation.v1"
                or value["consent_sha256"] != authority["manifestSha256"]
                or value["account_scope"] != "login.gov:" + account
                or value["expected_origin"] != origin
                or url.scheme + "://" + url.netloc != origin or url.fragment
                or any(c.isspace() or not c.isprintable() for c in value["expected_url"])
                or "\\" in value["expected_url"]
                or value["service_tab_handle"] != expected_handle
                or set(operation) != fields[index] or operation["kind"] != kinds[index]
                or any(type(v) is not str for v in operation.values())):
            raise ValueError(FAILED)
        selectors = [v for k, v in operation.items() if k.endswith("selector")]
        if (len(set(selectors)) != len(selectors)
                or any(not s or len(s) > 1024 or s.strip() != s
                       or any(not c.isprintable() for c in s) for s in selectors)):
            raise ValueError(FAILED)
        if index and operation["account_value"] != account:
            raise ValueError(FAILED)
    if (operations[0]["operation"]["email"] != account
            or operations[0]["operation"]["password"] != PASSWORD_SLOT
            or operations[1]["operation"]["code"] != CODE_SLOT):
        raise ValueError(FAILED)
    scope = auth["discovery"]
    source = operations[1]["operation"]["source_scope"]
    prefix = scope["channel_id"] + ":"
    if (not source.startswith(prefix) or not re.fullmatch(r"[0-9]{10}\.[0-9]{6}", source[len(prefix):])
            or not scope["oldest_ts"] <= source[len(prefix):] <= scope["latest_ts"]):
        raise ValueError(FAILED)


def _read(directory, name):
    fd = os.open(name, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK,
                 dir_fd=directory)
    with os.fdopen(fd, "rb") as source:
        _STORAGE.check_file(source.fileno())
        value = source.read(_STORAGE.LIMIT + 1)
        _STORAGE.check_file(source.fileno())
    if len(value) > _STORAGE.LIMIT:
        raise ValueError(FAILED)
    return value


def prepare_authority(ctx, plan_id, digest, reviewed_operations, retained, root):
    """Create once from an independently reviewed recipe and no-launch handle.

    This is trusted local provisioning, not a review assertion accepted over a
    socket. Existing authority is never replaced, including after failure.
    """
    directory = None
    try:
        _STORAGE.harden()
        manifest = _current(ctx, plan_id, digest, "approved")
        operations = deepcopy(reviewed_operations)
        if retained.get("ready") is not True or operations[0]["service_tab_handle"] != retained["serviceTabHandle"]:
            raise ValueError(FAILED)
        authority = {"schema": "agent-browser.private-coordinator.v1", "planId": plan_id,
                     "manifestSha256": digest, "recipeSha256": _recipe_digest(operations),
                     "endpoint": retained["endpoint"], "sessionName": manifest["authentication"]["session_name"],
                     "expiresAt": manifest["expires_at"], "operations": operations}
        _validate(authority, manifest)
        data = _canonical(authority)
        if len(data) > _STORAGE.LIMIT:
            raise ValueError(FAILED)
        directory = _STORAGE.open_directory(Path(root))
        if len(_read(directory, "authentication.key")) != 32:
            raise ValueError(FAILED)
        fd = os.open("execution.json", os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC,
                     0o600, dir_fd=directory)
        with os.fdopen(fd, "wb") as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
        os.fsync(directory)
        return {"state": "private_authority_prepared", "renewal_enabled": False}
    except Exception:
        raise ValueError(FAILED) from None
    finally:
        if directory is not None:
            os.close(directory)


def _receive(stream, size, deadline):
    result = bytearray()
    while len(result) < size:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise ValueError(FAILED)
        stream.settimeout(remaining)
        part = stream.recv(size - len(result))
        if not part:
            raise ValueError(FAILED)
        result.extend(part)
    return bytes(result)


def execute_approved_renewal(ctx, plan_id, digest, root):
    """One durable claim; fixed private connector; installation only after completion.

    Every uncertainty consumes the attempt. No callbacks, retries, browser
    lifecycle operations or runtime activation are exposed to the caller.
    """
    from litscout.app.credential_broker import run_credential_broker
    from litscout.app.credential_delivery import receive_sam_credential_delivery
    directory, retained, installed = None, [], []
    try:
        _STORAGE.harden()
        directory = _STORAGE.open_directory(Path(root))
        authority = _STORAGE.decode_json(_read(directory, "execution.json"))
        key = _read(directory, "authentication.key")
        if len(key) != 32 or authority["planId"] != plan_id or authority["manifestSha256"] != digest:
            raise ValueError(FAILED)
        _validate(authority, _current(ctx, plan_id, digest, "approved"))
        _SAM._resolve(ctx, plan_id, digest, retained)

        def handoff(manifest):
            _validate(authority, manifest)
            if len(retained) != 1:
                raise ValueError(FAILED)
            connection, discovery_scope, result = retained.pop()
            source = result.backup_code_source
            if authority["operations"][1]["operation"]["source_scope"] != source.channel_id + ":" + source.message_ts:
                raise ValueError(FAILED)
            metadata = os.stat("executor.sock", dir_fd=directory, follow_symlinks=False)
            if (not stat.S_ISSOCK(metadata.st_mode) or metadata.st_uid != os.geteuid()
                    or stat.S_IMODE(metadata.st_mode) != 0o600):
                raise ValueError(FAILED)
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as stream:
                stream.settimeout(5)
                stream.connect(f"/proc/self/fd/{directory}/executor.sock")
                peer = struct.unpack("3i", stream.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                if peer[1] != os.geteuid():
                    raise ValueError(FAILED)
                nonce = os.urandom(32)
                context = nonce + plan_id.encode("ascii") + digest.encode("ascii") + authority["recipeSha256"].encode("ascii")
                stream.sendall(nonce)
                proof = _receive(stream, 32, time.monotonic() + 10)
                if not hmac.compare_digest(proof, hmac.digest(key, b"ABPX1-server\0" + context, "sha256")):
                    raise ValueError(FAILED)
                _validate(authority, _current(ctx, plan_id, digest, "consumed"))
                # Readiness comes from the authenticated server's fresh retained
                # broker/page preflight, never from the stored handle alone.
                # The worker independently rechecks again before DOM actions.
                identity = dict(authority["operations"][0]["service_tab_handle"],
                                ready=True, endpoint=authority["endpoint"])
                _SAM._CONTROLLER.bind(manifest, digest, identity, root)
                material = _SAM._DISCOVERY.extract(connection, discovery_scope, result)
                if len(material.backup_codes) != 1 or material.email != manifest["account_email"].lower():
                    raise ValueError(FAILED)
                # Private source revalidation may take time. Expiry or config
                # drift during it must fail before sending any credential.
                _validate(authority, _current(ctx, plan_id, digest, "consumed"))
                operations = deepcopy(authority["operations"])
                operations[0]["operation"]["password"] = material.password
                operations[1]["operation"]["code"] = material.backup_codes[0]
                body = _canonical(operations)
                if not 0 < len(body) <= 262144:
                    raise ValueError(FAILED)
                header = b"ABPH1\0" + struct.pack("!I", len(body))
                stream.settimeout(5)
                stream.sendall(header + body + hmac.digest(key, header + body, "sha256"))
                completion = _receive(stream, 32, time.monotonic() + 120)
                if not hmac.compare_digest(completion, hmac.digest(key, b"ABPX1-complete\0" + context, "sha256")):
                    raise ValueError(FAILED)
                installed.append(receive_sam_credential_delivery(ctx, plan_id, digest, stream,
                    authentication_key=key, expected_endpoint=authority["endpoint"]))

        run_credential_broker(ctx, plan_id, digest, handoff)
        if len(installed) != 1:
            raise ValueError(FAILED)
        return installed[0]
    except Exception:
        raise ValueError(FAILED) from None
    finally:
        retained.clear()
        if directory is not None:
            os.close(directory)
