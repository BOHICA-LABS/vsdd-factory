"""The two commands (plan records.md 7): ``vsdd lock release`` and
``vsdd decide`` — exit 0 settled, 1 refused, 2 usage or caller errors."""

from __future__ import annotations

import json

from .state_fixtures import make_project
from vsdd.cli import dispatch
from vsdd.state import locks, decisions, runs
from vsdd.state.callers import calls_dir

TERMINAL = {"HOME": "/nonexistent-vsdd-home"}


def _run(root, *args, stdin_tty=True, environ=None):
    out, err = __import__("io").StringIO(), __import__("io").StringIO()
    code = dispatch.main(
        ["-C", str(root), *args],
        stdout=out,
        stderr=err,
        stdin_tty=stdin_tty,
        environ=environ if environ is not None else dict(TERMINAL),
    )
    return code, out.getvalue(), err.getvalue()


def _write_marker(root, session):
    key = "abcd1234abcd1234abcd1234abcd1234"
    directory = calls_dir(root)
    directory.mkdir(parents=True, exist_ok=True)
    (directory / f"{key}.json").write_text(
        json.dumps(
            {
                "v": 1,
                "key": key,
                "session_id": session,
                "agent_id": "",
                "agent_type": "",
                "cwd": str(root),
                "ts": "2026-11-24T12:00:00.000Z",
            }
        ),
        encoding="utf-8",
    )
    return key


def _begin(root, session):
    locks.run_begin(
        root,
        scope=locks.lock_scope(root),
        workflow="story",
        subject="S-9.01",
        session=session,
        cycle="product-1",
    )


def test_cmd_lock_release(tmp_path):
    root = make_project(tmp_path)
    _begin(root, "sess-mine")

    # Own session goes; the marker names the session.
    key = _write_marker(root, "sess-mine")
    code, out, _err = _run(root, "lock", "release", environ={"VSDD_CALLER": key, "HOME": "/x"})
    assert code == 0
    payload = json.loads(out)
    assert payload["released"][0]["session"] == "sess-mine"
    assert payload["holder"] is None
    assert not (calls_dir(root) / f"{key}.json").exists()  # the marker is consumed

    # Another session's live entry remains: exit 1 with the holder.
    _begin(root, "sess-other")
    code, out, _err = _run(root, "lock", "release")
    assert code == 1
    payload = json.loads(out)
    assert payload["holder"]["session"] == "sess-other"
    assert payload["error"]

    # Stale entries go without a marker (the person in their own terminal).
    path = root / ".factory" / "local" / "lock.json"
    entry = json.loads(path.read_text(encoding="utf-8"))
    entry["locks"][0]["seen"] = "2026-10-01T00:00:00Z"
    entry["locks"][0]["taken"] = "2026-10-01T00:00:00Z"
    path.write_text(json.dumps(entry), encoding="utf-8")
    code, out, _err = _run(root, "lock", "release")
    assert code == 0
    assert json.loads(out)["released"][0]["session"] == "sess-other"

    # --force with no terminal exits 2 (person only).
    _begin(root, "sess-live")
    code, _out, err = _run(root, "lock", "release", "--force", stdin_tty=False)
    assert code == 2
    code, out, _err = _run(root, "lock", "release", "--force")
    assert code == 0
    assert json.loads(out)["holder"] is None


def test_cmd_lock_release_while_a_flush_holds_the_lock(tmp_path):
    """With flush.lock busy the release is loud (S3R-records-C2-2): --force
    exits 1 — not 0 with holder null — names a kept entry as the holder, the
    error says a flush is running, and lock.json keeps its entry."""
    import subprocess
    import sys

    root = make_project(tmp_path)
    _begin(root, "sess-live")
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
        code, out, _err = _run(root, "lock", "release", "--force")
        assert code == 1
        payload = json.loads(out)
        assert payload["released"] == []
        assert payload["holder"] is not None, "a busy release never reads as nothing held"
        assert payload["holder"]["session"] == "sess-live"
        assert "flush" in payload["error"]
        # lock.json is unchanged and the run is still live.
        entries = locks.lock_entries(root)
        assert [entry["session"] for entry in entries] == ["sess-live"]
        assert runs.run_read(root, entries[0]["run"])["ended"] == ""
    finally:
        holder.kill()
        holder.wait()

    # With flush.lock free again, the same command succeeds.
    code, out, _err = _run(root, "lock", "release", "--force")
    assert code == 0
    assert json.loads(out)["holder"] is None
    assert locks.lock_entries(root) == []


def test_cmd_lock_release_names_a_kept_entry_not_the_first_lock_entry(tmp_path):
    """With flush.lock busy the holder is the kept entry — this session's own,
    held back because its run_end was busy — never ``lock_entries()[0``
    (finding step3-records-fix-3-C1-3): another session's live entry stands
    first in lock.json, so the first entry is not the kept one."""
    import subprocess
    import sys

    root = make_project(tmp_path)
    for scope, session in (("repo-a:develop", "sess-other"), ("repo-b:develop", "sess-mine")):
        joined = locks.run_begin(
            root,
            scope=scope,
            workflow="story",
            subject="S-9.01",
            session=session,
            cycle="product-1",
        )
        assert joined["ok"], joined
    runs_before = {entry["session"]: entry["run"] for entry in locks.lock_entries(root)}
    assert list(runs_before) == ["sess-other", "sess-mine"]

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
        key = _write_marker(root, "sess-mine")
        code, out, _err = _run(root, "lock", "release", environ={"VSDD_CALLER": key, "HOME": "/x"})
        assert code == 1
        payload = json.loads(out)
        assert payload["released"] == []
        assert payload["holder"]["session"] == "sess-mine", payload["holder"]
        assert "flush" in payload["error"], payload["error"]
        # lock.json keeps both entries in order and both runs stay live.
        entries = locks.lock_entries(root)
        assert [entry["session"] for entry in entries] == ["sess-other", "sess-mine"]
        for session, run_id in runs_before.items():
            assert runs.run_read(root, run_id)["ended"] == "", session
    finally:
        holder.kill()
        holder.wait()


def test_cmd_lock_release_while_the_run_lock_is_held(tmp_path):
    """With ``lock.flock`` itself held — by another release or a survey, no
    flush involved — the release exits 1 after its wait and says the run lock
    is busy, not that a flush is running (finding
    step3-records-fix-3-C1-3); no entry is named that was never matched."""
    import subprocess
    import sys
    import time

    root = make_project(tmp_path)
    _begin(root, "sess-live")
    lock_file = root / ".factory" / "local" / "lock.flock"
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
        started = time.monotonic()
        code, out, _err = _run(root, "lock", "release", "--force")
        elapsed = time.monotonic() - started
        assert code == 1
        assert elapsed < 4.0, f"the wait must bound the call, took {elapsed:.2f}s"
        payload = json.loads(out)
        assert payload["released"] == []
        assert payload["holder"] is None, "nothing was matched: no entry to name"
        assert "run lock is busy" in payload["error"], payload["error"]
        assert "flush" not in payload["error"], payload["error"]
        # lock.json is unchanged and the run is still live.
        entries = locks.lock_entries(root)
        assert [entry["session"] for entry in entries] == ["sess-live"]
        assert runs.run_read(root, entries[0]["run"])["ended"] == ""
    finally:
        holder.kill()
        holder.wait()

    # With lock.flock free again, the same command succeeds.
    code, out, _err = _run(root, "lock", "release", "--force")
    assert code == 0
    assert json.loads(out)["holder"] is None
    assert locks.lock_entries(root) == []


def test_cmd_decide(tmp_path):
    root = make_project(tmp_path)
    decisions.decision_open(root, cycle="product-1", kind="release", packet="# Release\n")

    # A person in a terminal settles; exit 0 with the record.
    code, out, _err = _run(root, "decide", "release", "approve")
    assert code == 0
    payload = json.loads(out)
    assert payload["decision"]["status"] == "approved"
    assert payload["decision"]["settled_by"] == "terminal"
    assert payload["error"] == ""

    # A refused choice exits 1 and names the choices.
    decisions.decision_open(root, cycle="product-1", kind="spec", packet="# Spec\n")
    code, out, _err = _run(root, "decide", "spec", "hold", "--json")
    assert code == 1
    payload = json.loads(out)
    assert payload["error"] == "bad_choice"
    assert "approve" in payload["choices"]
    assert payload["decision"] is None

    # The event log carries the decision events, as session terminal, and
    # each names the decision, the choice and — for a refusal — the code
    # (tables/event-types.toml).
    events = (root / ".factory" / "local" / "events").glob("*.jsonl")
    lines = [json.loads(line) for day in events for line in day.read_text(encoding="utf-8").splitlines()]
    types = [line["type"] for line in lines]
    assert "decision.recorded" in types
    assert "decision.refused" in types
    assert all(line["session_id"] == "terminal" for line in lines)
    recorded = [line for line in lines if line["type"] == "decision.recorded"]
    assert recorded[0]["decision"] == "release"
    assert recorded[0]["choice"] == "approve"
    refused = [line for line in lines if line["type"] == "decision.refused"]
    assert refused[0]["decision"] == "spec"
    assert refused[0]["choice"] == "hold"
    assert refused[0]["code"] == "bad_choice"


def test_cmd_decide_log_failure_after_settle(tmp_path, monkeypatch):
    """A failing event log after the settle changes nothing: exit 0 and the
    record is settled anyway — the log is the witness, not the outcome."""
    from vsdd.events import EventLog

    root = make_project(tmp_path)
    decisions.decision_open(root, cycle="product-1", kind="release", packet="# Release\n")

    def broken(self, *args, **kwargs):
        raise OSError("the events disk is gone")

    monkeypatch.setattr(EventLog, "log", broken)
    code, out, _err = _run(root, "decide", "release", "approve")
    monkeypatch.undo()
    assert code == 0
    payload = json.loads(out)
    assert payload["decision"]["status"] == "approved"
    assert decisions.decision_read(root, "release")["status"] == "approved"


def test_cmd_decide_exits_two(tmp_path):
    root = make_project(tmp_path)
    decisions.decision_open(root, cycle="product-1", kind="release", packet="# Release\n")

    # A caller with a marker is an agent: exit 2, nothing settled.
    key = _write_marker(root, "agent-session")
    code, _out, err = _run(root, "decide", "release", "approve", environ={"VSDD_CALLER": key, "HOME": "/x"})
    assert code == 2
    assert decisions.decision_read(root, "release")["status"] == "open"

    # Bad grammar exits 2 (a positional value).
    code, _out, _err = _run(root, "decide", "release", "approve", "B")
    assert code == 2
    assert decisions.decision_read(root, "release")["status"] == "open"

    # --json after -- is note text: the output is the indented text form, and
    # the note keeps the token.
    code, out, _err = _run(
        root, "decide", "release", "hold", "--", "wait", "--json", "please"
    )
    assert code == 0
    payload = json.loads(out)  # the text form is indented JSON
    assert payload["decision"]["note"] == "wait --json please"
