"""Real Python/Rust local socket tests. Synthetic records and isolated roots only."""
import importlib.util
import hmac
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


client = module("controller_client_test", "private-controller-client.py")
adapter = module("controller_adapter_test", "private-sam-discovery.py")


class ControllerTests(unittest.TestCase):
    def setUp(self):
        self.binary = os.environ["AGENT_BROWSER_PRIVATE_CONTROLLER_TEST_BINARY"]
        self.temp = tempfile.TemporaryDirectory(prefix="ab-controller-fixture-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "private"
        result = subprocess.run([self.binary, "private-controller", "setup", str(self.root)],
                                capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, "isolated setup failed")
        self.process = subprocess.Popen(
            [self.binary, "private-controller", "serve", str(self.root)],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.addCleanup(self.stop)
        deadline = time.monotonic() + 5
        while not (self.root / "controller.sock").exists():
            self.assertIsNone(self.process.poll(), "isolated server exited")
            if time.monotonic() >= deadline:
                self.fail("isolated server startup timeout")
            time.sleep(0.01)

    def stop(self):
        if self.process.poll() is None:
            self.process.terminate()
        self.process.communicate(timeout=5)

    def binding(self):
        return {"operation": "bind", "planId": "a" * 32, "manifestSha256": "b" * 64,
                "profileId": "sam-test", "browserId": "session:sam-test", "sessionName": "sam-test",
                "targetId": "A" * 32, "endpoint": "ws://127.0.0.1:9222/devtools/browser/test",
                "expiresAt": "2099-01-01T00:00:00Z"}

    def test_authenticated_probe_binding_and_second_server_refusal(self):
        self.assertEqual(client.probe(self.root), {"success": True, "renewalEnabled": False})
        body = self.binding()
        self.assertTrue(client._request(body, self.root)["success"])
        with self.assertRaisesRegex(ValueError, client.FAILED):
            client._request(body, self.root)
        changed = {**body, "manifestSha256": "c" * 64}
        with self.assertRaises(ValueError):
            client._request(changed, self.root)
        self.assertEqual((self.root / "authentication.key").stat().st_mode & 0o777, 0o600)
        self.assertEqual(self.root.stat().st_mode & 0o777, 0o700)
        # A second server cannot take over or unlink the first server's socket.
        second = subprocess.run([self.binary, "private-controller", "serve", str(self.root)],
                                capture_output=True, timeout=5)
        self.assertNotEqual(second.returncode, 0)
        self.assertTrue(client.probe(self.root)["success"])

    def test_rejects_bad_mac_unknown_action_and_expired_binding(self):
        with socket.socket(socket.AF_UNIX) as stream:
            stream.settimeout(3)
            stream.connect(str(self.root / "controller.sock"))
            challenge = stream.recv(32, socket.MSG_WAITALL)
            self.assertEqual(len(challenge), 32)
            body = b'{"operation":"probe"}'
            stream.sendall(struct.pack("!I", len(body)) + body + bytes(32))
            stream.shutdown(socket.SHUT_WR)
            # Refusal may close without response or provide a fixed error.
            data = stream.recv(1024)
            self.assertNotIn(b'"success":true', data)
        for body in [{"operation": "renew"}, {"operation": "probe", "extra": "forbidden"},
                     {**self.binding(), "expiresAt": "2000-01-01T00:00:00Z"}]:
            with self.assertRaises(ValueError):
                client._request(body, self.root)
        self.assertTrue(client.probe(self.root)["success"])

    def test_insecure_key_is_refused_without_repair(self):
        path = self.root / "authentication.key"
        path.chmod(0o644)
        with self.assertRaises(ValueError):
            client.probe(self.root)
        self.assertEqual(path.stat().st_mode & 0o777, 0o644)

    def test_captured_request_cannot_replay_with_new_challenge(self):
        key = (self.root / "authentication.key").read_bytes()
        saved = None
        for replay in (False, True):
            with socket.socket(socket.AF_UNIX) as stream:
                stream.settimeout(3)
                stream.connect(str(self.root / "controller.sock"))
                challenge = stream.recv(32, socket.MSG_WAITALL)
                body = b'{"operation":"probe"}'
                if not replay:
                    saved = struct.pack("!I", len(body)) + body + hmac.digest(
                        key, b"ABPC1-request\0" + challenge + body, "sha256")
                stream.sendall(saved)
                stream.shutdown(socket.SHUT_WR)
                length = struct.unpack("!I", stream.recv(4, socket.MSG_WAITALL))[0]
                response = stream.recv(length, socket.MSG_WAITALL)
                tag = stream.recv(32, socket.MSG_WAITALL)
                self.assertTrue(hmac.compare_digest(tag, hmac.digest(
                    key, b"ABPC1-response\0" + challenge + response, "sha256")))
                self.assertIs(json.loads(response)["success"], not replay)

    def test_litscout_claim_precedes_real_socket_binding_and_retry_is_blocked(self):
        from litscout.app.context import ServiceContext
        from litscout.app.credential_approvals import (
            CredentialPlanRequest, CredentialDecisionRequest, create_credential_plan, decide_credential_plan)
        from litscout.app.credential_discovery import run_credential_discovery
        from litscout.app.jobs import _session_for
        from litscout.config.settings import Secrets, ServicesConfig, ServiceConfig
        from litscout.store.db import CredentialApproval
        ctx = ServiceContext(db_path=str(Path(self.temp.name) / "state.sqlite"), secrets=Secrets(),
            runtime={"secrets_path": str(Path(self.temp.name) / "secrets.yml")},
            services=ServicesConfig(services={"sam_contract_awards": ServiceConfig(backend="fake",
                base_url="https://api.sam.gov", options={"credential_lifecycle": {
                    "renew_url": "https://sam.gov/workspace/profile/account-details"}})}))
        auth = {"mode": "sam_login_gov_password_backup_code", "profile_id": "sam-test",
                "session_name": "sam-test", "browser_id": "session:sam-test", "target_id": "A" * 32,
                "login_origin": "https://secure.login.gov", "discovery": {
                    "kind": "slack_channel", "team_id": "TEG3AC109", "channel_id": "C07CA08AKUH",
                    "oldest_ts": "1788800000.000000", "latest_ts": "1788900000.000000",
                    "max_messages": 50, "max_pages": 5}}
        plan = create_credential_plan(ctx, CredentialPlanRequest(service="sam_contract_awards", action="renew",
            account_email="operator@example.org", authentication=auth))
        retained = {name: self.binding()[name] for name in (
            "profileId", "browserId", "sessionName", "targetId", "endpoint")}
        retained["ready"] = True
        for field in ("profileId", "browserId", "sessionName", "targetId", "ready"):
            changed = {**retained, field: False if field == "ready" else "foreign"}
            with patch.object(client, "_request") as dispatch:
                with self.assertRaises(ValueError):
                    client.bind(plan["manifest"], plan["manifest_sha256"], changed, self.root)
                dispatch.assert_not_called()
        calls = []
        def send(body, root=None):
            with _session_for(ctx.db_path, ctx.db_url) as session:
                self.assertEqual(session.get(CredentialApproval, plan["id"]).state, "consumed")
            calls.append(1)
            return client._request(body, self.root)
        with patch.object(adapter._CONTROLLER, "_request", side_effect=send):
            with self.assertRaises(ValueError):
                adapter.bind_approved_controller(ctx, plan["id"], plan["manifest_sha256"], retained)
            self.assertEqual(calls, [])
            decide_credential_plan(ctx, plan["id"], CredentialDecisionRequest(
                decision="approve", manifest_sha256=plan["manifest_sha256"]))
            ref = {"channel_id": "C07CA08AKUH", "message_ts": "1788850000.000001"}
            run_credential_discovery(ctx, plan["id"], plan["manifest_sha256"], lambda _: {
                "credential_source": ref, "backup_code_source": ref, "scanned_messages": 1, "scanned_pages": 1})
            result = adapter.bind_approved_controller(ctx, plan["id"], plan["manifest_sha256"], retained)
            self.assertEqual(result["state"], "controller_binding_stored")
            self.assertFalse(result["renewal_enabled"])
            with self.assertRaises(ValueError):
                adapter.bind_approved_controller(ctx, plan["id"], plan["manifest_sha256"], retained)
            self.assertEqual(calls, [1])


if __name__ == "__main__":
    unittest.main()
