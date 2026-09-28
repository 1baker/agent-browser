#!/usr/bin/env python3
"""Synthetic tests for the Windows Credential Manager private SAM source."""

import importlib.util
import json
import os
from pathlib import Path
import pty
import select
import signal
import sys
import termios
import threading
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "private_sam_local_tested", Path(__file__).with_name("private-sam-local-credentials.py"))
local = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(local)


class CredentialManagerFixture:
    def __init__(self):
        self.payloads = {}
        self.revisions = {}
        self.calls = []

    def __call__(self, command, *, input, stdout, stderr, timeout, check):
        operation = command[command.index("-Operation") + 1]
        target = command[command.index("-Target") + 1]
        self.calls.append((tuple(command), bytes(input)))
        if operation == "store":
            self.payloads[target] = bytes(input)
            self.revisions[target] = self.revisions.get(target, 133700000000000000) + 1
            output = b"stored"
        elif operation == "read":
            output = self.payloads.get(target, b"")
            if not output:
                return SimpleNamespace(returncode=1, stdout=b"", stderr=b"fixed")
        elif operation == "metadata":
            if target not in self.payloads:
                output = b'{"present":false}'
            else:
                output = json.dumps({"present": True,
                    "last_written_filetime": str(self.revisions[target])},
                    separators=(",", ":")).encode()
        elif operation == "delete":
            self.payloads.pop(target, None)
            self.revisions.pop(target, None)
            output = b"deleted"
        else:
            raise AssertionError(operation)
        return SimpleNamespace(returncode=0, stdout=output, stderr=b"")


class LocalCredentialTests(unittest.TestCase):
    def setUp(self):
        self.fixture = CredentialManagerFixture()
        self.harden = patch.object(local, "harden")
        self.harden.start()
        self.addCleanup(self.harden.stop)
        self.target = "AgentBrowser/Test/synthetic"
        self.authority = local.LocalDiscoveryAuthority(
            local.STORE_ID, "operator@example.org")
        self.material = local.PrivateMaterial(
            "operator@example.org", "SYNTHETIC_PASSWORD_SENTINEL!", ("ABCD-1234-5678",))

    def store(self):
        return local.store(self.material, target=self.target, runner=self.fixture)

    def test_secret_uses_stdin_only_and_roundtrip_is_revision_bound(self):
        reference = self.store()
        result = local.discover(self.authority, target=self.target, runner=self.fixture)
        recovered = local.extract(self.authority, result)
        self.assertEqual(reference, result.credential_source)
        self.assertEqual(recovered.email, "operator@example.org")
        self.assertEqual(recovered.password, "SYNTHETIC_PASSWORD_SENTINEL!")
        self.assertEqual(recovered.backup_codes, ("ABCD-1234-5678",))
        for command, _ in self.fixture.calls:
            self.assertNotIn("SYNTHETIC_PASSWORD_SENTINEL", " ".join(command))
        self.assertTrue(any(b"SYNTHETIC_PASSWORD_SENTINEL" in body
                            for _, body in self.fixture.calls))
        with self.assertRaisesRegex(local.LocalCredentialError, "provenance_required"):
            local.extract(self.authority, result)

    def test_changed_source_and_wrong_account_fail_before_material_release(self):
        self.store()
        result = local.discover(self.authority, target=self.target, runner=self.fixture)
        self.fixture.revisions[self.target] += 1
        with self.assertRaisesRegex(local.LocalCredentialError, "source_changed"):
            local.extract(self.authority, result)

        self.material = local.PrivateMaterial(
            "different@example.org", "SYNTHETIC_PASSWORD_SENTINEL!", ("ABCD-1234-5678",))
        self.store()
        result = local.discover(self.authority, target=self.target, runner=self.fixture)
        with self.assertRaisesRegex(local.LocalCredentialError, "account_mismatch"):
            local.extract(self.authority, result)

    def test_invalid_and_missing_values_have_fixed_nonsecret_errors(self):
        with self.assertRaisesRegex(local.LocalCredentialError, "material_missing"):
            local.discover(self.authority, target=self.target, runner=self.fixture)
        for value in [
            {"schema": local.SCHEMA, "email": "operator@example.org",
             "password": " SECRET_SENTINEL", "backup_codes": ["ABCD-1234"]},
            {"schema": local.SCHEMA, "email": "operator@example.org",
             "password": "valid", "backup_codes": ["SECRET_SENTINEL"]},
            {"schema": local.SCHEMA, "email": "operator@example.org",
             "password": "valid", "backup_codes": ["ABCD-1234", "EFGH-5678"]},
            {"schema": local.SCHEMA, "email": "operator@example.org",
             "password": "valid",
             "backup_codes": ["ABCD-1234-5678 EFGH-9012-3456"]},
        ]:
            with self.assertRaises(local.LocalCredentialError) as error:
                local._material(value)
            self.assertNotIn("SECRET_SENTINEL", str(error.exception))

    def test_material_cannot_be_pickled_or_rendered(self):
        import pickle
        self.assertNotIn("SYNTHETIC_PASSWORD_SENTINEL", repr(self.material))
        with self.assertRaises(TypeError):
            pickle.dumps(self.material)

    def test_production_pipe_reader_rejects_oversized_stdout_and_stderr(self):
        for stream in ("stdout", "stderr"):
            descriptor = "sys.stdout" if stream == "stdout" else "sys.stderr"
            command = [sys.executable, "-c",
                f"import sys; {descriptor}.write('x' * 9000); {descriptor}.flush()"]
            with self.assertRaisesRegex(
                    local.LocalCredentialError, "private_sam_local_store_failed"):
                local._bounded_process(command, b"")

    def test_missing_controlling_terminal_never_reads_stdin(self):
        class ForbiddenInput:
            def read(self, *_):
                raise AssertionError("stdin must not be read")

        with patch.object(local.os, "open", side_effect=OSError), \
                patch.object(local.sys, "stdin", ForbiddenInput()):
            with self.assertRaisesRegex(
                    local.LocalCredentialError, "hidden_input_unavailable"):
                local.prompt_material()

    def test_real_pty_accepts_six_hidden_prompts_without_echoing_material(self):
        master, slave = pty.openpty()
        original_terminal = termios.tcgetattr(slave)
        result = {}
        prompts = [
            (b"Login.gov email (hidden): ", b"synthetic@example.org"),
            (b"Repeat Login.gov email (hidden): ", b"synthetic@example.org"),
            (b"Login.gov password (hidden): ", b"SYNTHETIC_PASSWORD_SENTINEL!"),
            (b"Repeat Login.gov password (hidden): ", b"SYNTHETIC_PASSWORD_SENTINEL!"),
            (b"One unused backup code (hidden): ", b"ABCD-1234-5678"),
            (b"Repeat backup code (hidden): ", b"ABCD-1234-5678"),
        ]

        def prompt():
            try:
                result["material"] = local._prompt_material_from_fd(slave)
            except BaseException as error:
                result["error"] = error

        worker = threading.Thread(target=prompt, daemon=True)
        transcript = bytearray()
        try:
            worker.start()
            deadline = time.monotonic() + 5
            for expected, response in prompts:
                while expected not in transcript:
                    remaining = deadline - time.monotonic()
                    self.assertGreater(remaining, 0, bytes(transcript))
                    readable, _, _ = select.select([master], [], [], remaining)
                    self.assertEqual(readable, [master], bytes(transcript))
                    transcript.extend(os.read(master, 4096))
                os.write(master, response + b"\n")
            worker.join(5)
            self.assertFalse(worker.is_alive())
            while select.select([master], [], [], 0)[0]:
                transcript.extend(os.read(master, 4096))
            if "error" in result:
                raise result["error"]
            material = result["material"]
            self.assertEqual(material.email, "synthetic@example.org")
            self.assertEqual(material.password, "SYNTHETIC_PASSWORD_SENTINEL!")
            self.assertEqual(material.backup_codes, ("ABCD-1234-5678",))
            for _, response in prompts:
                self.assertNotIn(response, transcript)
            self.assertEqual(termios.tcgetattr(slave), original_terminal)
        finally:
            os.close(master)
            os.close(slave)

    def test_prompt_material_opens_the_real_controlling_terminal(self):
        prompts = [
            (b"Login.gov email (hidden): ", b"synthetic@example.org"),
            (b"Repeat Login.gov email (hidden): ", b"synthetic@example.org"),
            (b"Login.gov password (hidden): ", b"SYNTHETIC_PASSWORD_SENTINEL!"),
            (b"Repeat Login.gov password (hidden): ", b"SYNTHETIC_PASSWORD_SENTINEL!"),
            (b"One unused backup code (hidden): ", b"ABCD-1234-5678"),
            (b"Repeat backup code (hidden): ", b"ABCD-1234-5678"),
        ]
        pid, master = pty.fork()
        if pid == 0:
            try:
                material = local.prompt_material()
                valid = (material.email == "synthetic@example.org"
                    and material.password == "SYNTHETIC_PASSWORD_SENTINEL!"
                    and material.backup_codes == ("ABCD-1234-5678",))
                os._exit(0 if valid else 2)
            except BaseException:
                os._exit(3)

        transcript = bytearray()
        status = None
        deadline = time.monotonic() + 5
        try:
            for expected, response in prompts:
                while expected not in transcript:
                    remaining = deadline - time.monotonic()
                    self.assertGreater(remaining, 0, bytes(transcript))
                    readable, _, _ = select.select([master], [], [], remaining)
                    self.assertEqual(readable, [master], bytes(transcript))
                    transcript.extend(os.read(master, 4096))
                os.write(master, response + b"\n")
            while time.monotonic() < deadline:
                waited, status = os.waitpid(pid, os.WNOHANG)
                if waited == pid:
                    break
                readable, _, _ = select.select([master], [], [], 0.05)
                if readable:
                    try:
                        transcript.extend(os.read(master, 4096))
                    except OSError:
                        pass
            else:
                os.kill(pid, signal.SIGKILL)
                _, status = os.waitpid(pid, 0)
                self.fail("hidden prompt child did not exit")
            self.assertTrue(os.WIFEXITED(status))
            self.assertEqual(os.WEXITSTATUS(status), 0)
            for _, response in prompts:
                self.assertNotIn(response, transcript)
        finally:
            if status is None:
                try:
                    os.kill(pid, signal.SIGKILL)
                    os.waitpid(pid, 0)
                except ProcessLookupError:
                    pass
            os.close(master)


if __name__ == "__main__":
    unittest.main()
