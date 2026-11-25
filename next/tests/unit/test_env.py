"""The handlers of ``vsdd env start`` and ``vsdd env stop`` at handler level (G1).

The command-line wrapping is ``jobs-b.md``'s; these tests call ``env.start``
and ``env.stop`` the way the command module will.
"""

from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path

import pytest

from vsdd.events import EventLog, events_dir_for
from vsdd.gates import env
from vsdd.gates.env import SETTLE_SECONDS, start, stop
from vsdd.gates.jobs import JobSpec, attach, job_key, root_of, stop_products
from vsdd.project import Project

RUN = "echo {port} > started.txt; echo $VSDD_PORT >> started.txt; sleep 30"

#: A process that takes the flock on the file it is given and holds it.
HOLD_LOCK = (
    "import fcntl, os, sys, time;"
    "fd = os.open(sys.argv[1], os.O_RDWR | os.O_CREAT);"
    "fcntl.flock(fd, fcntl.LOCK_EX); time.sleep(30)"
)

HELPER = Path(__file__).resolve().parent / "_jobs_helper.py"


def make_project(tmp_path: Path, run: str = RUN) -> Path:
    root = tmp_path / "proj"
    (root / ".factory").mkdir(parents=True)
    config = "schema = 1\n"
    if run is not None:
        config += f'[commands]\nrun = """{run}"""\n'
    (root / ".factory" / "factory.toml").write_text(config, encoding="utf-8")
    (root / ".worktrees" / "_integration").mkdir(parents=True)
    return root


def record_for(root: Path, subject: str) -> dict:
    key = job_key("env", subject, "")
    path = root_of(root) / ".factory" / "local" / "jobs" / f"{key}.json"
    return json.loads(path.read_text(encoding="utf-8"))


def wait_for_lock(path: Path, timeout: float = 5.0) -> None:
    """Wait until a spawned holder has really taken the flock at ``path``."""
    from vsdd.gates.jobs import _lock_held

    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if _lock_held(path):
            return
        time.sleep(0.05)
    raise AssertionError(f"nobody ever took the lock at {path}")


def caller(session: str, agent: str) -> dict:
    return {"session_id": session, "agent_id": agent, "agent_type": "clerk", "cwd": "/"} \
        if session else None


@pytest.fixture
def root(tmp_path: Path, monkeypatch):
    project = make_project(tmp_path)
    monkeypatch.setattr(env, "SETTLE_SECONDS", 0.3)
    yield project
    stop_products(project, grace=1.5)


# --- start ------------------------------------------------------------------


def test_start_records_caller_and_port(root: Path):
    output, code = start(root, "demo-1", caller("sess-1", "agent-1"))
    assert code == 0
    assert output["ok"] is True
    assert set(output) == {"ok", "url", "port", "pid"}
    assert output["url"] == f"http://127.0.0.1:{output['port']}"
    assert 28100 <= output["port"] <= 28199
    record = record_for(root, "demo-1")
    assert record["state"] == "running"
    assert record["agent_id"] == "agent-1"
    assert record["session_id"] == "sess-1"
    assert record["pid"] == output["pid"]
    # Both spellings reached the command: {port} in the string, $VSDD_PORT in the env.
    started = (root / ".worktrees" / "_integration" / "started.txt").read_text().splitlines()
    assert started == [str(output["port"]), str(output["port"])]


def test_start_is_idempotent_at_same_commit(root: Path):
    first, code1 = start(root, "demo-1", caller("sess-1", "agent-1"))
    assert code1 == 0
    second, code2 = start(root, "demo-1", caller("sess-2", "agent-2"))
    assert code2 == 0
    assert second == first
    assert record_for(root, "demo-1")["pid"] == first["pid"]


def test_start_replaces_product_of_older_commit(root: Path):
    first, _ = start(root, "demo-1", caller("sess-1", "agent-1"))
    record = record_for(root, "demo-1")
    record["commit"] = "0" * 40  # an older commit than the worktree's head
    (root_of(root) / ".factory" / "local" / "jobs" / f"{record['key']}.json").write_text(
        json.dumps(record), encoding="utf-8"
    )
    second, code = start(root, "demo-1", caller("sess-2", "agent-2"))
    assert code == 0
    assert second["pid"] != first["pid"]
    assert record_for(root, "demo-1")["commit"] == ""
    assert_gone(first["pid"])  # the old product dies


def test_held_answer_decides_on_a_read_after_the_probe(root: Path):
    from vsdd.gates.env import _held_answer

    started, code = start(root, "demo-1", caller("sess-1", "agent-1"))
    assert code == 0
    store = root_of(root) / ".factory" / "local" / "jobs"
    key = job_key("env", "demo-1", "")
    fresh = json.loads((store / f"{key}.json").read_text(encoding="utf-8"))
    assert fresh["state"] == "running"
    # A 'starting' snapshot taken before the start call wrote 'running' and
    # released the settle lock: the product it names is healthy, so the
    # answer comes from the fresh read and the product survives (D14,
    # S3R-jobs-C2-1).
    snapshot = dict(fresh, state="starting")
    answered = _held_answer(root, key, snapshot, fresh["commit"], store / f"{key}.log")
    assert answered is not None, "the healthy product was treated as a stale start"
    answer, answer_code = answered
    assert answer_code == 0
    assert answer == {"ok": True, "url": fresh["url"], "port": fresh["port"], "pid": fresh["pid"]}
    time.sleep(0.3)
    try:
        os.kill(fresh["pid"], 0)
    except OSError:
        raise AssertionError("the running product was killed off a stale snapshot")


def _product_record(root: Path, key: str, **fields) -> dict:
    record = {
        "v": 1,
        "key": key,
        "kind": "product",
        "command": RUN,
        "subject": "demo-1",
        "commit": "",
        "state": "running",
        "created": "2026-01-01T00:00:00Z",
        "started": None,
        "finished": None,
        "pid": 999999,  # the house dead pid
        "nonce": "b" * 16,
        "session_id": "sess-A",
        "agent_id": "agent-A",
        "port": 28100,
        "url": "http://127.0.0.1:28100",
        "cwd": str(root / ".worktrees" / "_integration"),
    }
    record.update(fields)
    return record


def test_held_answer_answers_the_live_record_over_a_dead_pid_snapshot(root: Path):
    from vsdd.gates.env import _held_answer

    started, code = start(root, "demo-1", caller("sess-1", "agent-1"))
    assert code == 0
    store = root_of(root) / ".factory" / "local" / "jobs"
    key = job_key("env", "demo-1", "")
    # A stale running snapshot at the head commit: answering from it would
    # return the dead pid 999999 instead of the live product (D14,
    # S3R-jobs-fix-3-C1-2).
    snapshot = _product_record(root, key)
    answered = _held_answer(root, key, snapshot, "", store / f"{key}.log")
    assert answered is not None
    answer, answer_code = answered
    assert answer_code == 0
    assert answer["pid"] == started["pid"], "the answer came from the stale snapshot"
    time.sleep(0.3)
    try:
        os.kill(started["pid"], 0)
    except OSError:
        raise AssertionError("the running product was killed off a stale snapshot")


def test_held_answer_does_not_stop_on_a_stale_other_commit_snapshot(root: Path):
    from vsdd.gates.env import _held_answer

    started, code = start(root, "demo-1", caller("sess-1", "agent-1"))
    assert code == 0
    store = root_of(root) / ".factory" / "local" / "jobs"
    key = job_key("env", "demo-1", "")
    # A stale running snapshot at another commit: acting on it would stop the
    # live product the holder is serving and answer nothing (D14).
    snapshot = _product_record(root, key, commit="0" * 40)
    answered = _held_answer(root, key, snapshot, "", store / f"{key}.log")
    assert answered is not None, "the healthy product was replaced off a stale snapshot"
    answer, answer_code = answered
    assert answer_code == 0
    assert answer["pid"] == started["pid"]
    time.sleep(0.3)
    try:
        os.kill(started["pid"], 0)
    except OSError:
        raise AssertionError("the running product was killed off a stale snapshot")


def _overlapping_starts_over_a_stale_record(root: Path, monkeypatch, commit: str) -> None:
    store = root_of(root) / ".factory" / "local" / "jobs"
    store.mkdir(parents=True, exist_ok=True)
    key = job_key("env", "demo-1", "")
    # A stale running record whose product is long gone; the commit decides
    # which wrong branch the pre-fix code took: answering the dead pid at the
    # head commit, stopping the first start's new product at another commit.
    (store / f"{key}.json").write_text(
        json.dumps(_product_record(root, key, commit=commit)), encoding="utf-8"
    )
    lock_path = store / f"{key}.lock"
    holder = subprocess.Popen(
        [sys.executable, "-c", HOLD_LOCK, str(lock_path)], start_new_session=True
    )
    entered = threading.Event()
    original_held_answer = env._held_answer

    def announced(*args, **kwargs):
        entered.set()  # the second start is past its failed lock probe
        return original_held_answer(*args, **kwargs)

    monkeypatch.setattr(env, "_held_answer", announced)
    outcomes: list = []

    def second_start() -> None:
        outcomes.append(start(root, "demo-1", caller("sess-2", "agent-2")))

    try:
        wait_for_lock(lock_path)
        thread = threading.Thread(target=second_start)
        thread.start()
        assert entered.wait(5.0), "the second start never reached the held state"
        holder.terminate()  # the lock comes free; the first start begins
        holder.wait(timeout=10)
        first, first_code = start(root, "demo-1", caller("sess-1", "agent-1"))
        assert first_code == 0, first
        thread.join(timeout=15)
        assert outcomes, "the second start never returned"
        second, second_code = outcomes[0]
        assert second_code == 0, second
        # Both overlapping starts return the same live pid (D14).
        assert second["pid"] == first["pid"], "the second start answered the stale record"
        time.sleep(0.3)
        os.kill(first["pid"], 0)
    finally:
        if holder.poll() is None:
            holder.kill()


def test_overlapping_starts_agree_over_a_stale_record_at_the_head_commit(
    root: Path, monkeypatch
):
    _overlapping_starts_over_a_stale_record(root, monkeypatch, commit="")


def test_overlapping_starts_agree_over_a_stale_record_at_another_commit(
    root: Path, monkeypatch
):
    _overlapping_starts_over_a_stale_record(root, monkeypatch, commit="0" * 40)


# --- the held-state cases a start must decide (S3R-jobs-fix-3-C2-1) -----------
#
# What a start must decide about the key it wants, at the head commit and at
# another:
#
#   1. a start call in flight (its settle lock held) is waited for: its
#      running record is answered at the same commit and its product replaced
#      at another one, and a settling call that dies mid-settle while a second
#      start waits is taken over from instead of waited out (D14);
#   2. a running product is answered at the same commit and replaced at
#      another one (D14);
#   3. a product whose leader died while its group lives is case 2: the lock
#      is the group's, inherited by every member, so liveness is judged with
#      killpg as stop_products signals, never with the pid alone;
#   4. a stale record with nothing alive behind a free lock is cleared;
#   5. a holder that exits without writing - or one that has taken the lock
#      but not yet written, record or none - is waited on like a start in
#      flight: answered at the same commit, replaced at another, a freed lock
#      taken, and only a failure after _STARTING_WAIT distinguishes it.

#: A product whose group outlives its leader: the shell forks ``sleep`` and
#: waits; killing the shell leaves ``sleep`` holding the inherited lock.
DEAD_LEADER_RUN = "sleep 30 & wait"

#: A holder that takes the flock, holds it for a moment and exits, writing
#: nothing to the store.
EXITING_HOLDER = (
    "import fcntl, os, sys, time;"
    "fd = os.open(sys.argv[1], os.O_RDWR | os.O_CREAT);"
    "fcntl.flock(fd, fcntl.LOCK_EX); time.sleep(float(sys.argv[2])); os.close(fd)"
)


def test_start_joins_a_start_call_still_in_flight(root: Path):
    from vsdd.gates.jobs import _lock_held

    store = root_of(root) / ".factory" / "local" / "jobs"
    key = job_key("env", "demo-1", "")
    settle_lock = store / f"{key}.settle.lock"
    outcomes: list = []

    def first_start() -> None:
        outcomes.append(start(root, "demo-1", caller("sess-1", "agent-1")))

    thread = threading.Thread(target=first_start)
    thread.start()
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        try:
            record = record_for(root, "demo-1")
        except FileNotFoundError:
            record = {}
        if record.get("state") == "starting" and _lock_held(settle_lock):
            break
        time.sleep(0.05)
    else:
        raise AssertionError("the first start never reached the settle window")
    # The second start joins the first: it waits for the running record and
    # answers it instead of failing or starting a second product (D14).
    second, code = start(root, "demo-1", caller("sess-2", "agent-2"))
    assert code == 0, second
    thread.join(timeout=15)
    assert outcomes, "the first start never returned"
    first, first_code = outcomes[0]
    assert first_code == 0, first
    assert second["pid"] == first["pid"]
    time.sleep(0.3)
    os.kill(first["pid"], 0)


def test_start_answers_when_the_leaders_group_still_holds_the_lock(
    tmp_path: Path, monkeypatch
):
    project = make_project(tmp_path, run=DEAD_LEADER_RUN)
    monkeypatch.setattr(env, "SETTLE_SECONDS", 0.3)
    try:
        first, code = start(project, "demo-1", caller("sess-1", "agent-1"))
        assert code == 0, first
        leader = first["pid"]
        os.kill(leader, signal.SIGKILL)
        assert_gone(leader)  # reaped; the sleep it forked lives on in the group
        store = root_of(project) / ".factory" / "local" / "jobs"
        from vsdd.gates.jobs import _lock_held

        assert _lock_held(store / f"{job_key('env', 'demo-1', '')}.lock"), (
            "the surviving group no longer holds the product lock"
        )
        # The dead leader is not a start in flight: the record is answered at
        # once (pre-fix: a 5 s wait and exit 1, S3R-jobs-fix-3-C2-1).
        second, second_code = start(project, "demo-1", caller("sess-2", "agent-2"))
        assert second_code == 0, second
        assert second["pid"] == leader
        os.killpg(leader, 0)  # the group the answer names is still alive
    finally:
        stop_products(project, grace=1.5)


def test_start_replaces_a_leaders_surviving_group_at_another_commit(
    tmp_path: Path, monkeypatch
):
    project = make_project(tmp_path, run=DEAD_LEADER_RUN)
    monkeypatch.setattr(env, "SETTLE_SECONDS", 0.3)
    try:
        first, code = start(project, "demo-1", caller("sess-1", "agent-1"))
        assert code == 0, first
        leader = first["pid"]
        os.kill(leader, signal.SIGKILL)
        assert_gone(leader)
        record = record_for(project, "demo-1")
        record["commit"] = "0" * 40  # another commit than the worktree's head
        (root_of(project) / ".factory" / "local" / "jobs" / f"{record['key']}.json").write_text(
            json.dumps(record), encoding="utf-8"
        )
        # The group, not the dead leader, decides: it is stopped and replaced
        # (pre-fix: a 5 s wait and exit 1, S3R-jobs-fix-3-C2-1).
        second, second_code = start(project, "demo-1", caller("sess-2", "agent-2"))
        assert second_code == 0, second
        assert second["pid"] != leader
        for _ in range(50):
            try:
                os.killpg(leader, 0)
            except OSError:
                break
            time.sleep(0.1)
        else:
            raise AssertionError("the surviving group was never stopped")
        assert record_for(project, "demo-1")["pid"] == second["pid"]
    finally:
        stop_products(project, grace=1.5)


def test_start_clears_a_stale_record_with_nothing_alive(root: Path):
    store = root_of(root) / ".factory" / "local" / "jobs"
    store.mkdir(parents=True, exist_ok=True)
    key = job_key("env", "demo-1", "")
    for commit in ("", "0" * 40):  # the head commit, then another one
        (store / f"{key}.json").write_text(
            json.dumps(_product_record(root, key, commit=commit)), encoding="utf-8"
        )
        output, code = start(root, "demo-1", caller("sess-1", "agent-1"))
        assert code == 0, output
        assert output["pid"] != 999999
        record = record_for(root, "demo-1")
        assert record["state"] == "running"
        assert record["pid"] == output["pid"]
        stop_products(root, subject="demo-1")  # a clean slate for the next leg
        assert not (store / f"{key}.json").exists()


def test_start_takes_the_lock_when_a_holder_exits_without_writing(root: Path):
    store = root_of(root) / ".factory" / "local" / "jobs"
    store.mkdir(parents=True, exist_ok=True)
    key = job_key("env", "demo-1", "")
    lock_path = store / f"{key}.lock"
    for commit in ("", "0" * 40):  # the head commit, then another one
        (store / f"{key}.json").write_text(
            json.dumps(_product_record(root, key, commit=commit)), encoding="utf-8"
        )
        holder = subprocess.Popen(
            [sys.executable, "-c", EXITING_HOLDER, str(lock_path), "0.8"],
            start_new_session=True,
        )
        try:
            wait_for_lock(lock_path)
            began = time.monotonic()
            output, code = start(root, "demo-1", caller("sess-1", "agent-1"))
            elapsed = time.monotonic() - began
            assert code == 0, output
            # The freed lock is taken, not waited out (pre-fix: the full
            # _STARTING_WAIT and exit 1, S3R-jobs-fix-3-C2-1).
            assert elapsed < 4.0, "the start waited out _STARTING_WAIT on a free lock"
            assert output["pid"] != 999999
            assert record_for(root, "demo-1")["state"] == "running"
        finally:
            if holder.poll() is None:
                holder.kill()
        stop_products(root, subject="demo-1")


def _split_heads(monkeypatch, first: str, second: str):
    """Give the two start threads of one subject different head commits.

    Both calls run in one worktree, so only the patch separates them: a thread
    that calls the returned marker before ``start`` reads ``first``, every
    other thread reads ``second``.
    """
    marked: list[int] = []

    def fake_head(worktree: Path) -> str:
        return first if marked and threading.get_ident() == marked[0] else second

    monkeypatch.setattr(env, "_head", fake_head)
    return lambda: marked.append(threading.get_ident())


def _poll_gone(pid: int, timeout: float = 5.0) -> None:
    """A process this test is not the parent of (its owner died with it) is
    gone when the signal probe says so; its new parent reaps it."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            os.kill(pid, 0)
        except OSError:
            return
        time.sleep(0.1)
    raise AssertionError(f"process {pid} is still alive")


def test_start_replaces_a_start_in_flight_at_another_commit(root: Path, monkeypatch):
    from vsdd.gates.jobs import _lock_held

    mark_first = _split_heads(monkeypatch, "a" * 40, "b" * 40)
    monkeypatch.setattr(env, "SETTLE_SECONDS", 1.2)
    store = root_of(root) / ".factory" / "local" / "jobs"
    settle_lock = store / f"{job_key('env', 'demo-1', '')}.settle.lock"
    outcomes: list = []

    def first_start() -> None:
        mark_first()
        outcomes.append(start(root, "demo-1", caller("sess-1", "agent-1")))

    thread = threading.Thread(target=first_start)
    thread.start()
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        try:
            record = record_for(root, "demo-1")
        except FileNotFoundError:
            record = {}
        if record.get("state") == "starting" and _lock_held(settle_lock):
            break
        time.sleep(0.05)
    else:
        raise AssertionError("the first start never reached the settle window")
    # The second start joins at head b*: the settling record says a*, so
    # joining means replacing - a new pid, the old product gone, the record at
    # the new commit (D14; pre-fix: the first call's pid answered with the
    # record still at a*, S3R-jobs-fix-4-C1-1).
    began = time.monotonic()
    second, code = start(root, "demo-1", caller("sess-2", "agent-2"))
    elapsed = time.monotonic() - began
    assert code == 0, second
    assert elapsed < 4.0, "the join waited out _STARTING_WAIT"
    thread.join(timeout=15)
    assert outcomes, "the first start never returned"
    first, first_code = outcomes[0]
    assert first_code == 0, first
    assert second["pid"] != first["pid"]
    record = record_for(root, "demo-1")
    assert record["commit"] == "b" * 40
    assert record["pid"] == second["pid"]
    assert_gone(first["pid"])  # the replaced product is dead


def test_start_takes_over_when_the_settling_start_call_dies(root: Path, monkeypatch):
    from vsdd.gates.jobs import _lock_held

    store = root_of(root) / ".factory" / "local" / "jobs"
    key = job_key("env", "demo-1", "")
    settle_lock = store / f"{key}.settle.lock"
    # The helper's start call settles for the real SETTLE_SECONDS (2 s): a
    # window wide enough to kill it in.
    helper = subprocess.Popen(
        [sys.executable, "-B", str(HELPER), "env-start", str(root), "demo-1"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    entered = threading.Event()
    original_held_answer = env._held_answer

    def announced(*args, **kwargs):
        entered.set()  # the second start has joined the settle wait
        return original_held_answer(*args, **kwargs)

    monkeypatch.setattr(env, "_held_answer", announced)
    outcomes: list = []

    def second_start() -> None:
        outcomes.append(start(root, "demo-1", caller("sess-2", "agent-2")))

    try:
        deadline = time.monotonic() + 10
        record: dict = {}
        while time.monotonic() < deadline:
            try:
                record = record_for(root, "demo-1")
            except FileNotFoundError:
                record = {}
            if (
                record.get("state") == "starting"
                and isinstance(record.get("pid"), int)
                and _lock_held(settle_lock)
            ):
                break
            time.sleep(0.05)
        else:
            raise AssertionError("the settling start never wrote its record")
        thread = threading.Thread(target=second_start)
        thread.start()
        assert entered.wait(5.0), "the second start never joined the settle wait"
        began = time.monotonic()
        helper.kill()  # the settling call dies; its product keeps the lock
        helper.wait(timeout=10)
        killed_pid = record["pid"]
        thread.join(timeout=15)
        assert outcomes, "the second start never returned"
        second, code = outcomes[0]
        elapsed = time.monotonic() - began
        # Taken over from, not waited out (pre-fix: exit 1, 'another start of
        # this subject is still settling', after _STARTING_WAIT with the
        # orphaned product alive, S3R-jobs-fix-4-C1-1).
        assert code == 0, second
        assert elapsed < 4.0, "the dead settling call was waited out"
        assert second["pid"] != killed_pid
        _poll_gone(killed_pid)  # the orphan was stopped by its recorded pid
        assert record_for(root, "demo-1")["state"] == "running"
    finally:
        if helper.poll() is None:
            helper.kill()


def _overlapping_start_yet_to_write(root: Path, monkeypatch, commit: str) -> None:
    go = threading.Event()
    original_launch = env._launch

    def delayed_launch(*args, **kwargs):
        go.wait(5.0)  # the lock is taken; no record is in the store yet
        return original_launch(*args, **kwargs)

    monkeypatch.setattr(env, "_launch", delayed_launch)
    mark_first = _split_heads(monkeypatch, "a" * 40, commit) if commit else None
    store = root_of(root) / ".factory" / "local" / "jobs"
    lock_path = store / f"{job_key('env', 'demo-1', '')}.lock"
    entered = threading.Event()
    original_held_answer = env._held_answer

    def announced(*args, **kwargs):
        # The second start's snapshot is taken (no record: the first call is
        # still held in the delayed launch), so the first may now write.
        entered.set()
        go.set()
        return original_held_answer(*args, **kwargs)

    monkeypatch.setattr(env, "_held_answer", announced)
    outcomes: list = []

    def first_start() -> None:
        if mark_first is not None:
            mark_first()
        outcomes.append(start(root, "demo-1", caller("sess-1", "agent-1")))

    thread = threading.Thread(target=first_start)
    thread.start()
    wait_for_lock(lock_path)
    began = time.monotonic()
    second, code = start(root, "demo-1", caller("sess-2", "agent-2"))
    elapsed = time.monotonic() - began
    assert entered.is_set(), "the second start never reached the held state"
    thread.join(timeout=15)
    assert outcomes, "the first start never returned"
    first, first_code = outcomes[0]
    assert code == 0, second
    assert first_code == 0, first
    assert elapsed < 4.0, "the start waited out _STARTING_WAIT on a delayed holder"
    if commit:
        # At another commit joining means replacing: a new pid, the old
        # product gone, the record at the new commit (D14).
        assert second["pid"] != first["pid"]
        record = record_for(root, "demo-1")
        assert record["commit"] == commit
        assert record["pid"] == second["pid"]
        assert_gone(first["pid"])
    else:
        # The first call's product is answered, not a second one started.
        assert second["pid"] == first["pid"]
        time.sleep(0.3)
        os.kill(first["pid"], 0)


def test_start_waits_for_a_start_yet_to_write_its_record(root: Path, monkeypatch):
    _overlapping_start_yet_to_write(root, monkeypatch, "")


def test_start_replaces_a_start_yet_to_write_its_record_at_another_commit(
    root: Path, monkeypatch
):
    _overlapping_start_yet_to_write(root, monkeypatch, "0" * 40)


def test_start_waits_out_a_recordless_holder(root: Path):
    store = root_of(root) / ".factory" / "local" / "jobs"
    store.mkdir(parents=True, exist_ok=True)
    key = job_key("env", "demo-1", "")
    lock_path = store / f"{key}.lock"
    for _leg in ("", "0" * 40):  # no record is involved; the legs alike
        holder = subprocess.Popen(
            [sys.executable, "-c", EXITING_HOLDER, str(lock_path), "0.5"],
            start_new_session=True,
        )
        try:
            wait_for_lock(lock_path)
            began = time.monotonic()
            output, code = start(root, "demo-1", caller("sess-1", "agent-1"))
            elapsed = time.monotonic() - began
            # The recordless holder is waited on, not failed at once (pre-fix:
            # exit 1, 'the product lock stays held', in 0.01 s,
            # S3R-jobs-fix-4-C1-2).
            assert code == 0, output
            assert elapsed < 4.0, "the recordless holder was not waited on"
            assert record_for(root, "demo-1")["state"] == "running"
        finally:
            if holder.poll() is None:
                holder.kill()
        stop_products(root, subject="demo-1")  # a clean slate for the next leg


def test_two_subjects_get_different_ports(root: Path):
    one, code1 = start(root, "demo-1", caller("sess-1", "agent-1"))
    two, code2 = start(root, "demo-2", caller("sess-1", "agent-1"))
    assert code1 == 0 and code2 == 0
    assert one["port"] != two["port"]


def test_start_fails_loudly_when_command_exits(tmp_path: Path):
    root = make_project(tmp_path, run="echo boom >&2; exit 3")
    output, code = start(root, "demo-1", caller("sess-1", "agent-1"))
    assert code == 1
    assert output["ok"] is False
    assert "3" in output["error"]
    assert "boom" in output["error"]
    key = job_key("env", "demo-1", "")
    assert not (root / ".factory" / "local" / "jobs" / f"{key}.json").exists()


def test_start_refuses_an_oversize_product_record(tmp_path: Path):
    from vsdd.gates.jobs import RECORD_CAP, RECORD_HEADROOM

    root = make_project(tmp_path, run=f"true {'#' * (RECORD_CAP + 500)}; sleep 30")
    output, code = start(root, "demo-1", caller("sess-1", "agent-1"))
    assert code == 1
    assert output["ok"] is False
    # The refusal names the bar that was applied and keeps the cap in the
    # text (S3R-jobs-C2-2).
    assert str(RECORD_CAP) in output["error"]
    assert f"{RECORD_CAP - RECORD_HEADROOM}-byte limit" in output["error"]
    key = job_key("env", "demo-1", "")
    assert not (root / ".factory" / "local" / "jobs" / f"{key}.json").exists(), (
        "a refused start leaves no record behind"
    )


def test_start_recovers_from_a_start_call_killed_mid_settle(root: Path):
    helper = subprocess.Popen(
        [sys.executable, "-B", str(HELPER), "env-start", str(root), "demo-1"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        deadline = time.monotonic() + 10
        record: dict = {}
        while time.monotonic() < deadline:
            try:
                record = record_for(root, "demo-1")
            except FileNotFoundError:
                record = {}
            if record.get("state") == "starting" and isinstance(record.get("pid"), int):
                break
            time.sleep(0.05)
        else:
            raise AssertionError("the start call never wrote a starting record naming its pid")
        helper.kill()  # killed inside the settle window; the product keeps the lock
        helper.wait(timeout=10)
        killed_pid = record["pid"]
        # The next start answers, having stopped the orphaned product by the
        # pid the killed call wrote right after forking it (S3R-jobs-C1-2).
        output, code = start(root, "demo-1", caller("sess-2", "agent-2"))
        assert code == 0, output
        assert output["pid"] != killed_pid
        assert_gone(killed_pid)
        assert record_for(root, "demo-1")["state"] == "running"
    finally:
        if helper.poll() is None:
            helper.kill()


def test_start_without_run_command_is_negative(tmp_path: Path):
    root = make_project(tmp_path, run=None)
    output, code = start(root, "demo-1", caller("sess-1", "agent-1"))
    assert code == 1
    assert output["ok"] is False
    assert "commands.run" in output["error"]
    assert "log" in output


def test_start_without_worktree_is_negative(tmp_path: Path):
    root = make_project(tmp_path, run=None)
    (root / ".worktrees" / "_integration").rmdir()  # the worktrees directory alone: no _integration
    output, code = start(root, "demo-1", caller("sess-1", "agent-1"))
    assert code == 1
    assert output["ok"] is False
    assert "worktree" in output["error"]


def test_start_bad_subject_raises(root: Path):
    with pytest.raises(ValueError):
        start(root, "../evil", None)
    with pytest.raises(ValueError):
        start(root, "", None)
    with pytest.raises(ValueError):
        start(root, "x" * 100, None)
    with pytest.raises(ValueError):
        start(root, "-no-lead-dot", None)


def test_settle_is_two_seconds_by_design():
    assert SETTLE_SECONDS == 2.0


def test_start_retries_on_the_next_port(tmp_path: Path, monkeypatch):
    project = make_project(tmp_path, run='if [ "$VSDD_PORT" = 28100 ]; then exit 1; fi; sleep 30')
    monkeypatch.setattr(env, "SETTLE_SECONDS", 0.3)
    try:
        output, code = start(project, "demo-1", caller("sess-1", "agent-1"))
        assert code == 0, output
        # The taken port was left behind, not retried three times (D15).
        assert output["port"] == 28101
    finally:
        stop_products(project, grace=1.5)


def test_concurrent_starts_claim_distinct_ports(root: Path):
    outcomes: list = []
    failures: list = []

    def boot(n: int) -> None:
        try:
            output, code = start(root, f"demo-{n}", caller(f"sess-{n}", f"agent-{n}"))
            assert code == 0, f"start {n} failed: {output}"
            outcomes.append(output["port"])
        except Exception as exc:  # pragma: no cover - reported through the assert
            failures.append(exc)

    threads = [threading.Thread(target=boot, args=(n,)) for n in range(1, 5)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    assert failures == []
    assert len(outcomes) == 4
    assert len(set(outcomes)) == 4, f"two starts shared a port: {sorted(outcomes)}"


def test_start_runs_a_story_in_its_worktree(root: Path):
    (root / ".worktrees" / "S-9.01").mkdir(parents=True, exist_ok=True)
    output, code = start(root, "S-9.01", caller("sess-1", "agent-1"))
    assert code == 0, output
    started = (root / ".worktrees" / "S-9.01" / "started.txt").read_text().splitlines()
    assert started == [str(output["port"]), str(output["port"])]
    assert not (root / ".worktrees" / "_integration" / "started.txt").exists()


def test_start_runs_anything_but_a_story_in_the_integration_worktree(root: Path):
    (root / ".worktrees" / "spike-x").mkdir(parents=True, exist_ok=True)
    output, code = start(root, "spike-x", caller("sess-1", "agent-1"))
    assert code == 0, output
    started = (root / ".worktrees" / "_integration" / "started.txt").read_text().splitlines()
    assert started == [str(output["port"]), str(output["port"])]
    assert not (root / ".worktrees" / "spike-x" / "started.txt").exists()


# --- stop -------------------------------------------------------------------


def test_stop_stops_only_the_callers_products(root: Path):
    start(root, "demo-1", caller("sess-A", "agent-A"))
    start(root, "demo-2", caller("sess-B", "agent-B"))
    output, code = stop(root, caller("sess-A", "agent-A"), terminal=False)
    assert code == 0
    assert output["ok"] is True
    assert [(entry["subject"], entry["port"]) for entry in output["stopped"]] == [("demo-1", output["stopped"][0]["port"])]
    assert output["stopped"][0]["pid"] > 0
    assert record_for(root, "demo-2")["state"] == "running"  # B's product lives
    assert not (root / ".factory" / "local" / "jobs" / f"{job_key('env', 'demo-1', '')}.json").exists()


def test_stop_without_marker_needs_a_terminal(root: Path):
    start(root, "demo-1", caller("sess-1", "agent-1"))
    output, code = stop(root, None, terminal=False)
    assert code == 1
    assert output["ok"] is False
    assert output["stopped"] == []
    assert "vsdd -C <dir> env stop" in output["error"]
    assert record_for(root, "demo-1")["state"] == "running"
    everything, code = stop(root, None, terminal=True)
    assert code == 0
    assert [entry["subject"] for entry in everything["stopped"]] == ["demo-1"]
    key = job_key("env", "demo-1", "")
    assert not (root / ".factory" / "local" / "jobs" / f"{key}.json").exists()


def test_stop_with_nothing_running_is_negative(root: Path):
    output, code = stop(root, caller("sess-1", "agent-1"), terminal=False)
    assert code == 1
    assert output["ok"] is False
    assert output["stopped"] == []


def assert_gone(pid: int) -> None:
    """A started product is our child: reap it, and treat a zombie as dead."""
    for _ in range(50):
        try:
            done, _ = os.waitpid(pid, os.WNOHANG)
            if done == pid:
                return
        except ChildProcessError:
            return  # reaped elsewhere or not our child
        try:
            os.kill(pid, 0)
        except OSError:
            return
        time.sleep(0.1)
    raise AssertionError(f"process {pid} is still alive")


def test_stop_products_by_subject_session_and_ports(root: Path, tmp_path: Path):
    from vsdd.gates.jobs import list_jobs, product_ports

    start(root, "demo-1", caller("sess-A", "agent-A"))
    trapped = make_project(tmp_path / "b", run="trap '' TERM; sleep 30")
    start(trapped, "demo-2", caller("sess-B", "agent-B"))
    assert product_ports(root, session_id="sess-A") == {28100}
    assert product_ports(trapped, session_id=None) == {28100}

    # A Project object works like a path; a missing filter matches nothing.
    as_object = Project(root=root, state_dir=root / ".factory", worktree=None)
    stopped = stop_products(as_object, subject="demo-1")
    assert [(s["subject"], s["port"]) for s in stopped] == [("demo-1", 28100)]
    assert product_ports(root) == set()

    # A product that swallows TERM still dies: the grace is honoured, then KILL.
    began = time.monotonic()
    stopped = stop_products(trapped, subject="demo-2", grace=2.0)
    elapsed = time.monotonic() - began
    assert [s["subject"] for s in stopped] == ["demo-2"]
    assert elapsed >= 2.0, "the grace waited for the trapped product"
    assert elapsed < 10.0
    assert product_ports(trapped) == set()
    assert list_jobs(trapped) == []
    assert_gone(stopped[0]["pid"])


def test_stop_products_leave_a_starting_record_and_a_held_lock_alone(root: Path):
    import fcntl

    store = root / ".factory" / "local" / "jobs"
    store.mkdir(parents=True, exist_ok=True)
    key = job_key("env", "demo-1", "")

    def record(state: str, pid: int | None) -> dict:
        return {
            "v": 1,
            "key": key,
            "kind": "product",
            "command": RUN,
            "subject": "demo-1",
            "commit": "",
            "state": state,
            "created": "2026-01-01T00:00:00Z",
            "started": None,
            "finished": None,
            "pid": pid,
            "nonce": "b" * 16,
            "session_id": "sess-A",
            "agent_id": "agent-A",
            "port": 28100,
            "url": "http://127.0.0.1:28100",
            "cwd": str(root / ".worktrees" / "_integration"),
        }

    # A 'starting' product whose start call is alive: the settle lock is held
    # (it is never passed to the product), so the record is left alone.
    (store / f"{key}.json").write_text(json.dumps(record("starting", None)), encoding="utf-8")
    lock_path = store / f"{key}.lock"
    fd = os.open(lock_path, os.O_RDWR | os.O_CREAT, 0o644)
    settle_fd = os.open(store / f"{key}.settle.lock", os.O_RDWR | os.O_CREAT, 0o644)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX)
        fcntl.flock(settle_fd, fcntl.LOCK_EX)
        assert stop_products(root, subject="demo-1") == []
        assert (store / f"{key}.json").exists(), "a starting product is left to settle"
        assert lock_path.exists()
    finally:
        os.close(settle_fd)
        os.close(fd)

    # Running, with the product holding the product lock itself: TERM then
    # KILL end it, the lock comes free and the key is deleted and reported.
    holder = subprocess.Popen([sys.executable, "-c", HOLD_LOCK, str(lock_path)], start_new_session=True)
    try:
        wait_for_lock(lock_path)
        (store / f"{key}.json").write_text(json.dumps(record("running", holder.pid)), encoding="utf-8")
        stopped = stop_products(root, subject="demo-1", grace=2.0)
        assert [entry["subject"] for entry in stopped] == ["demo-1"]
        assert not (store / f"{key}.json").exists()
        assert not lock_path.exists(), "the freed lock file is swept with the key"
    finally:
        holder.kill()

    # A product whose lock is still held after the KILL (a holder killpg
    # cannot reach) keeps its record and is not reported stopped: its port
    # must stay taken (S3R-jobs-C1-2).
    keeper = subprocess.Popen(["sleep", "5"], start_new_session=True)
    try:
        fd = os.open(lock_path, os.O_RDWR | os.O_CREAT, 0o644)
        try:
            fcntl.flock(fd, fcntl.LOCK_EX)
            (store / f"{key}.json").write_text(json.dumps(record("running", keeper.pid)), encoding="utf-8")
            assert stop_products(root, subject="demo-1", grace=0.1) == []
            assert (store / f"{key}.json").exists(), "the record stays so the port stays taken"
        finally:
            os.close(fd)
    finally:
        keeper.kill()


def test_stop_products_stops_a_stale_starting_record(root: Path):
    store = root / ".factory" / "local" / "jobs"
    store.mkdir(parents=True, exist_ok=True)
    key = job_key("env", "demo-1", "")
    lock_path = store / f"{key}.lock"
    # The product itself holds the product lock, in its own group, like the
    # process a start call forks.
    holder = subprocess.Popen(
        [sys.executable, "-c", HOLD_LOCK, str(lock_path)],
        start_new_session=True,
    )
    try:
        wait_for_lock(lock_path)
        # A starting record whose settle lock is free: the start call died
        # mid-settle, so the record is stale and the product is stopped by
        # the pid it wrote (S3R-jobs-C1-2).
        stale = {
            "v": 1,
            "key": key,
            "kind": "product",
            "command": RUN,
            "subject": "demo-1",
            "commit": "",
            "state": "starting",
            "created": "2026-01-01T00:00:00Z",
            "started": None,
            "finished": None,
            "pid": holder.pid,
            "nonce": "b" * 16,
            "session_id": "sess-A",
            "agent_id": "agent-A",
            "port": 28100,
            "url": "http://127.0.0.1:28100",
            "cwd": str(root / ".worktrees" / "_integration"),
        }
        (store / f"{key}.json").write_text(json.dumps(stale), encoding="utf-8")
        stopped = stop_products(root, subject="demo-1", grace=1.0)
        assert [entry["subject"] for entry in stopped] == ["demo-1"]
        assert stopped[0]["pid"] == holder.pid
        assert not (store / f"{key}.json").exists(), "the stale record goes with the product"
        assert_gone(holder.pid)
    finally:
        if holder.poll() is None:
            holder.kill()


# --- the five job-layer events (D21) -----------------------------------------


def _job_spec(root: Path, **kwargs) -> JobSpec:
    kwargs.setdefault("command", "gate red")
    kwargs.setdefault("subject", "S-9.01")
    kwargs.setdefault("args", {"command": "echo hello", "cwd": str(root)})
    return JobSpec(commit="", work="vsdd.gates.jobs:run_shell", error_shape={}, **kwargs)


def _wait_for_job_state(root: Path, sp: JobSpec, state: str, timeout: float = 15.0) -> dict:
    path = root / ".factory" / "local" / "jobs" / f"{job_key(sp.command, sp.subject, sp.commit)}.json"
    deadline = time.monotonic() + timeout
    record: dict = {}
    while time.monotonic() < deadline:
        try:
            record = json.loads(path.read_text(encoding="utf-8"))
        except FileNotFoundError:
            record = {}
        if record.get("state") == state:
            return record
        time.sleep(0.1)
    raise AssertionError(f"the job never reached {state!r}: {record}")


def test_the_five_job_events_are_written(root: Path, tmp_path: Path, monkeypatch):
    events_dir = tmp_path / "events"
    monkeypatch.setenv("VSDD_EVENTS_DIR", str(events_dir))
    # attach with a session id writes job.started and job.done.
    sp = _job_spec(root, session_id="sess-ev")
    outcome = attach(root, sp, wait=30, retry="retry")
    assert outcome.output["ok"] is True
    # A killed worker writes job.lost on the next attach.
    loser = _job_spec(
        root,
        subject="S-9.02",
        session_id="sess-ev",
        args={"command": "sleep 3; echo late", "cwd": str(root)},
    )
    assert attach(root, loser, wait=0, retry="retry").pending is True
    running = _wait_for_job_state(root, loser, "running")
    os.kill(running["pid"], signal.SIGKILL)
    time.sleep(0.5)
    recovered = attach(root, loser, wait=30, retry="retry")
    assert recovered.output["ok"] is True
    # env start and env stop with a caller session write env.started, env.stopped.
    started, code = start(root, "demo-ev", caller("sess-ev", "agent-ev"))
    assert code == 0
    stop(root, caller("sess-ev", "agent-ev"), terminal=False)

    log = EventLog(events_dir_for(root), "sess-ev")
    events = log.read()
    types = {event["type"] for event in events}
    assert {"job.started", "job.done", "job.lost", "env.started", "env.stopped"} <= types
    for event in events:
        assert event["session_id"] == "sess-ev"
        assert event["id"]
        assert event["command"]
        assert event["detail"]
