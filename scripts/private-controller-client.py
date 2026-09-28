"""Authenticated Linux-local controller binding client; never browser execution.

Only the trusted LitScout adapter calls bind after durable approved-plan claim.
The shared key authenticates that adapter, not browser health or user consent.
There is deliberately no credential, renewal, recovery, or general command API.
"""
import hashlib
import hmac
import importlib.util
import json
import os
from pathlib import Path
import pwd
import socket
import stat
import struct
import time

_SPEC = importlib.util.spec_from_file_location(
    "_controller_storage", Path(__file__).with_name("setup-private-slack.py"))
_STORAGE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_STORAGE)
FAILED = "private_controller_failed_closed"
LIMIT = 16384


def _request(body, root=None):
    """Five-second authenticated exchange, with no retries or diagnostic data."""
    directory = None
    try:
        _STORAGE.harden()
        root = Path(root) if root is not None else (
            Path(pwd.getpwuid(os.geteuid()).pw_dir) / ".agent-browser/private-controller")
        directory = _STORAGE.open_directory(root)
        fd = os.open("authentication.key", os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK,
                     dir_fd=directory)
        with os.fdopen(fd, "rb") as source:
            _STORAGE.check_file(source.fileno())
            key = source.read(33)
            _STORAGE.check_file(source.fileno())
        if len(key) != 32:
            raise ValueError(FAILED)
        metadata = os.stat("controller.sock", dir_fd=directory, follow_symlinks=False)
        if (not stat.S_ISSOCK(metadata.st_mode) or metadata.st_uid != os.geteuid()
                or stat.S_IMODE(metadata.st_mode) != 0o600):
            raise ValueError(FAILED)
        payload = json.dumps(body, separators=(",", ":"), ensure_ascii=True).encode()
        if not 0 < len(payload) <= LIMIT:
            raise ValueError(FAILED)
        deadline = time.monotonic() + 5
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as stream:
            def remaining():
                value = deadline - time.monotonic()
                if value <= 0:
                    raise ValueError(FAILED)
                stream.settimeout(value)

            def receive(size):
                result = bytearray()
                while len(result) < size:
                    remaining()
                    data = stream.recv(size - len(result))
                    if not data:
                        raise ValueError(FAILED)
                    result.extend(data)
                return bytes(result)

            remaining()
            # Linux descriptor-anchored path prevents ancestor replacement from
            # redirecting the key/socket pair after validation.
            stream.connect(f"/proc/self/fd/{directory}/controller.sock")
            peer = struct.unpack("3i", stream.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
            if peer[1] != os.geteuid():
                raise ValueError(FAILED)
            challenge = receive(32)
            tag = hmac.digest(key, b"ABPC1-request\0" + challenge + payload, "sha256")
            remaining()
            stream.sendall(struct.pack("!I", len(payload)) + payload + tag)
            stream.shutdown(socket.SHUT_WR)
            length = struct.unpack("!I", receive(4))[0]
            if not 0 < length <= LIMIT:
                raise ValueError(FAILED)
            response = receive(length)
            tag = receive(32)
            if not hmac.compare_digest(tag, hmac.digest(
                    key, b"ABPC1-response\0" + challenge + response, "sha256")):
                raise ValueError(FAILED)
            remaining()
            if stream.recv(1):
                raise ValueError(FAILED)
            value = _STORAGE.decode_json(response)
            if value != {"success": True, "renewalEnabled": False}:
                raise ValueError(FAILED)
            return {"success": True, "renewalEnabled": False}
    except Exception:
        raise ValueError(FAILED) from None
    finally:
        if directory is not None:
            os.close(directory)


def probe(root=None):
    return _request({"operation": "probe"}, root)


def bind(manifest, digest, retained, root=None):
    """Trusted caller supplies independently broker-resolved retained identity.

    This creates binding metadata only. It cannot claim a plan or obtain browser
    authority; the adapter must have committed LitScout's one-shot claim first.
    """
    try:
        expected = hashlib.sha256(json.dumps(manifest, sort_keys=True, separators=(",", ":"),
                                            ensure_ascii=True).encode()).hexdigest()
        if expected != digest or retained.get("ready") is not True:
            raise ValueError(FAILED)
        auth = manifest["authentication"]
        names = {"profileId": "profile_id", "browserId": "browser_id",
                 "sessionName": "session_name", "targetId": "target_id"}
        if any(retained.get(wire) != auth[field] for wire, field in names.items()):
            raise ValueError(FAILED)
        body = {"operation": "bind", "planId": manifest["request_id"],
                "manifestSha256": digest, "expiresAt": manifest["expires_at"],
                "endpoint": retained["endpoint"]}
        body.update({wire: auth[field] for wire, field in names.items()})
        return _request(body, root)
    except Exception:
        raise ValueError(FAILED) from None


if __name__ == "__main__":
    try:
        print(json.dumps(probe()))
    except Exception:
        print(json.dumps({"success": False, "error": FAILED}))
        raise SystemExit(1) from None
