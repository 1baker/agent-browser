"""Cross-repository synthetic adapter tests; run with LitScout on PYTHONPATH."""
import importlib.util
import email.message
import io
import json
import contextlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch, MagicMock

from litscout.app.context import ServiceContext
from litscout.app.credential_approvals import (
    CredentialPlanRequest, CredentialDecisionRequest,
    create_credential_plan, decide_credential_plan,
)
from litscout.app.jobs import _session_for
from litscout.config.settings import Secrets, ServicesConfig, ServiceConfig
from litscout.store.db import CredentialDiscovery, CredentialApproval

SPEC = importlib.util.spec_from_file_location("sam_adapter", Path(__file__).with_name("private-sam-discovery.py"))
adapter = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(adapter)


class AdapterTests(unittest.TestCase):
    def setUp(self):
        self.enterContext(patch.object(adapter._DISCOVERY._SETUP, "valid_token",
                                      side_effect=lambda value: value == "NOT_A_CREDENTIAL_TEST_FIXTURE"))
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        self.ctx = ServiceContext(db_path=str(root / "state.sqlite"), secrets=Secrets(),
            runtime={"secrets_path": str(root / "secrets.yml")},
            services=ServicesConfig(services={"sam_contract_awards": ServiceConfig(
                backend="fake", base_url="https://api.sam.gov", options={
                    "credential_lifecycle": {"renew_url": "https://sam.gov/workspace/profile/account-details"}})}))
        self.plan = create_credential_plan(self.ctx, CredentialPlanRequest(
            service="sam_contract_awards", action="renew", account_email="operator@example.org",
            authentication={"mode": "sam_login_gov_password_backup_code",
                "profile_id": "sam-test", "session_name": "sam-test", "browser_id": "session:sam-test",
                "target_id": "A" * 32, "login_origin": "https://secure.login.gov",
                "discovery": {"kind": "slack_channel", "team_id": "TEG3AC109", "channel_id": "C07CA08AKUH",
                    "oldest_ts": "1788800000.000000", "latest_ts": "1788900000.000000",
                    "max_messages": 50, "max_pages": 5}}))

    def approve(self):
        decide_credential_plan(self.ctx, self.plan["id"], CredentialDecisionRequest(
            decision="approve", manifest_sha256=self.plan["manifest_sha256"]))

    def run_adapter(self):
        with patch.object(adapter._DISCOVERY._SETUP, "harden"):
            return adapter.resolve_approved_sources(self.ctx, self.plan["id"], self.plan["manifest_sha256"])

    def test_unapproved_plan_never_opens_connection(self):
        with patch.object(adapter, "_connection") as connection:
            with self.assertRaisesRegex(ValueError, "private_sam_discovery_failed"):
                self.run_adapter()
            connection.assert_not_called()

    def test_actual_admission_to_private_collector_bridge_and_no_replay(self):
        self.approve()
        module = adapter._DISCOVERY
        def collect(connection, authority):
            self.assertEqual(connection, "SYNTHETIC_PRIVATE_CONNECTION")
            self.assertEqual(authority.channel_id, "C07CA08AKUH")
            self.assertEqual(authority.account_email, "operator@example.org")
            self.assertEqual(authority.max_messages, 50)
            with _session_for(self.ctx.db_path, self.ctx.db_url) as session:
                self.assertEqual(session.get(CredentialDiscovery, self.plan["id"]).state, "started")
            return module.DiscoveryResult(module.MessageReference(authority.channel_id, "1788850000.000001"),
                module.MessageReference(authority.channel_id, "1788850000.000002"), 2, 1)
        with patch.object(adapter, "_connection", return_value="SYNTHETIC_PRIVATE_CONNECTION") as connection, \
                patch.object(module, "discover", side_effect=collect) as discovery:
            result = self.run_adapter()
            self.assertEqual(result["state"], "resolved")
            self.assertFalse(result["renewal_enabled"])
            with self.assertRaises(ValueError):
                self.run_adapter()
            self.assertEqual(connection.call_count, 1)
            self.assertEqual(discovery.call_count, 1)

    def test_private_connection_error_is_fixed_and_consumed(self):
        self.approve()
        with patch.object(adapter, "_connection", side_effect=RuntimeError("SECRET_SENTINEL")) as connection:
            for _ in range(2):
                with self.assertRaises(ValueError) as error:
                    self.run_adapter()
                self.assertEqual(str(error.exception), "private_sam_discovery_failed")
            self.assertEqual(connection.call_count, 1)

    def private_fixture(self):
        token = "NOT_A_CREDENTIAL_TEST_FIXTURE"
        connection = dict(schema="agent-browser.private-slack.v1", token=token,
                          team_id="TEG3AC109", channel_id="C07CA08AKUH", bot_id="B12345678", scopes=["groups:history"])
        first = dict(type="message", ts="1788850000.000001", text="login.gov\nemail: operator@example.org\npassword: SECRET_SENTINEL")
        second = dict(type="message", ts="1788850000.000002", text="login.gov\nemail: operator@example.org\nbackup code: ABCD-1234-5678")
        auth = dict(ok=True, team_id=connection["team_id"], bot_id=connection["bot_id"])
        def page(messages):
            return dict(ok=True, messages=messages, has_more=False, response_metadata={"next_cursor": ""})
        class Response(io.BytesIO):
            status = 200
            def __init__(self, value):
                super().__init__(json.dumps(value).encode())
                self.headers = email.message.Message()
                self.headers["Content-Type"] = "application/json"
                self.headers["X-OAuth-Scopes"] = "groups:history"
        transport = MagicMock()
        transport.open.side_effect = [Response(value) for value in (
            auth, page([first, second]), auth, page([first]), page([second]))]
        return connection, transport

    def test_real_discovery_provenance_survives_claim_to_actual_extract(self):
        self.approve()
        connection, transport = self.private_fixture()
        calls = []
        def consume(manifest, material):
            with _session_for(self.ctx.db_path, self.ctx.db_url) as session:
                self.assertEqual(session.get(CredentialApproval, self.plan["id"]).state, "consumed")
                self.assertEqual(session.get(CredentialDiscovery, self.plan["id"]).state, "resolved")
            self.assertEqual(material.password, "SECRET_SENTINEL")
            self.assertEqual(material.backup_codes, ("ABCD-1234-5678",))
            calls.append(1)
            return "SECRET_SENTINEL"
        output = io.StringIO()
        with patch.object(adapter, "_connection", return_value=connection), \
                patch.object(adapter._DISCOVERY._SETUP, "harden"), \
                patch.object(adapter._DISCOVERY._SETUP, "opener", return_value=transport), \
                contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            receipt = adapter.execute_approved_sources(self.ctx, self.plan["id"], self.plan["manifest_sha256"], consume)
            with self.assertRaises(ValueError):
                adapter.execute_approved_sources(self.ctx, self.plan["id"], self.plan["manifest_sha256"], consume)
        self.assertEqual(calls, [1])
        self.assertEqual(transport.open.call_count, 5)
        self.assertEqual(output.getvalue(), "")
        self.assertNotIn("SECRET_SENTINEL", repr(receipt))

    def test_callback_failure_leaves_execution_consumed(self):
        self.approve()
        connection, transport = self.private_fixture()
        with patch.object(adapter, "_connection", return_value=connection), \
                patch.object(adapter._DISCOVERY._SETUP, "harden"), \
                patch.object(adapter._DISCOVERY._SETUP, "opener", return_value=transport):
            def fail(*_):
                raise RuntimeError("SECRET_SENTINEL")
            with self.assertRaises(ValueError) as error:
                adapter.execute_approved_sources(self.ctx, self.plan["id"], self.plan["manifest_sha256"], fail)
            self.assertEqual(str(error.exception), "private_sam_handoff_failed")
        with _session_for(self.ctx.db_path, self.ctx.db_url) as session:
            self.assertEqual(session.get(CredentialApproval, self.plan["id"]).state, "consumed")

    def test_extraction_failure_never_calls_consumer_and_remains_consumed(self):
        self.approve()
        connection, transport = self.private_fixture()
        consumer = MagicMock()
        with patch.object(adapter, "_connection", return_value=connection), \
                patch.object(adapter._DISCOVERY._SETUP, "harden"), \
                patch.object(adapter._DISCOVERY._SETUP, "opener", return_value=transport), \
                patch.object(adapter._DISCOVERY, "extract", side_effect=RuntimeError("SECRET_SENTINEL")):
            with self.assertRaises(ValueError) as error:
                adapter.execute_approved_sources(self.ctx, self.plan["id"], self.plan["manifest_sha256"], consumer)
            self.assertEqual(str(error.exception), "private_sam_handoff_failed")
        consumer.assert_not_called()
        with _session_for(self.ctx.db_path, self.ctx.db_url) as session:
            self.assertEqual(session.get(CredentialApproval, self.plan["id"]).state, "consumed")


if __name__ == "__main__":
    unittest.main()
