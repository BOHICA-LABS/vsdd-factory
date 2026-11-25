"""The red gate (commands/gate-red.md): measures that a story's new tests
fail on assertions and freezes their identities; ``--refreeze`` records
changed frozen tests. One gate, one module: the records' registry imports
``gate_<name>.py`` by file name and calls what it registers here."""

from __future__ import annotations

from . import gatelib
from .gatelib import measure as _measure


def measure(project, subject: str, run: str, *, wait: int | None = None, retry: str = "",
            refreeze: bool = False) -> dict:
    """The red gate's measurement (plan gates.md 3.4): attaches to the gate's
    own job, never measuring in the caller's process. ``wait=0`` answers
    pending at once; ``None`` is the default 100 seconds. Raises for nothing
    a caller can cause: a refusal is an ``ok: false`` result. Red measures
    the head only: red evidence lost after implementation commits returns
    ``passed: false``, never a pass."""
    return _measure(project, subject, run, gate="red", wait=wait, retry=retry,
                    refreeze=refreeze)


def work(ctx) -> dict:
    """The job's work (D1): the worker repeats the checks and measures."""
    return gatelib.work_context(ctx)


from ..state.gate_evidence import register_gate  # noqa: E402 - the registry

register_gate("red", measure)
