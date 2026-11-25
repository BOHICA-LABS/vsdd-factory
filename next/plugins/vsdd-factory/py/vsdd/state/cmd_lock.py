"""``vsdd lock release`` (commands/lock-release.md; plan records.md 3).

Releases the run lock: the entries of the caller's own session and the stale
ones; exits 1 with the holder when another session's live entry remains.
``--force`` releases any lock and is person only. Releasing ends the run that
held the lock (topics/run-lock.md). While ``flush.lock`` is busy the release
cannot end the runs: it exits 1 naming a kept entry, with an error that says
a flush is running — never a silent success (finding S3R-records-C2-2). When
instead the run lock ``lock.flock`` itself is held — by another release or a
survey — the error says so, not that a flush is running (finding
step3-records-fix-3-C1-3).
"""

from __future__ import annotations

import argparse

from ..cli.dispatch import Command, Reply
from .callers import caller
from .locks import lock_entries, lock_release
from .store import StoreBusy

_SUMMARY = "release the run lock (stale entries and this session's; --force for any)"

_FLUSH_BUSY_ERROR = (
    "a flush is running; the release could not end the runs of the entries it matched — "
    "retry when the flush finishes"
)

_LOCK_BUSY_ERROR = (
    "the run lock is busy; another release or a survey holds it — retry shortly"
)


def _run(inv, args: argparse.Namespace) -> Reply:
    try:
        return _release(inv, args)
    except StoreBusy as busy:
        # The busy-run_end StoreBusy carries the kept entries; a bare
        # lock.flock timeout (the release never reached the entries) does not.
        kept = getattr(busy, "kept", None)
        if isinstance(kept, list) and kept:
            released = getattr(busy, "released", [])
            return Reply(
                ok=False,
                data={
                    "released": [_names(entry) for entry in released],
                    # A kept entry holds the lock: name it, never null.
                    "holder": _names(kept[0]),
                    "error": _FLUSH_BUSY_ERROR,
                },
            )
        return Reply(
            ok=False,
            data={"released": [], "holder": None, "error": _LOCK_BUSY_ERROR},
        )


def _release(inv, args: argparse.Namespace) -> Reply:
    if getattr(args, "force", False):
        inv.require_person("lock release --force")
        released = lock_release(inv.root, force=True)
        return Reply(
            data={"released": [_names(entry) for entry in released], "holder": None, "error": ""}
        )
    session = ""
    if inv.caller_key:
        marker = caller(inv.caller_key, project=inv.root)
        if marker is not None:
            session = marker.get("session_id", "")
    released = lock_release(inv.root, session=session or None, stale=True)
    remaining = lock_entries(inv.root)
    data = {
        "released": [_names(entry) for entry in released],
        "holder": remaining[0] if remaining else None,
        "error": "",
    }
    if remaining:
        data["error"] = (
            "another session's live run holds the lock; "
            "vsdd lock release --force releases it when you know that session is gone"
        )
        return Reply(ok=False, data=data)
    return Reply(data=data)


def _names(entry: dict) -> dict:
    return {
        "scope": entry.get("scope", ""),
        "run": entry.get("run", ""),
        "workflow": entry.get("workflow", ""),
        "subject": entry.get("subject", ""),
        "session": entry.get("session", ""),
    }


COMMANDS = (
    Command(
        words=("lock", "release"),
        summary=_SUMMARY,
        run=_run,
        needs="initialised",
        configure=lambda parser: parser.add_argument(
            "--force", action="store_true", help="release any lock; person only"
        ),
    ),
)
