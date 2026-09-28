#!/usr/bin/env python3
"""Hidden local SAM/Login.gov importer backed only by Windows Credential Manager."""

from dataclasses import dataclass
import hashlib
import hmac
import json
import os
from pathlib import Path
import re
import resource
import selectors
import subprocess
import sys
import termios
import time
import weakref


STORE_ID = "agent_browser_sam_login_gov_v1"
TARGET = "AgentBrowser/SAM/LoginGov/v1"
SCHEMA = "agent-browser.private-sam-local-credential.v1"
POWERSHELL = Path("/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe")
HELPER = Path(__file__).with_name("private-windows-credential-store.ps1")
MAX_OUTPUT = 8192
_EMAIL = re.compile(r"^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9-]+(?:\.[a-z0-9-]+)+$")
_FILETIME = re.compile(r"^[1-9][0-9]{10,19}$")
_BACKUP_CODE = re.compile(r"^(?:[0-9A-Fa-f]{12}|[0-9A-Fa-f]{4}(?:-[0-9A-Fa-f]{4}){2})$")


class LocalCredentialError(Exception):
    """Fixed diagnostics only; never include a credential or subprocess body."""


@dataclass(frozen=True, repr=False)
class LocalCredentialReference:
    store_id: str
    last_written_filetime: str


@dataclass(frozen=True, repr=False)
class LocalDiscoveryAuthority:
    store_id: str
    account_email: str

    def validate(self) -> None:
        if (self.store_id != STORE_ID or not isinstance(self.account_email, str)
                or len(self.account_email) > 254 or _EMAIL.fullmatch(self.account_email) is None):
            raise LocalCredentialError("private_sam_local_invalid_authority")


@dataclass(frozen=True, repr=False, eq=False)
class LocalDiscoveryResult:
    credential_source: LocalCredentialReference
    backup_code_source: LocalCredentialReference
    scanned_messages: int = 0
    scanned_pages: int = 0


class PrivateMaterial:
    __slots__ = ("_email", "_password", "_backup_codes", "__weakref__")

    def __init__(self, email: str, password: str, backup_codes: tuple[str, ...]):
        self._email = email
        self._password = password
        self._backup_codes = backup_codes

    @property
    def email(self) -> str:
        return self._email

    @property
    def password(self) -> str:
        return self._password

    @property
    def backup_codes(self) -> tuple[str, ...]:
        return self._backup_codes

    def __reduce_ex__(self, protocol):
        raise TypeError("private_sam_local_material_not_serializable")


_PROVENANCE = weakref.WeakKeyDictionary()


def harden() -> None:
    if sys.platform != "linux" or not POWERSHELL.is_file() or not HELPER.is_file():
        raise LocalCredentialError("private_sam_local_store_unavailable")
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise LocalCredentialError("private_sam_local_invalid_payload")
        result[key] = value
    return result


def _run(operation: str, value: bytes | bytearray = b"", *, target: str = TARGET,
         runner=subprocess.run) -> bytes:
    harden()
    if operation not in {"store", "read", "metadata", "delete"}:
        raise LocalCredentialError("private_sam_local_invalid_operation")
    command = [
        str(POWERSHELL), "-NoLogo", "-NoProfile", "-NonInteractive",
        "-ExecutionPolicy", "Bypass", "-File", str(HELPER),
        "-Operation", operation, "-Target", target,
    ]
    try:
        if runner is subprocess.run:
            result = _bounded_process(command, value)
        else:
            result = runner(command, input=value, stdout=subprocess.PIPE,
                stderr=subprocess.PIPE, timeout=20, check=False)
    except (OSError, subprocess.SubprocessError):
        raise LocalCredentialError("private_sam_local_store_unavailable") from None
    if result.returncode != 0 or len(result.stdout) > MAX_OUTPUT:
        raise LocalCredentialError("private_sam_local_store_failed")
    return bytes(result.stdout)


def _bounded_process(command: list[str], value: bytes | bytearray):
    process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, bufsize=0)
    output, error = bytearray(), bytearray()
    selector = selectors.DefaultSelector()
    try:
        selector.register(process.stdout, selectors.EVENT_READ, (output, MAX_OUTPUT))
        selector.register(process.stderr, selectors.EVENT_READ, (error, 4096))
        offset = 0
        if value:
            selector.register(process.stdin, selectors.EVENT_WRITE, None)
        else:
            process.stdin.close()
        deadline = time.monotonic() + 20
        while selector.get_map():
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise subprocess.TimeoutExpired(command, 20)
            for key, _ in selector.select(remaining):
                if key.fileobj is process.stdin:
                    try:
                        written = os.write(process.stdin.fileno(), value[offset:offset + 4096])
                    except BrokenPipeError:
                        written = 0
                    offset += written
                    if written == 0 or offset == len(value):
                        selector.unregister(process.stdin)
                        process.stdin.close()
                    continue
                buffer, limit = key.data
                chunk = os.read(key.fileobj.fileno(), min(4096, limit + 1 - len(buffer)))
                if not chunk:
                    selector.unregister(key.fileobj)
                    key.fileobj.close()
                    continue
                buffer.extend(chunk)
                if len(buffer) > limit:
                    raise LocalCredentialError("private_sam_local_store_failed")
        timeout = deadline - time.monotonic()
        if timeout <= 0:
            raise subprocess.TimeoutExpired(command, 20)
        stdout = bytes(output)
        returncode = process.wait(timeout=timeout)
        output.clear()
        error.clear()
        return subprocess.CompletedProcess(command, returncode, stdout, b"")
    except BaseException:
        output.clear()
        error.clear()
        process.kill()
        process.wait()
        raise
    finally:
        selector.close()
        for stream in (process.stdin, process.stdout, process.stderr):
            if stream is not None and not stream.closed:
                stream.close()


def _metadata(*, target: str = TARGET, runner=subprocess.run) -> LocalCredentialReference | None:
    try:
        value = json.loads(_run("metadata", target=target, runner=runner),
                           object_pairs_hook=_unique_object)
    except (ValueError, UnicodeError, json.JSONDecodeError):
        raise LocalCredentialError("private_sam_local_invalid_metadata") from None
    if value == {"present": False}:
        return None
    if (not isinstance(value, dict) or set(value) != {"present", "last_written_filetime"}
            or value["present"] is not True
            or not isinstance(value["last_written_filetime"], str)
            or _FILETIME.fullmatch(value["last_written_filetime"]) is None):
        raise LocalCredentialError("private_sam_local_invalid_metadata")
    return LocalCredentialReference(STORE_ID, value["last_written_filetime"])


def _code(value: str) -> bool:
    return _BACKUP_CODE.fullmatch(value) is not None


def _material(value) -> PrivateMaterial:
    if not isinstance(value, dict) or set(value) != {"schema", "email", "password", "backup_codes"}:
        raise LocalCredentialError("private_sam_local_invalid_payload")
    email, password, codes = value["email"], value["password"], value["backup_codes"]
    if (value["schema"] != SCHEMA or not isinstance(email, str)
            or len(email) > 254 or _EMAIL.fullmatch(email) is None
            or not isinstance(password, str) or not 1 <= len(password) <= 4096
            or password != password.strip()
            or any(ord(char) < 32 or ord(char) == 127 for char in password)
            or not isinstance(codes, list) or len(codes) != 1
            or any(not isinstance(code, str) or not _code(code) for code in codes)):
        raise LocalCredentialError("private_sam_local_invalid_payload")
    return PrivateMaterial(email, password, tuple(codes))


def store(material: PrivateMaterial, *, target: str = TARGET, runner=subprocess.run) -> LocalCredentialReference:
    validated = _material({"schema": SCHEMA, "email": material.email,
                           "password": material.password,
                           "backup_codes": list(material.backup_codes)})
    payload = bytearray(json.dumps({"schema": SCHEMA, "email": validated.email,
                                    "password": validated.password,
                                    "backup_codes": list(validated.backup_codes)},
                                   sort_keys=True, separators=(",", ":"), ensure_ascii=False,
                                   allow_nan=False).encode("utf-8"))
    if len(payload) > 2560:
        payload.clear()
        raise LocalCredentialError("private_sam_local_invalid_payload")
    try:
        if _run("store", payload, target=target, runner=runner) != b"stored":
            raise LocalCredentialError("private_sam_local_store_failed")
        reference = _metadata(target=target, runner=runner)
        if reference is None:
            raise LocalCredentialError("private_sam_local_store_failed")
        observed = _run("read", target=target, runner=runner)
        if _metadata(target=target, runner=runner) != reference:
            raise LocalCredentialError("private_sam_local_store_failed")
        if not hmac.compare_digest(
                hashlib.sha256(observed).digest(), hashlib.sha256(payload).digest()):
            raise LocalCredentialError("private_sam_local_store_failed")
        return reference
    finally:
        payload.clear()


def discover(authority: LocalDiscoveryAuthority, *, target: str = TARGET,
             runner=subprocess.run) -> LocalDiscoveryResult:
    authority.validate()
    reference = _metadata(target=target, runner=runner)
    if reference is None:
        raise LocalCredentialError("private_sam_local_material_missing")
    result = LocalDiscoveryResult(reference, reference)
    _PROVENANCE[result] = (authority, reference, target, runner)
    return result


def extract(authority: LocalDiscoveryAuthority, result: LocalDiscoveryResult) -> PrivateMaterial:
    authority.validate()
    try:
        original, expected, target, runner = _PROVENANCE.pop(result)
    except (KeyError, TypeError):
        raise LocalCredentialError("private_sam_local_provenance_required") from None
    if authority != original or result.credential_source != expected or result.backup_code_source != expected:
        raise LocalCredentialError("private_sam_local_provenance_required")
    before = _metadata(target=target, runner=runner)
    if before != expected:
        raise LocalCredentialError("private_sam_local_source_changed")
    payload = _run("read", target=target, runner=runner)
    after = _metadata(target=target, runner=runner)
    if after != expected:
        raise LocalCredentialError("private_sam_local_source_changed")
    try:
        decoded = json.loads(payload, object_pairs_hook=_unique_object)
    except (ValueError, UnicodeError, json.JSONDecodeError):
        raise LocalCredentialError("private_sam_local_invalid_payload") from None
    material = _material(decoded)
    if material.email != authority.account_email:
        raise LocalCredentialError("private_sam_local_account_mismatch")
    return material


def source_scope(reference: LocalCredentialReference) -> str:
    if (not isinstance(reference, LocalCredentialReference) or reference.store_id != STORE_ID
            or not isinstance(reference.last_written_filetime, str)
            or _FILETIME.fullmatch(reference.last_written_filetime) is None):
        raise LocalCredentialError("private_sam_local_invalid_reference")
    return "local_os_credential:" + reference.store_id


def _tty_write(descriptor: int, value: str) -> None:
    encoded = value.encode("utf-8")
    offset = 0
    while offset < len(encoded):
        written = os.write(descriptor, encoded[offset:])
        if written < 1:
            raise OSError("terminal write failed")
        offset += written


def _hidden(prompt: str, descriptor: int, max_bytes: int) -> str:
    characters = bytearray()
    try:
        original = termios.tcgetattr(descriptor)
        hidden = list(original)
        hidden[3] &= ~termios.ECHO
        termios.tcsetattr(descriptor, termios.TCSAFLUSH, hidden)
    except (OSError, termios.error):
        raise LocalCredentialError("private_sam_local_hidden_input_unavailable") from None
    try:
        _tty_write(descriptor, prompt)
        while True:
            character = os.read(descriptor, 1)
            if character in {b"", b"\n", b"\r"}:
                break
            if len(characters) == max_bytes:
                raise LocalCredentialError("private_sam_local_invalid_payload")
            characters.extend(character)
        return characters.decode("utf-8")
    finally:
        characters.clear()
        termios.tcsetattr(descriptor, termios.TCSADRAIN, original)
        _tty_write(descriptor, "\n")


def _same(left: str, right: str) -> bool:
    return hmac.compare_digest(left.encode("utf-8"), right.encode("utf-8"))


def _prompt_material_from_fd(descriptor: int) -> PrivateMaterial:
    if not os.isatty(descriptor):
        raise LocalCredentialError("private_sam_local_hidden_input_unavailable")
    email = _hidden("Login.gov email (hidden): ", descriptor, 254).lower()
    email_confirmation = _hidden("Repeat Login.gov email (hidden): ", descriptor, 254).lower()
    if not _same(email, email_confirmation):
        raise LocalCredentialError("private_sam_local_confirmation_mismatch")
    password = _hidden("Login.gov password (hidden): ", descriptor, 4096)
    confirmation = _hidden("Repeat Login.gov password (hidden): ", descriptor, 4096)
    if not _same(password, confirmation):
        raise LocalCredentialError("private_sam_local_confirmation_mismatch")
    code = _hidden("One unused backup code (hidden): ", descriptor, 14)
    repeated = _hidden("Repeat backup code (hidden): ", descriptor, 14)
    if not _same(code, repeated):
        raise LocalCredentialError("private_sam_local_confirmation_mismatch")
    return _material({"schema": SCHEMA, "email": email, "password": password,
                      "backup_codes": [code]})


def prompt_material() -> PrivateMaterial:
    descriptor = None
    try:
        descriptor = os.open(
            "/dev/tty", os.O_RDWR | os.O_NOCTTY | os.O_CLOEXEC | os.O_NOFOLLOW)
        return _prompt_material_from_fd(descriptor)
    except (OSError, EOFError, UnicodeError, termios.error):
        raise LocalCredentialError("private_sam_local_hidden_input_unavailable") from None
    finally:
        if descriptor is not None:
            os.close(descriptor)


def main() -> int:
    try:
        harden()
        material = prompt_material()
        store(material)
        print("private_sam_credentials_stored")
        return 0
    except LocalCredentialError as error:
        print(str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
