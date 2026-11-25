"""The cycle and closure record stores (records/cycle.md, records/closure.md).

Cycle records: ``cycles/<cycle>/cycle.json``; closure records:
``cycles/<cycle>/units/<workflow>[.<subject>].json``; both tracked, written
with ``indent=2`` and key order kept, because the order of ``stages`` is data.
:func:`closure_update` is the one write path for every writer, the survey
included: it owns the read-modify-write under ``flush.lock`` and the
validation.

The note store (:func:`read_notes`) lives here too: ``notes.json`` at the
state root is the other tracked record. Init writes ``{"notes": []}``, and a
project with that form reads as no notes.
"""

from __future__ import annotations

import json
from pathlib import Path

from .store import (
    DecisionRefused,
    StoreRefused,
    _literal_segment,
    locked,
    read_record,
    root_of,
    safe_segment,
    write_record,
)

__all__: list[str] = []  # the facade lists only state-api names

CYCLE_CAP = 2_000
CLOSURE_CAP = 16_000


def cycles_dir(project) -> Path:
    return root_of(project) / ".factory" / "cycles"


def cycle_dir(project, cycle: str) -> Path:
    return cycles_dir(project) / safe_segment(cycle)


def cycle_read(project, cycle: str) -> dict | None:
    """The cycle record, or ``None``."""
    return read_record(cycle_dir(project, cycle) / "cycle.json", "cycle-record")


def cycle_write(project, record: dict) -> dict:
    """Write one cycle record: the opening command, ``vsdd feature route``
    (the route) and the release checkpoint (the close fields).

    A read-modify-write like every tracked write, so it goes under
    ``flush.lock`` (decision D1)."""
    cycle = record.get("id", "")
    path = cycle_dir(project, cycle) / "cycle.json"
    with locked(project, "flush.lock"):
        write_record(path, record, cap=CYCLE_CAP, schema="cycle-record")
    return record


def cycles_open(project) -> list[dict]:
    """Every cycle without ``closed``, ``_standing`` excluded, by ``opened``."""
    records: list[dict] = []
    directory = cycles_dir(project)
    if not directory.is_dir():
        return records
    for entry in sorted(directory.iterdir()):
        if not entry.is_dir() or entry.name == "_standing":
            continue
        record = read_record(entry / "cycle.json", "cycle-record")
        if record is None or record.get("closed"):
            continue
        records.append(record)
    records.sort(key=lambda record: (record.get("opened", ""), record.get("id", "")))
    return records


# ---------------------------------------------------------------- closures ---


def closure_path(project, cycle: str, workflow: str, subject: str = "") -> Path:
    directory = cycle_dir(project, cycle) / "units"
    name = safe_segment(workflow)
    if subject:
        name = f"{name}.{_literal_segment(subject)}"
    return directory / f"{name}.json"


def closure_read(project, cycle: str, workflow: str, subject: str = "") -> dict | None:
    """The unit's closure record, or ``None``."""
    return read_record(closure_path(project, cycle, workflow, subject), "closure-record")


def closures_list(project, cycle: str) -> list[dict]:
    """Every closure record of the cycle, by path."""
    directory = cycle_dir(project, cycle) / "units"
    if not directory.is_dir():
        return []
    records: list[tuple[str, dict]] = []
    for path in sorted(directory.glob("*.json")):
        record = read_record(path, "closure-record")
        if record is not None:
            records.append((path.name, record))
    return [record for _, record in records]


def closure_update(project, cycle: str, workflow: str, subject: str, mutate) -> dict:
    """The one write path. Under ``flush.lock`` it loads the record or builds
    a fresh ``{schema: 1, workflow, subject, cycle, state: "open", stages:
    {}}``, calls ``mutate(record)``, validates and writes. Admission sets
    ``delivery.base`` through it. The fresh record is built here, never
    copied from a template, so one new closure's mutation cannot leak into
    the next one."""
    with locked(project, "flush.lock"):
        path = closure_path(project, cycle, workflow, subject)
        record = read_record(path, "closure-record")
        if record is None:
            record = {
                "schema": 1,
                "workflow": workflow,
                "subject": subject,
                "cycle": cycle,
                "state": "open",
                "stages": {},
            }
        mutate(record)
        write_record(path, record, cap=CLOSURE_CAP, schema="closure-record")
        return record


def closure_find(project, workflow: str, subject: str) -> dict | None:
    """The closure record for one workflow and subject over the open cycles
    and ``_standing``; ``ambiguous_unit`` when two hold one."""
    from .cycles import cycles_open as _open  # local import keeps the module flat

    seen: list[tuple[str, dict]] = []
    names = [record["id"] for record in _open(project)]
    if "_standing" not in names:
        names.append("_standing")
    for cycle in names:
        record = closure_read(project, cycle, workflow, subject)
        if record is not None:
            seen.append((cycle, record))
    if not seen:
        return None
    if len(seen) > 1:
        raise DecisionRefused(
            "ambiguous_unit",
            f"{workflow}.{subject} has closure records in " + ", ".join(c for c, _ in seen),
            choices=[c for c, _ in seen],
        )
    return seen[0][1]


# ------------------------------------------------------------------- notes ---


def read_notes(project) -> list:
    """The person's notes from ``notes.json`` at the state root, oldest first,
    as they are stored: a plain string for the first notes, and a
    ``{at, text}`` object once the status report enriches them.

    Init writes the file as ``{"notes": []}``, and that form reads as no
    notes. A missing file is no notes; a file that does not parse is
    :class:`StoreCorrupt`.
    """
    path = root_of(project) / ".factory" / "notes.json"
    try:
        text = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return []
    except OSError as exc:
        from .store import StoreCorrupt

        raise StoreCorrupt("corrupt", f"{path}: cannot be read: {exc}") from exc
    try:
        data = json.loads(text)
    except json.JSONDecodeError as exc:
        from .store import StoreCorrupt

        raise StoreCorrupt("corrupt", f"{path}: does not parse: {exc}") from exc
    notes = data.get("notes") if isinstance(data, dict) else None
    if not isinstance(notes, list):
        return []
    return list(notes)
