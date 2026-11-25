"""The job library: keys, the store, attach, the worker, the semaphore (G1).

Jobs use ``run_shell``; a project is a temporary directory with a
``factory.toml``. Every test that starts a job exercises the real detached
worker, so a few of these take a second or two.
"""

from __future__ import annotations

import fcntl
import json
import os
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path

import pytest

from vsdd.gates.jobs import (
    DEFAULT_WAIT,
    MAX_WAIT,
    JobSpec,
    _limit_message,
    attach,
    clamp_wait,
    command_env,
    job_key,
    list_jobs,
    prune,
    root_of,
)
from vsdd.schema import validate

HELPER = Path(__file__).resolve().parent / "_jobs_helper.py"
PENDING_SCHEMA = json.loads(
    (Path(__file__).resolve().parents[2] / "plugins/vsdd-factory/schemas/pending-output.schema.json").read_text(
        encoding="utf-8"
    )
)


def make_project(tmp_path: Path, extra: str = "") -> Path:
    root = tmp_path / "proj"
    (root / ".factory").mkdir(parents=True)
    (root / ".factory" / "factory.toml").write_text(f"schema = 1\n{extra}", encoding="utf-8")
    return root


def spec(root: Path, command: str = "gate red", **kwargs) -> JobSpec:
    args = kwargs.pop("args", None) or {"command": "echo hello", "cwd": str(root)}
    shape = kwargs.pop("error_shape", None) or {}
    return JobSpec(
        command=command,
        subject=kwargs.pop("subject", "S-9.01"),
        commit=kwargs.pop("commit", ""),
        work=kwargs.pop("work", "vsdd.gates.jobs:run_shell"),
        args=args,
        error_shape=shape,
        **kwargs,
    )


def record_for(root: Path, sp: JobSpec) -> dict:
    path = root / ".factory" / "local" / "jobs" / f"{job_key(sp.command, sp.subject, sp.commit)}.json"
    return json.loads(path.read_text(encoding="utf-8"))


def result_for(root: Path, sp: JobSpec) -> Path:
    return root / ".factory" / "local" / "jobs" / f"{job_key(sp.command, sp.subject, sp.commit)}.result.json"


def wait_for_state(root: Path, sp: JobSpec, state: str, timeout: float = 15.0) -> dict:
    deadline = time.monotonic() + timeout
    record: dict = {}
    while time.monotonic() < deadline:
        try:
            record = record_for(root, sp)
        except FileNotFoundError:
            record = {}  # the starter has not written the record yet
        if record.get("state") == state:
            return record
        time.sleep(0.1)
    raise AssertionError(f"the job never reached state {state!r}: {record}")


# --- the key and the wait --------------------------------------------------


def test_job_key_is_stable_and_distinct():
    first = job_key("gate red", "S-9.01", "a" * 40)
    assert first == job_key("gate red", "S-9.01", "a" * 40)
    assert len(first) == 16
    assert all(ch in "0123456789abcdef" for ch in first)
    assert first != job_key("gate red", "S-9.01", "b" * 40)
    assert first != job_key("gate red", "S-9.02", "a" * 40)
    assert first != job_key("gate green", "S-9.01", "a" * 40)


def test_clamp_wait():
    assert clamp_wait(None) == DEFAULT_WAIT == 100
    # 540: D10's bar once the platform probe (round 4, Q7) proved a 590 s
    # foreground call with timeout 600000 ms inside a step.
    assert clamp_wait(MAX_WAIT + 300) == MAX_WAIT == 540
    assert clamp_wait(0) == 0
    assert clamp_wait(5) == 5
    with pytest.raises(ValueError):
        clamp_wait(-1)


# --- the happy path --------------------------------------------------------


def test_first_call_starts_and_returns_result(tmp_path: Path):
    root = make_project(tmp_path)
    sp = spec(root)
    outcome = attach(root, sp, wait=30, retry="vsdd gate red")
    assert outcome.pending is False
    assert outcome.output["ok"] is True
    assert outcome.output["returncode"] == 0
    assert "seconds" in outcome.output
    assert record_for(root, sp)["state"] == "done"


def test_same_command_twice_starts_one_job(tmp_path: Path):
    root = make_project(tmp_path)
    counter = root / "runs.txt"
    sp = spec(root, args={"command": f"echo run >> {counter}; echo hello", "cwd": str(root)})
    first = attach(root, sp, wait=0, retry="vsdd gate red --json")
    assert first.pending is True
    assert first.output == {"ok": True, "state": "pending", "retry": "vsdd gate red --json"}
    assert validate(first.output, PENDING_SCHEMA) == []
    second = attach(root, sp, wait=30, retry="vsdd gate red --json")
    assert second.pending is False
    assert second.output["ok"] is True
    assert record_for(root, sp)["starts"] == 1
    assert record_for(root, sp)["state"] == "done"
    assert len(counter.read_text(encoding="utf-8").splitlines()) == 1, "the command ran once"


def test_second_process_reattaches(tmp_path: Path):
    root = make_project(tmp_path)
    sp = spec(root)
    payload = json.dumps(
        {
            "command": sp.command,
            "subject": sp.subject,
            "commit": "",
            "args": {"command": "echo from the helper", "cwd": str(root)},
        }
    )
    done = subprocess.run(
        [sys.executable, "-B", str(HELPER), "attach", str(root), "0", payload],
        capture_output=True,
        text=True,
        timeout=60,
        check=True,
    )
    answer = json.loads(done.stdout.strip().splitlines()[-1])
    assert answer == {"pending": True, "output": {"ok": True, "state": "pending", "retry": "retry from the helper"}}
    outcome = attach(root, sp, wait=30, retry="retry")
    assert outcome.pending is False
    assert outcome.output == {"ok": True, "returncode": 0, "seconds": outcome.output["seconds"]}
    assert record_for(root, sp)["starts"] == 1


def test_job_outlives_killed_starter(tmp_path: Path):
    root = make_project(tmp_path)
    sp = spec(root)
    payload = json.dumps(
        {
            "command": sp.command,
            "subject": sp.subject,
            "args": {"command": "echo survivor", "cwd": str(root)},
        }
    )
    helper = subprocess.Popen(
        [sys.executable, "-B", str(HELPER), "attach", str(root), "30", payload],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        start_new_session=True,
    )
    try:
        running = wait_for_state(root, sp, "running")
        worker_pid = running["pid"]
        listing = subprocess.run(
            ["ps", "-o", "ppid=", "-p", str(worker_pid)],
            capture_output=True,
            text=True,
            timeout=10,
            check=True,
        ).stdout.strip()
        assert listing, "the worker must be alive while it runs"
        worker_ppid = int(listing.split()[0])
        assert worker_ppid != helper.pid, "the worker's parent must not be the starter"
        # Kill the starter's whole process group; the detached worker lives on.
        os.killpg(helper.pid, signal.SIGKILL)
        helper.wait(timeout=10)
        outcome = attach(root, sp, wait=30, retry="retry")
        assert outcome.pending is False
        assert outcome.output["ok"] is True
        assert record_for(root, sp)["starts"] == 1
    finally:
        if helper.poll() is None:
            helper.kill()


def test_collected_result_is_not_served_again(tmp_path: Path):
    root = make_project(tmp_path)
    counter = root / "runs.txt"
    sp = spec(root, args={"command": f"echo run >> {counter}; echo done", "cwd": str(root)})
    first = attach(root, sp, wait=30, retry="retry")
    assert first.output["ok"] is True
    assert record_for(root, sp)["collected"] is True
    second = attach(root, sp, wait=30, retry="retry")
    assert second.output["ok"] is True
    assert len(counter.read_text(encoding="utf-8").splitlines()) == 2, "the work runs again"
    # A result nobody collected is served to the next call.
    sp2 = spec(root, subject="S-9.02", args={"command": "echo once", "cwd": str(root)})
    assert attach(root, sp2, wait=0, retry="retry").pending is True
    served = attach(root, sp2, wait=30, retry="retry")
    assert served.output["ok"] is True
    assert record_for(root, sp2)["starts"] == 1


def test_two_waiters_get_one_result(tmp_path: Path):
    root = make_project(tmp_path)
    counter = root / "runs.txt"
    sp = spec(root, args={"command": f"echo run >> {counter}; sleep 1; echo shared", "cwd": str(root)})
    outcomes: list = []
    lock = threading.Lock()

    def waiter():
        outcome = attach(root, sp, wait=60, retry="retry")
        with lock:
            outcomes.append(outcome)

    threads = [threading.Thread(target=waiter) for _ in range(2)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    assert len(outcomes) == 2
    assert all(not o.pending and o.output["ok"] for o in outcomes)
    assert record_for(root, sp)["starts"] == 1
    assert len(counter.read_text(encoding="utf-8").splitlines()) == 1, (
        "one run behind both results: the nonce decides who is served (D5)"
    )


# --- loss, the semaphore, the limits ---------------------------------------


def _restart_after_lost_worker(root: Path, sp) -> None:
    """Kill the worker once it runs; the restart kills the orphaned group (D6)."""
    assert attach(root, sp, wait=0, retry="retry").pending is True
    running = wait_for_state(root, sp, "running")
    worker_pid = running["pid"]
    deadline = time.monotonic() + 5
    while not isinstance(running.get("command_pgid"), int):
        assert time.monotonic() < deadline, "the running record never named the command's group"
        time.sleep(0.1)
        running = record_for(root, sp)
    command_pgid = running["command_pgid"]
    os.kill(command_pgid, 0)  # the command's group runs beside the worker
    os.kill(worker_pid, signal.SIGKILL)
    time.sleep(0.5)  # let the lock come free
    outcomes: list = []
    thread = threading.Thread(target=lambda: outcomes.append(attach(root, sp, wait=30, retry="retry")))
    thread.start()
    # The restart must kill the orphaned command group: a second copy of the
    # gate command may not run on in the same worktree.
    deadline = time.monotonic() + 3.0
    while time.monotonic() < deadline:
        try:
            os.kill(command_pgid, 0)
        except OSError:
            break  # the group is gone
        time.sleep(0.1)
    else:
        raise AssertionError("the orphaned command group survived the restart")
    thread.join()
    assert outcomes[0].pending is False
    assert outcomes[0].output["ok"] is True
    assert record_for(root, sp)["starts"] == 2


def test_lost_worker_restarts_once(tmp_path: Path):
    root = make_project(tmp_path)
    sp = spec(root, args={"command": "sleep 5; echo late", "cwd": str(root)})
    _restart_after_lost_worker(root, sp)


def _locale_installed(name: str) -> bool:
    try:
        listing = subprocess.run(
            ["locale", "-a"], capture_output=True, text=True, timeout=10, check=False
        ).stdout
    except (OSError, subprocess.TimeoutExpired):
        return False
    stem = name.split(".")[0].lower()
    return any(line.strip().lower().startswith(stem + ".") for line in listing.splitlines())


LOCALES_WITH_OWN_PS_STAMPS = ["en_GB.UTF-8", "de_DE.UTF-8"]


@pytest.mark.parametrize("locale_name", LOCALES_WITH_OWN_PS_STAMPS)
def test_leader_begun_near_reads_a_localised_ps(monkeypatch, locale_name):
    from vsdd.gates.jobs import _leader_begun_near, _now

    if not _locale_installed(locale_name):
        pytest.skip(f"{locale_name} is not installed")
    # ps prints the birth time in the caller's locale: "Tue 24 Nov ..." under
    # en_GB, "Di. 24 Nov. ..." under de_DE. The read is pinned to C on the ps
    # child, so the parse works wherever the caller runs (S3R-jobs-fix-3-C1-1).
    monkeypatch.setenv("LC_ALL", locale_name)
    group = subprocess.Popen(["sleep", "5"], start_new_session=True)
    try:
        assert _leader_begun_near(group.pid, _now()) is True
    finally:
        group.kill()


@pytest.mark.parametrize("locale_name", LOCALES_WITH_OWN_PS_STAMPS)
def test_lost_worker_restarts_once_under_a_localised_ps(tmp_path, monkeypatch, locale_name):
    if not _locale_installed(locale_name):
        pytest.skip(f"{locale_name} is not installed")
    monkeypatch.setenv("LC_ALL", locale_name)
    root = make_project(tmp_path)
    sp = spec(root, args={"command": "sleep 5; echo late", "cwd": str(root)})
    _restart_after_lost_worker(root, sp)


def test_job_that_keeps_dying_fails_loudly(tmp_path: Path):
    root = make_project(tmp_path)
    counter = root / "runs.txt"
    shape = {"gate": "red", "subject": "S-9.01", "passed": False, "sha": "", "evidence": {}, "failures": []}
    sp = spec(root, error_shape=shape, args={"command": f"echo run >> {counter}; kill -9 $PPID", "cwd": str(root)})
    outcome = attach(root, sp, wait=30, retry="retry")
    assert outcome.pending is False
    assert outcome.output["ok"] is False
    assert outcome.output["passed"] is False
    assert outcome.output["gate"] == "red"
    log_path = root / ".factory" / "local" / "jobs" / f"{job_key(sp.command, sp.subject, sp.commit)}.log"
    assert log_path.name in outcome.output["error"]
    assert "log" in outcome.output["error"]
    assert "\n" not in outcome.output["error"], "a job-layer failure is one line"
    assert len(counter.read_text(encoding="utf-8").splitlines()) == 2, "two starts, two runs"
    # A collected failure is never replayed: the next call measures again (D5).
    second = attach(root, sp, wait=30, retry="retry")
    assert second.pending is False
    assert second.output["ok"] is False
    assert len(counter.read_text(encoding="utf-8").splitlines()) > 2, "the work ran again"


def test_loss_cap_comes_from_the_record(tmp_path: Path):
    root = make_project(tmp_path)
    sp = spec(root, args={"command": "sleep 2; kill -9 $PPID", "cwd": str(root)})
    # Call 1: attempt 1 starts; the worker dies two seconds in, after this
    # call's wait has already returned pending.
    assert attach(root, sp, wait=1, retry="retry").pending is True
    wait_for_state(root, sp, "running")
    time.sleep(3.0)
    # Call 2: one restart (starts 2); the worker dies again, once more after
    # the call's wait.
    assert attach(root, sp, wait=1, retry="retry").pending is True
    time.sleep(3.0)
    # Call 3: the job's history caps it - no third start, the failure is loud
    # however short each call was (D6).
    third = attach(root, sp, wait=5, retry="retry")
    assert third.pending is False
    assert third.output["ok"] is False
    assert record_for(root, sp)["starts"] == 2


def test_semaphore_caps_running_jobs(tmp_path: Path):
    root = make_project(tmp_path, extra="[limits]\nparallel_gates = 1\n")
    first = spec(root, subject="S-9.01", args={"command": "sleep 2; echo one", "cwd": str(root)})
    second = spec(root, subject="S-9.02", args={"command": "echo two", "cwd": str(root)})
    results: dict = {}

    def run(name, sp):
        results[name] = attach(root, sp, wait=60, retry="retry")

    first_thread = threading.Thread(target=run, args=("first", first))
    first_thread.start()
    try:
        wait_for_state(root, first, "running")
        second_thread = threading.Thread(target=run, args=("second", second))
        second_thread.start()
        try:
            # While the first job runs, the second never reaches running: one
            # slot, so one worker at a time (D8).
            deadline = time.monotonic() + 1.5
            while time.monotonic() < deadline:
                try:
                    record = record_for(root, second)
                except FileNotFoundError:
                    record = {}
                assert record.get("state") != "running", "parallel_gates = 1 was ignored"
                time.sleep(0.1)
        finally:
            second_thread.join()
    finally:
        first_thread.join()
    assert results["first"].output["ok"] is True
    assert results["second"].output["ok"] is True
    first_finished = record_for(root, first)["finished"]
    second_started = record_for(root, second)["started"]
    assert second_started >= first_finished, "the second job started only after the first finished"
    assert record_for(root, second)["starts"] == 1


def test_time_limit_stops_the_job(tmp_path: Path):
    root = make_project(tmp_path)
    pid_file = root / "sleeper.txt"
    sp = spec(
        root,
        limit_seconds=1,
        args={"command": f"sleep 30 & echo $! > {pid_file}; wait", "cwd": str(root)},
    )
    started = time.monotonic()
    outcome = attach(root, sp, wait=30, retry="retry")
    assert outcome.pending is False
    assert outcome.output["ok"] is False
    log_path = root / ".factory" / "local" / "jobs" / f"{job_key(sp.command, sp.subject, sp.commit)}.log"
    assert outcome.output["error"] == f"ran longer than 1 seconds (limits.gate_minutes); log: {log_path}"
    assert _limit_message(3600) == "ran longer than 60 minutes (limits.gate_minutes)"
    assert time.monotonic() - started < 20, "the limit, not the 30-second sleep, ends the job"
    sleeper = int(pid_file.read_text().strip())
    for _ in range(50):
        try:
            os.kill(sleeper, 0)
        except OSError:
            break
        time.sleep(0.1)
    else:
        raise AssertionError("the sleep survived its job's time limit")


def test_work_crash_keeps_command_shape(tmp_path: Path):
    root = make_project(tmp_path)
    shape = {"gate": "green", "subject": "S-9.01", "passed": False, "sha": "", "evidence": {}, "failures": []}
    sp = spec(root, error_shape=shape, args={"cwd": str(root)})  # no "command" argument
    outcome = attach(root, sp, wait=30, retry="retry")
    assert outcome.pending is False
    assert outcome.output["ok"] is False
    assert outcome.output["passed"] is False
    assert outcome.output["gate"] == "green"
    assert "KeyError" in outcome.output["error"]
    assert "the job crashed" in outcome.output["error"]
    log_path = root / ".factory" / "local" / "jobs" / f"{job_key(sp.command, sp.subject, sp.commit)}.log"
    assert "Traceback" in log_path.read_text(encoding="utf-8")
    assert log_path.name in outcome.output["error"], "the crash names the log"
    assert "\n" not in outcome.output["error"], "a job-layer failure is one line"


def test_bad_spec_is_refused(tmp_path: Path):
    root = make_project(tmp_path)
    store = root / ".factory" / "local" / "jobs"
    with pytest.raises(ValueError):
        attach(root, spec(root, work="not_vsdd:work"), wait=5, retry="retry")
    with pytest.raises(ValueError):
        attach(root, spec(root, command=""), wait=5, retry="retry")
    with pytest.raises(ValueError):
        attach(root, spec(root, subject=""), wait=5, retry="retry")
    with pytest.raises(ValueError):
        attach(root, spec(root, args={"x": {1, 2}}), wait=5, retry="retry")  # a set is not JSON
    with pytest.raises(ValueError):
        # A record over the 4,000-byte cap.
        attach(root, spec(root, args={"blob": "x" * 5000}), wait=5, retry="retry")
    with pytest.raises(ValueError):
        attach(root, spec(root), wait=-1, retry="retry")
    assert not store.exists(), "a refused spec writes nothing, not even the store"


def test_write_record_refuses_even_after_shedding(tmp_path: Path):
    from vsdd.gates.jobs import RECORD_CAP, _write_record

    store = tmp_path / "store"
    store.mkdir(parents=True)
    key = "a" * 16

    def fat_record(**extra):
        base = {
            "v": 1,
            "key": key,
            "kind": "job",
            "state": "queued",
            "created": "2026-01-01T00:00:00Z",
            "started": None,
            "finished": None,
            "pid": None,
            "nonce": "n" * 16,
            "session_id": None,
            "agent_id": None,
            "starts": 1,
            "collected": False,
            "work": "vsdd.gates.jobs:run_shell",
        }
        base.update(extra)
        return base

    # Even once args and error_shape are shed, the record is over the cap:
    # _write_record refuses instead of writing a record nothing may read back.
    bloated = fat_record(
        command="c" * 3850, subject="S-9.01", args={"blob": "x" * 400}, error_shape={"e": "y" * 400}
    )
    over = len(json.dumps(bloated, ensure_ascii=False, sort_keys=True).encode("utf-8"))
    assert over > RECORD_CAP
    with pytest.raises(ValueError):
        _write_record(store, key, bloated)
    assert not (store / f"{key}.json").exists()

    # A record over the cap only through its optional fields is written, shed.
    slimmer = fat_record(command="gate red", args={"blob": "x" * 5000}, error_shape={"e": "y" * 5000})
    _write_record(store, key, slimmer)
    stored = json.loads((store / f"{key}.json").read_text(encoding="utf-8"))
    assert "args" not in stored and "error_shape" not in stored


def sized_spec(root: Path, target_len: int, **kwargs) -> JobSpec:
    """A spec whose queued record measures ``target_len`` bytes."""
    from vsdd.gates.jobs import _job_record, _new_nonce, _resolve_limit

    probe = spec(root, args={"command": "echo ok", "cwd": str(root), "blob": ""}, **kwargs)
    key = job_key(probe.command, probe.subject, probe.commit)
    shape = _job_record(key, probe, _new_nonce(), 1, _resolve_limit(root, probe))
    room = target_len - len(json.dumps(shape, ensure_ascii=False, sort_keys=True).encode("utf-8"))
    assert room > 0
    return spec(root, args={"command": "echo ok", "cwd": str(root), "blob": "x" * room}, **kwargs)


def test_record_within_the_headroom_is_served(tmp_path: Path):
    from vsdd.gates.jobs import RECORD_CAP, RECORD_HEADROOM

    root = make_project(tmp_path)
    # Just under the admission bar: admitted, its record grows by tens of
    # bytes at the worker's writes, still under the cap, and the result is
    # served (S3R-jobs-C1-3).
    sp = sized_spec(root, RECORD_CAP - RECORD_HEADROOM - 20)
    outcome = attach(root, sp, wait=30, retry="retry")
    assert outcome.pending is False
    assert outcome.output["ok"] is True
    assert record_for(root, sp)["state"] == "done"


def test_record_over_the_headroom_is_refused_before_anything_is_written(tmp_path: Path):
    from vsdd.gates.jobs import RECORD_CAP, RECORD_HEADROOM

    root = make_project(tmp_path)
    store = root / ".factory" / "local" / "jobs"
    # Between the admission bar and the cap: the old check admitted it, and
    # the worker's later writes grew over the cap, killed the worker and left
    # the job pending forever. Admission measures the whole life (C1-3), and
    # the refusal names the bar, not the cap (S3R-jobs-C2-2).
    sp = sized_spec(root, RECORD_CAP - 30)
    with pytest.raises(ValueError) as raised:
        attach(root, sp, wait=5, retry="retry")
    message = str(raised.value)
    assert f"{RECORD_CAP - RECORD_HEADROOM}-byte limit" in message, message
    assert f"{RECORD_CAP}-byte cap" in message, "the cap is still named: " + message
    assert f"{RECORD_CAP - 30} bytes" in message, "the measured size is named: " + message
    assert not store.exists()


def test_worker_falls_back_when_the_record_is_over_the_cap(tmp_path: Path):
    from vsdd.gates.jobs import _minimal_record

    root = make_project(tmp_path)
    store = root / ".factory" / "local" / "jobs"
    store.mkdir(parents=True, exist_ok=True)
    key = job_key("gate red", "S-9.01", "")
    # An irreducible record (huge command: _write_record's shedding cannot
    # shrink it): the worker's own writes must fall back to the minimal
    # record, not die with the result unwritten (S3R-jobs-C1-3).
    fat = {
        "v": 1,
        "key": key,
        "kind": "job",
        "command": "c" * 5000,
        "subject": "S-9.01",
        "commit": "",
        "state": "queued",
        "created": "2026-01-01T00:00:00Z",
        "started": None,
        "finished": None,
        "pid": None,
        "nonce": "n" * 16,
        "session_id": None,
        "agent_id": None,
        "starts": 1,
        "collected": False,
        "work": "vsdd.gates.jobs:run_shell",
        "args": {"command": "echo ok", "cwd": str(root)},
        "error_shape": {},
        "limit_seconds": 30,
    }
    (store / f"{key}.json").write_text(json.dumps(fat), encoding="utf-8")
    _worker(_payload_for(root, key, args={"command": "echo ok", "cwd": str(root)}))
    result = json.loads((store / f"{key}.result.json").read_text(encoding="utf-8"))
    assert result["ok"] is True, "the worker kept running and wrote its result"
    record = json.loads((store / f"{key}.json").read_text(encoding="utf-8"))
    # The surviving record is the minimal one, grown only by the small fields
    # of the final writes; the bulky fields are gone with the worker alive.
    minimal = _minimal_record(fat, "done")
    assert all(record[name] == value for name, value in minimal.items())
    assert "args" not in record and "command" not in record


def test_stale_command_group_survives_the_restart(tmp_path: Path):
    root = make_project(tmp_path)
    store = root / ".factory" / "local" / "jobs"
    store.mkdir(parents=True, exist_ok=True)
    key = job_key("gate red", "S-9.01", "")
    outsider = subprocess.Popen(["sleep", "30"], start_new_session=True)
    try:
        # A stale record whose command_pgid names an unrelated live group and
        # whose cmd lock is free (no worker ever ran here): the restart may
        # not kill the group, which may belong to anything after the old one
        # exited or a reboot (D3, S3R-jobs-C1-1).
        record = {
            "v": 1,
            "key": key,
            "kind": "job",
            "command": "gate red",
            "subject": "S-9.01",
            "commit": "",
            "state": "running",
            "created": "2026-01-01T00:00:00Z",
            "started": None,
            "finished": None,
            "pid": 999999,
            "nonce": "n" * 16,
            "session_id": None,
            "agent_id": None,
            "starts": 1,
            "collected": False,
            "work": "vsdd.gates.jobs:run_shell",
            "args": {"command": "echo ok", "cwd": str(root)},
            "error_shape": {},
            "limit_seconds": 30,
            "command_pgid": outsider.pid,
        }
        (store / f"{key}.json").write_text(json.dumps(record), encoding="utf-8")
        sp = spec(root)
        outcome = attach(root, sp, wait=30, retry="retry")
        assert outcome.pending is False
        assert outcome.output["ok"] is True
        assert record_for(root, sp)["starts"] == 2, "the lost worker was restarted"
        try:
            os.killpg(outsider.pid, 0)
        except OSError:
            raise AssertionError("the unrelated live group was killed")
    finally:
        outsider.kill()


def test_leader_begun_near_matches_the_ps_birth_time():
    from vsdd.gates.jobs import _leader_begun_near, _now

    group = subprocess.Popen(["sleep", "5"], start_new_session=True)
    try:
        assert _leader_begun_near(group.pid, _now()) is True
        assert _leader_begun_near(group.pid, "2020-01-01T00:00:00Z") is False
        assert _leader_begun_near(group.pid, None) is False, "no recorded time: no kill"
        assert _leader_begun_near(group.pid, "not-a-stamp") is False
        assert _leader_begun_near(10_000_000, _now()) is False, "no such process: no kill"
    finally:
        group.kill()


def test_escapee_of_an_earlier_attempt_neither_delays_nor_authorises(tmp_path: Path):
    root = make_project(tmp_path)
    store = root / ".factory" / "local" / "jobs"
    key = job_key("gate red", "S-9.01", "")
    escapee_pid_file = root / "escapee.pid"
    # The command spawns a descendant that leaves its group with setsid and
    # keeps the inherited cmd lock descriptor, then finishes: the job ends
    # while the escapee still holds <key>.cmd.lock (S3R-jobs-C1-1).
    escapee_command = (
        f"'{sys.executable}' -c 'import os,sys,time; os.setsid(); "
        f"open(sys.argv[1],\"w\").write(str(os.getpid())); time.sleep(60)' "
        f"'{escapee_pid_file}' & sleep 1; echo ready"
    )
    sp = spec(root, args={"command": escapee_command, "cwd": str(root)})
    first = attach(root, sp, wait=30, retry="retry")
    assert first.pending is False
    assert first.output["ok"] is True
    from vsdd.gates.jobs import _lock_held

    assert _lock_held(store / f"{key}.cmd.lock"), "the escapee holds the cmd lock"
    escapers = [int(escapee_pid_file.read_text())]
    outsider = subprocess.Popen(["sleep", "30"], start_new_session=True)
    try:
        # A stale running record naming an unrelated live group - the shape a
        # reused group number takes - with no result: the next attach takes
        # the loss branch.
        stale = {
            "v": 1,
            "key": key,
            "kind": "job",
            "command": "gate red",
            "subject": "S-9.01",
            "commit": "",
            "state": "running",
            "created": "2026-01-01T00:00:00Z",
            "started": "2026-01-01T00:00:00Z",
            "finished": None,
            "pid": 999999,
            "nonce": "n" * 16,
            "session_id": None,
            "agent_id": None,
            "starts": 1,
            "collected": False,
            "work": "vsdd.gates.jobs:run_shell",
            "args": {"command": "echo ok", "cwd": str(root)},
            "error_shape": {},
            "limit_seconds": 30,
            "command_pgid": outsider.pid,
            "command_started": "2026-01-01T00:00:00Z",
        }
        (store / f"{key}.json").write_text(json.dumps(stale), encoding="utf-8")
        (store / f"{key}.result.json").unlink(missing_ok=True)
        began = time.monotonic()
        second = attach(root, sp, wait=30, retry="retry")
        elapsed = time.monotonic() - began
        assert second.pending is False
        assert second.output["ok"] is True
        assert record_for(root, sp)["starts"] == 2
        # The re-run did not wait out the escapee's five seconds: attach gave
        # this attempt a fresh cmd lock.
        assert elapsed < 5.0, "the re-run waited for the escapee's cmd lock"
        escapers.append(int(escapee_pid_file.read_text()))
        try:
            os.killpg(outsider.pid, 0)
        except OSError:
            raise AssertionError("the unrelated live group was killed")
    finally:
        outsider.kill()
        for pid in escapers:
            try:
                os.kill(pid, signal.SIGKILL)
            except OSError:
                pass


def test_worker_without_the_cmd_lock_records_no_group(tmp_path: Path):
    root = make_project(tmp_path)
    store = root / ".factory" / "local" / "jobs"
    store.mkdir(parents=True, exist_ok=True)
    key = job_key("gate red", "S-9.01", "")
    # A cmd lock the worker cannot open: the worker runs without it and then
    # records no command group, because the lock is what would later
    # authorise the kill (S3R-jobs-C1-1).
    (store / f"{key}.cmd.lock").mkdir()
    _worker(_payload_for(root, key, args={"command": "echo ok", "cwd": str(root)}))
    result = json.loads((store / f"{key}.result.json").read_text(encoding="utf-8"))
    assert result["ok"] is True
    record = json.loads((store / f"{key}.json").read_text(encoding="utf-8"))
    assert record.get("command_pgid") is None, "a lockless worker named a group"


# --- the context ------------------------------------------------------------


def test_context_run_writes_output_and_log(tmp_path: Path):
    from vsdd.gates.jobs import JobContext

    root = make_project(tmp_path)
    out = tmp_path / "out.txt"
    log = tmp_path / "job.log"
    log.write_text("", encoding="utf-8")
    ctx = JobContext(
        root=root,
        key="testkey",
        args={},
        config=None,
        log=log,
        deadline=time.monotonic() + 30,
        guard={"pgid": None},
    )
    result = ctx.run("echo picked; echo err >&2", cwd=root, output=out)
    assert result.returncode == 0
    assert result.output == out
    assert "picked" in out.read_text(encoding="utf-8")
    text = log.read_text(encoding="utf-8")
    assert "$ echo picked; echo err >&2" in text
    assert "# exit 0" in text
    with pytest.raises(ValueError):
        ctx.run("", cwd=root)
    over = ctx.run("echo overwritten", cwd=root, output=out)
    assert "overwritten" in out.read_text(encoding="utf-8")
    assert "picked" not in out.read_text(encoding="utf-8").split("overwritten")[0][-200:]
    assert over.seconds >= 0


def test_command_env_scrubs_tool_variables(tmp_path: Path, monkeypatch):
    from vsdd.config import loads

    monkeypatch.setenv("PYTHONSAFEPATH", "1")
    monkeypatch.setenv("PYTHONUTF8", "1")
    monkeypatch.setenv("PYTHONPYCACHEPREFIX", "/tmp/cache")
    monkeypatch.setenv("UV_RUN_RECURSION_DEPTH", "2")
    monkeypatch.setenv("VSDD_CALLER", "marker-key")
    monkeypatch.setenv("VIRTUAL_ENV", sys.prefix)
    own_bin = str(Path(sys.prefix) / "bin")
    monkeypatch.setenv("PATH", os.pathsep.join([own_bin, "/usr/bin", "/bin"]))
    config = loads('schema = 1\n[commands.env]\nFACTORY_FLAG = "on"\n')
    env = command_env(tmp_path, config)
    for name in ("PYTHONSAFEPATH", "PYTHONUTF8", "PYTHONPYCACHEPREFIX", "UV_RUN_RECURSION_DEPTH"):
        assert name not in env
    assert not any(name.startswith("VSDD_") for name in env)
    assert "VIRTUAL_ENV" not in env
    assert own_bin not in env["PATH"].split(os.pathsep)
    assert "/usr/bin" in env["PATH"].split(os.pathsep)
    assert env["FACTORY_FLAG"] == "on"


# --- listing and pruning ----------------------------------------------------


def test_list_jobs_and_prune(tmp_path: Path):
    root = make_project(tmp_path)
    live = spec(root, subject="S-9.01", args={"command": "sleep 5; echo live", "cwd": str(root)})
    assert attach(root, live, wait=0, retry="retry").pending is True
    listed = list_jobs(root)
    assert len(listed) == 1
    assert listed[0]["subject"] == "S-9.01"
    assert listed[0]["alive"] is True
    assert listed[0]["has_result"] is False
    assert prune(root) == []

    finished = attach(root, live, wait=30, retry="retry")
    assert finished.output["ok"] is True
    assert record_for(root, live)["state"] == "done"

    # A finished, lock-free key whose files are old is pruned.
    jobs_dir = root / ".factory" / "local" / "jobs"
    old_key = job_key("gate green", "S-8.01", "")
    (jobs_dir / f"{old_key}.json").write_text(
        json.dumps({"v": 1, "key": old_key, "kind": "job", "state": "done", "collected": True}),
        encoding="utf-8",
    )
    (jobs_dir / f"{old_key}.log").write_text("old\n", encoding="utf-8")
    stamp = time.time() - 8 * 86400
    for path in (jobs_dir / f"{old_key}.json", jobs_dir / f"{old_key}.log"):
        os.utime(path, (stamp, stamp))
    removed = prune(root)
    assert removed == [old_key]
    assert not (jobs_dir / f"{old_key}.json").exists()
    survivors = [entry["key"] for entry in list_jobs(root)]
    assert job_key("gate red", "S-9.01", "") in survivors


def test_list_jobs_skips_a_foreign_file(tmp_path: Path):
    root = make_project(tmp_path)
    store = root / ".factory" / "local" / "jobs"
    store.mkdir(parents=True, exist_ok=True)
    (store / "not-a-key.json").write_text("{ torn", encoding="utf-8")
    assert list_jobs(root) == []
    assert prune(root) == []


def test_root_of_accepts_the_three_shapes(tmp_path: Path):
    class Holder:
        root = Path(tmp_path)

    assert root_of(tmp_path) == Path(tmp_path)
    assert root_of(str(tmp_path)) == Path(tmp_path)
    assert root_of(Holder()) == Path(tmp_path)


# --- the worker's own safety nets -------------------------------------------


def _payload_for(root: Path, key: str, **extra) -> dict:
    payload = {
        "root": str(root),
        "key": key,
        "command": "gate red",
        "subject": "S-9.01",
        "commit": "",
        "work": "vsdd.gates.jobs:run_shell",
        "args": {"command": "echo hello", "cwd": str(root)},
        "error_shape": {},
        "session_id": None,
        "limit_seconds": 30,
    }
    payload.update(extra)
    return payload


def _worker(payload: dict, **kwargs) -> subprocess.CompletedProcess:
    command = [sys.executable, "-B", str(HELPER), "worker", payload["root"], json.dumps(payload)]
    return subprocess.run(command, capture_output=True, text=True, timeout=60, **kwargs)


def test_alarm_backstop_ends_an_overrun_outside_ctx_run(tmp_path: Path):
    root = make_project(tmp_path)
    store = root / ".factory" / "local" / "jobs"
    store.mkdir(parents=True, exist_ok=True)
    key = job_key("gate red", "S-9.01", "")
    # run_sleep never calls ctx.run: only the worker's alarm backstop (D9) can
    # end a work function that runs past the limit.
    payload = _payload_for(
        root,
        key,
        work="vsdd.gates.jobs:run_sleep",
        args={"seconds": 30},
        limit_seconds=1,
        alarm_margin=1,
    )
    began = time.monotonic()
    _worker(payload)
    assert time.monotonic() - began < 20, "the alarm backstop ends the worker"
    log_path = store / f"{key}.log"
    result = json.loads((store / f"{key}.result.json").read_text(encoding="utf-8"))
    assert result["ok"] is False
    assert result["error"] == f"ran longer than 1 seconds (limits.gate_minutes); log: {log_path}"
    record = json.loads((store / f"{key}.json").read_text(encoding="utf-8"))
    assert record["state"] == "done"


def test_worker_marks_the_lock_non_inheritable(tmp_path: Path):
    root = make_project(tmp_path)
    store = root / ".factory" / "local" / "jobs"
    store.mkdir(parents=True, exist_ok=True)
    key = job_key("gate red", "S-9.01", "")
    lock_path = store / f"{key}.lock"
    fd = os.open(lock_path, os.O_RDWR | os.O_CREAT, 0o644)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX)  # the caller holds the lock; the worker only inherits it
        payload = _payload_for(root, key, lock_fd=fd)
        # pass_fds keeps the descriptor's number in the child.
        done = _worker(payload, pass_fds=(fd,))
        assert json.loads(done.stdout.strip().splitlines()[-1]) == {"inheritable": False}
    finally:
        os.close(fd)


def test_result_is_served_only_when_done(tmp_path: Path):
    root = make_project(tmp_path)
    sp = spec(root)
    key = job_key(sp.command, sp.subject, sp.commit)
    store = root / ".factory" / "local" / "jobs"
    store.mkdir(parents=True, exist_ok=True)
    lock_fd = os.open(store / f"{key}.lock", os.O_RDWR | os.O_CREAT, 0o644)
    try:
        fcntl.flock(lock_fd, fcntl.LOCK_EX)  # a worker is alive
        nonce = "a" * 16
        record = {
            "v": 1,
            "key": key,
            "kind": "job",
            "state": "running",
            "nonce": nonce,
            "collected": False,
            "starts": 1,
        }
        (store / f"{key}.json").write_text(json.dumps(record), encoding="utf-8")
        outcomes: list = []
        thread = threading.Thread(
            target=lambda: outcomes.append(attach(root, sp, wait=10, retry="retry"))
        )
        thread.start()
        time.sleep(1.0)  # the attach call has polled and is waiting on the record
        # The worker wrote its result and died before writing done. A result
        # may not be served from that half-written state: the done write is
        # the last word on collected and nonce (D5).
        (store / f"{key}.result.json").write_text(
            json.dumps({"ok": True, "returncode": 0, "seconds": 0.1}), encoding="utf-8"
        )
        time.sleep(0.6)
        assert outcomes == [], "a result was served from a record that was not done"
        record["state"] = "done"
        (store / f"{key}.json").write_text(json.dumps(record), encoding="utf-8")
        fcntl.flock(lock_fd, fcntl.LOCK_UN)
        thread.join(timeout=15)
        assert len(outcomes) == 1
        assert outcomes[0].output == {"ok": True, "returncode": 0, "seconds": 0.1}
        final = json.loads((store / f"{key}.json").read_text(encoding="utf-8"))
        assert final["collected"] is True
        assert final["state"] == "done"
    finally:
        os.close(lock_fd)


def test_prune_removes_abandoned_temporaries(tmp_path: Path):
    root = make_project(tmp_path)
    store = root / ".factory" / "local" / "jobs"
    store.mkdir(parents=True, exist_ok=True)
    old_key = job_key("gate red", "S-9.01", "")
    other_key = job_key("gate green", "S-9.02", "")
    abandoned = store / f".{old_key}.json.as7Xq1b"  # the mkstemp form of _atomic_write
    abandoned.write_text("{ torn", encoding="utf-8")
    stamp = time.time() - 8 * 86400
    os.utime(abandoned, (stamp, stamp))
    fresh = store / f".{other_key}.result.json.zz9qQ2c"
    fresh.write_text("{}", encoding="utf-8")
    assert prune(root) == [old_key]
    assert not abandoned.exists(), "a kill between mkstemp and replace leaves no litter"
    assert fresh.exists(), "a young temporary is kept"
