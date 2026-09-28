"""Isolated subprocess and mocked browser/proc tests. No operator runtime use."""

import copy
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


SPEC = importlib.util.spec_from_file_location(
    "cold_process", Path(__file__).parent / "lib/local-dashboard-cold-process.py"
)
MOD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MOD)


class ProcessTests(unittest.TestCase):
    def child(self, ignore=False, args=()):
        script = (
            "import signal,sys; "
            + ("signal.signal(signal.SIGTERM, signal.SIG_IGN); " if ignore else "")
            + "print('ready', flush=True); sys.stdin.readline()"
        )
        process = subprocess.Popen(
            [sys.executable, "-c", script, *args], stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        )
        self.assertEqual(process.stdout.readline().strip(), "ready")

        def cleanup():
            if process.poll() is None:
                process.stdin.write("done\n")
                process.stdin.flush()
            process.wait(timeout=5)
            process.stdin.close()
            process.stdout.close()
            process.stderr.close()
        self.addCleanup(cleanup)
        return process

    def test_capture_and_graceful_pidfd_exit(self):
        child = self.child()
        identity = MOD.capture_process(child.pid)
        self.assertEqual(identity["pid"], child.pid)
        self.assertEqual(identity["euid"], os.geteuid())
        self.assertEqual(len(identity["exe"]["sha256"]), 64)
        self.assertTrue(MOD.verify_process(identity))
        descriptor = MOD.open_verified_pidfd(identity)
        os.close(descriptor)
        result = MOD.terminate_verified_process(identity, timeout=2)
        self.assertEqual(result, {"exited": True, "signalled": True, "pid": child.pid})
        self.assertEqual(child.wait(timeout=2), -15)

    def test_changed_identity_cannot_signal_either_child(self):
        first, second = self.child(), self.child()
        identity = MOD.capture_process(first.pid)
        wrong = copy.deepcopy(identity)
        wrong["pid"] = second.pid
        wrong["startTicks"] += 100
        with mock.patch.object(MOD.signal, "pidfd_send_signal") as send:
            with self.assertRaisesRegex(RuntimeError, "identity mismatch"):
                MOD.terminate_verified_process(wrong, timeout=1)
            send.assert_not_called()
        self.assertIsNone(first.poll())
        self.assertIsNone(second.poll())
        for field, value in [("euid", -1), ("bootId", "changed")]:
            wrong = copy.deepcopy(identity)
            wrong[field] = value
            with self.assertRaises(RuntimeError):
                MOD.verify_process(wrong)
        wrong = copy.deepcopy(identity)
        wrong["exe"]["sha256"] = "0" * 64
        with self.assertRaises(RuntimeError):
            MOD.verify_process(wrong)

    def test_pidfd_postopen_recheck_closes_descriptor(self):
        child = self.child()
        identity = MOD.capture_process(child.pid)
        descriptor = os.pidfd_open(child.pid)
        with mock.patch.object(MOD.os, "pidfd_open", return_value=descriptor):
            with mock.patch.object(MOD, "verify_process", side_effect=[True, RuntimeError("reused")]):
                with self.assertRaisesRegex(RuntimeError, "reused"):
                    MOD.open_verified_pidfd(identity)
        with self.assertRaises(OSError):
            os.fstat(descriptor)
        self.assertIsNone(child.poll())

    def test_ignored_sigterm_times_out_without_fallback(self):
        child = self.child(ignore=True)
        identity = MOD.capture_process(child.pid)
        with mock.patch.object(MOD.os, "kill", side_effect=AssertionError("PID signal forbidden")):
            with self.assertRaises(TimeoutError):
                MOD.terminate_verified_process(identity, timeout=0.05)
        self.assertIsNone(child.poll())

    def test_invalid_pid_timeout_and_absent_process(self):
        for pid in [0, 1, -1, True, "2"]:
            with self.assertRaises(RuntimeError):
                MOD.capture_process(pid)
        child = self.child()
        identity = MOD.capture_process(child.pid)
        for timeout in [0, -1, 61, float("nan"), float("inf"), True]:
            with self.assertRaises(RuntimeError):
                MOD.terminate_verified_process(identity, timeout)
        child.stdin.write("done\n")
        child.stdin.flush()
        child.wait(timeout=2)
        with self.assertRaises(OSError):
            MOD.verify_process(identity)


def row(address="0100007F", port="240F", inode=456, state="0A"):
    return f" 0: {address}:{port} 00000000:0000 {state} 0000:0000 00:0000 00000000 1000 0 {inode}\n"


class BrowserTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="cold-browser-proof-test-")
        self.addCleanup(directory.cleanup)
        self.profile = str(Path(directory.name) / "profile")
        Path(self.profile).mkdir(mode=0o700)
        self.active = Path(self.profile) / "DevToolsActivePort"
        self.active.write_text("9231\n/devtools/browser/fixture-uuid\n")
        self.url = "ws://127.0.0.1:9231/devtools/browser/fixture-uuid"
        self.process = {"pid": 333, "bootId": "fixture", "startTicks": 50, "euid": 1000,
                        "exe": {"path": "/fixture/chrome", "dev": 1, "ino": 2, "sha256": "a" * 64}}
        self.argv = f"chrome\0--user-data-dir={self.profile}\0".encode()
        self.tcp = "header\n" + row()
        self.tcp6 = "header\n"
        for patcher in [
            mock.patch.object(MOD, "capture_process", return_value=self.process),
            mock.patch.object(MOD, "_read_bytes", side_effect=lambda path, *args: self.argv if path == "/proc/333/cmdline" else self.fail("unexpected proc read")),
            mock.patch.object(MOD, "_read_text", side_effect=lambda path: self.tcp if path == "/proc/333/net/tcp" else self.tcp6 if path == "/proc/333/net/tcp6" else self.fail("unexpected proc read")),
            mock.patch.object(MOD, "_fd_links", return_value=["socket:[456]"]),
        ]:
            patcher.start()
            self.addCleanup(patcher.stop)

    def test_exact_browser_snapshot(self):
        identity = MOD.capture_browser(333, self.profile, self.url)
        self.assertTrue(MOD.verify_browser(identity))
        self.assertEqual(identity["profile"]["ino"], Path(self.profile).stat().st_ino)
        self.assertEqual(identity["listener"]["inode"], 456)
        wrong = copy.deepcopy(identity)
        wrong["profile"]["ino"] += 1
        with self.assertRaises(RuntimeError):
            MOD.verify_browser(wrong)

    def test_profile_argument_parser(self):
        self.assertEqual(MOD._profile_argument(b"chrome\0--user-data-dir\0/fixture/profile\0"), "/fixture/profile")
        for argv in [b"chrome", b"chrome\0", b"chrome\0--user-data-dir=relative\0",
                     b"chrome\0--user-data-dir\0", b"chrome\0--user-data-dir=/a\0--user-data-dir=/b\0",
                     b"chrome\0--user-data-dir=/a\0--type=renderer\0", b"chrome\0--type\0browser\0--user-data-dir=/a\0"]:
            with self.assertRaises(RuntimeError):
                MOD._profile_argument(argv)

    def test_listener_must_be_unique_and_owned(self):
        with mock.patch.object(MOD, "_fd_links", return_value=["socket:[999]"]):
            with self.assertRaisesRegex(RuntimeError, "not owned"):
                MOD.capture_browser(333, self.profile, self.url)
        self.tcp += row(inode=999)
        with self.assertRaisesRegex(RuntimeError, "not unique"):
            MOD.capture_browser(333, self.profile, self.url)
        self.tcp = "header\n" + row(state="01")
        with self.assertRaisesRegex(RuntimeError, "not unique"):
            MOD.capture_browser(333, self.profile, self.url)

    def test_ipv6_exact_loopback(self):
        self.tcp = "header\n"
        self.tcp6 += row(address="00000000000000000000000001000000")
        identity = MOD.capture_browser(333, self.profile, "ws://[::1]:9231/devtools/browser/fixture-uuid")
        self.assertEqual(identity["listener"]["family"], 6)

    def test_endpoint_and_active_port_constraints(self):
        for url in [self.url + "?x=1", self.url + "#x", self.url.replace("127.0.0.1", "localhost"),
                    self.url.replace("127.0.0.1", "0.0.0.0"), self.url.replace("ws:", "wss:"),
                    self.url.replace("127.0.0.1", "user@127.0.0.1")]:
            with self.assertRaises(RuntimeError):
                MOD.capture_browser(333, self.profile, url)
        self.active.write_text("9232\n/devtools/browser/fixture-uuid\n")
        with self.assertRaisesRegex(RuntimeError, "active port"):
            MOD.capture_browser(333, self.profile, self.url)

    def test_symlink_profile_and_active_port_rejected(self):
        alias = Path(self.profile).parent / "alias"
        alias.symlink_to(self.profile)
        with self.assertRaises(RuntimeError):
            MOD.capture_browser(333, str(alias), self.url)
        content = self.active.read_text()
        self.active.unlink()
        other = Path(self.profile).parent / "other"
        other.write_text(content)
        self.active.symlink_to(other)
        with self.assertRaises(OSError):
            MOD.capture_browser(333, self.profile, self.url)

    def test_nonregular_active_port_fails_without_blocking(self):
        self.active.unlink()
        os.mkfifo(self.active, 0o600)
        with self.assertRaisesRegex(RuntimeError, "regular"):
            MOD.capture_browser(333, self.profile, self.url)

    def test_rechecks_process_after_browser_observations(self):
        changed = copy.deepcopy(self.process)
        changed["startTicks"] += 1
        with mock.patch.object(MOD, "capture_process", side_effect=[self.process, self.process, changed]):
            with self.assertRaisesRegex(RuntimeError, "identity mismatch"):
                MOD.capture_browser(333, self.profile, self.url)


class IdleTests(unittest.TestCase):
    child = ProcessTests.child

    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="cold-profile-idle-test-")
        self.addCleanup(directory.cleanup)
        self.profile = str(Path(directory.name) / "profile")
        Path(self.profile).mkdir(mode=0o700)

    def test_empty_and_absent_profile(self):
        with mock.patch.object(MOD, "_owned_process_snapshots", return_value=[]):
            self.assertEqual(MOD.require_profile_idle(self.profile), {"idle": True})
            Path(self.profile).rmdir()
            self.assertEqual(MOD.require_profile_idle(self.profile), {"idle": True})

    def test_exact_owned_child_and_renderer_are_not_idle(self):
        for args in [
            [f"--user-data-dir={self.profile}"],
            ["--user-data-dir", self.profile, "--type=renderer"],
        ]:
            child = self.child(args=args)
            with mock.patch.object(MOD.os, "listdir", return_value=[str(child.pid)]):
                with self.assertRaisesRegex(RuntimeError, "still references"):
                    MOD.require_profile_idle(self.profile)
            child.stdin.write("done\n")
            child.stdin.flush()
            child.wait(timeout=2)

    def test_argument_alias_matches_and_unrelated_argument_does_not(self):
        alias = Path(self.profile).parent / "alias"
        alias.symlink_to(self.profile)
        child = self.child(args=[f"--user-data-dir={alias}"])
        with mock.patch.object(MOD.os, "listdir", return_value=[str(child.pid)]):
            with self.assertRaisesRegex(RuntimeError, "still references"):
                MOD.require_profile_idle(self.profile)
        child.stdin.write("done\n")
        child.stdin.flush()
        child.wait(timeout=2)
        other = self.child(args=[f"prefix--user-data-dir={self.profile}"])
        with mock.patch.object(MOD.os, "listdir", return_value=[str(other.pid)]):
            self.assertEqual(MOD.require_profile_idle(self.profile), {"idle": True})

    def test_samehost_stale_lock_preserved_and_live_pid_rejected(self):
        child = self.child()
        identity = MOD.capture_process(child.pid)
        lock = Path(self.profile) / "SingletonLock"
        target = f"{MOD.socket.gethostname()}-{child.pid}"
        lock.symlink_to(target)
        with mock.patch.object(MOD, "_owned_process_snapshots", return_value=[]):
            with self.assertRaisesRegex(RuntimeError, "PID is still present"):
                MOD.require_profile_idle(self.profile)
            child.stdin.write("done\n")
            child.stdin.flush()
            child.wait(timeout=2)
            self.assertFalse(MOD._process_exists(identity["pid"]))
            self.assertEqual(MOD.require_profile_idle(self.profile), {"idle": True})
        self.assertEqual(os.readlink(lock), target)

    def test_otherhost_or_regular_lock_unknown(self):
        lock = Path(self.profile) / "SingletonLock"
        lock.symlink_to("other-host-fixture-234")
        with self.assertRaisesRegex(RuntimeError, "host or PID is unknown"):
            MOD.require_profile_idle(self.profile)
        lock.unlink()
        lock.write_text("not a Chrome lock")
        with self.assertRaisesRegex(RuntimeError, "expected symlink"):
            MOD.require_profile_idle(self.profile)

    def test_symlink_ancestor_even_if_leaf_absent(self):
        ancestor = Path(self.profile).parent / "alias-parent"
        ancestor.symlink_to(Path(self.profile).parent)
        with self.assertRaisesRegex(RuntimeError, "ancestor"):
            MOD.require_profile_idle(str(ancestor / "missing-profile"))
        for profile in ["/", "/tmp", "relative/profile"]:
            with self.assertRaises(RuntimeError):
                MOD.require_profile_idle(profile)

    def test_unreadable_owned_proc_fails_unless_kernel_gone(self):
        child = self.child()
        real_read = MOD._read_bytes

        def unreadable(path, *args):
            if path == f"/proc/{child.pid}/cmdline":
                raise PermissionError("fixture unreadable argv")
            return real_read(path, *args)

        with mock.patch.object(MOD.os, "listdir", return_value=[str(child.pid)]):
            with mock.patch.object(MOD, "_read_bytes", side_effect=unreadable):
                with self.assertRaises(PermissionError):
                    MOD.require_profile_idle(self.profile)
        child.stdin.write("done\n")
        child.stdin.flush()
        child.wait(timeout=2)
        with mock.patch.object(MOD.os, "listdir", return_value=[str(child.pid)]):
            self.assertEqual(MOD.require_profile_idle(self.profile), {"idle": True})

    def test_dangling_socket_preserved_and_real_socket_unknown(self):
        link = Path(self.profile) / "SingletonSocket"
        missing = str(Path(self.profile).parent / "absent-socket")
        link.symlink_to(missing)
        with mock.patch.object(MOD, "_owned_process_snapshots", return_value=[]):
            self.assertEqual(MOD.require_profile_idle(self.profile), {"idle": True})
        self.assertEqual(os.readlink(link), missing)
        listener = MOD.socket.socket(MOD.socket.AF_UNIX, MOD.socket.SOCK_STREAM)
        self.addCleanup(listener.close)
        listener.bind(missing)
        listener.listen()
        unix = f"header\n0000: 00000002 00000000 00010000 0001 01 456 {missing}\n"
        with mock.patch.object(MOD, "_read_text", return_value=unix):
            with self.assertRaisesRegex(RuntimeError, "active listener"):
                MOD.require_profile_idle(self.profile)
        with mock.patch.object(MOD, "_read_text", return_value="header\n"):
            with self.assertRaisesRegex(RuntimeError, "liveness is unknown"):
                MOD.require_profile_idle(self.profile)
        self.assertTrue(link.is_symlink())


if __name__ == "__main__":
    unittest.main()
