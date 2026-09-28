"""Offline authority and Unix-channel tests. No browser or real credentials."""
from copy import deepcopy
import hashlib
import hmac
import importlib.util
import json
import os
from pathlib import Path
import socket
import struct
import threading
import time
import unittest
from unittest.mock import patch


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


coordinator = load("renewal_coordinator", "private-renewal-coordinator.py")
fixtures = load("renewal_fixtures", "test-private-sam-discovery.py")


class CoordinatorTests(unittest.TestCase):
    def setUp(self):
        self.fixture = fixtures.AdapterTests("test_unapproved_plan_never_opens_connection")
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.ctx, self.plan = self.fixture.ctx, self.fixture.plan
        self.fixture.approve()
        self.root = Path(self.ctx.db_path).parent / "controller"
        self.root.mkdir(mode=0o700)
        fd = os.open(self.root / "authentication.key", os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, "wb") as output:
            output.write(b"a" * 32)
        self.key = b"a" * 32
        self.manifest = coordinator._current(self.ctx, self.plan["id"], self.plan["manifest_sha256"], "approved")
        handle = {"profileId": "sam-test", "browserId": "session:sam-test", "sessionName": "sam-test",
                  "targetId": "A" * 32, "tabId": "tab-test", "leaseId": "sam-test", "leaseState": "exclusive",
                  "leaseHeartbeatExpected": True, "valid": True, "staleReason": None}
        stages = [
            dict(kind="login", email_selector="#email", password_selector="#password", submit_selector="#login",
                 email="operator@example.org", password=coordinator.PASSWORD_SLOT),
            dict(kind="backup_code", code_selector="#code", submit_selector="#submit", account_selector="#account",
                 account_value="operator@example.org", code=coordinator.CODE_SLOT,
                 source_scope="C07CA08AKUH:1788850000.000002"),
            dict(kind="renew", submit_selector="#renew", account_selector="#account", account_value="operator@example.org"),
            dict(kind="read_key", selector="#key", account_selector="#account", account_value="operator@example.org"),
        ]
        self.operations = []
        for index, operation in enumerate(stages):
            origin = "https://secure.login.gov" if index < 2 else "https://sam.gov"
            url = origin + "/fixture-" + str(index)
            self.operations.append(dict(schema="agent-browser.private-operation.v1", service_tab_handle=dict(handle, url=url),
                expected_origin=origin, expected_url=url, consent_sha256=self.plan["manifest_sha256"],
                account_scope="login.gov:operator@example.org", operation=operation))
        self.retained = dict(ready=True, endpoint="ws://127.0.0.1:19222/devtools/browser/fixture",
                             serviceTabHandle=self.operations[0]["service_tab_handle"])

    def prepare(self):
        return coordinator.prepare_authority(self.ctx, self.plan["id"], self.plan["manifest_sha256"],
                                             self.operations, self.retained, self.root)

    def test_create_once_private_and_unchanged(self):
        self.prepare()
        path = self.root / "execution.json"
        before = path.read_bytes()
        self.assertEqual(path.stat().st_mode & 0o777, 0o600)
        with self.assertRaisesRegex(ValueError, coordinator.FAILED):
            self.prepare()
        self.assertEqual(before, path.read_bytes())

    def test_template_drift_and_secrets_rejected(self):
        for mutate in (
            lambda ops: ops[0]["operation"].update(password="SECRET-CANARY"),
            lambda ops: ops[1]["service_tab_handle"].update(targetId="other"),
            lambda ops: ops[2].update(expected_url="https://example.org/"),
            lambda ops: ops[1]["operation"].update(source_scope="OTHER:1788850000.000002"),
        ):
            original = deepcopy(self.operations)
            mutate(self.operations)
            with self.assertRaisesRegex(ValueError, coordinator.FAILED):
                self.prepare()
            self.assertFalse((self.root / "execution.json").exists())
            self.operations = original

    def test_recipe_utf8_canonicalization(self):
        self.operations[0]["operation"]["email_selector"] = "#é"
        self.prepare()
        authority = json.loads((self.root / "execution.json").read_bytes())
        expected = hashlib.sha256(json.dumps(self.operations, sort_keys=True, ensure_ascii=False,
                                              separators=(",", ":")).encode()).hexdigest()
        self.assertEqual(authority["recipeSha256"], expected)

    def test_unapproved_never_creates_authority(self):
        from litscout.app.credential_approvals import decide_credential_plan, CredentialDecisionRequest
        decide_credential_plan(self.ctx, self.plan["id"], CredentialDecisionRequest(
            decision="revoke", manifest_sha256=self.plan["manifest_sha256"]))
        with self.assertRaisesRegex(ValueError, coordinator.FAILED):
            self.prepare()
        self.assertFalse((self.root / "execution.json").exists())

    def test_symlink_key_rejected(self):
        key_path = self.root / "authentication.key"
        key_path.rename(self.root / "actual-key")
        key_path.symlink_to(self.root / "actual-key")
        with self.assertRaisesRegex(ValueError, coordinator.FAILED):
            self.prepare()
        self.assertFalse((self.root / "execution.json").exists())

    def test_weak_key_permissions_rejected(self):
        os.chmod(self.root / "authentication.key", 0o644)
        with self.assertRaisesRegex(ValueError, coordinator.FAILED):
            self.prepare()
        self.assertFalse((self.root / "execution.json").exists())

    def execute_fixture(self, bad_proof=False, bad_completion=False, codes=("1234-5678",), slow_extract=False):
        self.prepare()
        authority = json.loads((self.root / "execution.json").read_bytes())
        listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.addCleanup(listener.close)
        listener.bind(str(self.root / "executor.sock"))
        os.chmod(self.root / "executor.sock", 0o600)
        listener.listen(1)
        listener.settimeout(3)
        received, errors = [], []

        def server():
            try:
                with listener.accept()[0] as stream:
                    def receive(n):
                        return coordinator._receive(stream, n, time.monotonic() + 3)
                    nonce = receive(32)
                    context = nonce + self.plan["id"].encode() + self.plan["manifest_sha256"].encode() + authority["recipeSha256"].encode()
                    stream.sendall(b"x" * 32 if bad_proof else hmac.digest(self.key, b"ABPX1-server\0" + context, "sha256"))
                    if bad_proof or len(codes) != 1:
                        self.assertEqual(stream.recv(1), b"")
                        return
                    header = receive(10)
                    body = receive(struct.unpack("!I", header[6:])[0])
                    self.assertEqual(header[:6], b"ABPH1\0")
                    self.assertEqual(receive(32), hmac.digest(self.key, header + body, "sha256"))
                    received.append(json.loads(body))
                    stream.sendall(b"x" * 32 if bad_completion else hmac.digest(self.key, b"ABPX1-complete\0" + context, "sha256"))
            except BaseException as error:
                errors.append(error)

        thread = threading.Thread(target=server, daemon=True)
        thread.start()
        discovery = coordinator._SAM._DISCOVERY
        def collect(connection, scope):
            return discovery.DiscoveryResult(discovery.MessageReference(scope.channel_id, "1788850000.000001"),
                discovery.MessageReference(scope.channel_id, "1788850000.000002"), 2, 1)
        material = discovery.PrivateMaterial("operator@example.org", "SECRET-CANARY", codes)
        def extraction(*args):
            if slow_extract:
                # Model source extraction finishing six seconds later without
                # sleeping or calling a network service. Socket read deadlines
                # are established separately after each protocol phase.
                clock_patch = patch.object(coordinator.time, "monotonic", side_effect=lambda: original_clock() + 6)
                self.enterContext(clock_patch)
            return material
        original_clock = time.monotonic
        with patch.object(coordinator._SAM, "_connection", return_value="SYNTHETIC"), \
                patch.object(discovery, "discover", side_effect=collect), \
                patch.object(discovery, "extract", side_effect=extraction) as extract, \
                patch.object(coordinator._SAM._CONTROLLER, "bind", return_value={}) as bind, \
                patch("litscout.app.credential_delivery.receive_sam_credential_delivery", return_value={"state": "installed"}) as deliver:
            args = (self.ctx, self.plan["id"], self.plan["manifest_sha256"], self.root)
            if bad_proof or bad_completion or len(codes) != 1:
                with self.assertRaisesRegex(ValueError, coordinator.FAILED):
                    coordinator.execute_approved_renewal(*args)
                deliver.assert_not_called()
            else:
                self.assertEqual(coordinator.execute_approved_renewal(*args), {"state": "installed"})
                deliver.assert_called_once()
            if bad_proof:
                extract.assert_not_called()
                bind.assert_not_called()
            with self.assertRaisesRegex(ValueError, coordinator.FAILED):
                coordinator.execute_approved_renewal(*args)
        thread.join(4)
        self.assertFalse(thread.is_alive())
        self.assertEqual(errors, [])
        return received

    def test_channel_success_and_no_replay(self):
        received = self.execute_fixture()
        self.assertEqual(received[0][0]["operation"]["password"], "SECRET-CANARY")

    def test_forged_server_never_extracts(self):
        self.execute_fixture(bad_proof=True)

    def test_forged_completion_never_installs(self):
        self.execute_fixture(bad_completion=True)

    def test_ambiguous_backup_codes_never_dispatch(self):
        self.execute_fixture(codes=("1234-5678", "2345-6789"))

    def test_extraction_past_five_seconds_still_hands_off(self):
        received = self.execute_fixture(slow_extract=True)
        self.assertEqual(received[0][0]["operation"]["password"], "SECRET-CANARY")

    def test_endpoint_query_rejected(self):
        self.retained["endpoint"] += "?unexpected=query"
        with self.assertRaisesRegex(ValueError, coordinator.FAILED):
            self.prepare()


if __name__ == "__main__":
    unittest.main()
