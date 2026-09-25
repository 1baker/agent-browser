"""Synthetic offline discovery tests; no credentials, network or private files."""
import contextlib
import email.message
import importlib.util
import io
import json
import copy
import pickle
from pathlib import Path
import unittest
from unittest.mock import MagicMock, patch
import urllib.parse
import urllib.error

SPEC = importlib.util.spec_from_file_location("private_discovery", Path(__file__).with_name("private-slack-discovery.py"))
discovery = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(discovery)
TOKEN = "NOT_A_CREDENTIAL_TEST_FIXTURE"
ACCOUNT = "operator@example.org"
PASSWORD = "SYNTHETIC_PASSWORD_SENTINEL"
CODE = "ABCD-1234-5678"


def authority(**changes):
    values = dict(team_id=discovery._SETUP.TEAM, channel_id=discovery._SETUP.CHANNEL,
                  account_email=ACCOUNT, oldest_ts="1700000000.000000", latest_ts="1800000000.000000",
                  max_messages=100, max_pages=5)
    return discovery.DiscoveryAuthority(**{**values, **changes})


def connection():
    return dict(schema="agent-browser.private-slack.v1", token=TOKEN,
                team_id=discovery._SETUP.TEAM, channel_id=discovery._SETUP.CHANNEL,
                bot_id="B12345678", scopes=["groups:history"])


def message(ts, text, **extra):
    return dict(type="message", ts=ts, text=text, **extra)


def credentials():
    return message("1780000000.000001", f"login.gov\nemail: {ACCOUNT}\npassword: {PASSWORD}")


def backup():
    return message("1780000000.000002", f"login.gov\naccount: {ACCOUNT}\nbackup codes:\n- {CODE}")


def page(messages, cursor="", more=False, **extra):
    return dict(ok=True, messages=messages, has_more=more, response_metadata=dict(next_cursor=cursor), **extra)


class Response(io.BytesIO):
    def __init__(self, body):
        super().__init__(json.dumps(body).encode())
        self.status = 200
        self.headers = email.message.Message()
        self.headers["Content-Type"] = "application/json"
        self.headers["X-OAuth-Scopes"] = "groups:history"


class DiscoveryTests(unittest.TestCase):
    def setUp(self):
        self.enterContext(patch.object(discovery._SETUP, "valid_token", side_effect=lambda value: value == TOKEN))
        self.harden = patch.object(discovery._SETUP, "harden")
        self.harden.start()
        self.addCleanup(self.harden.stop)

    def client(self, pages, auth=None):
        client = MagicMock()
        client.open.side_effect = [Response(auth or {"ok": True, "team_id": discovery._SETUP.TEAM, "bot_id": "B12345678"})] + [Response(value) for value in pages]
        return client

    def execute(self, pages, grant=None):
        return discovery.discover(connection(), grant or authority(), self.client(pages))

    def test_exact_refs_counts_and_private_repr_without_body_output(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            result = self.execute([page([credentials(), backup()])])
        self.assertEqual(result.credential_source.message_ts, credentials()["ts"])
        self.assertEqual(result.backup_code_source.message_ts, backup()["ts"])
        self.assertEqual((result.scanned_messages, result.scanned_pages), (2, 1))
        self.assertEqual(output.getvalue(), "")
        for value in [PASSWORD, CODE, ACCOUNT, TOKEN]:
            self.assertNotIn(value, repr(result))
            self.assertNotIn(value, repr(authority()))

    def test_cursor_is_followed_even_for_short_page_and_scope_never_changes(self):
        client = self.client([page([credentials()], cursor="opaque_cursor", more=True), page([backup()])])
        result = discovery.discover(connection(), authority(), client)
        self.assertEqual(result.scanned_pages, 2)
        self.assertEqual(client.open.call_count, 3)
        for call in client.open.call_args_list[1:]:
            request = call.args[0]
            query = urllib.parse.parse_qs(urllib.parse.urlsplit(request.full_url).query)
            self.assertEqual(query["channel"], [discovery._SETUP.CHANNEL])
            self.assertEqual(query["oldest"], [authority().oldest_ts])
            self.assertEqual(query["latest"], [authority().latest_ts])
            self.assertNotIn(ACCOUNT, request.full_url)
            self.assertNotIn(TOKEN, request.full_url)

    def test_incomplete_limits_missing_cursor_loop_and_retention_fail_closed(self):
        cases = [
            ([page([credentials(), backup()], cursor="next", more=True)], authority(max_messages=2)),
            ([page([credentials(), backup()], cursor="next", more=True)], authority(max_pages=1)),
            ([page([credentials(), backup()], more=True)], authority()),
            ([page([credentials()], cursor="next", more=True), page([backup()], cursor="next", more=True)], authority()),
            ([page([credentials(), backup()], is_limited=True)], authority()),
        ]
        for pages, grant in cases:
            with self.subTest(case=len(pages)), self.assertRaises(discovery.DiscoveryError) as error:
                self.execute(pages, grant)
            self.assertEqual(str(error.exception), "private_slack_discovery_incomplete")

    def test_ambiguous_candidates_accounts_and_uninspected_threads_fail_closed(self):
        duplicate = {**credentials(), "ts": "1780000000.000003"}
        competing = {**credentials(), "text": credentials()["text"] + "\naccount: other@example.org"}
        for messages in [[credentials(), duplicate, backup()], [competing, backup()],
                         [{**credentials(), "reply_count": 1}, backup()]]:
            with self.assertRaises(discovery.DiscoveryError) as error:
                self.execute([page(messages)])
            self.assertNotIn(PASSWORD, str(error.exception))
        with self.assertRaises(discovery.DiscoveryError) as error:
            self.execute([page([{**credentials(), "reply_count": 1}, backup()])])
        self.assertEqual(str(error.exception), "private_slack_discovery_uninspected_thread")

    def test_invalid_window_timestamp_duplicates_and_authority_limits(self):
        for changes in [dict(team_id="TOTHER1234"), dict(channel_id="COTHER1234"), dict(max_pages=True),
                        dict(max_messages=501), dict(max_pages=21), dict(oldest_ts=authority().latest_ts),
                        dict(account_email="Operator@example.org")]:
            with self.assertRaises(discovery.DiscoveryError): authority(**changes)
        for first in [{**credentials(), "ts": "1600000000.000001"}, {**credentials(), "ts": "not-a-ts"}]:
            with self.assertRaises(discovery.DiscoveryError): self.execute([page([first, backup()])])
        with self.assertRaises(discovery.DiscoveryError): self.execute([page([credentials(), credentials(), backup()])])

    def test_wrong_team_bot_and_scopes_never_read_history(self):
        for auth in [dict(ok=True, team_id="TOTHER1234", bot_id="B12345678"),
                     dict(ok=True, team_id=discovery._SETUP.TEAM, bot_id="B87654321")]:
            client = self.client([], auth)
            with self.assertRaises(discovery.DiscoveryError): discovery.discover(connection(), authority(), client)
            self.assertEqual(client.open.call_count, 1)
        client = MagicMock()
        with self.assertRaises(discovery.DiscoveryError):
            discovery.discover({**connection(), "scopes": ["groups:history", "chat:write"]}, authority(), client)
        client.open.assert_not_called()

    def test_explicit_labels_only_and_other_accounts_are_not_selected(self):
        cases = ["credentials are somewhere", f"login.gov\n{ACCOUNT}\n{PASSWORD}",
                 f"https://login.gov.evil.example\nemail: {ACCOUNT}\npassword: {PASSWORD}",
                 f"login.gov\nemail: different@example.org\npassword: {PASSWORD}"]
        for body in cases:
            with self.assertRaises(discovery.DiscoveryError):
                self.execute([page([message(credentials()["ts"], body), backup()])])
        combined = {**credentials(), "text": credentials()["text"] + f"\nbackup code: {CODE}"}
        result = self.execute([page([combined])])
        self.assertEqual(result.credential_source.message_ts, result.backup_code_source.message_ts)

    def test_network_or_attacker_error_never_escapes_as_text(self):
        client = MagicMock()
        client.open.side_effect = OSError(PASSWORD)
        with self.assertRaises(discovery.DiscoveryError) as error:
            discovery.discover(connection(), authority(), client)
        self.assertEqual(str(error.exception), "private_slack_discovery_failed")
        with self.assertRaises(discovery.DiscoveryError) as error:
            self.execute([dict(ok=False, error=PASSWORD)])
        self.assertEqual(str(error.exception), "private_slack_discovery_request_failed")

    def test_missing_completion_metadata_and_scope_drift_fail_closed(self):
        incomplete = page([credentials(), backup()])
        del incomplete["has_more"]
        with self.assertRaises(discovery.DiscoveryError) as error:
            self.execute([incomplete])
        self.assertEqual(str(error.exception), "private_slack_discovery_incomplete")
        for position in [0, 1]:
            responses = [Response({"ok": True, "team_id": discovery._SETUP.TEAM, "bot_id": "B12345678"}),
                         Response(page([credentials(), backup()]))]
            responses[position].headers.replace_header("X-OAuth-Scopes", "groups:history,canvases:read")
            client = MagicMock()
            client.open.side_effect = responses
            with self.assertRaises(discovery.DiscoveryError) as error:
                discovery.discover(connection(), authority(), client)
            self.assertEqual(str(error.exception), "private_slack_discovery_scope_mismatch")
            self.assertEqual(client.open.call_count, position + 1)

    def test_history_http_429_has_fixed_rate_limit_diagnostic_without_body_read(self):
        client = MagicMock()
        body = MagicMock()
        client.open.side_effect = [
            Response({"ok": True, "team_id": discovery._SETUP.TEAM, "bot_id": "B12345678"}),
            urllib.error.HTTPError("https://slack.com/api/conversations.history", 429, PASSWORD, {}, body),
        ]
        with self.assertRaises(discovery.DiscoveryError) as error:
            discovery.discover(connection(), authority(), client)
        self.assertEqual(str(error.exception), "private_slack_discovery_rate_limited")
        body.read.assert_not_called()

    def test_extract_exact_snapshots_preserves_all_codes_and_has_no_serializer(self):
        codes = {**backup(), "text": backup()["text"] + "\n- EFGH-9876-5432"}
        result = self.execute([page([credentials(), codes])])
        client = self.client([page([credentials()]), page([codes])])
        output = io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            material = discovery.extract(connection(), authority(), result, client)
        self.assertEqual(material.email, ACCOUNT)
        self.assertEqual(material.password, PASSWORD)
        self.assertEqual(material.backup_codes, (CODE, "EFGH-9876-5432"))
        self.assertEqual(output.getvalue(), "")
        self.assertFalse(hasattr(material, "__dict__"))
        for serializer in (json.dumps, pickle.dumps):
            with self.assertRaises(TypeError):
                serializer(material)
        for secret in (ACCOUNT, PASSWORD, CODE, TOKEN):
            self.assertNotIn(secret, repr(material))
            self.assertNotIn(secret, repr(result.__dict__))
        for call, expected in zip(client.open.call_args_list[1:], [credentials()["ts"], codes["ts"]]):
            query = urllib.parse.parse_qs(urllib.parse.urlsplit(call.args[0].full_url).query)
            self.assertEqual(query["oldest"], [expected])
            self.assertEqual(query["latest"], [expected])
            self.assertEqual(query["limit"], ["1"])

    def test_extract_requires_original_result_and_matching_authority_before_network(self):
        result = self.execute([page([credentials(), backup()])])
        fake = discovery.DiscoveryResult(result.credential_source, result.backup_code_source, 2, 1)
        for candidate in (fake, copy.copy(result), None):
            client = MagicMock()
            with self.assertRaises(discovery.DiscoveryError):
                discovery.extract(connection(), authority(), candidate, client)
            client.open.assert_not_called()
        client = MagicMock()
        with self.assertRaises(discovery.DiscoveryError):
            discovery.extract(connection(), authority(max_pages=4), result, client)
        client.open.assert_not_called()

    def test_extract_rejects_edited_missing_wrong_timestamp_and_incomplete_pages(self):
        changed = {**credentials(), "text": credentials()["text"] + "\naccount: other@example.org"}
        for first in [page([changed]), page([]), page([backup()]), page([credentials()], more=True),
                      page([credentials()], cursor="more"), page([credentials()], is_limited=True),
                      page([credentials(), credentials()]), page([{**credentials(), "edited": {"ts": "1790000000.000001"}}])]:
            result = self.execute([page([credentials(), backup()])])
            with self.assertRaises(discovery.DiscoveryError) as error:
                discovery.extract(connection(), authority(), result, self.client([first]))
            self.assertNotIn(PASSWORD, str(error.exception))

    def test_extract_pins_fresh_identity_and_each_scope_header(self):
        result = self.execute([page([credentials(), backup()])])
        client = self.client([], dict(ok=True, team_id="TOTHER1234", bot_id="B12345678"))
        with self.assertRaises(discovery.DiscoveryError):
            discovery.extract(connection(), authority(), result, client)
        self.assertEqual(client.open.call_count, 1)
        for position in (0, 1):
            result = self.execute([page([credentials(), backup()])])
            responses = [Response(dict(ok=True, team_id=discovery._SETUP.TEAM, bot_id="B12345678")),
                         Response(page([credentials()]))]
            responses[position].headers.replace_header("X-OAuth-Scopes", "groups:history,canvases:read")
            client = MagicMock()
            client.open.side_effect = responses
            with self.assertRaises(discovery.DiscoveryError) as error:
                discovery.extract(connection(), authority(), result, client)
            self.assertEqual(str(error.exception), "private_slack_discovery_scope_mismatch")

    def test_extract_combined_source_fetched_once_and_malformed_code_list_rejected(self):
        combined = {**credentials(), "text": credentials()["text"] + f"\nbackup code: {CODE}"}
        result = self.execute([page([combined])])
        client = self.client([page([combined])])
        self.assertEqual(discovery.extract(connection(), authority(), result, client).backup_codes, (CODE,))
        self.assertEqual(client.open.call_count, 2)
        invalid = {**backup(), "text": backup()["text"] + "\n- MALFORMED"}
        result = self.execute([page([credentials(), invalid])])
        with self.assertRaises(discovery.DiscoveryError) as error:
            discovery.extract(connection(), authority(), result, self.client([page([credentials()]), page([invalid])]))
        self.assertEqual(str(error.exception), "private_slack_extraction_ambiguous_material")

    def test_extract_is_consumed_before_network_success_or_failure(self):
        for failed in (False, True):
            result = self.execute([page([credentials(), backup()])])
            client = self.client([page([credentials()]), page([backup()])])
            if failed:
                client.open.side_effect = OSError(PASSWORD)
                with self.assertRaises(discovery.DiscoveryError) as error:
                    discovery.extract(connection(), authority(), result, client)
                self.assertEqual(str(error.exception), "private_slack_extraction_failed")
            else:
                discovery.extract(connection(), authority(), result, client)
            retry = MagicMock()
            with self.assertRaises(discovery.DiscoveryError) as error:
                discovery.extract(connection(), authority(), result, retry)
            self.assertEqual(str(error.exception), "private_slack_extraction_provenance_required")
            retry.open.assert_not_called()

    def test_password_punctuation_is_literal_and_boundary_whitespace_rejected(self):
        for password in ("'literal'", '"literal"', "`literal`", "'literal`", "literal internal space", "``"):
            source = {**credentials(), "text": f"login.gov\nemail: {ACCOUNT}\npassword: {password}"}
            result = self.execute([page([source, backup()])])
            material = discovery.extract(connection(), authority(), result,
                                         self.client([page([source]), page([backup()])]))
            self.assertEqual(material.password, password)
        for suffix in ("  leading", "\tleading", " trailing ", " trailing\t", " internal\tcontrol"):
            source = {**credentials(), "text": f"login.gov\nemail: {ACCOUNT}\npassword:{suffix}"}
            with self.assertRaises(discovery.DiscoveryError) as error:
                self.execute([page([source, backup()])])
            self.assertEqual(str(error.exception), "private_slack_extraction_ambiguous_material")
        self.assertEqual(discovery._material(f"login.gov\nemail: {ACCOUNT}\npassword:no-separator", ACCOUNT)[0],
                         "no-separator")


if __name__ == "__main__":
    unittest.main()
