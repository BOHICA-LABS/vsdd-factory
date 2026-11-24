"""The event log: the one writer and reader (hooks.md 2.12).

Daily JSONL files, one JSON object per line, in ``<project>/.factory/local/
events/<UTC date>.jsonl``; the environment variable ``VSDD_EVENTS_DIR``
overrides the directory for tests. Every line carries ``v``, ``ts`` (ISO 8601,
UTC, millisecond precision), ``type`` (dotted), ``severity`` (``info``,
``warn``, ``block``, ``error``) and ``session_id``; lines about a tool call add
``agent_type``, ``agent_id``, ``event``, ``tool`` and ``ms``.

Hygiene, as the design fixes it: each line is one ``write`` on a file opened
with ``O_APPEND`` so concurrent agents do not interleave; lines are capped at
4 KB by dropping optional fields, never by truncating the required five;
commands are cut to 200 characters; string fields pass through a redactor that
masks secret-shaped ``NAME=value`` pairs and common token formats; and once a
day's file passes 20 MB only ``block`` and ``error`` severities are written for
the rest of that day. ``prune`` deletes files older than the retention window
(the hook runner calls it at session end).

Calls from worktrees log to the main project's file: the caller resolves the
project root to the main checkout (``project.py``) before writing here.
"""

from __future__ import annotations

import json
import os
import re
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

from .schema import ValidationError, validate

__all__ = [
    "SEVERITIES",
    "EVENT_SCHEMA",
    "EventError",
    "EventLog",
    "events_dir_for",
    "redact",
    "cut_command",
]

SEVERITIES = ("info", "warn", "block", "error")

MAX_LINE_BYTES = 4096
MAX_DAY_BYTES = 20 * 1024 * 1024
COMMAND_LIMIT = 200
DEFAULT_RETENTION_DAYS = 30

_TS_RE = re.compile(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z")
_TYPE_RE = re.compile(r"[a-z0-9][a-z0-9_-]*(\.[a-z0-9][a-z0-9_-]*)+")

# The value of a secret-shaped assignment: a quoted string with spaces, or any
# run of non-space characters. Quoted values are matched whole, so
# PASSWORD="correct horse battery staple" loses nothing but its name.
_SECRET_VALUE = r'(?:"[^"]*"|\'[^\']*\'|\S+)'

SECRET_NAME_RE = re.compile(
    rf"(?i)\b([A-Z0-9_]*(?:KEY|TOKEN|SECRET|PASSWORD|PASSWD|CREDENTIAL|AUTHORIZATION|COOKIE)[A-Z0-9_]*)=({_SECRET_VALUE})"
)
# "--password hunter2" (the value is the next word, quoted values included)
FLAG_SPACE_SECRET_RE = re.compile(
    rf"(?i)(--(?:password|token|secret|api-key)[-\w]*\s+){_SECRET_VALUE}"
)
# "--password=hunter2", "--api-token=hunter2": any flag whose name carries
# password, token or secret, with the value after '='.
FLAG_EQ_SECRET_RE = re.compile(
    rf"(?i)(--?[-\w]*(?:password|token|secret)[-\w]*=){_SECRET_VALUE}"
)
AUTH_USER_RE = re.compile(r"(?i)(-u\s*[^\s:@/]+:)(\S+)")
HEADER_SECRET_RE = re.compile(
    rf"(?i)((?:x-)?(?:api-key|authorization|cookie|auth-token)\s*:\s*){_SECRET_VALUE}"
)
TOKEN_PATTERNS = (
    re.compile(r"ghp_[A-Za-z0-9]{20,}"),
    re.compile(r"github_pat_[A-Za-z0-9_]{20,}"),
    re.compile(r"gh[pousr]_[A-Za-z0-9]{20,}"),
    re.compile(r"AKIA[0-9A-Z]{16}"),
    re.compile(r"sk-[A-Za-z0-9_-]{16,}"),
    re.compile(r"xox[baprs]-[A-Za-z0-9-]{10,}"),
    re.compile(r"Bearer\s+[A-Za-z0-9._-]{16,}"),
)

#: The schema the writer validates against; ``vsdd events schema`` prints it,
#: so the documented schema cannot drift from the code (hooks.md 2.12). Every
#: keyword is one the shipped checker (:mod:`vsdd.schema`) enforces, and
#: :meth:`EventLog.build` validates every event against it before writing —
#: except ``pattern``, which build() applies itself through ``_TS_RE``.
EVENT_SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "$id": "event.schema.json",
    "title": "Factory event log line",
    "type": "object",
    "properties": {
        "v": {"type": "integer", "enum": [1]},
        "ts": {"type": "string", "pattern": _TS_RE.pattern},
        "type": {"type": "string", "description": "dotted event type"},
        "severity": {"enum": list(SEVERITIES)},
        "session_id": {"type": "string", "minLength": 1},
        "agent_type": {"type": "string"},
        "agent_id": {"type": "string"},
        "event": {"type": "string"},
        "tool": {"type": "string"},
        "ms": {"type": "integer", "minimum": 0},
        "rule": {"type": "string"},
        "code": {"type": "string"},
        "reason": {"type": "string"},
        "fix": {"type": "string"},
        "target": {"type": "string"},
        "command": {"type": "string"},
        "id": {"type": "string"},
        "detail": {
            "type": ["string", "object", "array"],
            "description": "Free-form payload; string leaves are redacted like every other field.",
        },
    },
    "required": ["v", "ts", "type", "severity", "session_id"],
    "additionalProperties": True,
}

_REQUIRED = ("v", "ts", "type", "severity", "session_id")


class EventError(ValueError):
    """An event is outside the log's schema and was not written."""


def redact(text: str) -> str:
    """Mask secret-shaped assignments, flags, headers and common token formats.

    A value is matched as a quoted string (whole, spaces included) or one
    non-space run, so ``PASSWORD="correct horse battery staple"`` loses
    nothing but its name.
    """
    masked = SECRET_NAME_RE.sub(lambda m: f"{m.group(1)}=<redacted>", text)
    masked = FLAG_SPACE_SECRET_RE.sub(lambda m: f"{m.group(1)}<redacted>", masked)
    masked = FLAG_EQ_SECRET_RE.sub(lambda m: f"{m.group(1)}<redacted>", masked)
    masked = AUTH_USER_RE.sub(lambda m: f"{m.group(1)}<redacted>", masked)
    masked = HEADER_SECRET_RE.sub(lambda m: f"{m.group(1)}<redacted>", masked)
    for pattern in TOKEN_PATTERNS:
        masked = pattern.sub("<redacted>", masked)
    return masked


def _redact_deep(value: object) -> object:
    """Redact every string leaf of a nested value (dicts, lists, scalars)."""
    if isinstance(value, str):
        return redact(value)
    if isinstance(value, dict):
        return {key: _redact_deep(item) for key, item in value.items()}
    if isinstance(value, list):
        return [_redact_deep(item) for item in value]
    return value


def cut_command(command: str, limit: int = COMMAND_LIMIT) -> str:
    """Cut a command to the log's character budget."""
    return command if len(command) <= limit else command[: limit - 1] + "…"


def _encode(event: dict) -> str:
    return json.dumps(event, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def _fit(event: dict) -> str:
    """Build a line within the 4 KB cap: drop optional fields, then compress.

    The five required fields survive every time; if even their canonical form
    will not fit, the session id is shortened rather than the line lost.
    """
    if len(_encode(event).encode("utf-8")) <= MAX_LINE_BYTES:
        return _encode(event)
    droppable = sorted(
        (key for key in event if key not in _REQUIRED),
        key=lambda key: len(str(event[key])),
        reverse=True,
    )
    trimmed = dict(event)
    for key in droppable:
        del trimmed[key]
        encoded = _encode(trimmed)
        if len(encoded.encode("utf-8")) <= MAX_LINE_BYTES:
            return encoded
    slim = {key: trimmed[key] for key in _REQUIRED}
    slim["session_id"] = str(slim["session_id"])[:64]
    slim["trimmed"] = True
    return _encode(slim)


class EventLog:
    """Appends events to the daily JSONL file and reads them back."""

    def __init__(self, directory: Path | str, session_id: str):
        self.directory = Path(directory)
        self.session_id = session_id

    # -- writing -----------------------------------------------------------

    def _path_for(self, day: date) -> Path:
        return self.directory / f"{day.isoformat()}.jsonl"

    def _append(self, path: Path, line: str) -> None:
        self.directory.mkdir(parents=True, exist_ok=True)
        # One write on an O_APPEND handle: concurrent agents do not interleave.
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o644)
        try:
            os.write(fd, (line + "\n").encode("utf-8"))
        finally:
            os.close(fd)

    def log(self, type_: str, severity: str = "info", **fields: object) -> bool:
        """Validate, hygiene-filter and append one event. True when written.

        Raises :class:`EventError` on a malformed type or severity: a bad event
        is a programming error in the caller, and the log must not drift from
        its schema.
        """
        event = self.build(type_, severity, **fields)
        over_cap = False
        path = self._path_for(datetime.now(timezone.utc).date())
        try:
            over_cap = path.exists() and path.stat().st_size >= MAX_DAY_BYTES
        except OSError:
            pass
        if over_cap and severity not in ("block", "error"):
            return False
        self._append(path, _fit(event))
        return True

    def build(self, type_: str, severity: str = "info", **fields: object) -> dict:
        """Validate and hygiene-filter one event without writing it.

        The finished event is validated against ``EVENT_SCHEMA`` — the schema
        ``vsdd events schema`` prints — so a line in the log never fails it,
        and no field value escapes the redactor by not being a string.
        """
        problems = self.check(type_, severity)
        if not str(self.session_id).strip():
            problems.append("session_id must not be empty")
        if problems:
            raise EventError("; ".join(problems))
        event: dict = {
            "v": 1,
            "ts": datetime.now(timezone.utc).isoformat(timespec="milliseconds")
            .replace("+00:00", "Z"),
            "type": type_,
            "severity": severity,
            "session_id": self.session_id,
        }
        for key, value in fields.items():
            if value is None:
                continue
            value = _redact_deep(value)
            if key == "command" and isinstance(value, str):
                value = cut_command(value)
            event[key] = value
        errors = validate(event, EVENT_SCHEMA)
        if _TS_RE.fullmatch(str(event["ts"])) is None:
            errors.append(ValidationError("$.ts", "ts is not ISO 8601 UTC, milliseconds"))
        if errors:
            raise EventError("; ".join(str(e) for e in errors))
        return event

    @staticmethod
    def check(type_: str, severity: str) -> list[str]:
        problems: list[str] = []
        if not _TYPE_RE.fullmatch(type_):
            problems.append(f"event type {type_!r} must be dotted (section.event)")
        if severity not in SEVERITIES:
            problems.append(f"severity {severity!r} must be one of {SEVERITIES}")
        return problems

    # -- reading -----------------------------------------------------------

    def read(self, day: date | None = None) -> list[dict]:
        """Every well-formed event of one day (default: today, UTC)."""
        path = self._path_for(day or datetime.now(timezone.utc).date())
        if not path.is_file():
            return []
        events: list[dict] = []
        for line in path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                events.append(json.loads(line))
            except json.JSONDecodeError:
                continue  # a torn or hand-edited line never stops the reader
        return events

    def prune(self, retention_days: int = DEFAULT_RETENTION_DAYS) -> list[str]:
        """Delete daily files older than the retention window; return names."""
        cutoff = datetime.now(timezone.utc).date() - timedelta(days=retention_days)
        removed: list[str] = []
        if not self.directory.is_dir():
            return removed
        for path in self.directory.glob("*.jsonl"):
            try:
                day = date.fromisoformat(path.stem)
            except ValueError:
                continue  # not a daily file; leave it alone
            if day < cutoff:
                try:
                    path.unlink()
                    removed.append(path.name)
                except OSError:
                    pass
        return removed


def events_dir_for(project_root: Path | str) -> Path:
    """The events directory for a project; ``VSDD_EVENTS_DIR`` wins for tests."""
    override = os.environ.get("VSDD_EVENTS_DIR")
    if override:
        return Path(override)
    return Path(project_root) / ".factory" / "local" / "events"
