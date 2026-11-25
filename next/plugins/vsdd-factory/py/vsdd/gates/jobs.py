"""Detached, re-attaching jobs and the product environment (package G1).

Every long command runs its whole body in a detached worker process and the
command itself re-attaches: the first call starts the job, every call waits for
it and returns either the result or ``{"ok": true, "state": "pending",
"retry": <the same command>}`` (topics/long-commands.md). There is no job id
to carry: the key is the hash of the command, its subject and the commit.

The store is ``<root>/.factory/local/jobs/``. Per key: ``<key>.json`` (the
record, at most 4,000 bytes, written atomically), ``<key>.log``,
``<key>.result.json`` (jobs only; a result is its own file because a gate
result can exceed the record cap) and ``<key>.lock``. Two further per-key
locks hang beside the record: ``<key>.cmd.lock`` is held by the worker and
inherited by its command; ``attach`` unlinks it before each new attempt, so a
descendant that escaped an earlier command holds only an unlinked inode, and
a kill on a recorded ``command_pgid`` waits for the group leader's ``ps``
start time to match the record's ``command_started``; ``<key>.settle.lock``
is held by an ``env start`` call and is never passed to the product, which
separates a live start call from a stale ``starting`` record. ``slots/<n>.lock`` are
the semaphore of ``limits.parallel_gates``; ``ports.lock`` serialises port
choice for products.

Liveness is a file lock (D3): the starter takes ``flock`` on ``<key>.lock``
and hands the descriptor to the worker (or product). "Alive" means the lock
cannot be taken; no process-id reuse, no heartbeat; a kill or a reboot frees
it. ``pass_fds`` makes the descriptor inheritable in the worker against PEP
446's default, so the worker marks it non-inheritable again at startup; child
commands run with ``close_fds``, so a leftover child never makes a dead job
look alive.

This module imports the standard library, ``vsdd.config``, ``vsdd.project``
and ``vsdd.events`` only, so state or hook code may import it with no cycle.
"""

from __future__ import annotations

import calendar
import fcntl
import hashlib
import importlib
import json
import os
import re
import signal
import subprocess
import sys
import tempfile
import time
import traceback
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

from ..config import Config, ConfigError
from ..config import load as _load_config
from ..events import EventError, EventLog, events_dir_for
from ..project import FACTORY_DIR

__all__ = [
    "ALARM_MARGIN",
    "DEFAULT_WAIT",
    "MAX_WAIT",
    "POLL",
    "RECORD_CAP",
    "RETAIN_DAYS",
    "CommandResult",
    "JobContext",
    "JobSpec",
    "JobTimeout",
    "Outcome",
    "RECORD_HEADROOM",
    "attach",
    "clamp_wait",
    "command_env",
    "job_key",
    "list_jobs",
    "prune",
    "product_ports",
    "root_of",
    "run_shell",
    "run_sleep",
    "stop_products",
    "store_of",
    "worker_main",
]

#: A call waits 100 seconds by default: inside the Bash tool's two-minute floor.
DEFAULT_WAIT = 100
#: One call never waits longer than this. D10 capped it at 240 until a
#: ten-minute foreground call was proven, which the platform's round-4 probe
#: Q7 then ran (a 590 s foreground call with an explicit timeout of 600000 ms
#: to its end inside a step, 2026-11-24; rewrite/spike/probes-round-4/CHECK.md).
MAX_WAIT = 540
#: Seconds between attaches' polls of the record and the lock.
POLL = 0.2
#: The record's byte cap.
RECORD_CAP = 4000
#: Headroom demanded at admission over the cap: a queued record's later writes
#: (state, started, finished, pid, command_pgid) grow by tens of bytes, so a
#: record admitted within this margin of the cap stays writable to the end
#: (S3R-jobs-C1-3). The worker's over-cap fallback is the second belt.
RECORD_HEADROOM = 128
#: Finished, lock-free keys younger than this are kept.
RETAIN_DAYS = 7
#: Seconds past the run-time limit at which the worker's alarm backstop fires
#: (D9): the belt behind ``ctx.run``'s own deadline.
ALARM_MARGIN = 30

_WORK_RE = re.compile(r"^vsdd(\.[a-z_][a-z0-9_]*)+:[a-z_][a-z0-9_]*$")
_KEY_RE = re.compile(r"[0-9a-f]{16}")

#: Tool variables a configured command must not inherit (D12).
_SCRUBBED_VARS = (
    "PYTHONSAFEPATH",
    "PYTHONUTF8",
    "PYTHONPYCACHEPREFIX",
    "UV_RUN_RECURSION_DEPTH",
)


class JobTimeout(Exception):
    """A job's command ran past the remaining run-time limit."""


# ---------------------------------------------------------------------------
# Arguments shared by every function: a Project, any object with ``.root``,
# a str or a Path.


def root_of(root: object) -> Path:
    """The project root of a ``Project``, an object with ``.root``, a str or a Path."""
    if isinstance(root, (str, os.PathLike)):
        return Path(root)
    inner = getattr(root, "root", None)
    if isinstance(inner, (str, os.PathLike)):
        return Path(inner)
    return Path(root)


def store_of(root: object) -> Path:
    """The job store under a project root (a ``Project``, ``.root``, str or Path)."""
    return root_of(root) / FACTORY_DIR / "local" / "jobs"


def _now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _load_config_or_none(root: Path) -> Config | None:
    try:
        return _load_config(root / FACTORY_DIR / "factory.toml")
    except (OSError, ConfigError, ValueError):
        return None


def _new_nonce() -> str:
    return os.urandom(8).hex()


def _atomic_write(path: Path, data: bytes) -> None:
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.")
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
        os.replace(tmp, path)
    except BaseException:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def _read_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None


def _write_record(store: Path, key: str, record: dict) -> None:
    data = json.dumps(record, ensure_ascii=False, sort_keys=True).encode("utf-8")
    if len(data) > RECORD_CAP:
        # Shed the bulky optional fields (the args live in the payload and the
        # log) before refusing; the cap binds products as well as jobs.
        trimmed = dict(record)
        for optional in ("args", "error_shape"):
            trimmed.pop(optional, None)
            data = json.dumps(trimmed, ensure_ascii=False, sort_keys=True).encode("utf-8")
            if len(data) <= RECORD_CAP:
                break
        if len(data) > RECORD_CAP:
            raise ValueError(f"the record for {key} is {len(data)} bytes, over the {RECORD_CAP}-byte cap")
    _atomic_write(store / f"{key}.json", data)


def _write_result(store: Path, key: str, result: dict) -> None:
    data = json.dumps(result, ensure_ascii=False, sort_keys=True).encode("utf-8")
    _atomic_write(store / f"{key}.result.json", data)


def _minimal_record(record: dict, state: str) -> dict:
    """The last-resort record for a write that busts the cap (S3R-jobs-C1-3).

    Carries only what serving and liveness read: a record near the cap must
    cost the job its result, not the worker's life. Small enough to fit under
    any cap this module sets.
    """
    return {
        "v": 1,
        "key": record.get("key"),
        "kind": record.get("kind"),
        "state": state,
        "nonce": record.get("nonce"),
        "collected": bool(record.get("collected")),
        "starts": record.get("starts", 1),
    }


def _truncate(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "wb"):
        pass


def _append_line(path: Path, line: str) -> None:
    with open(path, "ab") as handle:
        handle.write((line.rstrip("\n") + "\n").encode("utf-8"))


def _acquire(fd: int) -> bool:
    """Take an exclusive, non-blocking ``flock`` on an open descriptor."""
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return True
    except OSError:
        return False


def _lock_held(path: Path) -> bool:
    """True when another process holds the lock file (or holds one we cannot test)."""
    try:
        fd = os.open(path, os.O_RDWR)
    except FileNotFoundError:
        return False
    except OSError:
        return True
    try:
        if _acquire(fd):
            fcntl.flock(fd, fcntl.LOCK_UN)
            return False
        return True
    finally:
        os.close(fd)


def _pid_alive(pid: object) -> bool:
    """True when a process id names a process that can still be signalled.

    A number that belongs to nobody is gone; an answer that leaves the
    question open (a process we may not signal) is alive, because the caller
    acts on the safer assumption.
    """
    if not isinstance(pid, int) or pid <= 0:
        return False
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except OSError:
        return True
    return True


def _group_alive(pid: object) -> bool:
    """True when a process id names a process group that still has a member.

    A product's recorded pid is only its group leader: the leader forks the
    product's real work, every member inherits the product lock, and
    ``stop_products`` signals the whole group with ``killpg``. So a product
    lives exactly as long as its group answers a signal-zero probe
    (S3R-jobs-fix-3-C2-1): the group is gone when ``killpg`` says
    ``ProcessLookupError``; an answer that leaves the question open (a member
    we may not signal) is alive, as ``_pid_alive`` judges it.
    """
    if not isinstance(pid, int) or pid <= 0:
        return False
    try:
        os.killpg(pid, 0)
    except ProcessLookupError:
        return False
    except OSError:
        return True
    return True


def _emit(root: Path, session_id: str | None, type_: str, **fields) -> None:
    """Write one event when a session id is known; the writer requires one (D21)."""
    if not session_id:
        return
    try:
        EventLog(events_dir_for(root), session_id).log(type_, **fields)
    except (EventError, OSError):
        pass  # an event failure never fails the job that logged it


# ---------------------------------------------------------------------------
# The key and the spec


def job_key(command: str, subject: str, commit: str) -> str:
    """The first 16 hex digits of the SHA-256 of the three parts, newline-joined.

    ``command`` is the command's words plus every option that changes the work
    (``gate red --refreeze``); it never holds ``--run``, ``--wait``, ``--json``
    or ``-C`` (D11). ``commit`` is the full SHA the job measures, or ``""`` for
    a job keyed by its subject alone.
    """
    joined = "\n".join((command, subject, commit))
    return hashlib.sha256(joined.encode("utf-8")).hexdigest()[:16]


@dataclass(frozen=True)
class JobSpec:
    """What a job runs, and the shape its failure keeps (D7)."""

    command: str
    subject: str
    commit: str
    work: str
    args: dict
    error_shape: dict
    session_id: str | None = None
    limit_seconds: int | None = None


@dataclass(frozen=True)
class Outcome:
    """The answer of one ``attach``: pending with the retry object, or a result."""

    pending: bool
    output: dict


@dataclass(frozen=True)
class CommandResult:
    """How one command inside a job ended."""

    returncode: int
    seconds: float
    output: Path


def _spec_problem(spec: JobSpec) -> str | None:
    if not isinstance(spec.command, str) or not spec.command.strip():
        return "command is empty"
    if not isinstance(spec.subject, str) or not spec.subject:
        return "subject is empty"
    if not isinstance(spec.work, str) or not _WORK_RE.fullmatch(spec.work):
        return f"work {spec.work!r} is not vsdd.<module>:<function>"
    for name, value in (("args", spec.args), ("error_shape", spec.error_shape)):
        if not isinstance(value, dict):
            return f"{name} is not an object"
        try:
            json.dumps(value)
        except (TypeError, ValueError) as exc:
            return f"{name} is not JSON: {exc}"
    return None


def _job_record(key: str, spec: JobSpec, nonce: str, starts: int, limit: int) -> dict:
    return {
        "v": 1,
        "key": key,
        "kind": "job",
        "command": spec.command,
        "subject": spec.subject,
        "commit": spec.commit,
        "state": "queued",
        "created": _now(),
        "started": None,
        "finished": None,
        "pid": None,
        "nonce": nonce,
        "session_id": spec.session_id,
        "agent_id": None,
        "starts": starts,
        "collected": False,
        "work": spec.work,
        "args": spec.args,
        "error_shape": spec.error_shape,
        "limit_seconds": limit,
        "command_pgid": None,
    }


def _resolve_limit(root: Path, spec: JobSpec) -> int:
    if spec.limit_seconds is not None:
        return int(spec.limit_seconds)
    config = _load_config_or_none(root)
    if config is not None:
        try:
            return int(config.get("limits", "gate_minutes")) * 60
        except (KeyError, TypeError, ValueError):
            pass
    return 60 * 60


def clamp_wait(seconds: int | None) -> int:
    """The bounded wait of one call: default 100, at most MAX_WAIT (D10)."""
    if seconds is None:
        return DEFAULT_WAIT
    if seconds < 0:
        raise ValueError(f"wait must not be negative, got {seconds}")
    return min(int(seconds), MAX_WAIT)


# ---------------------------------------------------------------------------
# The worker


def _spawn_worker(root: Path, lock_fd: int, key: str, spec: JobSpec, limit: int) -> None:
    """Start the detached worker with the job lock handed over (D2, D3)."""
    package_root = Path(__file__).resolve().parents[2]
    worker = Path(__file__).resolve().parent / "job_worker.py"
    payload = {
        "root": str(root),
        "key": key,
        "command": spec.command,
        "subject": spec.subject,
        "commit": spec.commit,
        "work": spec.work,
        "args": spec.args,
        "error_shape": spec.error_shape,
        "session_id": spec.session_id,
        "limit_seconds": limit,
        "lock_fd": lock_fd,
    }
    cmd = [sys.executable]
    if sys.dont_write_bytecode:
        cmd.append("-B")  # D22: no bytecode in the plugin tree
    cmd += [str(worker), str(package_root), json.dumps(payload)]
    log = open(store_of(root) / f"{key}.log", "ab")  # noqa: SIM115 - handed to the child
    try:
        subprocess.Popen(
            cmd,
            pass_fds=(lock_fd,),
            start_new_session=True,
            stdin=subprocess.DEVNULL,
            stdout=log,
            stderr=subprocess.STDOUT,
        )
    finally:
        log.close()


def _resolve(work: str):
    module_part, _, name = work.partition(":")
    return getattr(importlib.import_module(module_part), name)


def _limit_message(limit_seconds: int) -> str:
    if limit_seconds >= 60 and limit_seconds % 60 == 0:
        return f"ran longer than {limit_seconds // 60} minutes (limits.gate_minutes)"
    return f"ran longer than {limit_seconds} seconds (limits.gate_minutes)"


def _job_failure(error_shape: dict, message: str, log_path: Path) -> dict:
    """One job-layer failure: the command's shape, one line, naming the log."""
    one_line = " ".join(str(message).split())
    return {**error_shape, "ok": False, "error": f"{one_line}; log: {log_path}"}


def _take_slot(store: Path, config: Config | None) -> None:
    """Take one of ``parallel_gates`` slot locks; a dead worker's slot frees itself (D8)."""
    count = 4
    if config is not None:
        try:
            count = int(config.get("limits", "parallel_gates"))
        except (KeyError, TypeError, ValueError):
            count = 4
    slots = store / "slots"
    slots.mkdir(parents=True, exist_ok=True)
    while True:
        for n in range(1, max(count, 1) + 1):
            fd = os.open(slots / f"{n}.lock", os.O_RDWR | os.O_CREAT, 0o644)
            if _acquire(fd):
                return  # held until the worker exits
            os.close(fd)
        time.sleep(POLL)


def _take_cmd_lock(path: Path) -> int | None:
    """Take the per-key command lock for the worker (D3, S3R-jobs-C1-1).

    The worker holds it and its commands inherit the descriptor through
    ``pass_fds``, so the flock lives exactly as long as the command's process
    tree. ``attach`` unlinks the file before each new attempt, so this
    worker's file is freshly created and taken at once; an ``attach`` that
    later finds it held knows a recorded ``command_pgid`` was born of the
    attempt whose lock it is. Returns ``None`` when the lock cannot be taken
    or the file cannot be opened: a worker that runs without the lock records
    no ``command_pgid``, and a missed kill is safe.
    """
    try:
        fd = os.open(path, os.O_RDWR | os.O_CREAT, 0o644)
    except OSError:
        return None
    for _ in range(25):  # five seconds, then start unlocked
        if _acquire(fd):
            return fd
        time.sleep(POLL)
    os.close(fd)
    return None


def _leader_begun_near(pgid: int, begun: str | None, skew: float = 5.0) -> bool:
    """True when the group's leader looks like the process the record named.

    Compares the leader's ``ps`` birth time with the ISO time the record
    carried (``command_started``, or ``started``): a number reused after the
    recorded group died belongs to a leader born later, far from the record's
    time (S3R-jobs-C1-1). Anything that cannot be read - no such process, no
    parsable stamp, no recorded time - answers False, because a missed kill
    is safe and the group may already be gone.
    """
    if not isinstance(begun, str) or not begun:
        return False
    try:
        wanted = calendar.timegm(time.strptime(begun, "%Y-%m-%dT%H:%M:%SZ"))
    except ValueError:
        return False
    try:
        probe = subprocess.run(
            ["/bin/ps", "-o", "lstart=", "-p", str(pgid)],
            capture_output=True,
            text=True,
            timeout=5,
            check=False,
            # Pin the child's locale: ps prints the birth time in the caller's
            # ("Tue 24 Nov ..." under en_GB, "Di. 24 Nov. ..." under de_DE),
            # which the fixed-format read below cannot parse. With LC_ALL=C the
            # stamp is always "Tue Nov 24 18:27:07 2026" (S3R-jobs-fix-3-C1-1).
            env={**os.environ, "LC_ALL": "C"},
        )
    except (OSError, subprocess.TimeoutExpired):
        return False
    stamp = " ".join(probe.stdout.split())
    if probe.returncode != 0 or not stamp:
        return False
    try:
        born = time.strptime(stamp, "%a %b %d %H:%M:%S %Y")
    except ValueError:
        return False
    return abs(time.mktime(born) - wanted) <= skew


def worker_main(payload: dict) -> None:
    """The worker's body: take a slot, run the work function, write the result.

    Runs in the detached grandchild process (job_worker.py forked its
    intermediate away). Never returns; exits when the work is written.
    """
    root = Path(payload["root"])
    key = payload["key"]
    store = store_of(root)
    record_path = store / f"{key}.json"
    log_path = store / f"{key}.log"
    lock_fd = payload.get("lock_fd")
    if isinstance(lock_fd, int) and lock_fd >= 0:
        # pass_fds made the lock inheritable against PEP 446's default; mark it
        # non-inheritable so no child of this worker can hold the job lock.
        os.set_inheritable(lock_fd, False)
    config = _load_config_or_none(root)
    limit = int(payload["limit_seconds"])
    guard: dict = {"pgid": None}
    cmd_lock_fd = _take_cmd_lock(store / f"{key}.cmd.lock")

    def on_term(signum, frame):  # noqa: ARG001 - signal handler signature
        if guard["pgid"]:
            try:
                os.killpg(guard["pgid"], signal.SIGKILL)
            except OSError:
                pass
        os._exit(1)

    def on_alarm(signum, frame):  # noqa: ARG001 - signal handler signature
        # The backstop (D9): the limit passed without a result, outside or
        # inside ctx.run. Kill the command's group, report the time limit
        # properly and leave a done record: never a silent, lost worker.
        pgid = guard["pgid"]
        if pgid:
            try:
                os.killpg(pgid, signal.SIGKILL)
            except OSError:
                pass
        failure = _job_failure(payload["error_shape"], _limit_message(limit), log_path)
        try:
            _write_result(store, key, failure)
            current = _read_json(record_path) or {}
            current.update({"state": "done", "finished": _now(), "pid": os.getpid()})
            try:
                _write_record(store, key, current)
            except ValueError:
                _write_record(store, key, _minimal_record(current, "done"))
        except (OSError, ValueError):
            pass
        _emit(
            root,
            payload.get("session_id"),
            "job.done",
            id=key,
            command=payload["command"],
            detail="past the run-time limit",
        )
        os._exit(0)

    signal.signal(signal.SIGTERM, on_term)
    signal.signal(signal.SIGALRM, on_alarm)
    _take_slot(store, config)  # queue time is outside the run-time limit (D8)
    record = _read_json(record_path) or {}
    record.update({"state": "running", "started": _now(), "pid": os.getpid()})
    try:
        _write_record(store, key, record)
    except ValueError:
        _write_record(store, key, _minimal_record(record, "running"))
    deadline = time.monotonic() + limit
    margin = int(payload.get("alarm_margin", ALARM_MARGIN))
    signal.alarm(limit + margin)  # D9: the backstop behind ctx.run's own deadline

    def note_command(pgid: int) -> None:
        """Record the running command's process group and when it began, so a
        lost worker's orphaned group can be identified at the next attach."""
        current = _read_json(record_path) or {}
        current.update({"command_pgid": pgid, "command_started": _now()})
        try:
            _write_record(store, key, current)
        except (OSError, ValueError):
            pass  # the field is an optimisation, never worth failing the job

    ctx = JobContext(
        root=root,
        key=key,
        args=payload["args"],
        config=config,
        log=log_path,
        deadline=deadline,
        guard=guard,
        limit_seconds=limit,
        # A worker that runs without the cmd lock records no group: its lock
        # is what later authorises the kill (S3R-jobs-C1-1).
        on_command=note_command if cmd_lock_fd is not None else None,
        cmd_lock_fd=cmd_lock_fd,
    )
    try:
        returned = _resolve(payload["work"])(ctx)
        if not isinstance(returned, dict) or not isinstance(returned.get("ok"), bool):
            raise ValueError("the work function returned no object with a boolean ok")
        result = returned
    except JobTimeout as exc:
        result = _job_failure(payload["error_shape"], str(exc), log_path)
    except BaseException as exc:  # noqa: BLE001 - any crash is the job's failure
        try:
            _append_line(log_path, traceback.format_exc())
        except OSError:
            pass
        result = _job_failure(
            payload["error_shape"], f"the job crashed: {type(exc).__name__}: {exc}", log_path
        )
    guard["pgid"] = None
    _write_result(store, key, result)
    record = _read_json(record_path) or {}
    record.update({"state": "done", "finished": _now(), "pid": os.getpid()})
    try:
        _write_record(store, key, record)
    except ValueError:
        _write_record(store, key, _minimal_record(record, "done"))
    _emit(
        root,
        payload.get("session_id"),
        "job.done",
        id=key,
        command=payload["command"],
        detail="ok" if result.get("ok") else "failed",
    )


# ---------------------------------------------------------------------------
# The context a work function runs with


class JobContext:
    """The handles of a running job: what the work function receives."""

    def __init__(
        self,
        root: Path,
        key: str,
        args: dict,
        config: Config | None,
        log: Path,
        deadline: float,
        guard: dict,
        limit_seconds: int | None = None,
        on_command=None,
        cmd_lock_fd: int | None = None,
    ):
        self.root = root
        self.key = key
        self.args = args
        self.config = config
        self.log = log
        self._deadline = deadline
        self._guard = guard
        self._limit_seconds = limit_seconds
        self._on_command = on_command
        #: The per-key command lock, inherited by every command this context
        #: runs (D3, S3R-jobs-C1-1): while the flock lives, the recorded
        #: ``command_pgid`` is still this job's command.
        self._cmd_lock_fd = cmd_lock_fd

    def remaining(self) -> float:
        """Seconds of run-time limit left (queue time never counted, D8)."""
        return self._deadline - time.monotonic()

    def run(
        self,
        command: str,
        *,
        cwd: Path,
        env: dict[str, str] | None = None,
        output: Path | None = None,
    ) -> CommandResult:
        """Run ``command`` through ``/bin/sh`` in its own process group.

        Standard output and error go to ``output`` (truncated first) when
        given, else are appended to the job log; the log always gets one line
        naming the command and one with the exit code. At the deadline the
        group is signalled (TERM, 5 seconds, KILL) and ``JobTimeout`` is
        raised. An empty ``command`` is a ``ValueError``.
        """
        if not isinstance(command, str) or not command.strip():
            raise ValueError("the command is empty")
        started = time.monotonic()
        limit = max(self._deadline - started, 0.0)
        full_env = command_env(self.root, self.config)
        if env:
            full_env.update(env)
        target = Path(output) if output is not None else self.log
        if output is not None:
            _truncate(target)  # the output file is truncated first
        _append_line(self.log, f"$ {command}")
        passed = (self._cmd_lock_fd,) if self._cmd_lock_fd is not None else ()
        process = subprocess.Popen(
            ["/bin/sh", "-c", command],
            cwd=Path(cwd),
            stdin=subprocess.DEVNULL,
            stdout=open(target, "ab"),  # noqa: SIM115 - closed in the finally
            stderr=subprocess.STDOUT,
            env=full_env,
            start_new_session=True,
            pass_fds=passed,
        )
        self._guard["pgid"] = process.pid
        if self._on_command is not None:
            try:
                self._on_command(process.pid)
            except OSError:
                pass
        timed_out = False
        try:
            while True:
                code = process.poll()
                if code is not None:
                    break
                left = self.remaining()
                if left <= 0:
                    timed_out = True
                    self._signal_group(process.pid, signal.SIGTERM)
                    for _ in range(50):  # five seconds of grace, then KILL
                        if process.poll() is not None:
                            break
                        time.sleep(0.1)
                    if process.poll() is None:
                        self._signal_group(process.pid, signal.SIGKILL)
                        process.wait()
                    break
                time.sleep(min(POLL, max(left, 0.01)))
        finally:
            self._guard["pgid"] = None
            handle = process.stdout
            if handle is not None:
                handle.close()
        seconds = time.monotonic() - started
        code = process.returncode if process.returncode is not None else -1
        _append_line(self.log, f"# exit {code} ({seconds:.1f}s)" + (" at the limit" if timed_out else ""))
        if timed_out:
            # The message names the job's limit, not the seconds that happened
            # to be left when this command started.
            reported = int(self._limit_seconds) if self._limit_seconds is not None else int(limit)
            raise JobTimeout(_limit_message(reported))
        return CommandResult(code, seconds, target)

    @staticmethod
    def _signal_group(pid: int, sig: int) -> None:
        try:
            os.killpg(pid, sig)
        except OSError:
            pass  # a process that is already gone is fine


def run_shell(ctx: JobContext) -> dict:
    """The built-in work function (``vsdd.gates.jobs:run_shell``): run one command."""
    command = ctx.args["command"]
    result = ctx.run(command, cwd=Path(ctx.args["cwd"]))
    return {"ok": True, "returncode": result.returncode, "seconds": result.seconds}


def run_sleep(ctx: JobContext) -> dict:
    """Sleep ``ctx.args["seconds"]`` (``vsdd.gates.jobs:run_sleep``).

    A work function that never calls ``ctx.run``, so the tests can hold a job
    past its limit in Python and exercise the worker's alarm backstop (D9)
    on an overrun outside ``ctx.run``.
    """
    seconds = float(ctx.args["seconds"])
    time.sleep(seconds)
    return {"ok": True, "slept": seconds}


def command_env(root: Path, config: Config | None = None) -> dict[str, str]:
    """The environment configured commands run in (D12): the person's, scrubbed.

    Removes the tool's Python and uv variables, every ``VSDD_*`` name,
    ``VIRTUAL_ENV`` when it is this interpreter's, and this interpreter's
    ``bin`` from ``PATH``; then adds ``[commands.env]``.
    """
    env = {
        name: value
        for name, value in os.environ.items()
        if name not in _SCRUBBED_VARS and not name.startswith("VSDD_")
    }
    if env.get("VIRTUAL_ENV") == sys.prefix:
        del env["VIRTUAL_ENV"]
    own_bin = os.path.normpath(str(Path(sys.prefix) / "bin"))
    env["PATH"] = os.pathsep.join(
        part
        for part in env.get("PATH", "").split(os.pathsep)
        if part and os.path.normpath(part) != own_bin
    )
    if config is None:
        config = _load_config_or_none(root)
    if config is not None:
        table = config.get("commands").get("env") or {}
        env.update({str(k): str(v) for k, v in table.items()})
    return env


# ---------------------------------------------------------------------------
# attach: the one entry the commands use


def attach(root: object, spec: JobSpec, *, wait: int | None = None, retry: str) -> Outcome:
    """Start the job for ``spec``'s key, or re-attach to the one running.

    Returns the work function's result, a failure the job layer made, or the
    pending answer ``{"ok": True, "state": "pending", "retry": retry}``. Never
    raises for a failed job. Raises ``ValueError`` for a malformed spec or a
    negative wait (nothing written); ``OSError`` from an unwritable store
    passes up.
    """
    root = root_of(root)
    wait = clamp_wait(wait)  # ValueError on a negative wait
    problem = _spec_problem(spec)
    if problem is not None:
        raise ValueError(problem)
    key = job_key(spec.command, spec.subject, spec.commit)
    limit = _resolve_limit(root, spec)
    probe = _job_record(key, spec, _new_nonce(), 1, limit)
    probe_len = len(json.dumps(probe, ensure_ascii=False, sort_keys=True).encode("utf-8"))
    if probe_len > RECORD_CAP - RECORD_HEADROOM:
        # Measured with the later writes in mind (S3R-jobs-C1-3): the worker's
        # running and done records are fatter than the queued one, and a
        # refused spec writes nothing at all. The message names the bar that
        # was applied, not the cap (S3R-jobs-C2-2).
        raise ValueError(
            f"the record would be {probe_len} bytes, over the "
            f"{RECORD_CAP - RECORD_HEADROOM}-byte limit "
            f"(the {RECORD_CAP}-byte cap less {RECORD_HEADROOM} bytes kept for later writes)"
        )
    # Nothing above may touch the store: a refused spec writes nothing at all.
    store = store_of(root)
    store.mkdir(parents=True, exist_ok=True)
    record_path = store / f"{key}.json"
    result_path = store / f"{key}.result.json"
    lock_path = store / f"{key}.lock"
    log_path = store / f"{key}.log"
    prune(root)
    deadline = time.monotonic() + wait
    mine = None
    restarted = False
    while True:
        record = _read_json(record_path)
        result = _read_json(result_path)
        if record is not None and result is not None and _serves(record, mine):
            record["collected"] = True
            _write_record(store, key, record)
            return Outcome(False, result)
        fd = os.open(lock_path, os.O_RDWR | os.O_CREAT, 0o644)
        try:
            if _acquire(fd):
                # No worker is alive: re-check step 1 before touching anything.
                record = _read_json(record_path)
                result = _read_json(result_path)
                if record is not None and result is not None and _serves(record, mine):
                    record["collected"] = True
                    _write_record(store, key, record)
                    return Outcome(False, result)
                starts = 1
                loss = record is not None and result is None and record.get("state") in (
                    "queued",
                    "running",
                )
                if loss:
                    # The worker was lost (D6): log it, kill anything its command
                    # left running, restart once per call, then fail loudly with
                    # the command's own shape (D7).
                    _emit(
                        root,
                        spec.session_id,
                        "job.lost",
                        id=key,
                        command=spec.command,
                        detail=f"worker gone at state {record.get('state')}",
                    )
                    orphan = record.get("command_pgid")
                    if isinstance(orphan, int) and orphan > 0 and _lock_held(
                        store / f"{key}.cmd.lock"
                    ):
                        # The cmd lock is held only by the command's own process
                        # tree of the attempt that wrote this record: the worker
                        # opened the file fresh for that attempt and its commands
                        # inherited the descriptor (D3). The leader check keeps
                        # the group number itself honest: a number reused after
                        # the old group exited or a reboot names a leader born
                        # far from the record's time and is not signalled
                        # (S3R-jobs-C1-1).
                        begun = record.get("command_started") or record.get("started")
                        if _leader_begun_near(orphan, begun):
                            try:
                                os.killpg(orphan, signal.SIGKILL)
                            except OSError:
                                pass
                    prior = int(record.get("starts") or 1)
                    if restarted or prior >= 2:
                        # Once per call, and never more than twice for the job
                        # as a whole (D6): a job that dies every time is loud,
                        # whatever the length of one call.
                        failure = _job_failure(
                            spec.error_shape,
                            "the job lost its worker twice and kept dying",
                            log_path,
                        )
                        record.update({"state": "done", "finished": _now(), "collected": True})
                        _write_record(store, key, record)
                        _write_result(store, key, failure)
                        return Outcome(False, failure)
                    restarted = True
                    starts = prior + 1
                if result is not None:
                    result_path.unlink(missing_ok=True)
                nonce = _new_nonce()
                _truncate(log_path)
                _write_record(store, key, _job_record(key, spec, nonce, starts, limit))
                # A fresh cmd lock per attempt (S3R-jobs-C1-1): a descendant
                # that escaped an earlier command with setsid then holds only
                # an unlinked inode and can neither delay this attempt's
                # worker nor authorise a kill for it. Unlinked while this call
                # holds the job lock, so no live worker is disturbed.
                try:
                    (store / f"{key}.cmd.lock").unlink()
                except OSError:
                    pass
                _emit(
                    root,
                    spec.session_id,
                    "job.started",
                    id=key,
                    command=spec.command,
                    detail=f"attempt {starts}",
                )
                _spawn_worker(root, fd, key, spec, limit)
                mine = nonce
            elif record is not None and result is None:
                mine = record.get("nonce")  # a worker is alive; wait for its result
        finally:
            os.close(fd)  # the worker holds its own reference to the lock
        if time.monotonic() >= deadline:
            return Outcome(True, {"ok": True, "state": "pending", "retry": retry})
        time.sleep(POLL)


def _serves(record: dict, mine: str | None) -> bool:
    """A result is served while uncollected, and to the call that started it (D5).

    Only a record in ``state: done`` is served from: that write is the worker's
    last, so what it says of ``collected`` and ``nonce`` is final. Serving from
    an earlier snapshot would let a result come back after it was collected,
    or turn a done record back into a running one.
    """
    if record.get("state") != "done":
        return False
    return not record.get("collected") or record.get("nonce") == mine


# ---------------------------------------------------------------------------
# Listing and pruning


def _iter_records(store: Path):
    if not store.is_dir():
        return
    for path in sorted(store.glob("*.json")):
        if path.name.endswith(".result.json"):
            continue
        record = _read_json(path)
        if isinstance(record, dict) and _KEY_RE.fullmatch(str(record.get("key", ""))):
            yield record


_TEMP_RE = re.compile(r"^\.([0-9a-f]{16})\.(?:result\.)?json\.")


def _key_paths(store: Path) -> dict[str, list[Path]]:
    keys: dict[str, list[Path]] = {}
    if not store.is_dir():
        return keys
    for path in store.iterdir():
        if not path.is_file():
            continue
        name = path.name
        if name == "ports.lock":
            continue
        if name.startswith("."):
            # A temporary of _atomic_write: left when a kill fell between
            # mkstemp and os.replace. It belongs to its key, so prune sees it.
            match = _TEMP_RE.match(name)
            if match is not None:
                keys.setdefault(match.group(1), []).append(path)
            continue
        for suffix in (".result.json", ".settle.lock", ".cmd.lock", ".json", ".log", ".lock"):
            if name.endswith(suffix):
                key = name[: -len(suffix)]
                if _KEY_RE.fullmatch(key):
                    keys.setdefault(key, []).append(path)
                break
    return keys


def list_jobs(root: object) -> list[dict]:
    """Every job and product record, with ``alive`` and (jobs) ``has_result``.

    Never raises on a torn or foreign file; it skips it.
    """
    store = store_of(root)
    listed: list[dict] = []
    for record in _iter_records(store):
        entry = dict(record)
        key = str(record["key"])
        entry["alive"] = _lock_held(store / f"{key}.lock")
        if record.get("kind") == "job":
            entry["has_result"] = (store / f"{key}.result.json").is_file()
        listed.append(entry)
    return listed


def prune(root: object, *, days: int = RETAIN_DAYS) -> list[str]:
    """Delete every file of each lock-free key whose newest file is older than ``days``."""
    store = store_of(root)
    cutoff = time.time() - days * 86400
    removed: list[str] = []
    for key, paths in sorted(_key_paths(store).items()):
        lock_path = store / f"{key}.lock"
        if _lock_held(lock_path):
            continue
        newest = 0.0
        for path in paths:
            try:
                newest = max(newest, path.stat().st_mtime)
            except OSError:
                continue
        if newest and newest < cutoff:
            if not _unlink_lock_if_free(lock_path):
                continue  # a re-attach took the lock while we looked away
            for path in paths:
                if path == lock_path:
                    continue
                try:
                    path.unlink()
                except OSError:
                    pass
            removed.append(key)
    return removed


# ---------------------------------------------------------------------------
# Products


def _products(store: Path):
    for record in _iter_records(store):
        if record.get("kind") == "product":
            yield record


def stop_products(
    root: object,
    *,
    agent_id: str | None = None,
    session_id: str | None = None,
    subject: str | None = None,
    grace: float = 5.0,
) -> list[dict]:
    """Stop matching products; return ``{subject, port, pid}`` for each one stopped.

    A filter that is given must equal the record's field. A matching record
    whose lock is already free is deleted and not returned. One still in
    ``state: starting`` is stale when its settle lock is free - the start call
    that held it died mid-settle - and its product is stopped by the pid the
    start wrote right after forking it; while the settle lock is held, a live
    start call is still settling and is left alone. A product whose lock is
    still held after the KILL keeps its record and is not reported, so its
    port stays taken. The lock file is removed only after it has been taken,
    so a live holder is never made invisible. Never raises for a process
    that is gone.
    """
    root = root_of(root)
    store = store_of(root)
    stopped: list[dict] = []
    for record in list(_products(store)):
        if agent_id is not None and record.get("agent_id") != agent_id:
            continue
        if session_id is not None and record.get("session_id") != session_id:
            continue
        if subject is not None and record.get("subject") != subject:
            continue
        key = str(record["key"])
        lock_path = store / f"{key}.lock"
        if not _lock_held(lock_path):
            _delete_key(store, key)
            continue
        if record.get("state") == "starting":
            # The settle lock is held by the live start call and is never
            # passed to the product: held, a start call is still settling and
            # cleans up after itself. Free, the start call died mid-settle -
            # the record is stale and its product is stopped by pid below.
            if _lock_held(store / f"{key}.settle.lock"):
                continue
        pid = record.get("pid")
        if isinstance(pid, int) and pid > 0:
            _terminate(pid, grace, lock_path)
        if _lock_held(lock_path):
            # Something in the product's chain outlived the KILL (it escaped
            # its process group): its record stays and it is not reported
            # stopped, or its port would be handed out while it listens.
            continue
        _delete_key(store, key)
        stopped.append(
            {"subject": record.get("subject"), "port": record.get("port"), "pid": pid}
        )
        _emit(
            root,
            record.get("session_id"),
            "env.stopped",
            id=key,
            command=record.get("command"),
            detail=f"stopped {record.get('subject')}",
        )
    return stopped


def _terminate(pid: int, grace: float, lock_path: Path) -> None:
    """TERM the process group, wait up to ``grace`` for the lock, then KILL."""
    try:
        os.killpg(pid, signal.SIGTERM)
    except (OSError, ProcessLookupError):
        pass
    deadline = time.monotonic() + max(grace, 0.0)
    while time.monotonic() < deadline:
        if not _lock_held(lock_path):
            return
        time.sleep(min(0.1, max(deadline - time.monotonic(), 0.01)))
    try:
        os.killpg(pid, signal.SIGKILL)
    except (OSError, ProcessLookupError):
        pass
    time.sleep(0.05)


def _delete_key(store: Path, key: str) -> None:
    """Remove a key's files; the lock file goes only when nobody holds it.

    Unlinking a held lock hides a live product or worker and frees its port
    for a second start, so the lock is taken before it is removed.
    """
    for suffix in (".json", ".log", ".result.json"):
        path = store / f"{key}{suffix}"
        try:
            path.unlink()
        except OSError:
            pass
    for lock in (".lock", ".settle.lock", ".cmd.lock"):
        _unlink_lock_if_free(store / f"{key}{lock}")


def _unlink_lock_if_free(path: Path) -> bool:
    """Unlink a lock file only while this call holds it; False when held elsewhere."""
    try:
        fd = os.open(path, os.O_RDWR)
    except FileNotFoundError:
        return True
    except OSError:
        return False
    try:
        if not _acquire(fd):
            return False
        try:
            path.unlink()
        except OSError:
            pass
        return True
    finally:
        os.close(fd)  # closing releases the flock taken above


def product_ports(root: object, *, session_id: str | None = None) -> set[int]:
    """The ports of live products (those whose lock is held)."""
    store = store_of(root)
    ports: set[int] = set()
    for record in _products(store):
        if session_id is not None and record.get("session_id") != session_id:
            continue
        if not _lock_held(store / f"{record['key']}.lock"):
            continue
        port = record.get("port")
        if isinstance(port, int):
            ports.add(port)
    return ports
