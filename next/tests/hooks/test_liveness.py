"""Lock liveness: the refresh and the session-end release (plan
hook-runner-b 3.2, 7; test_liveness.py). A project from ``_repo.make_project``
holds a lock from ``run_begin``; the runner calls records' ``lock_seen`` and
``lock_release`` through the two core duties."""

from __future__ import annotations

import json
import time
from datetime import datetime, timezone
from pathlib import Path

import pytest

from tests.hooks import runner_helpers as helpers
from tests.hooks.test_runner import base_env, read_events
from tests.unit._repo import make_project as repo_make_project
from vsdd.state import locks as run_locks
from vsdd.state import runs as runs_store
from vsdd.state import store as run_store

BLOCKING = "tests.hooks.fixture_rules.blocking"

SESSION = "sess_liveness"
AGENT = "wf-1"

OTHER_SESSION = "sess_other_run"
OTHER_AGENT = "wf-2"


@pytest.fixture()
def locked_run(tmp_path):
    """A real project holding one run lock, taken through records."""
    project = repo_make_project(tmp_path)
    begun = _begin(project, SESSION)
    assert begun["ok"], begun
    return project, begun["run"]["id"]


def _begin(project, session, *, scope=None, stories=("S-3.01",)):
    return run_locks.run_begin(
        project,
        scope=scope or run_locks.lock_scope(project),
        workflow="wave",
        subject="W-1",
        session=session,
        cycle="c1",
        stories=stories,
    )


def _bash(project, command, *, session=SESSION, agent=AGENT, cwd=None):
    payload = helpers.payload("PreToolUse/bash-vsdd.json")
    payload["cwd"] = str(cwd or project)
    payload["tool_input"] = {"command": command}
    payload["session_id"] = session
    if agent:
        payload["agent_id"] = agent
        payload["agent_type"] = "workflow-subagent"
    else:
        payload.pop("agent_id", None)
        payload.pop("agent_type", None)
    return payload


def _step(project, run_id, *, session=SESSION, agent=AGENT):
    """A StructuredOutput step result with the identity a real step sends
    (S4R-rules-12): the fields of the step-result schema's step object, and
    the result's required keys, so the step-results rule stores it instead of
    refusing it as ``bad_identity``."""
    payload = helpers.payload("PreToolUse/output-structured.json")
    payload["cwd"] = str(project)
    payload["tool_input"] = {
        "step": {
            "run": run_id,
            "workflow": "wave",
            "subject": "W-1",
            "name": "implement",
            "attempt": 1,
            "method": "tdd",
        },
        "status": "done",
        "summary": "the step went",
        "head": "",
        "data": {},
    }
    payload["session_id"] = session
    payload["agent_id"] = agent
    return payload


def _end(project, session=SESSION, reason="exit"):
    return {
        "hook_event_name": "SessionEnd",
        "session_id": session,
        "reason": reason,
        "cwd": str(project),
    }


def _age_seen(project, seconds=120):
    """Put the holder's ``seen`` far enough back that one refresh writes."""
    path = Path(project) / ".factory" / "local" / "lock.json"
    record = run_store.read_record(path, "lock")
    stamp = datetime.fromtimestamp(
        run_store.parse_time(run_store.now()) - seconds, tz=timezone.utc
    )
    record["locks"][0]["seen"] = stamp.strftime("%Y-%m-%dT%H:%M:%SZ")
    run_store.write_record(path, record, cap=None, schema="lock")


def _seen(project):
    entries = run_locks.lock_entries(project)
    return run_store.parse_time(entries[0]["seen"])


def _fire(project, payload, *, entries=None):
    return helpers.run_hook(payload, "gate" if payload["hook_event_name"] == "PreToolUse" else "notify",
                            entries=entries, env=base_env(project))


def _launched_events(project):
    """The events the launcher wrote: it logs beside the project's state."""
    directory = Path(project) / ".factory" / "local" / "events"
    lines: list[dict] = []
    for day in sorted(directory.glob("*.jsonl")):
        for line in day.read_text(encoding="utf-8").splitlines():
            if line.strip():
                lines.append(json.loads(line))
    return lines


def test_lock_refresh(locked_run):
    project, run_id = locked_run

    # An agent's vsdd call carrying --run moves seen:
    _age_seen(project)
    before = _seen(project)
    code, out, err = _fire(project, _bash(project, f"vsdd gate --run {run_id}"))
    assert code == 0, err
    assert _seen(project) > before

    # A step result whose step.run is the run moves seen:
    _age_seen(project)
    before = _seen(project)
    code, out, err = _fire(project, _step(project, run_id))
    assert code == 0, err
    assert _seen(project) > before

    # An agent's call in the story's worktree moves seen (the story is in the
    # run's stories), through ctx.story:
    worktree = project / ".worktrees" / "S-3.01"
    worktree.mkdir(parents=True)
    git_dir = project / ".git" / "worktrees" / "S-3.01"
    git_dir.mkdir(parents=True)
    (worktree / ".git").write_text(f"gitdir: {git_dir}\n", encoding="utf-8")
    (git_dir / "HEAD").write_text("ref: story/S-3.01\n", encoding="utf-8")
    _age_seen(project)
    before = _seen(project)
    code, out, err = _fire(
        project, _bash(project, "cat f.txt", cwd=worktree)
    )
    assert code == 0, err
    assert _seen(project) > before

    # The main session's call (no agent_id) never refreshes:
    _age_seen(project)
    before = _seen(project)
    code, out, err = _fire(project, _bash(project, f"vsdd gate --run {run_id}", agent=None))
    assert code == 0, err
    assert _seen(project) == before

    # A stranger's call does not:
    _age_seen(project)
    before = _seen(project)
    code, out, err = _fire(
        project, _bash(project, "echo hi", session="sess_other", agent="wf-stranger")
    )
    assert code == 0, err
    assert _seen(project) == before

    # A call a rule blocked still refreshes — liveness is not a rule's to lose:
    _age_seen(project)
    before = _seen(project)
    code, out, err = _fire(
        project,
        _bash(project, f"vsdd gate --run {run_id}"),
        entries=(helpers_rule_blocking(),),
    )
    assert code == 10
    assert _seen(project) > before


def helpers_rule_blocking():
    return run_rule("probe", BLOCKING)


def test_lock_refresh_belongs_by_run_and_story(locked_run):
    """Belonging is decided by records from the facts the runner passes, not
    by the holder's session id (P3SEAM-24): another session's call refreshes
    when it names the run — ``--run``, ``--run=<id>``, a call by path, a
    ``step.run`` — or works in the run's story worktree, including one
    reached by a ``cd`` segment; a call naming another run id does not."""
    project, run_id = locked_run

    def fire_age(command):
        _age_seen(project)
        before = _seen(project)
        code, out, err = _fire(project, _bash(project, command, session=OTHER_SESSION, agent=OTHER_AGENT))
        assert code == 0, err
        return before

    # Another session's vsdd call carrying --run moves seen:
    before = fire_age(f"vsdd gate --run {run_id}")
    assert _seen(project) > before

    # The --run=<id> spelling too:
    before = fire_age(f"vsdd gate --run={run_id}")
    assert _seen(project) > before

    # A vsdd call made by path: the program's basename decides, as the
    # caller-mark duty reads it (S3R-hook-runner-b-9):
    before = fire_age(f"./vsdd gate --run {run_id}")
    assert _seen(project) > before

    # Another session's step result whose step.run is the run moves seen:
    _age_seen(project)
    before = _seen(project)
    code, out, err = _fire(project, _step(project, run_id, session=OTHER_SESSION, agent=OTHER_AGENT))
    assert code == 0, err
    assert _seen(project) > before

    # A story worktree reached by a cd segment — the segment's effective
    # directory decides, not the payload cwd (S3R-hook-runner-b-10):
    worktree = project / ".worktrees" / "S-3.01"
    worktree.mkdir(parents=True)
    git_dir = project / ".git" / "worktrees" / "S-3.01"
    git_dir.mkdir(parents=True)
    (worktree / ".git").write_text(f"gitdir: {git_dir}\n", encoding="utf-8")
    (git_dir / "HEAD").write_text("ref: story/S-3.01\n", encoding="utf-8")
    before = fire_age("cd .worktrees/S-3.01 && cat f.txt")
    assert _seen(project) > before

    # A call naming another run id does not:
    before = fire_age("vsdd gate --run r-stranger")
    assert _seen(project) == before


def run_rule(identifier: str, module: str) -> object:
    from vsdd.hooks.model import RuleEntry

    return RuleEntry(
        id=identifier,
        on=(("PreToolUse", ("Bash",)),),
        kind="gate",
        fail="closed",
        module=module,
    )


def test_refresh_is_cheap_without_a_lock(tmp_path):
    project = repo_make_project(tmp_path)
    payload = _bash(project, "echo hi")
    code, out, err = _fire(project, payload)
    assert code == 0
    assert out == "" and err == ""
    assert not (project / ".factory" / "local" / "lock.json").exists()

    # The same output on a second call, still no lock file:
    code2, out2, err2 = _fire(project, payload)
    assert (code2, out2, err2) == (code, out, err)
    assert not (project / ".factory" / "local" / "lock.json").exists()


def test_session_end_releases(tmp_path):
    project = repo_make_project(tmp_path)
    run_id = _begin(project, "sess_end")["run"]["id"]
    keep_id = _begin(project, "sess_keep", scope="other:develop")["run"]["id"]

    launch, _plugin = helpers.launch_copy(tmp_path / "copy")
    result = helpers.launch(
        "sh", launch, ["hook", "notify"], cwd=project,
        stdin_text=json.dumps(_end(project, session="sess_end", reason="exit")),
        env={"VSDD_FACTORY_ROOT": str(project)},
    )
    assert result.returncode == 0, result.stderr
    assert "Traceback" not in result.stderr

    sessions = sorted(entry["session"] for entry in run_locks.lock_entries(project))
    assert "sess_end" not in sessions
    assert "sess_keep" in sessions
    assert runs_store.run_read(project, run_id)["ended"] != ""
    assert runs_store.run_read(project, keep_id)["ended"] == ""

    released = [
        event
        for event in _launched_events(project)
        if event.get("type") == "lock.released"
    ]
    assert any(
        event.get("run") == run_id and event.get("reason") == "exit" for event in released
    )


def test_liveness_failure_is_loud(locked_run, monkeypatch):
    project, run_id = locked_run
    import vsdd.state.locks as real_locks

    def boom(*args, **kwargs):
        raise RuntimeError("probe-refresh")

    monkeypatch.setattr(real_locks, "lock_seen", boom)
    code, out, err = _fire(project, _bash(project, "echo hi"))
    assert code == 0  # never blocks
    assert out == "" and err == ""  # never changes the output
    faults = [e for e in read_events(project) if e.get("type") == "runner.liveness_error"]
    assert any("RuntimeError: probe-refresh" in str(e.get("error")) for e in faults)
    assert all(e.get("severity") == "error" for e in faults)

    # The real module still exposes both functions: a rename fails here.
    assert callable(real_locks.lock_seen)
    assert callable(real_locks.lock_release)


def test_release_failure_is_loud(locked_run, monkeypatch):
    project, run_id = locked_run
    import vsdd.state.locks as real_locks

    def boom(*args, **kwargs):
        raise RuntimeError("probe-release")

    monkeypatch.setattr(real_locks, "lock_release", boom)
    code, out, err = _fire(project, _end(project, reason="exit"))
    assert code == 0
    assert out == "" and err == ""
    faults = [e for e in read_events(project) if e.get("type") == "runner.liveness_error"]
    assert any("RuntimeError: probe-release" in str(e.get("error")) for e in faults)


def test_release_logs_a_busy_store(locked_run, monkeypatch):
    """The store signalling a busy flush is logged with the entries it did
    release, and nothing is released by other means. The exception is a
    plain ``StoreBusy`` carrying ``released``, the attribute
    ``locks.lock_release`` sets (S3R-records-C2-2)."""
    project, run_id = locked_run
    import vsdd.state.locks as real_locks
    from vsdd.state.store import StoreBusy

    def busy(*args, **kwargs):
        exc = StoreBusy(
            "busy",
            "a flush is running; the release kept 1 lock entry whose run "
            "could not be ended in time — retry when the flush finishes",
        )
        exc.released = [{"run": run_id, "session": SESSION}]
        raise exc

    monkeypatch.setattr(real_locks, "lock_release", busy)
    code, out, err = _fire(project, _end(project, reason="clear"))
    assert code == 0
    assert out == "" and err == ""
    events = read_events(project)
    assert any(
        e.get("type") == "lock.released" and e.get("run") == run_id and e.get("reason") == "clear"
        for e in events
    )
    assert any(e.get("type") == "runner.liveness_error" for e in events)


def test_flush_lock_held_never_blocks_session_end(tmp_path):
    """With the flush lock held, the session end still returns 0 promptly and
    says so (a StoreBusy is caught; the lock then goes stale by time)."""
    project = repo_make_project(tmp_path)
    begun = _begin(project, "sess_end")

    launch, _plugin = helpers.launch_copy(tmp_path / "copy")
    with run_store.locked(project, "flush.lock", wait=5.0):
        started = time.monotonic()
        result = helpers.launch(
            "sh", launch, ["hook", "notify"], cwd=project,
            stdin_text=json.dumps(_end(project, session="sess_end", reason="exit")),
            env={"VSDD_FACTORY_ROOT": str(project)},
        )
        elapsed = time.monotonic() - started
    assert result.returncode == 0, result.stderr
    assert "Traceback" not in result.stderr
    # Promptly: the store's wait is bounded by RELEASE_WAIT, on top of the
    # launcher's start-up — nothing near the stale window.
    assert elapsed < 1.5, elapsed
    # It says so, and the store's busy release kept the entry: the entry
    # stands and its run is not ended.
    sessions = [entry["session"] for entry in run_locks.lock_entries(project)]
    assert sessions == ["sess_end"]
    assert runs_store.run_read(project, begun["run"]["id"])["ended"] == ""
    faults = [
        event
        for event in _launched_events(project)
        if event.get("type") == "runner.liveness_error"
    ]
    assert any("StoreBusy" in str(event.get("error")) for event in faults)


class _LocalCtx:
    """The least a duty's call needs of the runner's context."""

    def __init__(self, project, session):
        self.project = project
        self.session_id = session
        self.raw = {"reason": "exit"}
        self.mode = "test"
        self.logged: list[tuple[str, str, dict]] = []

    def log(self, event_type, severity, **fields):
        self.logged.append((event_type, severity, fields))


def test_session_end_logs_released_entries_when_a_run_end_is_busy(
    tmp_path, monkeypatch
):
    """Through both units (finding step3-records-fix-3-C1-2): a session holds
    two lock entries, the second ``run_end`` is busy, and the ``StoreBusy``
    the store raises carries its partial success as ``released``. The
    session-end release logs ``lock.released`` for the entry that really was
    released — not nothing — and the kept entry stays in the lock."""
    from vsdd.hooks import liveness
    from vsdd.state.store import StoreBusy

    project = repo_make_project(tmp_path)
    run_a = _begin(project, SESSION, scope="repo-a:develop")["run"]["id"]
    run_b = _begin(project, SESSION, scope="repo-b:develop")["run"]["id"]

    real_run_end = runs_store.run_end
    calls = {"n": 0}

    def run_end_once_then_busy(project_, run_id, *args, **kwargs):
        # The first call succeeds; the second meets a flush holding the lock
        # and ends nothing.
        calls["n"] += 1
        if calls["n"] > 1:
            raise StoreBusy("busy", "flush.lock is held by another process")
        return real_run_end(project_, run_id, *args, **kwargs)

    monkeypatch.setattr(runs_store, "run_end", run_end_once_then_busy)

    ctx = _LocalCtx(project, SESSION)
    released = liveness.release(ctx)

    assert [entry["run"] for entry in released] == [run_a], released
    released_events = [fields for event, _sev, fields in ctx.logged if event == "lock.released"]
    assert [fields.get("run") for fields in released_events] == [run_a], ctx.logged
    assert any(event == "runner.liveness_error" for event, _sev, _f in ctx.logged), ctx.logged

    # The kept entry survives, its run still live; the released run is ended.
    entries = run_locks.lock_entries(project)
    assert [entry["run"] for entry in entries] == [run_b], entries
    assert runs_store.run_read(project, run_a)["ended"] != ""
    assert runs_store.run_read(project, run_b)["ended"] == ""
