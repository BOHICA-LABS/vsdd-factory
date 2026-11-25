"""Reading the caller marker (topics/caller-identity.md, records/caller-marker.md;
plan records.md section 3).

The hook runner writes one marker per ``vsdd`` call and prefixes the command
with ``VSDD_CALLER=<key> ``; a command calls ``caller(inv.caller_key,
project=inv.root)`` and nothing reads ``os.environ``. The marker is deleted
when it is read, the answer is cached per ``(root, key)``, and unread markers
older than ten minutes are pruned (the hook only writes them). A missing
field reads as ``""``; an empty ``agent_id`` is the main session.

On the hook path: git, configuration and registry code is imported inside
functions only.
"""

from __future__ import annotations

import re
import time
from pathlib import Path

from .store import StoreCorrupt, read_record, root_of

__all__ = ["caller"]  # the facade lists only state-api names

KEY_RE = re.compile(r"^[A-Za-z0-9_-]{8,64}$")
MARKER_AGE_SECONDS = 10 * 60
_ANSWER_CACHE: dict[tuple[str, str], dict | None] = {}

_FIELDS = ("session_id", "agent_id", "agent_type", "cwd")


def calls_dir(project) -> Path:
    return root_of(project) / ".factory" / "local" / "hooks" / "calls"


def caller(key: str, *, project) -> dict | None:
    """The calling session and agent, once: ``{session_id, agent_id,
    agent_type, cwd}`` with a missing field as ``""``. ``None`` for no key, a
    key outside ``[A-Za-z0-9_-]{8,64}``, or no marker file;
    :class:`StoreCorrupt` for a marker without a ``session_id``."""
    if not key or not isinstance(key, str) or not KEY_RE.fullmatch(key):
        return None
    directory = calls_dir(project)
    cache_key = (str(root_of(project).resolve()), key)
    if cache_key in _ANSWER_CACHE:
        return _ANSWER_CACHE[cache_key]
    _prune(directory)
    marker = read_record(directory / f"{key}.json")
    if marker is None:
        return None
    if not marker.get("session_id"):
        raise StoreCorrupt(
            "corrupt", f"{directory / (key + '.json')}: a caller marker carries no session_id"
        )
    answer = {field: str(marker.get(field, "")) for field in _FIELDS}
    path = directory / f"{key}.json"
    try:
        path.unlink()
    except OSError:
        pass  # the marker is read once; losing the file after the read is fine
    _ANSWER_CACHE[cache_key] = answer
    return answer


def _prune(directory: Path, *, now: float | None = None) -> None:
    """Delete unread markers whose file time is over ten minutes past."""
    if not directory.is_dir():
        return
    deadline = (now if now is not None else time.time()) - MARKER_AGE_SECONDS
    for path in directory.glob("*.json"):
        try:
            if path.stat().st_mtime < deadline:
                path.unlink()
        except OSError:
            continue
