"""The run lock (records/lock.md, topics/run-lock.md; plan records.md
section 3, decisions D7 and D8).

``.factory/local/lock.json``, one entry per repository and branch a run can
write, guarded by ``.factory/local/lock.flock`` (the take is atomic: the file
lock is held around the read, the check and the write). ``run_begin`` joins
the run record and the lock; ``lock_seen`` decides belonging from the facts
it is given, so re-entry and the liveness rule each exist once, here.

On the hook path (``lock_seen``, ``lock_release``): git, configuration and
registry code is imported inside functions only.
"""

from __future__ import annotations

import time
from datetime import datetime, timezone
from pathlib import Path

from .store import (
    StoreBusy,
    StoreRefused,
    locked,
    now as now_stamp,
    parse_time,
    read_record,
    root_of,
    write_record,
)

__all__ = ["lock_seen", "lock_release"]  # the facade lists only state-api names

LOCK_WAIT_DEFAULT = 3.0
DEFAULT_STALE_MINUTES = 20
SEEN_WRITE_INTERVAL = 60.0


# ------------------------------------------------------------------ scope ---


def lock_scope(project, repository: str = "") -> str:
    """``<repository | project.name | root name>:<branches.integration>``."""
    from ..config import load as load_config

    root = root_of(project)
    config = load_config(root / ".factory" / "factory.toml")
    data = getattr(config, "data", config)
    if not repository:
        repository = ""
        name = ""
        project_section = (data.get("project") or {}) if isinstance(data, dict) else {}
        name = project_section.get("name", "") if isinstance(project_section, dict) else ""
        if not name:
            name = root.name
        repository = name
    branches = (data.get("branches") or {}) if isinstance(data, dict) else {}
    integration = branches.get("integration", "develop") if isinstance(branches, dict) else "develop"
    return f"{repository}:{integration}"


# ------------------------------------------------------------------ store ---


def _lock_path(project) -> Path:
    return root_of(project) / ".factory" / "local" / "lock.json"


def lock_entries(project) -> list[dict]:
    """Every entry of the run lock; an empty table when there is none."""
    record = read_record(_lock_path(project), "lock")
    if record is None:
        return []
    return list(record.get("locks", []))


def _write_entries(project, entries: list[dict]) -> None:
    path = _lock_path(project)
    if entries:
        write_record(path, {"locks": entries}, cap=None, schema="lock")
        return
    path.unlink(missing_ok=True)


def lock_stale_minutes(project) -> int:
    """``state.lock_stale_minutes`` of the project's configuration, default
    20 — the window in which the holder must refresh ``seen``."""
    from ..config import ConfigError, load as load_config

    try:
        config = load_config(root_of(project) / ".factory" / "factory.toml")
        data = getattr(config, "data", config)
        section = (data.get("state") or {}) if isinstance(data, dict) else {}
        return int(section.get("lock_stale_minutes", DEFAULT_STALE_MINUTES))
    except (ConfigError, ValueError, OSError):
        return DEFAULT_STALE_MINUTES


def lock_is_stale(project, entry: dict, now: str | None = None) -> bool:
    """The holder counts as alive while ``seen`` is recent; after
    ``state.lock_stale_minutes`` (default 20) the lock is stale."""
    reference = entry.get("seen", "") or entry.get("taken", "")
    stamp = parse_time(reference) if reference else 0.0
    if stamp == 0.0:
        return False  # an unreadable holder is the person's to release by hand
    moment = parse_time(now) if now else time.time()
    return moment - stamp > lock_stale_minutes(project) * 60


# --------------------------------------------------------------- run_begin ---


def run_begin(
    project,
    *,
    scope: str,
    workflow: str,
    subject: str,
    session: str,
    cycle: str,
    now: str | None = None,
    **run_fields,
) -> dict:
    """The survey's one call: join the run record and the lock.

    Returns ``{ok: True, run, entered, ended}`` — ``entered`` is ``new``,
    ``reentered`` or ``took_over``, ``ended`` the run id a takeover ended —
    or ``{ok: False, holder, stale_at}`` with nothing written. A free scope
    gives a new run. A stale holder's run is ended, then new. A holder with
    this session, workflow and subject gives the same run if it is live with
    no step record; else it is ended, then new. Any other holder refuses.
    """
    from . import runs as runs_store
    from .steps import run_has_steps

    stamped = now or now_stamp()
    with locked(project, "lock.flock", wait=LOCK_WAIT_DEFAULT):
        entries = lock_entries(project)
        holder = next((entry for entry in entries if entry.get("scope") == scope), None)
        if holder is None:
            run = runs_store.run_open(
                project,
                workflow=workflow,
                cycle=cycle,
                session=session,
                subject=subject,
                now=stamped,
                **run_fields,
            )
            entries.append(
                {
                    "scope": scope,
                    "run": run.get("id", ""),
                    "workflow": workflow,
                    "subject": subject,
                    "session": session,
                    "taken": stamped,
                    "seen": stamped,
                }
            )
            _write_entries(project, entries)
            return {"ok": True, "run": run, "entered": "new", "ended": ""}
        if lock_is_stale(project, holder, now=stamped):
            ended = _take_over(project, holder, stamped)
            run = _fresh_run(project, workflow, cycle, session, subject, stamped, run_fields)
            holder.update(
                run=run.get("id", ""),
                workflow=workflow,
                subject=subject,
                session=session,
                taken=stamped,
                seen=stamped,
            )
            _write_entries(project, entries)
            return {"ok": True, "run": run, "entered": "took_over", "ended": ended}
        if (
            holder.get("session") == session
            and holder.get("workflow") == workflow
            and holder.get("subject") == subject
        ):
            run = runs_store.run_read(project, holder.get("run", ""))
            if (
                run is not None
                and run.get("ended", "") == ""
                and not run_has_steps(project, holder.get("run", ""))
            ):
                holder["seen"] = stamped
                _write_entries(project, entries)
                return {"ok": True, "run": run, "entered": "reentered", "ended": ""}
            ended = _take_over(project, holder, stamped)
            run = _fresh_run(project, workflow, cycle, session, subject, stamped, run_fields)
            holder.update(
                run=run.get("id", ""),
                workflow=workflow,
                subject=subject,
                session=session,
                taken=stamped,
                seen=stamped,
            )
            _write_entries(project, entries)
            return {"ok": True, "run": run, "entered": "took_over", "ended": ended}
        return {
            "ok": False,
            "holder": dict(holder),
            "stale_at": _stale_at(project, holder),
        }


def _stale_at(project, holder: dict) -> str:
    """When the holder becomes stale: ``seen`` plus
    ``state.lock_stale_minutes`` — not the last-seen time itself. An
    unreadable stamp reads back as itself, the person's to judge."""
    reference = holder.get("seen", "") or holder.get("taken", "")
    stamp = parse_time(reference) if reference else 0.0
    if stamp == 0.0:
        return reference
    expiry = stamp + lock_stale_minutes(project) * 60
    return datetime.fromtimestamp(expiry, tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _fresh_run(project, workflow, cycle, session, subject, stamped, run_fields) -> dict:
    from . import runs as runs_store

    return runs_store.run_open(
        project,
        workflow=workflow,
        cycle=cycle,
        session=session,
        subject=subject,
        now=stamped,
        **run_fields,
    )


def _take_over(project, holder: dict, stamped: str) -> str:
    """End the held run; returns the ended run's id."""
    from . import runs as runs_store

    run_id = holder.get("run", "")
    if run_id:
        runs_store.run_end(project, run_id, now=stamped)
    return run_id


# -------------------------------------------------------------- liveness ---


def lock_seen(
    project,
    *,
    session_id: str,
    agent_id: str = "",
    run: str = "",
    story: str = "",
    now: str | None = None,
) -> bool:
    """Refresh ``seen`` of the entry the call belongs to: ``run`` is its run,
    ``story`` is in its run's ``stories``, or ``session_id`` is its session.

    An empty ``agent_id`` — the main session — gives False. True when the
    call belongs to a lock. At most one write a minute per entry. It raises
    nothing, gives False if the file lock is not had in 1 s, and with no lock
    file or nothing to refresh reads once and takes no file lock.
    """
    try:
        return _lock_seen(project, session_id=session_id, agent_id=agent_id, run=run, story=story, now=now)
    except StoreBusy:
        return False
    except Exception:
        return False


def _lock_seen(
    project,
    *,
    session_id: str,
    agent_id: str,
    run: str,
    story: str,
    now: str | None,
) -> bool:
    from . import runs as runs_store

    if not agent_id:  # the main session never refreshes a lock
        return False
    entries = lock_entries(project)
    if not entries:
        return False
    stamped = now or now_stamp()
    epoch = parse_time(stamped)
    if not any(
        _belongs(entry, project, session_id, run, story, runs_store) for entry in entries
    ):
        return False
    due = _due_entries(entries, session_id, run, story, epoch, project, runs_store)
    if not due:
        return True  # belongs, and at most one write a minute has stood
    with locked(project, "lock.flock", wait=1.0):
        # The refresh reads again under the file lock and refreshes only the
        # entries still there: a concurrent release or stale takeover is not
        # written back over (finding: the stale-copy probe).
        entries = lock_entries(project)
        changed = _refresh(entries, project, session_id, run, story, stamped, epoch, runs_store)
        if changed:
            _write_entries(project, entries)
    return True


def _belongs(entry: dict, project, session_id: str, run: str, story: str, runs_store) -> bool:
    """Whether the call belongs to this entry: its run, a story in its run, or
    its session."""
    if run and entry.get("run", "") == run:
        return True
    if story:
        holder_run = runs_store.run_read(project, entry.get("run", ""))
        if holder_run is not None and story in holder_run.get("stories", []):
            return True
    if session_id and entry.get("session", "") == session_id:
        return True
    return False


def _due_entries(entries, session_id, run, story, epoch, project, runs_store) -> int:
    """How many matching entries are past their one-write-a-minute window."""
    due = 0
    for entry in entries:
        if not _belongs(entry, project, session_id, run, story, runs_store):
            continue
        if epoch - parse_time(entry.get("seen", "") or "") >= SEEN_WRITE_INTERVAL:
            due += 1
    return due


def _refresh(entries, project, session_id, run, story, stamped, epoch, runs_store) -> bool:
    """Refresh ``seen`` in place on the matching entries still present."""
    changed = False
    for entry in entries:
        if not _belongs(entry, project, session_id, run, story, runs_store):
            continue
        if epoch - parse_time(entry.get("seen", "") or "") < SEEN_WRITE_INTERVAL:
            continue  # at most one write a minute per entry
        entry["seen"] = stamped
        changed = True
    return changed


# --------------------------------------------------------------- release ---


def lock_release(
    project,
    *,
    run: str | None = None,
    session: str | None = None,
    stale: bool = False,
    force: bool = False,
    wait: float = LOCK_WAIT_DEFAULT,
    now: str | None = None,
) -> list[dict]:
    """Remove each entry matching any selector and end its run; returns the
    removed entries. ``StoreBusy`` after ``wait``, which bounds the whole
    call; ``ValueError`` with no selector.

    Ending a run takes ``flush.lock`` inside the ``lock.flock`` this call
    holds, so each :func:`runs.run_end` gets only the wait budget left. An
    entry whose run cannot be ended in time stays in the lock (its run is
    still live); after the kept entries are written, the call raises
    :class:`StoreBusy` whose ``released`` attribute carries the entries that
    were released and whose ``kept`` carries the entries held back — the
    exception is how a caller tells ``busy`` from ``nothing matched`` and
    names the entry it could not finish (findings S3R-records-C2-2 and
    step3-records-fix-3-C1-2)."""
    from . import runs as runs_store

    if not (run or session or stale or force):
        raise ValueError("lock_release needs a selector: run, session, stale or force")
    started = time.monotonic()
    stamped = now or now_stamp()
    kept_busy: list[dict] = []
    with locked(project, "lock.flock", wait=wait):
        entries = lock_entries(project)
        keep: list[dict] = []
        released: list[dict] = []
        for entry in entries:
            matches = False
            if force:
                matches = True
            if run is not None and entry.get("run", "") == run:
                matches = True
            if session is not None and entry.get("session", "") == session:
                matches = True
            if stale and lock_is_stale(project, entry, now=stamped):
                matches = True
            if matches:
                if entry.get("run"):
                    left = wait - (time.monotonic() - started)
                    try:
                        runs_store.run_end(project, entry["run"], now=stamped, wait=max(left, 0.0))
                    except StoreBusy:
                        keep.append(entry)  # its run is not ended: the entry stays
                        kept_busy.append(entry)
                        continue
                released.append(entry)
            else:
                keep.append(entry)
        _write_entries(project, keep)
    if kept_busy:
        busy = StoreBusy(
            "busy",
            f"a flush is running; the release kept {len(kept_busy)} lock "
            f"{'entry' if len(kept_busy) == 1 else 'entries'} whose run could not be ended "
            "in time — retry when the flush finishes",
        )
        busy.released = released
        busy.kept = kept_busy
        raise busy
    return released
