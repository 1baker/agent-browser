#!/usr/bin/env python3
"""Synthetic LitScout adapter coverage for the local OS credential source."""

import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from litscout.app.context import ServiceContext
from litscout.app.credential_approvals import (
    CredentialDecisionRequest, CredentialPlanRequest, create_credential_plan,
    decide_credential_plan, get_credential_plan,
)
from litscout.app.jobs import _session_for
from litscout.config.settings import Secrets, ServiceConfig, ServicesConfig
from litscout.store.db import CredentialDiscovery


SPEC = importlib.util.spec_from_file_location(
    "private_sam_local_adapter", Path(__file__).with_name("private-sam-discovery.py"))
adapter = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(adapter)


class LocalAdapterTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        self.ctx = ServiceContext(db_path=str(root / "state.sqlite"), secrets=Secrets(),
            runtime={"secrets_path": str(root / "secrets.yml")},
            services=ServicesConfig(services={"sam_contract_awards": ServiceConfig(
                backend="fake", base_url="https://api.sam.gov", options={
                    "credential_lifecycle": {
                        "renew_url": "https://sam.gov/workspace/profile/account-details"}})}))

    def plan(self):
        plan = create_credential_plan(self.ctx, CredentialPlanRequest(
            service="sam_contract_awards", action="renew",
            account_email="operator@example.org", authentication={
                "mode": "sam_login_gov_password_backup_code",
                "profile_id": "sam-test", "session_name": "sam-test",
                "browser_id": "session:sam-test", "target_id": "A" * 32,
                "login_origin": "https://secure.login.gov",
                "discovery": {"kind": "local_os_credential",
                    "provider": "windows_credential_manager",
                    "store_id": adapter._LOCAL.STORE_ID},
            }))
        decide_credential_plan(self.ctx, plan["id"], CredentialDecisionRequest(
            decision="approve", manifest_sha256=plan["manifest_sha256"]))
        return plan

    def local_result(self):
        reference = adapter._LOCAL.LocalCredentialReference(
            adapter._LOCAL.STORE_ID, "133700000000000001")
        return adapter._LOCAL.LocalDiscoveryResult(reference, reference)

    def test_local_resolution_never_opens_slack_connection(self):
        plan = self.plan()
        with patch.object(adapter._DISCOVERY._SETUP, "harden"), \
                patch.object(adapter, "_connection") as connection, \
                patch.object(adapter._LOCAL, "discover", return_value=self.local_result()):
            result = adapter.resolve_approved_sources(
                self.ctx, plan["id"], plan["manifest_sha256"])
        self.assertEqual(result["state"], "resolved")
        self.assertEqual(result["references"]["credential_source"], {
            "kind": "local_os_credential", "store_id": adapter._LOCAL.STORE_ID,
            "last_written_filetime": "133700000000000001"})
        self.assertEqual(result["references"]["scanned_messages"], 0)
        connection.assert_not_called()

    def test_local_private_material_enters_only_consumed_callback(self):
        plan = self.plan()
        material = adapter._LOCAL.PrivateMaterial(
            "operator@example.org", "SYNTHETIC_PASSWORD_SENTINEL!", ("ABCD-1234-5678",))
        observed = []
        with patch.object(adapter._DISCOVERY._SETUP, "harden"), \
                patch.object(adapter, "_connection") as connection, \
                patch.object(adapter._LOCAL, "discover", return_value=self.local_result()), \
                patch.object(adapter._LOCAL, "extract", return_value=material):
            result = adapter.execute_approved_sources(
                self.ctx, plan["id"], plan["manifest_sha256"],
                lambda manifest, secret: observed.append((manifest["request_id"], secret.password)))
        self.assertEqual(result["state"], "private_handoff_completed")
        self.assertEqual(observed, [(plan["id"], "SYNTHETIC_PASSWORD_SENTINEL!")])
        self.assertEqual(get_credential_plan(self.ctx, plan["id"])["state"], "consumed")
        connection.assert_not_called()

    def test_matching_persisted_revision_drift_never_reaches_extraction(self):
        plan = self.plan()
        original_resolve = adapter._resolve

        def tamper(ctx, plan_id, digest, retained):
            receipt = original_resolve(ctx, plan_id, digest, retained)
            with _session_for(self.ctx.db_path, self.ctx.db_url) as session:
                row = session.get(CredentialDiscovery, plan["id"])
                changed = dict(row.references_json)
                for name in ("credential_source", "backup_code_source"):
                    changed[name] = dict(changed[name])
                    changed[name]["last_written_filetime"] = "133700000000000002"
                row.references_json = changed
                session.add(row)
                session.commit()
            return receipt

        with patch.object(adapter._DISCOVERY._SETUP, "harden"), \
                patch.object(adapter._LOCAL, "discover", return_value=self.local_result()), \
                patch.object(adapter, "_resolve", side_effect=tamper), \
                patch.object(adapter._LOCAL, "extract") as extract:
            with self.assertRaisesRegex(ValueError, "private_sam_handoff_failed"):
                adapter.execute_approved_sources(
                    self.ctx, plan["id"], plan["manifest_sha256"], lambda *_: None)
        extract.assert_not_called()
        self.assertEqual(get_credential_plan(self.ctx, plan["id"])["state"], "consumed")


if __name__ == "__main__":
    unittest.main()
