#!/usr/bin/env python3
"""User-run, local Slack bootstrap; no message reads or daemon activation.

The token is plaintext protected by Unix ownership/modes, not encrypted and not
protected against the account owner, root, memory inspection, or backups.
"""
import getpass
import json
import os
from pathlib import Path
import pwd
import re
import resource
import secrets
import ssl
import stat
import sys
import time
import urllib.request
import warnings

CHANNEL = "C07CA08AKUH"
TEAM = "TEG3AC109"
SCOPES = frozenset({"groups:history", "canvases:read", "canvases:write"})
LIMIT = 64 * 1024
TIMEOUT = 10
CONFIG = "connection.json"


class SetupError(Exception):
    """Only fixed, repository-defined diagnostic codes may be supplied."""


def harden():
    if sys.platform != "linux":
        raise SetupError("private_slack_platform_unsupported")
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def valid_token(token):
    return isinstance(token, str) and 24 <= len(token) <= 512 and re.fullmatch(
        r"xoxb-[0-9]+-[0-9]+-[A-Za-z0-9]+", token
    ) is not None


def hidden_token():
    """Require a controlling tty; getpass may never fall back to echoed input."""
    try:
        fd = os.open("/dev/tty", os.O_RDWR | os.O_NOCTTY | os.O_CLOEXEC | os.O_NOFOLLOW)
        with os.fdopen(fd, "w", encoding="utf-8") as tty:
            if not os.isatty(tty.fileno()):
                raise SetupError("private_slack_hidden_input_unavailable")
            with warnings.catch_warnings():
                warnings.simplefilter("error", getpass.GetPassWarning)
                token = getpass.getpass("Slack bot token (hidden): ", stream=tty)
    except (OSError, EOFError, getpass.GetPassWarning):
        raise SetupError("private_slack_hidden_input_unavailable") from None
    if not valid_token(token):
        raise SetupError("private_slack_invalid_token")
    return token


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise SetupError("private_slack_redirect_rejected")


def opener():
    context = ssl.create_default_context()
    if not context.check_hostname or context.verify_mode != ssl.CERT_REQUIRED:
        raise SetupError("private_slack_tls_unavailable")
    return urllib.request.build_opener(
        urllib.request.ProxyHandler({}),
        urllib.request.HTTPSHandler(context=context, debuglevel=0),
        NoRedirect(),
    )


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise SetupError("private_slack_invalid_response")
        result[key] = value
    return result


def decode_json(data):
    try:
        return json.loads(data, object_pairs_hook=unique_object)
    except (ValueError, UnicodeError, RecursionError):
        raise SetupError("private_slack_invalid_response") from None


def validate_scopes(scopes):
    if not isinstance(scopes, list) or any(not isinstance(scope, str) for scope in scopes):
        raise SetupError("private_slack_scope_mismatch")
    actual = set(scopes)
    if len(actual) != len(scopes) or "groups:history" not in actual or not actual <= SCOPES:
        raise SetupError("private_slack_scope_mismatch")
    return sorted(actual)


def probe(token, transport=None):
    """Only auth.test is requested. No redirects, environment proxies or logs."""
    if not valid_token(token):
        raise SetupError("private_slack_invalid_token")
    request = urllib.request.Request(
        "https://slack.com/api/auth.test", data=b"", method="POST",
        headers={"Authorization": "Bearer " + token, "Accept": "application/json",
                 "Content-Type": "application/x-www-form-urlencoded", "Accept-Encoding": "identity"},
    )
    try:
        deadline = time.monotonic() + TIMEOUT
        with (transport or opener()).open(request, timeout=TIMEOUT) as response:
            if response.status != 200:
                raise SetupError("private_slack_auth_failed")
            if response.headers.get_content_type() != "application/json":
                raise SetupError("private_slack_invalid_response")
            headers = response.headers.get_all("x-oauth-scopes", [])
            if len(headers) != 1 or len(headers[0]) > 2048:
                raise SetupError("private_slack_scope_mismatch")
            scopes = validate_scopes([scope.strip() for scope in headers[0].split(",")])
            data = bytearray()
            while True:
                if time.monotonic() >= deadline:
                    raise SetupError("private_slack_auth_failed")
                chunk = response.read1(min(8192, LIMIT + 1 - len(data)))
                if time.monotonic() >= deadline:
                    raise SetupError("private_slack_auth_failed")
                if not chunk:
                    break
                data.extend(chunk)
                if len(data) > LIMIT:
                    raise SetupError("private_slack_invalid_response")
        result = decode_json(data)
        if not isinstance(result, dict) or result.get("ok") is not True:
            raise SetupError("private_slack_auth_failed")
        if result.get("team_id") != TEAM:
            raise SetupError("private_slack_wrong_workspace")
        bot = result.get("bot_id")
        if not isinstance(bot, str) or re.fullmatch(r"B[A-Z0-9]{8,20}", bot) is None:
            raise SetupError("private_slack_invalid_response")
        return {"schema": "agent-browser.private-slack.v1", "token": token,
                "team_id": TEAM, "bot_id": bot, "channel_id": CHANNEL, "scopes": scopes}
    except SetupError:
        raise
    except Exception:
        raise SetupError("private_slack_auth_failed") from None


def check_directory(fd, private=False):
    metadata = os.fstat(fd)
    uid = os.geteuid()
    if not stat.S_ISDIR(metadata.st_mode) or metadata.st_nlink == 0:
        raise SetupError("private_slack_invalid_storage")
    if private:
        valid = metadata.st_uid == uid and stat.S_IMODE(metadata.st_mode) == 0o700
    else:
        valid = metadata.st_uid in (uid, 0) and (
            not metadata.st_mode & 0o022 or metadata.st_uid == 0 and metadata.st_mode & stat.S_ISVTX
        )
    if not valid:
        raise SetupError("private_slack_invalid_storage")


def open_directory(root, create=False):
    root = Path(root)
    parts = root.parts
    if not root.is_absolute() or len(parts) < 2 or len(parts) > 128 or ".." in parts:
        raise SetupError("private_slack_invalid_storage")
    fd = os.open("/", os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        check_directory(fd)
        for index, part in enumerate(parts[1:]):
            if create:
                try:
                    os.mkdir(part, 0o700, dir_fd=fd)
                    os.fsync(fd)
                except FileExistsError:
                    pass
            next_fd = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW, dir_fd=fd)
            try:
                check_directory(next_fd, private=index == len(parts) - 2)
            except BaseException:
                os.close(next_fd)
                raise
            os.close(fd)
            fd = next_fd
        return fd
    except BaseException:
        os.close(fd)
        raise


def check_file(fd):
    metadata = os.fstat(fd)
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_uid != os.geteuid() \
            or stat.S_IMODE(metadata.st_mode) != 0o600 or metadata.st_nlink != 1 or metadata.st_size > LIMIT:
        raise SetupError("private_slack_invalid_storage")


def validate_connection(value):
    if not isinstance(value, dict) or set(value) != {"schema", "token", "team_id", "bot_id", "channel_id", "scopes"}:
        raise SetupError("private_slack_invalid_storage")
    if value["schema"] != "agent-browser.private-slack.v1" or not valid_token(value["token"]) \
            or value["team_id"] != TEAM or value["channel_id"] != CHANNEL \
            or not isinstance(value["bot_id"], str) or re.fullmatch(r"B[A-Z0-9]{8,20}", value["bot_id"]) is None:
        raise SetupError("private_slack_invalid_storage")
    validate_scopes(value["scopes"])


def existing_connection(directory):
    try:
        fd = os.open(CONFIG, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=directory)
    except FileNotFoundError:
        return False
    with os.fdopen(fd, "rb") as handle:
        check_file(handle.fileno())
        data = handle.read(LIMIT + 1)
        check_file(handle.fileno())
    if len(data) > LIMIT:
        raise SetupError("private_slack_invalid_storage")
    validate_connection(decode_json(data))
    return True


def publish(directory, value):
    """Create-new hard-link publication; never replace or delete connection.json."""
    check_directory(directory, private=True)
    validate_connection(value)
    data = json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    temporary = ".connection-" + secrets.token_hex(16) + ".tmp"
    fd = None
    linked = False
    try:
        fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600, dir_fd=directory)
        check_file(fd)
        pending = memoryview(data)
        while pending:
            count = os.write(fd, pending)
            if count <= 0:
                raise SetupError("private_slack_publication_failed")
            pending = pending[count:]
        check_file(fd)
        os.fsync(fd)
        os.link(temporary, CONFIG, src_dir_fd=directory, dst_dir_fd=directory, follow_symlinks=False)
        linked = True
        os.unlink(temporary, dir_fd=directory)
        os.fsync(directory)
        check_file(fd)
    except FileExistsError:
        raise SetupError("private_slack_connection_exists") from None
    except Exception:
        raise SetupError("private_slack_publication_uncertain" if linked else "private_slack_publication_failed") from None
    finally:
        if fd is not None:
            os.close(fd)
            try:
                os.unlink(temporary, dir_fd=directory)
                os.fsync(directory)
            except OSError:
                pass


def run(root, status=False):
    harden()
    try:
        directory = open_directory(root, create=not status)
    except FileNotFoundError:
        print("private_slack_not_configured")
        return 1
    try:
        if existing_connection(directory):
            print("private_slack_configured_locally_live_auth_not_retested")
            return 0
        if status:
            print("private_slack_not_configured")
            return 1
        token = hidden_token()
        connection = probe(token)
        publish(directory, connection)
        print("private_slack_configured_no_message_access_tested")
        return 0
    finally:
        os.close(directory)


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    # argparse's normal error messages echo rejected arguments, possibly tokens.
    if argv == ["--help"]:
        print("Run without arguments for hidden token setup, or with --status for local metadata-only validation.")
        return 0
    if argv not in ([], ["--status"]):
        print("private_slack_invalid_arguments", file=sys.stderr)
        return 2
    try:
        root = Path(pwd.getpwuid(os.geteuid()).pw_dir) / ".config/agent-browser/private-slack"
        return run(root, status=bool(argv))
    except SetupError as error:
        print(str(error), file=sys.stderr)
    except (KeyboardInterrupt, EOFError):
        print("private_slack_setup_cancelled", file=sys.stderr)
    except Exception:
        print("private_slack_setup_failed", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
