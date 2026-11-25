"""The green gate (commands/gate-green.md): measures build, full suite, lint,
format and unchanged frozen tests on a story's head commit and writes the
diff patch. One gate, one module: the records' registry imports
``gate_<name>.py`` by file name and calls what it registers here."""

from __future__ import annotations

from . import gatelib
from .gatelib import measure as _measure


def measure(project, subject: str, run: str, *, wait: int | None = None, retry: str = "",
            since: str = "") -> dict:
    """The green gate's measurement (plan gates.md 3.4): attaches to the
    gate's own job, never measuring in the caller's process. ``since`` names
    a commit that must be an ancestor of the head."""
    return _measure(project, subject, run, gate="green", wait=wait, retry=retry, since=since)


def work(ctx) -> dict:
    """The job's work (D1): the worker repeats the checks and measures."""
    return gatelib.work_context(ctx)


from ..state.gate_evidence import register_gate  # noqa: E402 - the registry

register_gate("green", measure)
