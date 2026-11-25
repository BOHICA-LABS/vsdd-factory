"""The step record store (records/step.md; plan records.md section 3).

``local/runs/<run-id>/steps/<workflow>/<subject>/<name>.<attempt>.json``; not
tracked; cap 64,000 bytes. Written only through :func:`store_step`, so the
path and the cap have one owner. A record is stored only for a live run, and
a second write to the same key is taken only from the same ``agent_id`` or
with identical content (decision D4): an agent whose result the platform
refused answers again with the same id, and must be able to replace it.

On the hook path: git, configuration and registry code is imported inside
functions only.
"""

from __future__ import annotations

import posixpath
from pathlib import Path

from .store import (
    StoreRefused,
    _literal_segment,
    read_record,
    root_of,
    safe_segment,
    write_record,
)

__all__ = ["store_step"]  # the facade lists only state-api names

STEP_CAP = 64_000


def steps_dir(project, run_id: str) -> Path:
    return root_of(project) / ".factory" / "local" / "runs" / safe_segment(run_id) / "steps"


def _identity(result: dict) -> tuple[str, str, str, str, int]:
    """The step identity every record carries, as ``(run, workflow, subject,
    name, attempt)``; ``bad_identity`` when it is missing or mistyped."""
    step = result.get("step")
    if not isinstance(step, dict):
        raise StoreRefused("bad_identity", "a step result carries no step object")
    for key in ("run", "workflow", "name"):
        if not isinstance(step.get(key), str) or not step[key]:
            raise StoreRefused("bad_identity", f"a step result needs a {key}")
    attempt = step.get("attempt")
    if not isinstance(attempt, int) or isinstance(attempt, bool) or attempt < 1:
        raise StoreRefused("bad_identity", "a step result needs an attempt of 1 or more")
    if not isinstance(step.get("subject"), str):
        raise StoreRefused("bad_identity", "a step result needs a subject, possibly empty")
    return step["run"], step["workflow"], step["subject"], step["name"], attempt


def step_key(workflow: str, subject: str, name: str, attempt: int) -> str:
    """The key the prelude gives ``spike open --step``:
    ``<workflow>/<subject>/<name>.<attempt>``."""
    return f"{workflow}/{_literal_segment(subject)}/{name}.{attempt}"


def step_path(project, run_id: str, key: str) -> Path:
    """The path of one step record, from the key ``workflow/subject/name.attempt``."""
    parts = key.split("/")
    if len(parts) != 3:
        raise StoreRefused("bad_identity", f"a step key is workflow/subject/name.attempt: {key!r}")
    workflow, subject, rest = parts
    if "." not in rest:
        raise StoreRefused("bad_identity", f"a step key ends in name.attempt: {key!r}")
    name, _, attempt = rest.rpartition(".")
    if not name or not attempt.isdigit():
        raise StoreRefused("bad_identity", f"a step key ends in name.attempt: {key!r}")
    return steps_dir(project, run_id) / safe_segment(workflow) / _literal_segment(subject) / (
        f"{safe_segment(name)}.{int(attempt)}.json"
    )


def _has_holdout_finding(result: dict) -> bool:
    """A finding whose location lies under ``.factory/holdout/`` (or under a
    ``holdout/`` directory, the state-relative spelling) needs the holdout
    evaluation file, which is not built yet; ``walled_not_built``. Only
    findings' locations count: a summary that merely mentions the wall does
    not wedge a step."""
    found: list[bool] = []

    def walk(node) -> None:
        if isinstance(node, dict):
            location = node.get("location")
            if isinstance(location, dict):
                path = location.get("path", "")
                if isinstance(path, str) and _under_holdout(path):
                    found.append(True)
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for item in node:
                walk(item)

    walk(result)
    return bool(found)


def _under_holdout(path: str) -> bool:
    """Whether a location path lies in the holdout wall, dot segments
    normalised (``.factory/./holdout/x`` too; S4R-rules-25). One definition:
    the step-results rule and the verdict read this same predicate."""
    text = posixpath.normpath(str(path).replace("\\", "/"))
    return (
        text.startswith(".factory/holdout/")
        or text.startswith("holdout/")
        or text in (".factory/holdout", "holdout")
    )


def store_step(project, result: dict, agent_type: str, head: str, *, agent_id: str = "") -> dict:
    """Store one step result with ``agent_type``, ``worktree_head = head`` and
    ``agent_id`` at the path of record step. Refuses ``bad_identity``,
    ``run_not_live``, ``schema_invalid``, ``over_cap``, ``step_exists`` and
    ``walled_not_built``; returns the stored record."""
    if not isinstance(agent_type, str) or not agent_type:
        raise StoreRefused("bad_identity", "store_step needs the calling agent's type")
    if not isinstance(head, str):
        raise StoreRefused("bad_identity", "worktree head must be a string")
    run_id, workflow, subject, name, attempt = _identity(result)
    # A record is stored only for a live run (topics/run-lock.md); an unknown
    # run is not a live run either, and run_not_live is a promised code.
    from . import runs as runs_store

    run_record = runs_store.run_read(project, run_id)
    if run_record is None:
        raise StoreRefused("run_not_live", f"no live run {run_id!r}")
    if run_record.get("ended", "") != "":
        raise StoreRefused("run_not_live", f"run {run_id!r} ended {run_record['ended']}")
    if _has_holdout_finding(result):
        raise StoreRefused(
            "walled_not_built",
            "a finding under .factory/holdout/ needs the holdout evaluation file, "
            "which is not built yet",
        )
    record = dict(result)
    record["agent_type"] = agent_type
    record["worktree_head"] = head
    record["agent_id"] = agent_id
    path = (
        steps_dir(project, run_id)
        / safe_segment(workflow)
        / _literal_segment(subject)
        / f"{safe_segment(name)}.{int(attempt)}.json"
    )
    stored = read_record(path, "step-record")
    if stored is not None and not _replace_ok(stored, record, agent_id):
        raise StoreRefused(
            "step_exists",
            f"step {run_id}/{workflow}/{subject}/{name}.{attempt} is already stored "
            f"by another agent",
        )
    write_record(path, record, cap=STEP_CAP, schema="step-record")
    return record


def _replace_ok(stored: dict, fresh: dict, agent_id: str) -> bool:
    """Decision D4: the same ``agent_id`` may replace its own step result, and
    so may a second write whose whole record is identical — every field the
    result carries (artifacts, gates, blocked, spike_request, grounded, the
    data), beside the hook's own additions, which must match too. Anyone
    else is refused."""
    if agent_id and stored.get("agent_id", "") == agent_id:
        return True
    hook = ("agent_type", "worktree_head", "agent_id")
    if all(stored.get(key) == fresh.get(key) for key in hook):
        left = {key: value for key, value in stored.items() if key not in hook}
        right = {key: value for key, value in fresh.items() if key not in hook}
        return left == right
    return False


def step_read(project, run_id: str, key: str) -> dict | None:
    """One step record by the prelude's key, or ``None``."""
    return read_record(step_path(project, run_id, key), "step-record")


def run_has_steps(project, run_id: str) -> bool:
    """True when the run holds at least one step record."""
    directory = steps_dir(project, run_id)
    if not directory.is_dir():
        return False
    return any(directory.rglob("*.json"))


def steps_list(project, run_id: str | None = None, *, workflow=None, subject=None) -> list[dict]:
    """One run's records or every run's, by run start then path; a corrupt
    record raises (nothing is silently dropped)."""
    from .runs import runs_list as _runs_list

    if run_id is not None:
        directory = steps_dir(project, run_id)
        if not directory.is_dir():
            return []
        return _collect(directory, workflow, subject)
    ordered: list[dict] = []
    for record in _runs_list(project):
        run_id_here = record.get("id", "")
        directory = steps_dir(project, run_id_here)
        if directory.is_dir():
            ordered.extend(_collect(directory, workflow, subject))
    return ordered


def _collect(directory: Path, workflow, subject) -> list[dict]:
    found: list[tuple[str, dict]] = []
    for path in sorted(directory.rglob("*.json")):
        relative = path.relative_to(directory)
        if workflow is not None and relative.parts[0] != workflow:
            continue
        if subject is not None and (
            len(relative.parts) < 2 or relative.parts[1] != _literal_segment(subject)
        ):
            continue
        found.append((str(relative), read_record(path, "step-record") or {}))
    return [record for _, record in sorted(found)]
