"""The record stores' foundation (plan records.md section 3, ``store``).

Common plumbing for every store in ``vsdd.state``: the error types with their
codes, the path-segment rule, timestamps, one record reader and writer (atomic,
validated, capped), the two file locks' shared helper and the store's own git
wrapper. The paths and caps are those of spec ``records/*.md``.

Writing: records under ``.factory/local/`` are written compact, tracked ones
with ``indent=2`` and key order kept; caps count bytes written; the temporary
file lives in ``.factory/local/tmp/``, where flush never looks. A failed write
leaves the old bytes in place.
"""

from __future__ import annotations

import errno
import fcntl
import json
import os
import re
import secrets
import string
import threading
import time
from contextlib import contextmanager
from datetime import datetime, timezone
from pathlib import Path

__all__: list[str] = []

SCHEMA_DIR = Path(__file__).resolve().parents[2].parent / "schemas"
TMP_SUBDIR = ("local", "tmp")
_REGISTRY = None


class StoreError(Exception):
    """A store failure with a machine-readable code (the plan's quoted words)."""

    def __init__(self, code: str, message: str | None = None):
        super().__init__(message or code)
        self.code = code


class StoreRefused(StoreError):
    """A refusal; ``None`` and ``[]`` mean only "nothing there".


    Only refusals subclass it: :class:`StoreBusy` and :class:`StoreCorrupt`
    are failures of the store, not answers to a request, and a consumer
    catching ``StoreRefused`` must not swallow them (plan records.md 3)."""


class StoreBusy(StoreError):
    """A file lock was not had in time."""


class StoreCorrupt(StoreError):
    """A file does not parse or fails its schema; names the path."""


class DecisionRefused(StoreRefused):
    """A decision refusal that can carry the alternatives (``ambiguous`',
    ``bad_choice``)."""

    def __init__(self, code: str, message: str | None = None, choices: list | None = None):
        super().__init__(code, message)
        self.choices = choices if choices is not None else []


# ------------------------------------------------------------- primitives ---


def root_of(project) -> Path:
    """The project root of a ``Project``, any object with ``.root``, a ``str``
    or a ``Path``."""
    if isinstance(project, (str, Path)):
        return Path(project)
    root = getattr(project, "root", None)
    if root is None:
        raise StoreError("bad_project", f"{type(project).__name__} carries no root")
    return Path(root)


def state_dir_of(project) -> Path:
    """``<root>/.factory`` of whatever the caller passed as project."""
    return root_of(project) / ".factory"


_SEGMENT_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")
_LITERAL_SEGMENTS = frozenset({"_standing", "_base", "_"})


def safe_segment(value: str) -> str:
    """One path segment, validated: ``^[A-Za-z0-9][A-Za-z0-9._-]*$`` without
    ``..`` — refused anywhere in the segment, an id is never allowed to climb
    — or one of ``_standing``, ``_base`` (base reports beside the evidence
    records) and ``_`` (an empty subject). Else ``unsafe_id``."""
    if not isinstance(value, str) or value == "":
        raise StoreRefused("unsafe_id", f"path segment must not be empty: {value!r}")
    if "/" in value or os.sep in value:
        raise StoreRefused("unsafe_id", f"path segment must not carry a separator: {value!r}")
    if value in _LITERAL_SEGMENTS:
        return value
    if ".." in value:
        raise StoreRefused("unsafe_id", f"{value!r} carries '..', which is not a safe segment")
    if _SEGMENT_RE.fullmatch(value):
        return value
    raise StoreRefused("unsafe_id", f"{value!r} is not a safe path segment")


def _literal_segment(value: str) -> str:
    """Like :func:`safe_segment` but accepting also the empty subject, spelled
    ``_`` where the caller gives it directly."""
    if value == "":
        return "_"
    return safe_segment(value)


def now() -> str:
    """ISO 8601 UTC, whole seconds, ending ``Z``."""
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def parse_time(stamped: str) -> float:
    """Epoch seconds of an ISO 8601 ``Z`` stamp; ``0.0`` when unparseable."""
    try:
        return datetime.fromisoformat(str(stamped).replace("Z", "+00:00")).timestamp()
    except (ValueError, TypeError, OverflowError):
        return 0.0


def random_suffix(length: int = 2) -> str:
    """A random id suffix over ``[0-9a-z]``."""
    alphabet = string.digits + string.ascii_lowercase
    return "".join(secrets.choice(alphabet) for _ in range(length))


# --------------------------------------------------------------- registry ---


def _registry():
    global _REGISTRY
    if _REGISTRY is None:
        from ..schema import load_registry

        _REGISTRY = load_registry(SCHEMA_DIR)
    return _REGISTRY


def schema_of(name: str) -> dict:
    """The shipped schema named ``<name>.schema.json``, from the one registry."""
    try:
        return _registry()[f"{name}.schema.json"]
    except KeyError as exc:
        raise StoreError("no_schema", f"no shipped schema named {name!r}") from exc


# ------------------------------------------------------- reading, writing ---


def read_record(path, schema: str | None = None) -> dict | None:
    """The parsed record at ``path``; ``None`` when absent.

    A file that does not parse, or that fails its named schema, raises
    :class:`StoreCorrupt` naming the path. ``schema`` is a shipped schema name
    (``"closure-record"``), read through the same registry the platform sees.
    """
    path = Path(path)
    try:
        text = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return None
    except OSError as exc:
        raise StoreCorrupt("corrupt", f"{path}: cannot be read: {exc}") from exc
    try:
        record = json.loads(text)
    except json.JSONDecodeError as exc:
        raise StoreCorrupt("corrupt", f"{path}: does not parse: {exc}") from exc
    if not isinstance(record, dict):
        raise StoreCorrupt("corrupt", f"{path}: a record must be a JSON object")
    if schema is not None:
        from ..schema import validate

        problems = validate(record, schema_of(schema), _registry())
        if problems:
            detail = "; ".join(str(problem) for problem in problems[:5])
            raise StoreCorrupt("corrupt", f"{path}: fails {schema}: {detail}")
    return record


def _under_local(path: Path) -> bool:
    """True when ``path`` lies under a ``.factory/local/`` directory: the
    component that follows ``.factory`` must be ``local``, so a project
    directory merely named ``local`` does not flip the form."""
    parts = path.parts
    if ".factory" not in parts:
        return False
    after = parts.index(".factory") + 1
    return after < len(parts) and parts[after] == "local"


def _encoded(obj: dict, *, compact: bool) -> bytes:
    if compact:
        return json.dumps(obj, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return json.dumps(obj, indent=2, ensure_ascii=False).encode("utf-8") + b"\n"


def write_via_tmp(path, data: bytes) -> int:
    """Write ``data`` at ``path`` through ``.factory/local/tmp/`` and
    ``os.replace``; returns the byte count.

    The atomic half of :func:`write_record`, for the files written beside a
    record — decision packets, evidence logs and reports. The temporary file
    lives where flush never looks, and the replace puts the bytes in place
    whole: a reader sees the old file or the new one, never a truncated one.
    """
    path = Path(path)
    if ".factory" in path.parts:
        anchor = path.parts.index(".factory")
        root = Path(*path.parts[: anchor + 1])
        tmp_dir = root.joinpath(*TMP_SUBDIR)
    else:
        tmp_dir = path.parent
    tmp_dir.mkdir(parents=True, exist_ok=True)
    tmp = tmp_dir / f".{path.name}.{os.getpid()}.{random_suffix(4)}.tmp"
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(tmp, "wb") as handle:
        handle.write(data)
        handle.flush()
        os.fsync(handle.fileno())
    try:
        os.replace(tmp, path)
    except OSError:
        tmp.unlink(missing_ok=True)
        raise
    return len(data)


def write_record(path, obj: dict, *, cap: int | None, schema: str | None = None) -> bytes:
    """Validate, then write ``obj`` at ``path`` through ``local/tmp/`` and
    ``os.replace``; returns the byte count. ``schema_invalid`` on a schema
    failure, ``over_cap`` past ``cap``; in both cases the old file stays.

    Records under ``.factory/local/`` are written compact, tracked ones with
    ``indent=2``; the cap counts the bytes that will be written.
    """
    path = Path(path)
    if schema is not None:
        from ..schema import validate

        problems = validate(obj, schema_of(schema), _registry())
        if problems:
            detail = "; ".join(str(problem) for problem in problems[:5])
            raise StoreRefused("schema_invalid", f"{path.name}: fails {schema}: {detail}")
    compact = _under_local(path)
    data = _encoded(obj, compact=compact)
    if cap is not None and len(data) > cap:
        raise StoreRefused(
            "over_cap", f"{path.name} is {len(data)} bytes; the cap is {cap}"
        )
    return write_via_tmp(path, data)


# ------------------------------------------------------------------ locks ---

_LOCK_TIMEOUT_POLL = 0.02


@contextmanager
def locked(project, name: str = "flush.lock", wait: float = 3.0):
    """Hold ``.factory/local/<name>`` as a file lock, re-entrant in one
    process; :class:`StoreBusy` when it is not had within ``wait`` seconds."""
    directory = state_dir_of(project)
    (directory / "local").mkdir(parents=True, exist_ok=True)
    path = directory / "local" / name
    key = (str(path), threading.get_ident())
    held = _held_locks.get(key)
    if held is not None:
        held[1] += 1
        try:
            yield path
        finally:
            held[1] -= 1
            if held[1] == 0:
                del _held_locks[key]
        return
    deadline = time.monotonic() + max(wait, 0.0)
    fd = os.open(path, os.O_RDWR | os.O_CREAT, 0o644)
    try:
        while True:
            try:
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except OSError as exc:
                if exc.errno not in (errno.EACCES, errno.EAGAIN):
                    raise
                if time.monotonic() >= deadline:
                    raise StoreBusy("busy", f"{path.name} is held by another process") from exc
                time.sleep(_LOCK_TIMEOUT_POLL)
        _held_locks[key] = [fd, 1]
        try:
            yield path
        finally:
            _held_locks.pop(key, None)
            fcntl.flock(fd, fcntl.LOCK_UN)
    finally:
        os.close(fd)


_held_locks: dict[tuple, list] = {}


# -------------------------------------------------------------------- git ---


def git(cwd, *args: str, timeout: int = 10, input: str | None = None) -> str:
    """S1's :func:`vsdd.state.gitrun.git` with its :class:`GitError` raised as
    ``git_failed`` — the store's one git path."""
    from .gitrun import GitError, git as run_git

    try:
        return run_git(cwd, *args, timeout=timeout, input=input)
    except GitError as exc:
        raise StoreRefused("git_failed", str(exc)) from exc
