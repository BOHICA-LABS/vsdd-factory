"""Lock liveness, the seam with the record stores (plan hook-runner-b 3.2;
[[topic:run-lock]]).

Two core duties, wired with no rule as the marker duty is (HD2; D1):

- :func:`refresh` runs on every ``PreToolUse`` call the runner handles and
  passes the call's facts to ``vsdd.state.locks.lock_seen``, which decides
  whether the call belongs to a locked run; the runner only passes facts.
- :func:`release` runs on ``SessionEnd``, whatever its reason (D3), and gives
  the session's entries to ``vsdd.state.locks.lock_release``.

Liveness never blocks. It is not a gate, and a fault in a lock file must not
wedge every tool call of a session; it is not silent either — an exception
from either duty, an import error included, is caught and logged as
``runner.liveness_error`` with severity ``error``, and the call goes on (D5,
D6).

This module imports ``vsdd.state.locks`` directly, not the ``vsdd.state``
facade ([[topic:state-api]]); the import stands inside the duty it serves, so
a fault there is a logged event and not a dead runner.
"""

from __future__ import annotations

import os

from .model import (
    EVENT_LOCK_RELEASED,
    EVENT_RUNNER_LIVENESS_ERROR,
    Ctx,
)

__all__ = ["EVENT_LOCK_RELEASED", "EVENT_RUNNER_LIVENESS_ERROR", "refresh", "release"]

#: The wait that keeps the release inside HD4's 1 s ``SessionEnd`` deadline,
#: itself inside the platform's 1.5 s (D4).
RELEASE_WAIT = 0.5


def refresh(ctx: Ctx) -> bool:
    """One ``PreToolUse`` call's refresh: ``True`` when the call belongs to a
    locked run and its holder was refreshed. Never raises; never changes the
    call."""
    try:
        return _refresh(ctx)
    except Exception as exc:
        _fault(ctx, exc)
        return False


def _refresh(ctx: Ctx) -> bool:
    from ..state import locks

    project = ctx.project
    if project is None:
        return False
    return locks.lock_seen(
        project,
        session_id=ctx.session_id,
        agent_id=ctx.agent_id,
        run=run_of(ctx),
        story=_story_of(ctx),
    )


def _is_vsdd_program(program: str) -> bool:
    """A ``vsdd`` call however it is spelled: the bare word or a path to it,
    as :func:`vsdd.hooks.caller.is_vsdd_call` reads the program of the one
    segment it approves."""
    return os.path.basename(program) == "vsdd"


def _story_of(ctx: Ctx) -> str:
    """The story the call works in, ``""`` when it works in none. A ``Bash``
    command has one effective directory per segment (rule-model 2.5), so the
    first segment whose directory sits in a story worktree decides —
    ``cd .worktrees/S-3.01 && …`` run from the project root counts
    (S3R-hook-runner-b-10). Anything else takes the story of ``where``, which
    is the payload ``cwd``."""
    if ctx.tool == "Bash":
        command = ctx.command
        if command is not None:
            for segment in command.segments:
                story = ctx.story_at(segment.where)
                if story:
                    return story
            return ""
    return ctx.story or ""


def run_of(ctx: Ctx) -> str:
    """The run id the call names, ``""`` when it names none: the value after
    ``--run`` (or in ``--run=<id>``) of a segment whose program is ``vsdd``,
    or ``tool_input.step.run`` of a ``StructuredOutput`` call."""
    if ctx.tool == "StructuredOutput":
        step = ctx.tool_input.get("step")
        if isinstance(step, dict):
            run = step.get("run", "")
            return run if isinstance(run, str) else ""
        return ""
    if ctx.tool == "Bash":
        command = ctx.command
        if command is not None:
            for segment in command.segments:
                if _is_vsdd_program(segment.program):
                    run = _run_argument(segment.args)
                    if run:
                        return run
    return ""


def _run_argument(args: tuple[str, ...]) -> str:
    for index, argument in enumerate(args):
        if argument == "--run":
            return args[index + 1] if index + 1 < len(args) else ""
        if argument.startswith("--run="):
            return argument[len("--run=") :]
    return ""


def release(ctx: Ctx) -> list[dict]:
    """One ``SessionEnd``'s release: the entries the session held, each
    logged. Never raises and never blocks the session's end (D3, D4). A
    ``StoreBusy`` — the store signalling it could not finish — is logged with
    whatever entries it did release, and nothing is released by other means."""
    try:
        return _release(ctx)
    except Exception as exc:
        _fault(ctx, exc)
        return []


def _release(ctx: Ctx) -> list[dict]:
    from ..state.locks import lock_release
    from ..state.store import StoreBusy

    project = ctx.project
    session = ctx.session_id
    if project is None or not session:
        return []
    reason = str(ctx.raw.get("reason", "") or "")
    try:
        released = lock_release(project, session=session, wait=RELEASE_WAIT)
    except StoreBusy as busy:
        released = _released_under_busy(busy)
        _log_released(ctx, released, reason)
        _fault(ctx, busy)
        return released
    _log_released(ctx, released, reason)
    return released


def _released_under_busy(busy: Exception) -> list[dict]:
    """The entries a ``StoreBusy`` says it released, when the store carries
    them; an empty list when it does not (then nothing was released). The
    attribute is ``released``, as :func:`vsdd.state.locks.lock_release` sets
    it (finding S3R-records-C2-2)."""
    entries = getattr(busy, "released", None)
    if not isinstance(entries, (list, tuple)):
        return []
    return [entry for entry in entries if isinstance(entry, dict)]


def _log_released(ctx: Ctx, released: list[dict], reason: str) -> None:
    for entry in released:
        ctx.log(EVENT_LOCK_RELEASED, "info", run=str(entry.get("run", "") or ""), reason=reason)


def _fault(ctx: Ctx, exc: Exception) -> None:
    """One liveness fault, said out loud; the log itself must not be able to
    take the duty down with it."""
    try:
        ctx.log(
            EVENT_RUNNER_LIVENESS_ERROR,
            "error",
            error=f"{type(exc).__name__}: {exc}",
            mode=ctx.mode,
        )
    except Exception:
        pass
