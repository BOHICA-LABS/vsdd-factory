"""The handlers of ``vsdd env start`` and ``vsdd env stop`` (package G1).

Each handler returns ``(output: dict, code: int)`` with code 0 or 1 (D18); the
command module (``cmd_env.py``, built with the second part of this package)
wraps them in ``Command`` objects. A started product is a job record of kind
``product``, keyed by subject (D14) with the worktree's head commit in the
record: a second start at the same commit returns the first, at a new commit
replaces it. Nothing leaks and no demo runs old code.

The port is chosen without a socket (D15): the lowest in 28100 to 28199 that
no live product record holds, chosen under ``ports.lock``. A product that
exits within ``SETTLE_SECONDS`` is retried on the next port, three in all, so
``ok: true`` means started, not yet listening.

Without a caller marker a person's terminal is assumed: the product is
recorded with null ids, and ``env stop`` stops every product only from a
terminal (D16) - from a tool call it stops nothing and exits 1, naming the
single-segment form that works.
"""

from __future__ import annotations

import fcntl
import json
import os
import re
import subprocess
import time
from pathlib import Path

from ..project import classify_worktree
from .jobs import (
    RECORD_CAP,
    RECORD_HEADROOM,
    _acquire,
    _append_line,
    _delete_key,
    _emit,
    _group_alive,
    _load_config_or_none,
    _lock_held,
    _minimal_record,
    _new_nonce,
    _now,
    _read_json,
    _truncate,
    _write_record,
    command_env,
    job_key,
    prune,
    root_of,
    stop_products,
    store_of,
)

__all__ = ["SETTLE_SECONDS", "start", "stop"]

#: A product that exits within this window (a taken port, a broken command)
#: is retried on the next port, three in all (D15).
SETTLE_SECONDS = 2.0

_PORT_LOW = 28100
_PORT_HIGH = 28199
_SUBJECT_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$")
_START_ATTEMPTS = 3
_SETTLE_POLL = 0.05
_STARTING_WAIT = 5.0


def _worktree_dir(root: Path, subject: str) -> Path:
    """Where the product runs (D17): the story worktree, else the integration one.

    Only a story id runs in its own worktree; anything else, a named spike
    included, runs in the integration worktree.
    """
    worktree = classify_worktree(subject)
    if worktree is None or worktree.kind != "story":
        worktree = classify_worktree("_integration")
    return root / worktree.path


def _head(worktree: Path) -> str:
    """The worktree's head commit; empty when git cannot tell (D17)."""
    try:
        result = subprocess.run(
            ["git", "-C", str(worktree), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return ""
    if result.returncode != 0:
        return ""
    return result.stdout.strip()


def _pick_port(store: Path, exclude: set[int]) -> int | None:
    """The lowest port in 28100-28199 neither a live product nor ``exclude`` holds.

    The caller holds ``ports.lock`` around the scan and the record write that
    claims the port (D15); this function only scans.
    """
    taken: set[int] = set(exclude)
    for path in store.glob("*.json"):
        if path.name.endswith(".result.json"):
            continue
        record = _read_json(path)
        if (
            isinstance(record, dict)
            and record.get("kind") == "product"
            and isinstance(record.get("port"), int)
            and _lock_held(store / f"{record.get('key')}.lock")
        ):
            taken.add(record["port"])
    for port in range(_PORT_LOW, _PORT_HIGH + 1):
        if port not in taken:
            return port
    return None


def _answer(record: dict) -> dict:
    return {
        "ok": True,
        "url": record.get("url"),
        "port": record.get("port"),
        "pid": record.get("pid"),
    }


def _unlink(path: Path) -> None:
    try:
        path.unlink()
    except OSError:
        pass


def _log_tail(path: Path, lines: int = 5) -> str:
    try:
        return " | ".join(path.read_text(encoding="utf-8").splitlines()[-lines:])
    except OSError:
        return ""


def _product_record(
    subject: str, caller, port: int, worktree: Path, head: str, run_command: str, key: str
) -> dict:
    ids = caller or {}
    return {
        "v": 1,
        "key": key,
        "kind": "product",
        "command": run_command,
        "subject": subject,
        "commit": head,
        "state": "starting",
        "created": _now(),
        "started": None,
        "finished": None,
        "pid": None,
        "nonce": _new_nonce(),
        "session_id": ids.get("session_id"),
        "agent_id": ids.get("agent_id"),
        "port": port,
        "url": f"http://127.0.0.1:{port}",
        "cwd": str(worktree),
    }


def _launch(
    root: Path,
    key: str,
    subject: str,
    caller,
    worktree: Path,
    head: str,
    lock_fd: int,
) -> tuple[dict, int]:
    """Start the product, holding the product lock; hand the lock to it (D3).

    This call also holds the per-key settle lock, which is never passed to
    the product (S3R-jobs-C1-2): while it is held, a ``starting`` record
    means a live start call; once this call is gone, a leftover ``starting``
    record is stale, and the next start or stop stops the product by the pid
    the locked body writes right after forking it.
    """
    store = store_of(root)
    settle_fd = os.open(store / f"{key}.settle.lock", os.O_RDWR | os.O_CREAT, 0o644)
    try:
        fcntl.flock(settle_fd, fcntl.LOCK_EX)  # this start call alone; released below
        return _launch_locked(root, key, subject, caller, worktree, head, lock_fd)
    finally:
        os.close(settle_fd)


def _launch_locked(
    root: Path,
    key: str,
    subject: str,
    caller,
    worktree: Path,
    head: str,
    lock_fd: int,
) -> tuple[dict, int]:
    store = store_of(root)
    log_path = store / f"{key}.log"
    config = _load_config_or_none(root)
    run_command = config.command("run") if config is not None else ""
    if not run_command:
        return {
            "ok": False,
            "error": "commands.run is empty: set it in factory.toml",
            "log": str(log_path),
        }, 1
    tried: set[int] = set()
    last_code = None
    for _attempt in range(_START_ATTEMPTS):
        # ports.lock is held from the scan until the record naming the port is
        # written: a concurrent start scanning under the lock then sees this
        # claim and cannot pick the same port (D15).
        ports_fd = os.open(store / "ports.lock", os.O_RDWR | os.O_CREAT, 0o644)
        try:
            deadline = time.monotonic() + _STARTING_WAIT
            while not _acquire(ports_fd):
                if time.monotonic() > deadline:
                    return {
                        "ok": False,
                        "error": "another start is choosing a port; ports.lock stayed held",
                        "log": str(log_path),
                    }, 1
                time.sleep(0.05)
            port = _pick_port(store, tried)
            if port is None:
                _delete_key(store, key)
                return {"ok": False, "error": "no port is free in 28100-28199", "log": str(log_path)}, 1
            tried.add(port)
            record = _product_record(subject, caller, port, worktree, head, run_command, key)
            size = len(json.dumps(record, ensure_ascii=False, sort_keys=True).encode("utf-8"))
            # Measured with the running write in mind (S3R-jobs-C1-3): it adds
            # started and the pid to the starting record. The message names the
            # bar that was applied, not the cap (S3R-jobs-C2-2).
            if size > RECORD_CAP - RECORD_HEADROOM:
                _delete_key(store, key)
                return {
                    "ok": False,
                    "error": (
                        f"the product record would be {size} bytes, over the "
                        f"{RECORD_CAP - RECORD_HEADROOM}-byte limit "
                        f"(the {RECORD_CAP}-byte cap less {RECORD_HEADROOM} bytes kept for later writes)"
                    ),
                    "log": str(log_path),
                }, 1
            _truncate(log_path)
            _write_record(store, key, record)
        finally:
            os.close(ports_fd)  # the record naming the port is written
        command = run_command.replace("{port}", str(port))
        env = command_env(root, config)
        env["VSDD_PORT"] = str(port)
        with open(log_path, "ab") as log:
            process = subprocess.Popen(  # noqa: S603 - the configured command, by design
                ["/bin/sh", "-c", command],
                cwd=worktree,
                stdin=subprocess.DEVNULL,
                stdout=log,
                stderr=subprocess.STDOUT,
                env=env,
                start_new_session=True,
                pass_fds=(lock_fd,),
            )
        # The pid is written before settling, so a start call killed mid-settle
        # leaves a record its product can be stopped by (S3R-jobs-C1-2).
        record.update({"pid": process.pid})
        try:
            _write_record(store, key, record)
        except ValueError:
            _write_record(store, key, _minimal_record(record, "starting"))
        settle_until = time.monotonic() + float(SETTLE_SECONDS)
        code = None
        while time.monotonic() < settle_until:
            code = process.poll()
            if code is not None:
                break
            time.sleep(_SETTLE_POLL)
        if code is None:
            record.update({"state": "running", "started": _now()})
            try:
                _write_record(store, key, record)
            except ValueError:
                _write_record(store, key, _minimal_record(record, "running"))
            _emit(
                root,
                record.get("session_id"),
                "env.started",
                id=key,
                command=run_command,
                detail=f"port {port}",
            )
            return _answer(record), 0
        last_code = code
        _append_line(log_path, f"# the run command exited with {code} on port {port}")
        # The lock file itself stays: this call still holds it for the next attempt,
        # and a product must never run with an unlinked lock under its feet.
        _unlink(store / f"{key}.json")
        _unlink(store / f"{key}.result.json")
    tail = _log_tail(log_path)
    # The record goes; the log stays: the error names it as the evidence, and a
    # later start truncates it when it writes its own.
    _unlink(store / f"{key}.json")
    _unlink(store / f"{key}.result.json")
    _unlink(store / f"{key}.lock")
    return {
        "ok": False,
        "error": f"the run command exited with {last_code} on three ports; log tail: {tail}",
        "log": str(log_path),
    }, 1


def _wait_for_running(
    root: Path, key: str, head: str, log_path: Path
) -> tuple[dict, int] | None:
    """Wait for the running record a start call in flight will write.

    The product lock is held over a record whose product's group is gone, or
    over no record at all yet, so a holder is between taking the lock and its
    product inheriting it. When the waited-for record arrives, it is answered
    at the same commit and its product replaced at another one. A ``starting``
    record whose settle lock has come free is a start call that died
    mid-settle: no running record will come, and the orphan product it left
    behind still holds the lock, so it is stopped by the pid the locked body
    wrote and ``None`` lets the caller take over (S3R-jobs-C1-2,
    S3R-jobs-fix-4-C1-1). When the product lock comes free before any of
    that - the holder died without writing - ``None`` sends the caller back
    to take the lock and start (S3R-jobs-fix-3-C2-1); when nothing arrives in
    time, the call gives up with the settling error.
    """
    store = store_of(root)
    deadline = time.monotonic() + _STARTING_WAIT
    while time.monotonic() < deadline:
        fresh = _read_json(store / f"{key}.json")
        if (
            isinstance(fresh, dict)
            and fresh.get("state") == "running"
            and _group_alive(fresh.get("pid"))
        ):
            if fresh.get("commit") == head:
                return _answer(fresh), 0
            stop_products(root, subject=fresh.get("subject"))
            return None
        if (
            isinstance(fresh, dict)
            and fresh.get("state") == "starting"
            and not _lock_held(store / f"{key}.settle.lock")
        ):
            # The settling start call died after writing the record: its
            # product still holds the lock, and it is stopped by the pid the
            # locked body wrote right after forking it (S3R-jobs-C1-2).
            stop_products(root, subject=fresh.get("subject"))
            return None
        if not _lock_held(store / f"{key}.lock"):
            # The holder is gone without writing the record: the lock is free
            # and nothing waits behind it (S3R-jobs-fix-3-C2-1).
            return None
        time.sleep(0.2)
    return {
        "ok": False,
        "error": "another start of this subject is still settling",
        "log": str(log_path),
    }, 1


def _held_answer(
    root: Path, key: str, record: dict | None, head: str, log_path: Path
) -> tuple[dict, int] | None:
    """Answer a start while somebody else holds the product lock.

    Decides on records read after the lock probe, never on the snapshot the
    caller read before it (S3R-jobs-fix-3-C1-2). Returns ``(output, code)``
    when the call is answered from the held state, or ``None`` when the lock
    is free again and this call must start.
    """
    store = store_of(root)
    state = record.get("state") if isinstance(record, dict) else None
    if state == "starting" and not _lock_held(store / f"{key}.settle.lock"):
        # The snapshot was read before this probe (S3R-jobs-C2-1): a live
        # start call writes 'running' before it releases the settle lock, so
        # only a record still 'starting' once the lock is found free is
        # stale. Decide on a fresh read, not the snapshot.
        fresh = _read_json(store / f"{key}.json")
        if isinstance(fresh, dict) and fresh.get("state") == "running":
            record, state = fresh, "running"
        else:
            # No start call is alive (its death freed the settle lock) and no
            # running record took its place: this record is a start call
            # killed mid-settle. Its product may still run with the lock
            # held; stop it by pid, clear the key, start over.
            stop_products(root, subject=record.get("subject"))
            return None
    if state == "running":
        # The snapshot was read before this probe, so the holder may have
        # replaced it since: answer or replace from a read after the probe
        # (S3R-jobs-fix-3-C1-2).
        fresh = _read_json(store / f"{key}.json")
        if isinstance(fresh, dict) and fresh.get("state") == "running":
            record = fresh
        if not _group_alive(record.get("pid")):
            # The lock is held over a record whose product's group is gone
            # (the pid is only the group leader, so the group is judged as
            # stop_products judges it): the holder is a start call that has
            # yet to write its own record. Wait for the running record it
            # will write - never answer from this one's dead pid and never
            # stop on it (D14).
            return _wait_for_running(root, key, head, log_path)
        if record.get("commit") == head:
            return _answer(record), 0
        # Held at another commit: replace the product, then start (D14).
        stop_products(root, subject=record.get("subject"))
        return None
    if state == "starting":
        # The settle lock is held: a live start call is still settling. Wait
        # for its record, deciding as _wait_for_running decides: the answer
        # comes only from a running record whose group lives, at this call's
        # commit - at another one the product is replaced (D14) - and a
        # settling call that dies is taken over from, not waited out
        # (S3R-jobs-fix-4-C1-1).
        return _wait_for_running(root, key, head, log_path)
    # A holder without a usable record: a start in flight that has taken the
    # lock but not yet written, or a holder about to exit without writing.
    # It is waited on like a start in flight instead of failing at once; a
    # lock held by a holder that escaped its group then fails after
    # _STARTING_WAIT, which is acceptable (S3R-jobs-fix-4-C1-2).
    return _wait_for_running(root, key, head, log_path)


def start(root: object, subject: str, caller) -> tuple[dict, int]:
    """Handle ``vsdd env start --subject <id>``; return ``(output, exit code)``.

    Raises ``ValueError`` for a subject outside the grammar, which the command
    module turns into exit 2.
    """
    root = root_of(root)
    if not isinstance(subject, str) or not _SUBJECT_RE.fullmatch(subject):
        raise ValueError(f"subject {subject!r} must match ^[A-Za-z0-9][A-Za-z0-9._-]{{0,63}}$")
    store = store_of(root)
    store.mkdir(parents=True, exist_ok=True)
    prune(root)
    worktree = _worktree_dir(root, subject)
    key = job_key("env", subject, "")
    record_path = store / f"{key}.json"
    lock_path = store / f"{key}.lock"
    log_path = store / f"{key}.log"
    if not worktree.is_dir():
        return {
            "ok": False,
            "error": f"the worktree {worktree} does not exist",
            "log": str(log_path),
        }, 1
    head = _head(worktree)
    fd = os.open(lock_path, os.O_RDWR | os.O_CREAT, 0o644)
    try:
        # The record is read after each probe, never before: a snapshot read
        # earlier than a failed probe may name a product the holder has
        # already replaced (S3R-jobs-fix-3-C1-2).
        if _acquire(fd):
            record = _read_json(record_path)
        else:
            record = _read_json(record_path)
            answered = _held_answer(root, key, record, head, log_path)
            if answered is not None:
                return answered
            os.close(fd)
            fd = os.open(lock_path, os.O_RDWR | os.O_CREAT, 0o644)
            if not _acquire(fd):
                return {
                    "ok": False,
                    "error": "the product lock stays held",
                    "log": str(log_path),
                }, 1
            record = _read_json(record_path)
        if record is not None and record.get("state") in ("running", "starting"):
            # Stale: its holder is gone. The lock file itself stays - this call
            # holds it, and the product must never run with an unlinked lock.
            _unlink(record_path)
            _unlink(store / f"{key}.result.json")
        return _launch(root, key, subject, caller, worktree, head, fd)
    finally:
        try:
            os.close(fd)
        except OSError:
            pass


def stop(root: object, caller, *, terminal: bool) -> tuple[dict, int]:
    """Handle ``vsdd env stop`` (no argument, D13); return ``(output, exit code)``.

    With a caller marker it stops that caller's products. Without one it stops
    every product only from a terminal; from a tool call it stops nothing and
    exits 1 (D16).
    """
    root = root_of(root)
    if caller is None:
        if not terminal:
            return {
                "ok": False,
                "stopped": [],
                "error": "no caller marker: only a terminal stops every product; "
                "run `vsdd -C <dir> env stop` as the whole command",
            }, 1
        stopped = stop_products(root)
    else:
        stopped = stop_products(
            root,
            agent_id=caller.get("agent_id"),
            session_id=caller.get("session_id"),
        )
    if not stopped:
        return {"ok": False, "stopped": [], "error": "no product was stopped"}, 1
    return {"ok": True, "stopped": stopped}, 0
