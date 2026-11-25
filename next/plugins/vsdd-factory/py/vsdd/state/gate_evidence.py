"""The evidence store and the per-gate lookup (records/evidence.md,
topics/evidence-lookup.md; plan records.md section 3).

``local/evidence/<subject>/<gate>.<sha12>.json``, written once per gate and
commit by rename — under ``flush.lock`` for the ``seq`` that orders
same-second records — with the raw output (``.log``, cut to the last 256 KB),
the JUnit report or the diff patch beside it. The lookup is per gate:
``red`` the latest passing or re-freeze record on a commit of the story branch
since its base; ``green`` and ``demo`` the passing record for the head or one
with the head's patch id; ``review`` what ``vsdd.state.verdicts.covering``
answers: the verdict computed from the stored passes when it covers the head,
else the closure's stored verdict when that covers it. Gate modules register
their measuring functions here at import (the survey imports no gate package).
"""

from __future__ import annotations

import importlib
import shutil
from pathlib import Path

from .store import (
    StoreCorrupt,
    StoreRefused,
    locked,
    now as now_stamp,
    read_record,
    root_of,
    safe_segment,
    write_record,
    write_via_tmp,
)

__all__ = ["evidence"]  # the facade lists only state-api names

EVIDENCE_CAP = 8_000
LOG_CUT = 256 * 1024
EVIDENCE_LOCK_WAIT = 2.0
_LOOKUPS = ("red", "green", "demo", "review")
_REGISTERED: dict[str, object] = {}


def evidence_dir(project, subject: str) -> Path:
    """``local/evidence/<subject>`` — ``_base`` holds the base reports beside
    the records and is a subject like any other."""
    return root_of(project) / ".factory" / "local" / "evidence" / safe_segment(subject)


def evidence_drop(project, subject: str) -> None:
    """Remove the subject's evidence directory; the unit closed."""
    directory = evidence_dir(project, subject)
    if directory.is_dir():
        shutil.rmtree(directory)


def _evidence_records(project, subject: str, gate: str | None = None) -> list[dict]:
    directory = evidence_dir(project, subject)
    if not directory.is_dir():
        return []
    found: list[tuple[str, dict]] = []
    for path in sorted(directory.glob(f"*.json" if gate is None else f"{safe_segment(gate)}.*.json")):
        record = read_record(path, "evidence-record")
        if record is None:
            continue
        found.append((path.name, record))
    found.sort(
        key=lambda pair: (pair[1].get("seq", 0), pair[1].get("at", ""), pair[0]),
    )
    return [record for _, record in found]


def evidence_write(project, record: dict, *, log: str | None = None, report=None) -> dict:
    """Stamp ``schema``, ``at`` and ``seq``, validate, and write the record —
    replacing that gate and commit's record — with the last 256 KB of the
    text ``log`` as ``.log`` and a copy of the file ``report`` as ``.xml``. A
    passing ``red`` record needs ``frozen``, ``tests`` and ``criteria``, and
    a ``refreeze`` record must list every file the latest earlier ``red``
    record froze (``refreeze_incomplete``). Raises ``StoreBusy`` when
    ``flush.lock`` is not had within ``EVIDENCE_LOCK_WAIT`` seconds: the
    sequence number is taken under that lock, so two records of one
    whole-second ``at`` stamp order by the write's order, not the file name
    (finding S3B-records-1)."""
    record = dict(record)
    record.setdefault("schema", 1)
    record["at"] = record.get("at") or now_stamp()
    gate = record.get("gate", "")
    subject = record.get("subject", "")
    sha = str(record.get("sha", ""))
    if gate not in ("red", "green", "demo", "review"):
        raise StoreRefused("schema_invalid", f"{gate!r} is not a gate with evidence")
    if not gate or len(sha) < 12:
        raise StoreRefused("schema_invalid", "an evidence record needs a gate and a sha")
    if gate == "red" and record.get("passed"):
        for key in ("frozen", "tests", "criteria"):
            if key not in record:
                raise StoreRefused(
                    "refreeze_incomplete",
                    f"a passing red record carries {key}; the frozen set is complete or nothing",
                )
        frozen_files = {entry.get("file") for entry in record["frozen"]}
        if any(not item for item in frozen_files):
            raise StoreRefused("refreeze_incomplete", "a frozen entry names its file")
        if len(frozen_files) != len(record["frozen"]):
            raise StoreRefused("refreeze_incomplete", "a frozen file is listed twice")
    if gate == "red" and record.get("refreeze"):
        frozen_files = {entry.get("file") for entry in record.get("frozen", [])}
        for earlier in reversed(_evidence_records(project, subject, "red")):
            if not (earlier.get("passed") or earlier.get("refreeze")):
                continue  # a failing red froze nothing to re-freeze
            for entry in earlier.get("frozen", []):
                if entry.get("file") not in frozen_files:
                    raise StoreRefused(
                        "refreeze_incomplete",
                        f"the re-freeze does not list {entry.get('file')!r}, "
                        "which the earlier red record froze",
                    )
            break  # the latest earlier red that passed or re-froze holds
    directory = evidence_dir(project, subject)
    directory.mkdir(parents=True, exist_ok=True)
    stem = f"{safe_segment(gate)}.{sha[:12]}"
    # The order is total and independent of the clock's whole-second stamp
    # (S3B-records-1): the sequence number is one more than the highest in
    # the subject's directory, taken under the store's lock together with the
    # write it numbers. ``at`` stays for people.
    with locked(project, "flush.lock", wait=EVIDENCE_LOCK_WAIT):
        record["seq"] = _next_seq(directory)
        # Written once, by rename, like the step records: a measuring command
        # never waits on a flush of its own making (decision D1). The side
        # files go through the temporary directory too, so a reader never sees
        # a truncated one.
        write_record(
            directory / f"{stem}.json", record, cap=EVIDENCE_CAP, schema="evidence-record"
        )
    if log is not None:
        text = str(log)
        if len(text) > LOG_CUT:
            text = text[-LOG_CUT:]
        write_via_tmp(directory / f"{stem}.log", text.encode("utf-8"))
    if report is not None:
        source = Path(report)
        if source.is_file():
            write_via_tmp(directory / f"{stem}.xml", source.read_bytes())
    return record


def _next_seq(directory: Path) -> int:
    """One more than the highest sequence number in the directory, so every
    rewrite and every new record orders after what is there. A file that
    does not parse counts as nothing: its replacement is being written."""
    highest = 0
    for path in directory.glob("*.json"):
        try:
            record = read_record(path)
        except StoreCorrupt:
            continue
        value = record.get("seq", 0) if record else 0
        if isinstance(value, int) and value > highest:
            highest = value
    return highest + 1


def evidence_read(project, subject: str, gate: str, sha: str) -> dict | None:
    """That commit's record, for the ``step-results`` rule."""
    if len(str(sha)) < 12:
        return None
    path = evidence_dir(project, subject) / f"{safe_segment(gate)}.{str(sha)[:12]}.json"
    return read_record(path, "evidence-record")


def patch_id(tree, base: str, head: str) -> str:
    """``git diff base head | git patch-id --stable`` in ``tree``; the id
    alone, not git's whole output line; empty when there is no diff."""
    from .store import git

    diff = git(tree, "diff", base, head)
    if not diff.strip():
        return ""
    line = git(tree, "patch-id", "--stable", input=diff).strip()
    return line.split(" ", 1)[0] if line else ""


def _worktree_of(project, subject: str) -> Path | None:
    """The subject's worktree: ``.worktrees/<subject>``, or the named spike
    ``.worktrees/spike-<subject>``; ``None`` without one."""
    root = root_of(project)
    for name in (subject, f"spike-{subject}"):
        candidate = root / ".worktrees" / safe_segment(name)
        if candidate.is_dir():
            return candidate
    return None


def _head_of(project, subject: str) -> str | None:
    from .store import git

    tree = _worktree_of(project, subject)
    if tree is None or not tree.is_dir():
        return None
    try:
        return git(tree, "rev-parse", "HEAD")
    except StoreRefused:
        return None


def _base_of(project, subject: str) -> str | None:
    from .cycles import closures_list, cycles_open

    names = [record["id"] for record in cycles_open(project)]
    if "_standing" not in names:
        names.append("_standing")
    for cycle in names:
        for record in closures_list(project, cycle):
            if record.get("subject", "") != subject:
                continue
            base = (record.get("delivery") or {}).get("base", "")
            if base:
                return base
    return None


def _is_ancestor(tree: Path, older: str, younger: str) -> bool:
    from .store import git

    try:
        git(tree, "merge-base", "--is-ancestor", older, younger)
        return True
    except StoreRefused:
        return False


def evidence(project, gate: str, subject: str) -> dict | None:
    """The per-gate lookup; ``None`` when nothing covers the head, or without
    a worktree or a base. Any other gate refuses ``gate_lookup_not_built``
    after the first lookup has imported the gate modules that are built."""
    from .store import git

    if gate not in _LOOKUPS:
        _ensure_gates_loaded()
        raise StoreRefused("gate_lookup_not_built", f"no evidence lookup for gate {gate!r}")
    head = _head_of(project, subject)
    if head is None:
        return None
    base = _base_of(project, subject)
    if not base:
        return None
    tree = _worktree_of(project, subject)
    records = _evidence_records(project, subject, gate)
    if gate == "red":
        for record in reversed(records):
            if not (record.get("passed") or record.get("refreeze")):
                continue
            sha = record.get("sha", "")
            if not sha:
                continue
            if sha == head or _is_ancestor(tree, sha, head):
                if sha == base or _is_ancestor(tree, base, sha):
                    return record
        return None
    if gate in ("green", "demo"):
        current = patch_id(tree, base, head)
        for record in reversed(records):
            if not record.get("passed"):
                continue
            if record.get("sha", "") == head:
                return record
        for record in reversed(records):
            if not record.get("passed"):
                continue
            mark = record.get("patch_id", "")
            if mark and mark == current:
                return record
        return None
    return _review_lookup(project, subject)


def _review_lookup(project, subject: str) -> dict | None:
    """The ``review`` gate's lookup (PL4-2): the verdict the tool computes
    from the stored passes when it covers the head, else the converged
    closure verdict that does — ``vsdd.state.verdicts.covering``, which reads
    the closure itself, so there is no cycle."""
    from .verdicts import covering

    return covering(project, subject)


def register_gate(name: str, fn) -> None:
    """Register a gate's measuring function under its gate name; a
    ``gate_<name>.py`` module calls this at import."""
    _REGISTERED[name] = fn


def registered_gate(name: str):
    """The measuring function registered for ``name``, importing the gate
    modules once; ``None`` when no gate module built it."""
    if name not in _REGISTERED:
        _ensure_gates_loaded()
    return _REGISTERED.get(name)


_GATES_LOADED = False
_GATES_LOADING = False


def _ensure_gates_loaded() -> None:
    """Import each ``vsdd.gates.gate_*`` module once, so the gate modules'
    registrations land. Only a missing ``vsdd.gates`` is swallowed: a gate
    module that fails is loud, and the loaded mark is set after the imports
    succeed, so a failure never leaves later lookups silently empty."""
    global _GATES_LOADED, _GATES_LOADING
    if _GATES_LOADED or _GATES_LOADING:
        return
    _GATES_LOADING = True
    try:
        try:
            package = importlib.import_module("vsdd.gates")
        except ModuleNotFoundError as exc:
            if exc.name == "vsdd.gates":
                _GATES_LOADED = True  # no gates package shipped: nothing to load
                return
            raise
        base = Path(getattr(package, "__file__", "")).parent
        if base.is_dir():
            for path in sorted(base.glob("gate_*.py")):
                importlib.import_module(f"vsdd.gates.{path.stem}")
        _GATES_LOADED = True
    finally:
        _GATES_LOADING = False
