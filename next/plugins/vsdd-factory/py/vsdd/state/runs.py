"""The run record store (records/run.md; plan records.md section 3).

``local/runs/<run-id>/run.json``; not tracked; cap 2,000 bytes. A run is live
while its ``ended`` is empty (decision D5). Run membership is the one check
every gate makes: :func:`run_require`.
"""

from __future__ import annotations

from pathlib import Path

from .store import (
    StoreRefused,
    locked,
    now as now_stamp,
    random_suffix,
    read_record,
    root_of,
    safe_segment,
    write_record,
)

__all__: list[str] = []  # the facade lists only state-api names

RUN_CAP = 2_000
_RUN_ID_PREFIX = "r-"


def run_path(project, run_id: str) -> Path:
    return root_of(project) / ".factory" / "local" / "runs" / safe_segment(run_id) / "run.json"


def runs_dir(project) -> Path:
    return root_of(project) / ".factory" / "local" / "runs"


def _new_id(stamped: str) -> str:
    return f"{_RUN_ID_PREFIX}{stamped[:10].replace('-', '')}-{stamped[11:16].replace(':', '')}-{random_suffix(2)}"


def run_open(
    project,
    *,
    workflow: str,
    cycle: str,
    session: str,
    subject: str = "",
    repository: str = "",
    parent: str = "",
    request: str = "",
    amendment: bool = False,
    stories=(),
    now: str | None = None,
) -> dict:
    """A new run record; only a workflow that takes no lock calls this
    directly — everything else goes through ``vsdd.state.locks.run_begin``."""
    stamped = now or now_stamp()
    if not workflow:
        raise StoreRefused("bad_identity", "a run needs a workflow")
    if not session:
        raise StoreRefused("bad_identity", "a run needs the calling session")
    record = {
        "workflow": workflow,
        "subject": subject,
        "repository": repository,
        "cycle": cycle,
        "session": session,
        "start": stamped,
        "agents": 0,
        "parent": parent,
        "stories": list(stories),
        "request": request,
        "amendment": bool(amendment),
        "ended": "",
    }
    directory = runs_dir(project)
    directory.mkdir(parents=True, exist_ok=True)
    while True:  # colliding ids are near-impossible; the loop makes it certain
        run_id = _new_id(stamped)
        path = run_path(project, run_id)
        try:
            path.parent.mkdir(exist_ok=False)  # the race-proof exists-then-write
        except FileExistsError:
            continue
        record["id"] = run_id
        write_record(path, record, cap=RUN_CAP, schema="run-record")
        return record


def run_read(project, run_id: str) -> dict | None:
    """The run record, or ``None`` for a run that does not exist."""
    return read_record(run_path(project, run_id), "run-record")


def run_live(project, run_id: str) -> bool:
    """True when the run exists and its ``ended`` is empty (decision D5)."""
    record = run_read(project, run_id)
    return record is not None and record.get("ended", "") == ""


def run_require(project, run_id: str, *, story: str | None = None) -> dict:
    """The run record for every command given ``--run``: ``run_unknown``,
    ``run_ended``, or, with ``story``, ``story_not_in_run`` — the one
    membership check (findings P3OPS-07, P3REQ-07)."""
    record = run_read(project, run_id)
    if record is None:
        raise StoreRefused("run_unknown", f"no run {run_id!r}")
    if record.get("ended", "") != "":
        raise StoreRefused("run_ended", f"run {run_id!r} ended {record['ended']}")
    if story is not None and story not in record.get("stories", []):
        raise StoreRefused("story_not_in_run", f"story {story!r} is not admitted to {run_id!r}")
    return record


def run_admit(project, run_id: str, story: str) -> dict:
    """Record ``story`` in the run; admitting it twice changes nothing.

    A read-modify-write, so it holds ``flush.lock`` (decision D1): a
    concurrent ``run_set`` or ``run_end`` cannot drop the admission."""
    with locked(project, "flush.lock"):
        record = run_require(project, run_id)
        if story not in record["stories"]:
            record["stories"].append(story)
            write_record(run_path(project, run_id), record, cap=RUN_CAP, schema="run-record")
        return record


def run_set(project, run_id: str, *, agents=None, amendment=None, request=None) -> dict:
    """Update the mutable fields of a live run: the agent count, the amendment
    flag, the request or question. Under ``flush.lock`` (decision D1)."""
    with locked(project, "flush.lock"):
        record = run_require(project, run_id)
        if agents is not None:
            record["agents"] = int(agents)
        if amendment is not None:
            record["amendment"] = bool(amendment)
        if request is not None:
            record["request"] = request
        write_record(run_path(project, run_id), record, cap=RUN_CAP, schema="run-record")
        return record


def run_end(project, run_id: str, *, now: str | None = None, wait: float = 3.0) -> dict | None:
    """Set ``ended`` once — the first end time stays; ``None`` for a run that
    does not exist. Under ``flush.lock`` (decision D1), waited for at most
    ``wait`` seconds: a caller already holding another file lock passes the
    budget it has left, so its own deadline bounds the whole call."""
    path = run_path(project, run_id)
    with locked(project, "flush.lock", wait=wait):
        record = read_record(path, "run-record")
        if record is None:
            return None
        if record.get("ended", "") == "":
            record["ended"] = now or now_stamp()
            write_record(path, record, cap=RUN_CAP, schema="run-record")
        return record


def runs_list(project, *, live: bool | None = None, workflow=None, subject=None) -> list[dict]:
    """Every run record, by ``start``; the filters narrow."""
    records: list[dict] = []
    directory = runs_dir(project)
    if not directory.is_dir():
        return records
    for entry in sorted(directory.iterdir()):
        record = read_record(entry / "run.json")
        if record is None:
            continue
        if live is True and record.get("ended", "") != "":
            continue
        if live is False and record.get("ended", "") == "":
            continue
        if workflow is not None and record.get("workflow", "") != workflow:
            continue
        if subject is not None and record.get("subject", "") != subject:
            continue
        records.append(record)
    records.sort(key=lambda record: (record.get("start", ""), record.get("id", "")))
    return records
