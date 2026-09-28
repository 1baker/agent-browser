#!/usr/bin/env python3
"""Offline synthetic tests: never contact Slack or read a real token/config."""
import contextlib
import email.message
import importlib.util
import io
import json
import os
from pathlib import Path
import fcntl
import pty
import select
import ssl
import subprocess
import sys
import tempfile
import termios
import time
import unittest
from unittest.mock import MagicMock, patch
import warnings

SPEC = importlib.util.spec_from_file_location("private_slack_setup", Path(__file__).with_name("setup-private-slack.py"))
setup = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(setup)
TOKEN = "NOT_A_CREDENTIAL_TEST_FIXTURE"
SENTINEL = "SYNTHETIC_SECRET_ERROR_SENTINEL"


class Response(io.BytesIO):
    def __init__(self, body=None, scopes="groups:history,canvases:read,canvases:write", status=200):
        super().__init__(json.dumps(body if body is not None else {"ok": True, "team_id": setup.TEAM, "bot_id": "B12345678"}).encode())
        self.status = status
        self.headers = email.message.Message()
        self.headers["Content-Type"] = "application/json; charset=utf-8"
        if scopes is not None:
            self.headers["X-OAuth-Scopes"] = scopes


def transport(response):
    value = MagicMock()
    value.open.return_value = response
    return value


def connection():
    return {"schema": "agent-browser.private-slack.v1", "token": TOKEN,
            "team_id": setup.TEAM, "bot_id": "B12345678", "channel_id": setup.CHANNEL,
            "scopes": ["groups:history", "canvases:read", "canvases:write"]}


class SetupTests(unittest.TestCase):
    def setUp(self):
        # Exercise transport/storage with inert data, never a token-shaped value.
        self.enterContext(patch.object(setup, "valid_token", side_effect=lambda value: value == TOKEN))
        self.temp = tempfile.TemporaryDirectory(prefix="private-slack-setup-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "private-slack"

    def directory(self):
        fd = setup.open_directory(self.root, create=True)
        self.addCleanup(os.close, fd)
        return fd

    def test_hidden_input_requires_tty_and_validates_without_echo_fallback(self):
        fake_tty = MagicMock()
        fake_tty.fileno.return_value = 88
        file_context = MagicMock()
        file_context.__enter__.return_value = fake_tty
        with patch.object(setup.os, "open", return_value=88), patch.object(setup.os, "fdopen", return_value=file_context), \
                patch.object(setup.os, "isatty", return_value=True), patch("builtins.input", side_effect=AssertionError("no fallback")):
            with patch.object(setup.getpass, "getpass", return_value=TOKEN) as prompt:
                self.assertEqual(setup.hidden_token(), TOKEN)
                self.assertIs(prompt.call_args.kwargs["stream"], fake_tty)
            def unsafe_fallback(*args, **kwargs):
                warnings.warn(SENTINEL, setup.getpass.GetPassWarning)
                raise AssertionError("fallback must be interrupted")
            with patch.object(setup.getpass, "getpass", side_effect=unsafe_fallback):
                with self.assertRaises(setup.SetupError) as error:
                    setup.hidden_token()
                self.assertEqual(str(error.exception), "private_slack_hidden_input_unavailable")
        with patch.object(setup.os, "open", side_effect=OSError(SENTINEL)):
            with self.assertRaises(setup.SetupError) as error:
                setup.hidden_token()
            self.assertNotIn(SENTINEL, str(error.exception))

    def test_harden_disables_core_dumps(self):
        with patch.object(setup.resource, "setrlimit") as limits:
            setup.harden()
            limits.assert_called_once_with(setup.resource.RLIMIT_CORE, (0, 0))

    def test_real_synthetic_pty_hides_token_and_restores_echo(self):
        master, slave = pty.openpty()
        code = (
            "import importlib.util,sys,termios; "
            "spec=importlib.util.spec_from_file_location('setup',sys.argv[1]); "
            "module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module); "
            "module.valid_token=lambda value: value=='NOT_A_CREDENTIAL_TEST_FIXTURE'; "
            "before=termios.tcgetattr(0)[3]; token=module.hidden_token(); "
            "assert token=='" + TOKEN + "'; "
            "assert termios.tcgetattr(0)[3]==before; print('synthetic_hidden_input_verified')"
        )
        def controlling_tty():
            os.setsid()
            fcntl.ioctl(0, termios.TIOCSCTTY, 0)
        process = None
        try:
            process = subprocess.Popen(
                [sys.executable, "-I", "-c", code, str(Path(setup.__file__).resolve())],
                stdin=slave, stdout=slave, stderr=slave, preexec_fn=controlling_tty,
            )
            os.close(slave)
            slave = None
            transcript = bytearray()
            sent = False
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                readable, _, _ = select.select([master], [], [], 0.1)
                if readable:
                    try:
                        chunk = os.read(master, 8192)
                    except OSError:
                        break
                    if not chunk:
                        break
                    transcript.extend(chunk)
                    if b"Slack bot token (hidden): " in transcript and not sent:
                        os.write(master, TOKEN.encode() + b"\n")
                        sent = True
                elif process.poll() is not None:
                    break
            self.assertTrue(sent)
            self.assertEqual(process.wait(timeout=2), 0)
            self.assertIn(b"synthetic_hidden_input_verified", transcript)
            self.assertNotIn(TOKEN.encode(), transcript)
        finally:
            if process is not None and process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            if slave is not None:
                os.close(slave)

    def test_tls_verified_proxy_free_and_redirects_refused(self):
        with patch.object(setup.urllib.request, "build_opener") as build:
            setup.opener()
        handlers = build.call_args.args
        self.assertEqual(handlers[0].proxies, {})
        self.assertTrue(handlers[1]._context.check_hostname)
        self.assertEqual(handlers[1]._context.verify_mode, ssl.CERT_REQUIRED)
        with self.assertRaises(setup.SetupError) as error:
            handlers[2].redirect_request(None, None, 302, SENTINEL, {}, "https://evil.example/")
        self.assertEqual(str(error.exception), "private_slack_redirect_rejected")

    def test_auth_only_post_and_fixed_workspace(self):
        client = transport(Response())
        value = setup.probe(TOKEN, client)
        self.assertEqual(value, {**connection(), "scopes": sorted(connection()["scopes"])})
        request = client.open.call_args.args[0]
        self.assertEqual(request.full_url, "https://slack.com/api/auth.test")
        self.assertEqual(request.method, "POST")
        self.assertEqual(request.data, b"")
        self.assertEqual(client.open.call_count, 1)
        self.assertEqual(client.open.call_args.kwargs["timeout"], setup.TIMEOUT)
        with self.assertRaises(setup.SetupError) as error:
            setup.probe(TOKEN, transport(Response({"ok": True, "team_id": "T99999999", "bot_id": "B12345678"})))
        self.assertEqual(str(error.exception), "private_slack_wrong_workspace")

    def test_scope_header_missing_excess_or_ambiguous_is_rejected(self):
        for scopes in [None, "canvases:read", "groups:history,chat:write", "groups:history,groups:history", ""]:
            with self.subTest(scopes=scopes), self.assertRaises(setup.SetupError):
                setup.probe(TOKEN, transport(Response(scopes=scopes)))
        response = Response()
        response.headers["X-OAuth-Scopes"] = "groups:history"
        with self.assertRaises(setup.SetupError):
            setup.probe(TOKEN, transport(response))
        self.assertEqual(setup.probe(TOKEN, transport(Response(scopes="groups:history")))["scopes"], ["groups:history"])

    def test_invalid_body_token_and_network_errors_never_reveal_values(self):
        for body in [{"ok": False, "error": SENTINEL}, [], {"ok": True, "team_id": setup.TEAM, "bot_id": SENTINEL}]:
            with self.assertRaises(setup.SetupError) as error:
                setup.probe(TOKEN, transport(Response(body)))
            self.assertNotIn(SENTINEL, str(error.exception))
        for token in [SENTINEL, TOKEN + "\n", "xoxp-123", TOKEN + " " + SENTINEL]:
            client = MagicMock()
            with self.assertRaises(setup.SetupError):
                setup.probe(token, client)
            client.open.assert_not_called()
        client = MagicMock()
        client.open.side_effect = OSError(SENTINEL)
        with self.assertRaises(setup.SetupError) as error:
            setup.probe(TOKEN, client)
        self.assertEqual(str(error.exception), "private_slack_auth_failed")
        response = Response()
        response.seek(0); response.write(b"x" * (setup.LIMIT + 1)); response.seek(0)
        with self.assertRaises(setup.SetupError):
            setup.probe(TOKEN, transport(response))
        with self.assertRaises(setup.SetupError):
            setup.decode_json(b'{"ok":true,"ok":false}')

    def test_owner_only_publication_and_existing_config_never_overwritten(self):
        directory = self.directory()
        setup.publish(directory, connection())
        path = self.root / setup.CONFIG
        before = path.read_bytes()
        self.assertEqual(self.root.stat().st_mode & 0o7777, 0o700)
        self.assertEqual(path.stat().st_mode & 0o7777, 0o600)
        self.assertEqual(path.stat().st_nlink, 1)
        self.assertTrue(setup.existing_connection(directory))
        with self.assertRaises(setup.SetupError):
            setup.publish(directory, {**connection(), "bot_id": "B87654321"})
        self.assertEqual(path.read_bytes(), before)
        self.assertEqual([entry.name for entry in self.root.iterdir()], [setup.CONFIG])

    def test_status_is_metadata_only_and_missing_status_creates_nothing(self):
        output = io.StringIO()
        with patch.object(setup, "harden"), patch.object(setup, "hidden_token") as hidden, patch.object(setup, "probe") as probe, contextlib.redirect_stdout(output):
            self.assertEqual(setup.run(self.root, status=True), 1)
            self.assertFalse(self.root.exists())
            setup.publish(self.directory(), connection())
            self.assertEqual(setup.run(self.root, status=True), 0)
            self.assertEqual(setup.run(self.root, status=False), 0)
            hidden.assert_not_called(); probe.assert_not_called()
        for value in [TOKEN, "B12345678", str(self.root), setup.TEAM, setup.CHANNEL]:
            self.assertNotIn(value, output.getvalue())

    def test_symlinks_modes_hardlinks_and_unsafe_ancestors_fail_closed(self):
        directory = self.directory()
        setup.publish(directory, connection())
        path = self.root / setup.CONFIG
        path.chmod(0o644)
        with self.assertRaises(setup.SetupError): setup.existing_connection(directory)
        path.chmod(0o600)
        alias = self.root / "second-link"
        os.link(path, alias)
        with self.assertRaises(setup.SetupError): setup.existing_connection(directory)
        alias.unlink()
        path.unlink(); path.symlink_to("missing")
        with self.assertRaises(OSError): setup.existing_connection(directory)
        directory_alias = Path(self.temp.name) / "alias"
        directory_alias.symlink_to(self.root)
        with self.assertRaises(OSError): setup.open_directory(directory_alias)
        self.root.chmod(0o755)
        with self.assertRaises(setup.SetupError): setup.open_directory(self.root)
        self.root.chmod(0o700)
        Path(self.temp.name).chmod(0o777)
        with self.assertRaises(setup.SetupError): setup.open_directory(self.root)
        Path(self.temp.name).chmod(0o700)

    def test_atomic_write_failure_has_no_partial_published_config(self):
        directory = self.directory()
        with patch.object(setup.os, "write", side_effect=OSError(SENTINEL)):
            with self.assertRaises(setup.SetupError) as error:
                setup.publish(directory, connection())
        self.assertEqual(str(error.exception), "private_slack_publication_failed")
        self.assertEqual(list(self.root.iterdir()), [])
        with patch.object(setup.os, "fsync", side_effect=[None, OSError(SENTINEL)]):
            with self.assertRaises(setup.SetupError) as error:
                setup.publish(directory, connection())
        self.assertEqual(str(error.exception), "private_slack_publication_uncertain")
        self.assertTrue(setup.existing_connection(directory))

    def test_unknown_arguments_and_unexpected_exception_diagnostics_are_fixed(self):
        output = io.StringIO()
        with contextlib.redirect_stderr(output):
            self.assertEqual(setup.main(["--token", TOKEN]), 2)
            with patch.object(setup, "run", side_effect=OSError(SENTINEL)):
                self.assertEqual(setup.main([]), 1)
        self.assertNotIn(TOKEN, output.getvalue())
        self.assertNotIn(SENTINEL, output.getvalue())


class ProductionTokenValidationTests(unittest.TestCase):
    def test_inert_fixtures_and_malformed_values_fail_before_network(self):
        # No validator mock here: production must reject every inert fixture.
        for value in (TOKEN, TOKEN + "\n", "xoxp-123", "", None, 123):
            with self.subTest(value=value):
                self.assertFalse(setup.valid_token(value))
                client = MagicMock()
                with self.assertRaises(setup.SetupError):
                    setup.probe(value, client)
                client.open.assert_not_called()


if __name__ == "__main__":
    unittest.main()
