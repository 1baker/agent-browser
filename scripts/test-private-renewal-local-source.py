#!/usr/bin/env python3
"""Coordinator validation for the local OS credential source."""

import importlib.util
from pathlib import Path
import unittest

from litscout.app.credential_approvals import (
    CredentialDecisionRequest, CredentialPlanRequest, create_credential_plan,
    decide_credential_plan,
)


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


coordinator = load("renewal_local_coordinator", "private-renewal-coordinator.py")
fixtures = load("renewal_local_fixtures", "test-private-renewal-coordinator.py")


class LocalSourceCoordinatorTests(unittest.TestCase):
    def setUp(self):
        self.fixture = fixtures.CoordinatorTests("test_create_once_private_and_unchanged")
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        plan = create_credential_plan(self.fixture.ctx, CredentialPlanRequest(
            service="sam_contract_awards", action="renew",
            account_email="operator@example.org", authentication={
                "mode": "sam_login_gov_password_backup_code",
                "profile_id": "sam-test", "session_name": "sam-test",
                "browser_id": "session:sam-test", "target_id": "A" * 32,
                "login_origin": "https://secure.login.gov",
                "discovery": {"kind": "local_os_credential",
                    "provider": "windows_credential_manager",
                    "store_id": coordinator._SAM._LOCAL.STORE_ID},
            }))
        decide_credential_plan(self.fixture.ctx, plan["id"], CredentialDecisionRequest(
            decision="approve", manifest_sha256=plan["manifest_sha256"]))
        self.fixture.plan = plan
        for operation in self.fixture.operations:
            operation["consent_sha256"] = plan["manifest_sha256"]
        self.fixture.operations[1]["operation"]["source_scope"] = (
            "local_os_credential:" + coordinator._SAM._LOCAL.STORE_ID)

    def test_local_source_scope_prepares_without_secret_material(self):
        result = self.fixture.prepare()
        self.assertEqual(result["state"], "private_authority_prepared")

    def test_local_source_scope_drift_fails_closed(self):
        for value in [
            "local_os_credential:other_slot",
            coordinator._SAM._LOCAL.STORE_ID,
            "slack:C07CA08AKUH:1788850000.000002",
        ]:
            self.fixture.operations[1]["operation"]["source_scope"] = value
            with self.assertRaisesRegex(ValueError, coordinator.FAILED):
                self.fixture.prepare()
            self.assertFalse((self.fixture.root / "execution.json").exists())


if __name__ == "__main__":
    unittest.main()
