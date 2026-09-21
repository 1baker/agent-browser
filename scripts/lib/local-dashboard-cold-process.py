"""Linux exact-process observations and pidfd-only graceful shutdown.

Observations do not grant lifecycle authority. The publisher must supply its
reviewed journal, admission fence and profile-idle policy before calling these
functions. No command-line stop interface is exposed.
"""

import hashlib
import ipaddress
import math
import os
from pathlib import Path
import re
import select
import signal
import socket
import stat
import sys
import time
from urllib.parse import urlsplit


def _fail(reason):
    raise RuntimeError(f"Cold process: {reason}")


def _pid(pid):
    if sys.platform != "linux" or type(pid) is not int or pid <= 1:
        _fail("positive non-init Linux PID required")
    return pid


def _read_bytes(path, limit=1024 * 1024):
    with open(path, "rb") as source:
        value = source.read(limit + 1)
    if len(value) > limit:
        _fail("bounded observation exceeded")
    return value


def _read_text(path):
    return _read_bytes(path).decode("utf-8", errors="strict")


def _start_ticks(pid):
    value = _read_text(f"/proc/{pid}/stat")
    try:
        ticks = int(value[value.rindex(")") + 1:].split()[19])
    except (ValueError, IndexError):
        _fail("process start unreadable")
    if ticks < 0:
        _fail("invalid process start")
    return ticks


def _euid(pid):
    rows = [line.split()[1:] for line in _read_text(f"/proc/{pid}/status").splitlines() if line.startswith("Uid:")]
    if len(rows) != 1 or len(rows[0]) != 4:
        _fail("effective uid unavailable")
    return int(rows[0][1])


def capture_process(pid):
    """Capture stable boot/start/uid and the actual open executable inode."""
    _pid(pid)
    boot = _read_text("/proc/sys/kernel/random/boot_id").strip()
    if not re.fullmatch(r"[a-f0-9-]{36}", boot):
        _fail("boot identity unavailable")
    before = _start_ticks(pid)
    uid = _euid(pid)
    executable = f"/proc/{pid}/exe"
    path = os.readlink(executable)
    if not path.startswith("/"):
        _fail("executable path is not absolute")
    fd = os.open(executable, os.O_RDONLY | os.O_CLOEXEC)
    try:
        metadata = os.fstat(fd)
        if not stat.S_ISREG(metadata.st_mode):
            _fail("executable is not regular")
        digest = hashlib.sha256()
        while True:
            chunk = os.read(fd, 1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
        after = os.fstat(fd)
        if (metadata.st_size, metadata.st_mtime_ns, metadata.st_ctime_ns) != (after.st_size, after.st_mtime_ns, after.st_ctime_ns):
            _fail("executable bytes changed")
        observed = os.stat(executable)
        if (metadata.st_dev, metadata.st_ino) != (observed.st_dev, observed.st_ino):
            _fail("executable changed")
    finally:
        os.close(fd)
    if before != _start_ticks(pid) or uid != _euid(pid) or path != os.readlink(executable):
        _fail("process changed during capture")
    if boot != _read_text("/proc/sys/kernel/random/boot_id").strip():
        _fail("boot changed during capture")
    return {"pid": pid, "bootId": boot, "startTicks": before, "euid": uid,
            "exe": {"path": path, "dev": metadata.st_dev, "ino": metadata.st_ino, "sha256": digest.hexdigest()}}


def verify_process(identity):
    """Return True only for an exact current match; all uncertainty raises."""
    if not isinstance(identity, dict) or capture_process(identity.get("pid")) != identity:
        _fail("process identity mismatch")
    return True


def open_verified_pidfd(identity):
    """Caller owns returned fd. Bracket pidfd_open with full identity checks."""
    if not hasattr(os, "pidfd_open") or not hasattr(signal, "pidfd_send_signal"):
        _fail("pidfd support required; no PID fallback")
    verify_process(identity)
    fd = os.pidfd_open(identity["pid"], 0)
    try:
        verify_process(identity)
        poll = select.poll()
        poll.register(fd, select.POLLIN)
        if poll.poll(0):
            _fail("process exited during pidfd capture")
        return fd
    except BaseException:
        os.close(fd)
        raise


def terminate_verified_process(identity, timeout=10):
    """Send only SIGTERM to a verified pidfd; timeout never escalates."""
    if isinstance(timeout, bool) or not isinstance(timeout, (int, float)) or not math.isfinite(timeout) or not 0 < timeout <= 60:
        _fail("timeout must be finite and within 0..60 seconds")
    fd = open_verified_pidfd(identity)
    try:
        verify_process(identity)
        try:
            signal.pidfd_send_signal(fd, signal.SIGTERM, None, 0)
        except ProcessLookupError:
            return {"exited": True, "signalled": False, "pid": identity["pid"]}
        poll = select.poll()
        poll.register(fd, select.POLLIN)
        deadline = time.monotonic() + timeout
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("Cold process: SIGTERM wait timed out; process preserved")
            events = poll.poll(math.ceil(remaining * 1000))
            if events:
                if any(event & select.POLLIN for _, event in events):
                    return {"exited": True, "signalled": True, "pid": identity["pid"]}
                _fail("unexpected pidfd readiness")
    finally:
        os.close(fd)


def _canonical_profile(profile):
    if not isinstance(profile, str) or not os.path.isabs(profile) or os.path.normpath(profile) != profile:
        _fail("canonical absolute profile required")
    path = Path(profile)
    for component in [*reversed(path.parents), path]:
        metadata = component.lstat()
        if not stat.S_ISDIR(metadata.st_mode):
            _fail("profile has non-directory or symlink ancestor")
    if path.resolve(strict=True) != path:
        _fail("profile alias rejected")
    metadata = path.stat()
    return {"path": profile, "dev": metadata.st_dev, "ino": metadata.st_ino}


def _profile_argument(cmdline):
    if not cmdline or not cmdline.endswith(b"\0"):
        _fail("NUL-terminated argv required")
    arguments = cmdline[:-1].decode("utf-8", errors="strict").split("\0")
    profiles = []
    for index, argument in enumerate(arguments):
        if argument == "--type" or argument.startswith("--type="):
            _fail("child browser process rejected")
        if argument.startswith("--user-data-dir="):
            profiles.append(argument.partition("=")[2])
        elif argument == "--user-data-dir":
            profiles.append(arguments[index + 1] if index + 1 < len(arguments) else "")
    if len(profiles) != 1 or not os.path.isabs(profiles[0]):
        _fail("one absolute profile argument required")
    return profiles[0]


def _endpoint(url):
    if not isinstance(url, str):
        _fail("CDP URL required")
    parsed = urlsplit(url)
    try:
        address = ipaddress.ip_address(parsed.hostname or "")
        port = parsed.port
    except ValueError:
        _fail("invalid local CDP URL")
    if (parsed.scheme != "ws" or str(address) not in ("127.0.0.1", "::1")
            or port is None or not 0 < port <= 65535 or parsed.username is not None
            or parsed.password is not None or parsed.query or parsed.fragment
            or not re.fullmatch(r"/devtools/browser/[A-Za-z0-9_-]+", parsed.path)):
        _fail("exact loopback browser WebSocket URL required")
    host = f"[{address}]" if address.version == 6 else str(address)
    if url != f"ws://{host}:{port}{parsed.path}":
        _fail("noncanonical CDP URL")
    return address, port, parsed.path


def _listener_rows(text, version, address, port):
    matches = []
    for line in text.splitlines()[1:]:
        fields = line.split()
        if len(fields) < 10:
            _fail("malformed TCP observation")
        encoded, encoded_port = fields[1].split(":")
        raw = bytes.fromhex(encoded)
        if version == 4 and len(raw) == 4:
            decoded = ipaddress.ip_address(raw[::-1])
        elif version == 6 and len(raw) == 16:
            decoded = ipaddress.ip_address(b"".join(raw[index:index + 4][::-1] for index in range(0, 16, 4)))
        else:
            _fail("malformed TCP address")
        if decoded == address and int(encoded_port, 16) == port and fields[3] == "0A":
            inode = int(fields[9])
            if inode <= 0:
                _fail("invalid listener inode")
            matches.append({"family": version, "address": str(address), "port": port, "inode": inode})
    return matches


def _fd_links(pid):
    links = []
    for name in os.listdir(f"/proc/{pid}/fd"):
        try:
            links.append(os.readlink(f"/proc/{pid}/fd/{name}"))
        except FileNotFoundError:
            # An unrelated descriptor may close during enumeration. Ownership
            # still requires a positively observed exact listening inode.
            continue
    return links


def _listener(pid, address, port):
    matches = _listener_rows(_read_text(f"/proc/{pid}/net/tcp"), 4, address, port)
    matches += _listener_rows(_read_text(f"/proc/{pid}/net/tcp6"), 6, address, port)
    if len(matches) != 1:
        _fail("listener not unique")
    listener = matches[0]
    if f"socket:[{listener['inode']}]" not in _fd_links(pid):
        _fail("listener not owned by browser process")
    return listener


def _active_port(profile):
    path = os.path.join(profile, "DevToolsActivePort")
    fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        if not stat.S_ISREG(os.fstat(fd).st_mode):
            _fail("active port is not a regular file")
        raw = os.read(fd, 4097)
        if len(raw) > 4096:
            _fail("active port exceeds bound")
        return raw.decode("utf-8", errors="strict").splitlines()
    finally:
        os.close(fd)


def capture_browser(pid, profile, cdp_url):
    """Bind a process to an exact profile and its own kernel TCP listener."""
    process = capture_process(pid)
    directory = _canonical_profile(profile)
    address, port, endpoint_path = _endpoint(cdp_url)
    if _profile_argument(_read_bytes(f"/proc/{pid}/cmdline")) != profile:
        _fail("browser profile argument mismatch")
    expected_active = [str(port), endpoint_path]
    if _active_port(profile) != expected_active:
        _fail("active port does not match exact CDP URL")
    listener = _listener(pid, address, port)
    verify_process(process)
    if (directory != _canonical_profile(profile)
            or _profile_argument(_read_bytes(f"/proc/{pid}/cmdline")) != profile
            or _active_port(profile) != expected_active
            or _listener(pid, address, port) != listener):
        _fail("browser identity changed during capture")
    verify_process(process)
    return {"process": process, "profile": directory, "cdpUrl": cdp_url, "listener": listener}


def verify_browser(identity):
    if not isinstance(identity, dict) or capture_browser(identity["process"]["pid"], identity["profile"]["path"], identity["cdpUrl"]) != identity:
        _fail("browser identity mismatch")
    return True


def _idle_profile_path(profile):
    """Validate all ancestors even while a receipted restore moves the leaf."""
    if not isinstance(profile, str) or not os.path.isabs(profile) or os.path.normpath(profile) != profile:
        _fail("canonical absolute idle profile required")
    path = Path(profile)
    if len(path.parts) < 4:
        _fail("broad root is not an idle profile")
    for component in reversed(path.parents):
        if not stat.S_ISDIR(component.lstat().st_mode):
            _fail("idle profile ancestor is not a real directory")
    try:
        metadata = path.lstat()
    except FileNotFoundError:
        return None
    if not stat.S_ISDIR(metadata.st_mode):
        _fail("idle profile is not a real directory")
    return (metadata.st_dev, metadata.st_ino)


def _process_exists(pid):
    try:
        os.stat(f"/proc/{pid}")
        return True
    except FileNotFoundError:
        return False


def _process_state(pid):
    value = _read_text(f"/proc/{pid}/stat")
    try:
        fields = value[value.rindex(")") + 1:].split()
        return fields[0], int(fields[19])
    except (ValueError, IndexError):
        _fail("idle process identity unreadable")


def _owned_process_snapshots():
    """Read bounded argv for this effective UID only; never emit argv/logs."""
    for name in os.listdir("/proc"):
        if not name.isascii() or not name.isdecimal():
            continue
        pid = int(name)
        try:
            os.stat(f"/proc/{pid}")
            # proc-directory ownership can become root for a nondumpable
            # same-UID process. Effective UID in status is authoritative.
            if _euid(pid) != os.geteuid():
                continue
            before = _process_state(pid)
            if _euid(pid) != os.geteuid():
                _fail("owned process uid changed during idle check")
            argv = _read_bytes(f"/proc/{pid}/cmdline")
            after = _process_state(pid)
            if before[1] != after[1] or _euid(pid) != os.geteuid():
                _fail("owned process changed during idle check")
            if not argv:
                if after[0] == "Z":
                    continue
                _fail("live owned process argv unavailable")
            yield pid, argv
        except OSError:
            if _process_exists(pid):
                raise
            # Kernel-proven disappearance is the only unreadable-row exception.
            continue


def _argv_profile_paths(pid, cmdline):
    if not cmdline.endswith(b"\0"):
        _fail("owned process argv is not NUL terminated")
    arguments = cmdline[:-1].decode("utf-8", errors="strict").split("\0")
    values = []
    for index, argument in enumerate(arguments):
        if argument.startswith("--user-data-dir="):
            value = argument.partition("=")[2]
        elif argument == "--user-data-dir":
            value = arguments[index + 1] if index + 1 < len(arguments) else ""
        else:
            continue
        if not value or value.startswith("--"):
            _fail("owned process profile argument is incomplete")
        if not os.path.isabs(value):
            value = os.path.join(os.readlink(f"/proc/{pid}/cwd"), value)
        # Resolve aliases to catch a process using a symlink to the exact profile.
        try:
            values.append(os.path.realpath(value, strict=True))
        except FileNotFoundError:
            # An absent final profile is expected during receipted restoration.
            values.append(os.path.realpath(value))
    return values


def _singleton_lock(profile):
    path = os.path.join(profile, "SingletonLock")
    try:
        metadata = os.lstat(path)
    except FileNotFoundError:
        return None
    if not stat.S_ISLNK(metadata.st_mode):
        _fail("SingletonLock is not the expected symlink")
    target = os.readlink(path)
    matched = re.fullmatch(r"([A-Za-z0-9_.-]+)-([1-9][0-9]*)", target)
    if not matched or matched.group(1) != socket.gethostname():
        _fail("SingletonLock host or PID is unknown")
    if _process_exists(int(matched.group(2))):
        _fail("SingletonLock PID is still present")
    return (metadata.st_dev, metadata.st_ino, target)


def _singleton_socket(profile):
    path = os.path.join(profile, "SingletonSocket")
    try:
        link_metadata = os.lstat(path)
    except FileNotFoundError:
        return None
    target = os.readlink(path) if stat.S_ISLNK(link_metadata.st_mode) else path
    if not os.path.isabs(target):
        target = os.path.join(profile, target)
    target = os.path.realpath(target)
    try:
        metadata = os.stat(target)
    except FileNotFoundError:
        # A dangling link cannot receive a connection; keep it intact.
        return (link_metadata.st_dev, link_metadata.st_ino, target, "absent")
    if not stat.S_ISSOCK(metadata.st_mode):
        _fail("SingletonSocket target is not a socket")
    for line in _read_text("/proc/net/unix").splitlines()[1:]:
        fields = line.split(maxsplit=7)
        if len(fields) < 7:
            _fail("unix listener observation malformed")
        # Filesystem socket inode and sockfs inode need not agree. A matching
        # pathname is positive evidence; absence is deliberately not authority.
        if len(fields) == 8 and fields[7] in (path, target):
            if int(fields[3], 16) & 0x10000:
                _fail("SingletonSocket has an active listener")
    # Different network namespaces and orphaned socket nodes are ambiguous.
    # Do not connect, unlink, or infer idleness from missing /proc/net/unix rows.
    _fail("SingletonSocket liveness is unknown")


def require_profile_idle(profile):
    """Prove no owned argv binding and no live/unknown Singleton owner.

    This is an instantaneous read-only observation, not a lock. The publisher
    must maintain its admission fence across this check and subsequent effects.
    No Singleton artifact is removed, and an absent leaf is allowed only for
    a caller-receipted restore (the caller must establish that authority).
    """
    before = _idle_profile_path(profile)
    lock = _singleton_lock(profile)
    singleton = _singleton_socket(profile)
    for pid, argv in _owned_process_snapshots():
        try:
            paths = _argv_profile_paths(pid, argv)
        except OSError:
            if not _process_exists(pid):
                continue
            raise
        if profile in paths:
            # Renderer --type flags do not exempt a matching profile process.
            _fail("live owned process still references exact profile")
    if (before != _idle_profile_path(profile) or lock != _singleton_lock(profile)
            or singleton != _singleton_socket(profile)):
        _fail("profile singleton or directory changed during idle check")
    return {"idle": True}
