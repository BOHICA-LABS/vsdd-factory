"""The run lock (plan records.md 7): run_begin, re-entry, lock_seen and
lock_release, with the facts of topics/run-lock.md."""

from __future__ import annotations

import json
import threading

import pytest

from .state_fixtures import make_project
from vsdd.state import locks, runs, steps
from vsdd.state.store import StoreBusy, StoreRefused


def _scope(root):
    return locks.lock_scope(root)


def test_lock_scope(tmp_path):
    root = make_project(tmp_path)
    assert _scope(root) == "proj:develop"
    assert locks.lock_scope(root, repository="member") == "member:develop"


def test_run_begin(tmp_path):
    root = make_project(tmp_path)
    first = locks.run_begin(
        root, scope=_scope(root), workflow="story", subject="S-9.01", session="sess-A", cycle="product-1"
    )
    assert first["ok"] and first["entered"] == "new"
    run_id = first["run"]["id"]

    # A second session gets the holder and no run.
    second = locks.run_begin(
        root, scope=_scope(root), workflow="story", subject="S-9.02", session="sess-B", cycle="product-1"
    )
    assert not second["ok"]
    assert second["holder"]["session"] == "sess-A"
    assert second["holder"]["run"] == run_id

    # A stale holder's run is ended, then new.
    path = root / ".factory" / "local" / "lock.json"
    entry = json.loads(path.read_text(encoding="utf-8"))
    entry["locks"][0]["seen"] = "2026-10-01T00:00:00Z"
    entry["locks"][0]["taken"] = "2026-10-01T00:00:00Z"
    path.write_text(json.dumps(entry), encoding="utf-8")
    third = locks.run_begin(
        root, scope=_scope(root), workflow="story", subject="S-9.02", session="sess-B", cycle="product-1"
    )
    assert third["ok"] and third["entered"] == "took_over"
    assert third["ended"] == run_id
    ended = runs.run_read(root, run_id)
    assert ended["ended"] != ""

    # A race has one winner: the file lock serializes the take over a free
    # scope; the losers each see the winner as their holder.
    locks.lock_release(root, force=True)
    results = []

    def contender(session):
        results.append(
            locks.run_begin(
                root,
                scope=_scope(root),
                workflow="story",
                subject="S-9.03",
                session=session,
                cycle="product-1",
            )
        )

    threads = [
        threading.Thread(target=contender, args=(name,)) for name in ("race-1", "race-2", "race-3")
    ]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    winners = [r for r in results if r["ok"]]
    assert len(winners) == 1
    assert winners[0]["entered"] == "new"


def test_reentry(tmp_path):
    root = make_project(tmp_path)
    scope = _scope(root)
    first = locks.run_begin(
        root, scope=scope, workflow="story", subject="S-9.01", session="sess-A", cycle="product-1"
    )
    run_id = first["run"]["id"]

    # Before the first step: the same run comes back (a clerk's retry).
    again = locks.run_begin(
        root, scope=scope, workflow="story", subject="S-9.01", session="sess-A", cycle="product-1"
    )
    assert again["entered"] == "reentered"
    assert again["run"]["id"] == run_id

    # After the first step: the old run is ended, the lock moves to a new run.
    steps.store_step(
        root,
        {
            "step": {"run": run_id, "workflow": "story", "name": "implement",
                     "subject": "S-9.01", "attempt": 1, "method": "tdd"},
            "status": "done",
            "summary": "ok",
            "head": "",
            "data": {},
        },
        "builder",
        "",
        agent_id="ag-1",
    )
    later = locks.run_begin(
        root, scope=scope, workflow="story", subject="S-9.01", session="sess-A", cycle="product-1"
    )
    assert later["entered"] == "took_over"
    assert later["ended"] == run_id
    assert later["run"]["id"] != run_id
    assert runs.run_read(root, run_id)["ended"] != ""


def test_lock_seen(tmp_path):
    root = make_project(tmp_path)
    scope = _scope(root)
    begun = locks.run_begin(
        root, scope=scope, workflow="story", subject="S-9.01", session="sess-A", cycle="product-1", stories=["S-9.01"],
        now="2026-11-24T12:29:00Z",
    )
    run_id = begun["run"]["id"]
    def seen_of():
        return locks.lock_entries(root)[0]["seen"]

    # Explicit stamps: the ISO stamps are whole seconds, so the test advances
    # the clock itself to watch the refreshes land.
    assert locks.lock_seen(root, session_id="sub-1", agent_id="ag-1", run=run_id, now="2026-11-24T12:30:00Z")
    assert seen_of() == "2026-11-24T12:30:00Z"
    assert locks.lock_seen(root, session_id="sub-2", agent_id="ag-2", story="S-9.01", now="2026-11-24T12:31:00Z")
    assert locks.lock_seen(root, session_id="sess-A", agent_id="ag-3", now="2026-11-24T12:32:00Z")
    assert seen_of() == "2026-11-24T12:32:00Z"

    # At most one write a minute: within the window the seen time stands.
    assert locks.lock_seen(root, session_id="sub-1", agent_id="ag-1", run=run_id, now="2026-11-24T12:32:30Z")
    assert seen_of() == "2026-11-24T12:32:00Z"
    assert locks.lock_seen(root, session_id="sub-1", agent_id="ag-1", run=run_id, now="2026-11-24T12:33:00Z")
    assert seen_of() == "2026-11-24T12:33:00Z"

    # The main session never refreshes, and a stranger belongs to no entry.
    assert not locks.lock_seen(root, session_id="sess-A", now="2026-11-24T12:34:00Z")
    assert not locks.lock_seen(root, session_id="stranger", agent_id="ghost", now="2026-11-24T12:34:00Z")
    assert seen_of() == "2026-11-24T12:33:00Z"

    # No lock file at all: False, quietly.
    (root / ".factory" / "local" / "lock.json").unlink()
    assert not locks.lock_seen(root, session_id="sub-1", agent_id="ag-1", run=run_id)


def test_lock_release(tmp_path):
    root = make_project(tmp_path)
    scope = _scope(root)
    begun = locks.run_begin(
        root, scope=scope, workflow="story", subject="S-9.01", session="sess-A", cycle="product-1"
    )
    run_id = begun["run"]["id"]

    # No selector is a usage error.
    with pytest.raises(ValueError):
        locks.lock_release(root)

    # Another session's entry is not touched without a selector that matches.
    gone = locks.lock_release(root, session="sess-B")
    assert gone == []

    # By run: the entry goes, its run ends.
    released = locks.lock_release(root, run=run_id)
    assert len(released) == 1 and released[0]["run"] == run_id
    assert runs.run_read(root, run_id)["ended"] != ""
    assert locks.lock_entries(root) == []

    # Stale and force selectors; StoreBusy when the file lock is not had.
    locks.run_begin(
        root, scope=scope, workflow="story", subject="S-9.01", session="sess-A", cycle="product-1"
    )
    released = locks.lock_release(root, force=True)
    assert len(released) == 1


def test_run_begin_stale_at_is_the_expiry(tmp_path):
    """``stale_at`` is when the holder becomes stale — seen plus
    ``state.lock_stale_minutes`` — not the last-seen time itself."""
    root = make_project(tmp_path)
    locks.run_begin(
        root, scope=_scope(root), workflow="story", subject="S-9.01",
        session="sess-A", cycle="product-1", now="2026-11-24T12:00:00Z",
    )
    # Ten minutes later the holder is alive; the take is refused, and
    # stale_at names the minute the holder expires.
    second = locks.run_begin(
        root, scope=_scope(root), workflow="story", subject="S-9.02",
        session="sess-B", cycle="product-1", now="2026-11-24T12:10:00Z",
    )
    assert not second["ok"]
    assert second["stale_at"] == "2026-11-24T12:20:00Z"

    # A raised window moves the expiry.
    from ._repo import set_keys

    set_keys(root, "state", lock_stale_minutes=30)
    third = locks.run_begin(
        root, scope=_scope(root), workflow="story", subject="S-9.03",
        session="sess-C", cycle="product-1", now="2026-11-24T12:10:00Z",
    )
    assert third["stale_at"] == "2026-11-24T12:30:00Z"


def test_lock_seen_refresh_does_not_undo_a_concurrent_release(tmp_path, monkeypatch):
    """A concurrent lock_release between lock_seen's read and its write is not
    written back over: the released entry stays released and its run stays
    ended. The wrapper simulates the interleaving: the release happens after
    the unlocked read, before the file lock is taken."""
    root = make_project(tmp_path)
    begun = locks.run_begin(
        root, scope=_scope(root), workflow="story", subject="S-9.01",
        session="sess-A", cycle="product-1", stories=["S-9.01"],
        now="2026-11-24T12:29:00Z",  # a minute before the refresh: a write is due
    )
    run_id = begun["run"]["id"]
    real_entries = locks.lock_entries
    fired = []

    def race_then_read(project):
        entries = real_entries(project)
        if not fired:
            fired.append(True)
            locks.lock_release(root, session="sess-A")  # the concurrent release
        return entries

    monkeypatch.setattr(locks, "lock_entries", race_then_read)
    locks.lock_seen(
        root, session_id="sub-1", agent_id="ag-1", run=run_id,
        now="2026-11-24T12:30:00Z",
    )
    monkeypatch.undo()
    assert locks.lock_entries(root) == [], "the released entry must stay released"
    assert runs.run_read(root, run_id)["ended"] != ""


def test_lock_seen_refresh_does_not_undo_a_stale_takeover(tmp_path, monkeypatch):
    """A concurrent stale takeover between lock_seen's read and its write
    keeps the new holder: the refreshed copy of the old entry is not written
    back, or the old ended run would hold the lock and the new run would have
    no entry."""
    root = make_project(tmp_path)
    begun = locks.run_begin(
        root, scope=_scope(root), workflow="story", subject="S-9.01",
        session="sess-A", cycle="product-1",
    )
    old_run = begun["run"]["id"]
    # Age the holder, so the concurrent run_begin can take it over as stale.
    path = root / ".factory" / "local" / "lock.json"
    entry = json.loads(path.read_text(encoding="utf-8"))
    entry["locks"][0]["seen"] = "2026-10-01T00:00:00Z"
    entry["locks"][0]["taken"] = "2026-10-01T00:00:00Z"
    path.write_text(json.dumps(entry), encoding="utf-8")

    real_entries = locks.lock_entries
    fired = []

    def race_then_read(project):
        entries = real_entries(project)
        if not fired:
            fired.append(True)
            locks.run_begin(
                root, scope=_scope(root), workflow="story", subject="S-9.02",
                session="sess-B", cycle="product-1",
            )  # the concurrent stale takeover
        return entries

    monkeypatch.setattr(locks, "lock_entries", race_then_read)
    locks.lock_seen(
        root, session_id="sub-1", agent_id="ag-1", run=old_run,
        now="2026-11-24T12:30:00Z",
    )
    monkeypatch.undo()
    holders = locks.lock_entries(root)
    assert len(holders) == 1
    assert holders[0]["session"] == "sess-B", "the new holder keeps the lock"
    assert holders[0]["run"] != old_run
    assert runs.run_read(root, old_run)["ended"] != ""


def test_lock_release_wait_bounds_the_whole_call(tmp_path):
    """Another process holding flush.lock must not stretch lock_release past
    its wait (PL3-8: SessionEnd gives 0.5 s): each run_end gets only the time
    left, an entry whose run cannot be ended in time stays in the lock — so
    lock.json and the run records agree — and the release is loud: StoreBusy
    within the wait, never a silent success (S3R-records-C2-2)."""
    import subprocess
    import sys
    import time as time_module

    root = make_project(tmp_path)
    begun = locks.run_begin(
        root, scope=_scope(root), workflow="story", subject="S-9.01",
        session="sess-A", cycle="product-1",
    )
    run_id = begun["run"]["id"]
    lock_file = root / ".factory" / "local" / "flush.lock"
    holder = subprocess.Popen(
        [
            sys.executable,
            "-I",
            "-c",
            "import fcntl, os, time\n"
            f"fd = os.open({str(lock_file)!r}, os.O_RDWR | os.O_CREAT)\n"
            "fcntl.flock(fd, fcntl.LOCK_EX)\n"
            "print('held', flush=True)\n"
            "time.sleep(5)\n",
        ],
        stdout=subprocess.PIPE,
        text=True,
    )
    try:
        assert holder.stdout.readline().strip() == "held"
        started = time_module.monotonic()
        with pytest.raises(StoreBusy) as busy:
            locks.lock_release(root, session="sess-A", wait=0.5)
        elapsed = time_module.monotonic() - started
        assert elapsed < 1.0, f"lock_release took {elapsed:.2f}s; wait must bound the call"
        # Nothing was released: lock.json is unchanged, the run is live.
        assert getattr(busy.value, "released", []) == []
        entries = locks.lock_entries(root)
        assert [entry["run"] for entry in entries] == [run_id]
        assert runs.run_read(root, run_id)["ended"] == ""
    finally:
        holder.kill()
        holder.wait()

    # With flush.lock free again, the same release ends the run and empties
    # the lock.
    gone = locks.lock_release(root, session="sess-A")
    assert [entry["run"] for entry in gone] == [run_id]
    assert runs.run_read(root, run_id)["ended"] != ""
    assert locks.lock_entries(root) == []


def test_lock_busy(tmp_path, monkeypatch):
    import subprocess
    import sys

    root = make_project(tmp_path)
    locks.run_begin(
        root, scope=_scope(root), workflow="story", subject="S-9.01", session="sess-A", cycle="product-1"
    )
    lock_file = root / ".factory" / "local" / "lock.flock"
    holder = subprocess.Popen(
        [
            sys.executable,
            "-I",
            "-c",
            "import fcntl, time\n"
            f"fd = os_fd = None\n"
            f"import os\n"
            f"fd = os.open({str(lock_file)!r}, os.O_RDWR | os.O_CREAT)\n"
            "fcntl.flock(fd, fcntl.LOCK_EX)\n"
            "print('held', flush=True)\n"
            "time.sleep(5)\n",
        ],
        stdout=subprocess.PIPE,
        text=True,
    )
    try:
        assert holder.stdout.readline().strip() == "held"
        monkeypatch.setattr(locks, "LOCK_WAIT_DEFAULT", 0.1)
        with pytest.raises(StoreBusy):
            locks.run_begin(
                root, scope=_scope(root), workflow="story", subject="S-9.02",
                session="sess-B", cycle="product-1",
            )
    finally:
        holder.kill()
        holder.wait()
