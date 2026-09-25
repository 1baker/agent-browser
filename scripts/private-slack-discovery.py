"""Private, bounded text-history discovery; no CLI, output, staging or execution.

The caller must verify digest-bound operator authority before constructing the
authority below. Account identity comes from the approved manifest, never Slack.
This first adapter recognizes explicit plaintext labels only. Uninspected thread
replies or retention-limited history fail closed. Files/canvases are not fetched.
Matched message references are candidates, not credential extraction or login proof.
"""
from dataclasses import dataclass
import html
import importlib.util
import json
from pathlib import Path
import re
import time
import urllib.error
import urllib.parse
import urllib.request
import weakref

_SPEC = importlib.util.spec_from_file_location("_private_slack_setup", Path(__file__).with_name("setup-private-slack.py"))
_SETUP = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_SETUP)
_TS = re.compile(r"^[0-9]{10}\.[0-9]{6}$")
_EMAIL = re.compile(r"^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9-]+(?:\.[a-z0-9-]+)+$")
_CONTEXT = re.compile(r"(?<![a-z0-9.-])(?:secure\.)?login\.gov(?![a-z0-9.-])", re.I)
_FIELD = re.compile(r"^\s*(?:[-*]\s+)?(account email|email|username|account|password|backup codes?|recovery codes?)\s*:\s*(.*?)\s*$", re.I)
_PASSWORD = re.compile(r"^\s*(?:[-*]\s+)?password\s*:(.*)$", re.I)


class DiscoveryError(Exception):
    """Only fixed diagnostics; never include bodies, account values or cursors."""


@dataclass(frozen=True, repr=False)
class DiscoveryAuthority:
    team_id: str
    channel_id: str
    account_email: str
    oldest_ts: str
    latest_ts: str
    max_messages: int
    max_pages: int

    def __post_init__(self):
        if self.team_id != _SETUP.TEAM or self.channel_id != _SETUP.CHANNEL \
                or not isinstance(self.account_email, str) or len(self.account_email) > 254 \
                or _EMAIL.fullmatch(self.account_email) is None \
                or not isinstance(self.oldest_ts, str) or _TS.fullmatch(self.oldest_ts) is None \
                or not isinstance(self.latest_ts, str) or _TS.fullmatch(self.latest_ts) is None \
                or self.oldest_ts >= self.latest_ts \
                or type(self.max_messages) is not int or not 1 <= self.max_messages <= 500 \
                or type(self.max_pages) is not int or not 1 <= self.max_pages <= 20:
            raise DiscoveryError("private_slack_discovery_invalid_authority")


@dataclass(frozen=True, repr=False)
class MessageReference:
    channel_id: str
    message_ts: str


@dataclass(frozen=True, repr=False, eq=False)
class DiscoveryResult:
    credential_source: MessageReference
    backup_code_source: MessageReference
    scanned_messages: int
    scanned_pages: int


# Identity-keyed, process-local provenance. No digest or body is attached to the
# public metadata object; copying/serializing references cannot mint provenance.
_SNAPSHOTS = weakref.WeakKeyDictionary()


class PrivateMaterial:
    """Deliberately opaque; explicit access only by the trusted staging caller.

    Python cannot erase immutable strings or protect against hostile code in the
    same process. This prevents accidental repr/JSON/pickle disclosure, not that.
    """
    __slots__ = ("_email", "_password", "_backup_codes")

    def __init__(self, email, password, backup_codes):
        self._email, self._password, self._backup_codes = email, password, tuple(backup_codes)

    @property
    def email(self):
        return self._email

    @property
    def password(self):
        return self._password

    @property
    def backup_codes(self):
        return self._backup_codes

    def __reduce_ex__(self, protocol):
        raise TypeError("private_slack_material_not_serializable")


def _snapshot(message):
    return json.dumps(message, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def _code(value):
    compact = value.replace(" ", "").replace("-", "")
    return 6 <= len(compact) <= 64 and compact.isascii() and compact.isalnum() and any(c.isdigit() for c in compact)


def _material(text, account, strict=False):
    """Parse privately; competing account/password labels are ambiguous.

    Password syntax is ``password:`` followed by zero or one ASCII separator
    space and the literal password. Quotes/backticks are password characters,
    not wrappers. Leading/trailing whitespace in the value is unsupported and
    rejected rather than trimmed; internal spaces are retained.
    """
    if not isinstance(text, str) or len(text) > 32768:
        raise DiscoveryError("private_slack_discovery_invalid_message")
    text = html.unescape(text)
    if not _CONTEXT.search(text):
        return None, ()
    lines = text.splitlines()
    accounts, passwords, backups = [], [], []
    for index, line in enumerate(lines):
        field = _FIELD.fullmatch(line)
        if not field:
            continue
        label, value = field.group(1).lower(), field.group(2).strip().strip("'\"`")
        if label in {"email", "username", "account", "account email"}:
            mailto = re.fullmatch(r"<mailto:([^>|]+)(?:\|([^>]+))?>", value)
            if mailto:
                if mailto.group(2) and mailto.group(2) != mailto.group(1):
                    raise DiscoveryError("private_slack_discovery_ambiguous_account")
                value = mailto.group(1)
            accounts.append(value.lower())
        elif label == "password":
            value = _PASSWORD.fullmatch(line).group(1)
            if value.startswith(" "):
                value = value[1:]
            if value != value.strip() or any(ord(char) < 32 or ord(char) == 127 for char in value):
                raise DiscoveryError("private_slack_extraction_ambiguous_material")
            if value and value.lower() not in {"redacted", "[redacted]", "<redacted>"} and value.strip("*"):
                passwords.append(value)
        else:
            if _code(value):
                backups.append(value)
            elif not value:
                before = len(backups)
                for following in lines[index + 1:]:
                    candidate = re.sub(r"^\s*(?:[-*]|[0-9]+[.)])\s+", "", following).strip().strip("'\"`")
                    if not _code(candidate):
                        if strict and following.strip() and not _FIELD.fullmatch(following):
                            raise DiscoveryError("private_slack_extraction_ambiguous_material")
                        break
                    backups.append(candidate)
                if strict and len(backups) == before:
                    raise DiscoveryError("private_slack_extraction_ambiguous_material")
            elif strict:
                raise DiscoveryError("private_slack_extraction_ambiguous_material")
    if not passwords and not backups:
        return None, ()
    if not accounts or any(_EMAIL.fullmatch(value) is None for value in accounts) or len(set(accounts)) != 1:
        raise DiscoveryError("private_slack_discovery_ambiguous_account")
    if accounts[0] != account:
        return None, ()
    if len(passwords) > 1:
        raise DiscoveryError("private_slack_discovery_ambiguous_candidates")
    return passwords[0] if passwords else None, tuple(backups)


def _classify(text, account):
    password, backups = _material(text, account)
    return bool(password), bool(backups)


def _page(connection, parameters, transport, deadline):
    request = urllib.request.Request(
        "https://slack.com/api/conversations.history?" + urllib.parse.urlencode(parameters),
        method="GET", headers={"Authorization": "Bearer " + connection["token"],
                               "Accept": "application/json", "Accept-Encoding": "identity"},
    )
    with transport.open(request, timeout=_SETUP.TIMEOUT) as response:
        if response.status == 429:
            raise DiscoveryError("private_slack_discovery_rate_limited")
        if response.status != 200 or response.headers.get_content_type() != "application/json":
            raise DiscoveryError("private_slack_discovery_request_failed")
        scope_headers = response.headers.get_all("x-oauth-scopes", [])
        if len(scope_headers) != 1 or len(scope_headers[0]) > 2048:
            raise DiscoveryError("private_slack_discovery_scope_mismatch")
        actual_scopes = _SETUP.validate_scopes([scope.strip() for scope in scope_headers[0].split(",")])
        if actual_scopes != sorted(connection["scopes"]):
            raise DiscoveryError("private_slack_discovery_scope_mismatch")
        data = bytearray()
        page_deadline = min(deadline, time.monotonic() + _SETUP.TIMEOUT)
        while True:
            if time.monotonic() >= page_deadline:
                raise DiscoveryError("private_slack_discovery_request_failed")
            chunk = response.read1(min(8192, _SETUP.LIMIT + 1 - len(data)))
            if time.monotonic() >= page_deadline:
                raise DiscoveryError("private_slack_discovery_request_failed")
            if not chunk:
                break
            data.extend(chunk)
            if len(data) > _SETUP.LIMIT:
                raise DiscoveryError("private_slack_discovery_response_too_large")
    page = _SETUP.decode_json(data)
    if not isinstance(page, dict) or page.get("ok") is not True:
        raise DiscoveryError("private_slack_discovery_request_failed")
    return page


def discover(connection, authority, transport=None):
    """No output, retries or authority expansion; bodies remain local in memory.

    Returns refs only after complete bounded cursor pagination and unique explicit
    textual candidates. Caller must separately resolve/extract/stage privately.
    Metadata does not prove provenance, account ownership or browser readiness.
    """
    try:
        if not isinstance(authority, DiscoveryAuthority):
            raise DiscoveryError("private_slack_discovery_invalid_authority")
        authority.__post_init__()
        _SETUP.harden()
        _SETUP.validate_connection(connection)
        if connection["team_id"] != authority.team_id or connection["channel_id"] != authority.channel_id:
            raise DiscoveryError("private_slack_discovery_authority_mismatch")
        transport = transport or _SETUP.opener()
        deadline = time.monotonic() + 60
        fresh = _SETUP.probe(connection["token"], transport)
        if fresh["team_id"] != authority.team_id or fresh["bot_id"] != connection["bot_id"]:
            raise DiscoveryError("private_slack_discovery_authority_mismatch")
        if fresh["scopes"] != sorted(connection["scopes"]):
            raise DiscoveryError("private_slack_discovery_scope_mismatch")
        credential_refs, backup_refs, seen, cursors = [], [], set(), set()
        snapshots = {}
        cursor, count, pages = "", 0, 0
        while True:
            if pages >= authority.max_pages or count >= authority.max_messages or time.monotonic() >= deadline:
                raise DiscoveryError("private_slack_discovery_incomplete")
            limit = min(100, authority.max_messages - count)
            parameters = {"channel": authority.channel_id, "oldest": authority.oldest_ts,
                          "latest": authority.latest_ts, "inclusive": "true", "limit": str(limit)}
            if cursor:
                parameters["cursor"] = cursor
            page = _page(connection, parameters, transport, deadline)
            pages += 1
            if page.get("is_limited"):
                raise DiscoveryError("private_slack_discovery_incomplete")
            if "channel" in page and page["channel"] != authority.channel_id:
                raise DiscoveryError("private_slack_discovery_authority_mismatch")
            messages = page.get("messages")
            if not isinstance(messages, list) or len(messages) > limit:
                raise DiscoveryError("private_slack_discovery_invalid_response")
            for message in messages:
                if not isinstance(message, dict):
                    raise DiscoveryError("private_slack_discovery_invalid_message")
                ts = message.get("ts")
                if not isinstance(ts, str) or _TS.fullmatch(ts) is None or ts in seen \
                        or not authority.oldest_ts <= ts <= authority.latest_ts:
                    raise DiscoveryError("private_slack_discovery_invalid_message")
                seen.add(ts)
                count += 1
                replies = message.get("reply_count", 0)
                if type(replies) is not int or replies < 0:
                    raise DiscoveryError("private_slack_discovery_invalid_message")
                if replies:
                    raise DiscoveryError("private_slack_discovery_uninspected_thread")
                credential, backup = _classify(message.get("text", ""), authority.account_email)
                reference = MessageReference(authority.channel_id, ts)
                if credential:
                    credential_refs.append(reference)
                if backup:
                    backup_refs.append(reference)
                if credential or backup:
                    snapshots[ts] = _snapshot(message)
                if len(credential_refs) > 1 or len(backup_refs) > 1:
                    raise DiscoveryError("private_slack_discovery_ambiguous_candidates")
            metadata = page.get("response_metadata", {})
            if not isinstance(metadata, dict):
                raise DiscoveryError("private_slack_discovery_invalid_response")
            next_cursor = metadata.get("next_cursor", "")
            if next_cursor is None:
                next_cursor = ""
            if "has_more" not in page:
                raise DiscoveryError("private_slack_discovery_incomplete")
            has_more = page["has_more"]
            if type(has_more) is not bool or not isinstance(next_cursor, str) or len(next_cursor) > 2048 \
                    or any(ord(char) < 33 or ord(char) > 126 for char in next_cursor):
                raise DiscoveryError("private_slack_discovery_invalid_response")
            if not next_cursor:
                if has_more:
                    raise DiscoveryError("private_slack_discovery_incomplete")
                break
            if next_cursor in cursors or not messages:
                raise DiscoveryError("private_slack_discovery_incomplete")
            cursors.add(next_cursor)
            cursor = next_cursor
        if len(credential_refs) != 1 or len(backup_refs) != 1:
            raise DiscoveryError("private_slack_discovery_candidates_not_found")
        result = DiscoveryResult(credential_refs[0], backup_refs[0], count, pages)
        _SNAPSHOTS[result] = (authority, connection["bot_id"], tuple(sorted(connection["scopes"])), snapshots)
        return result
    except DiscoveryError:
        raise
    except urllib.error.HTTPError as error:
        raise DiscoveryError("private_slack_discovery_rate_limited" if error.code == 429
                             else "private_slack_discovery_request_failed") from None
    except Exception:
        raise DiscoveryError("private_slack_discovery_failed") from None


def extract(connection, authority, result, transport=None):
    """Refetch exact in-process discovered snapshots; never select a backup code.

    No authority is minted here: the caller must already hold digest-bound
    consent. Results cannot survive restart; rediscovery needs a new admission.
    No automatic retry, staging, output, or browser action is performed.
    """
    try:
        if type(result) is not DiscoveryResult or result not in _SNAPSHOTS:
            raise DiscoveryError("private_slack_extraction_provenance_required")
        # Consume before authentication or history I/O. Failure/uncertainty must
        # not permit a second attempt through the same in-process capability.
        original, bot_id, scopes, snapshots = _SNAPSHOTS.pop(result)
        if type(authority) is not DiscoveryAuthority or authority != original:
            raise DiscoveryError("private_slack_discovery_authority_mismatch")
        authority.__post_init__()
        _SETUP.harden()
        _SETUP.validate_connection(connection)
        if connection["team_id"] != authority.team_id or connection["channel_id"] != authority.channel_id \
                or connection["bot_id"] != bot_id or tuple(sorted(connection["scopes"])) != scopes:
            raise DiscoveryError("private_slack_discovery_authority_mismatch")
        transport = transport or _SETUP.opener()
        fresh = _SETUP.probe(connection["token"], transport)
        if fresh["team_id"] != authority.team_id or fresh["bot_id"] != bot_id:
            raise DiscoveryError("private_slack_discovery_authority_mismatch")
        if tuple(fresh["scopes"]) != scopes:
            raise DiscoveryError("private_slack_discovery_scope_mismatch")
        deadline, material = time.monotonic() + 30, {}
        for reference in (result.credential_source, result.backup_code_source):
            ts = reference.message_ts
            if reference.channel_id != authority.channel_id or ts not in snapshots \
                    or not authority.oldest_ts <= ts <= authority.latest_ts:
                raise DiscoveryError("private_slack_extraction_provenance_required")
            if ts in material:
                continue
            page = _page(connection, {"channel": authority.channel_id, "oldest": ts,
                         "latest": ts, "inclusive": "true", "limit": "1"}, transport, deadline)
            metadata = page.get("response_metadata", {})
            if page.get("has_more") is not False or page.get("is_limited") \
                    or not isinstance(metadata, dict) or metadata.get("next_cursor") not in (None, ""):
                raise DiscoveryError("private_slack_discovery_incomplete")
            if "channel" in page and page["channel"] != authority.channel_id:
                raise DiscoveryError("private_slack_discovery_authority_mismatch")
            messages = page.get("messages")
            if not isinstance(messages, list) or len(messages) != 1 or not isinstance(messages[0], dict) \
                    or messages[0].get("ts") != ts or _snapshot(messages[0]) != snapshots[ts]:
                raise DiscoveryError("private_slack_extraction_source_changed")
            material[ts] = _material(messages[0].get("text", ""), authority.account_email, strict=True)
        password = material[result.credential_source.message_ts][0]
        backups = material[result.backup_code_source.message_ts][1]
        if not password or not backups:
            raise DiscoveryError("private_slack_extraction_source_changed")
        return PrivateMaterial(authority.account_email, password, backups)
    except DiscoveryError:
        raise
    except urllib.error.HTTPError as error:
        raise DiscoveryError("private_slack_discovery_rate_limited" if error.code == 429
                             else "private_slack_discovery_request_failed") from None
    except Exception:
        raise DiscoveryError("private_slack_extraction_failed") from None
