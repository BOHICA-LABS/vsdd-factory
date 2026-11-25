"""``vsdd env start`` and ``vsdd env stop`` on the command line (G1, second
part; plan step-3/jobs-b.md).

The handlers live in ``env.py``; this module only puts them on the command
line through S1's dispatcher (state-core.md 3.1). The adapters hold no logic:
the caller is read once through the record stores' marker reader (records.md
rule 4 — nothing here reads ``os.environ``), every refusal that is an answer
is the handler's exit 1, and only a malformed argument is exit 2. ``--json``
and ``--wait`` are the dispatcher's frame; a command that is not a job accepts
``--wait`` and ignores it (topics/long-commands.md, P3SEAM-20).

``needs`` is ``initialised``, not ``mounted``: a product must be stoppable
when the state mount is broken, and neither command writes tracked state.
Neither command is relayed by a script, so neither sets ``output``.
"""

from __future__ import annotations

import argparse

from ..cli.dispatch import Command, Refuse, Reply
from . import env

_SUMMARY_START = "start the product for --subject <id> on its own port"
_SUMMARY_STOP = "stop the caller's products; a terminal without a marker stops them all"


def _configure_start(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--subject",
        required=True,
        metavar="<id>",
        help="the subject the product serves (a story id, or the integration worktree)",
    )


def _configure_stop(parser: argparse.ArgumentParser) -> None:
    """``env stop`` takes no argument (D13): the caller selects what stops."""
    del parser  # the parser gains nothing; any option after the words is exit 2


def _caller_of(inv) -> dict | None:
    """The caller marker as a mapping, or ``None`` without one.

    Read through the record stores (records.md rule 4), once per run; the
    read deletes the marker, so a second command of the same session writes
    and reads its own.
    """
    from ..state.callers import caller

    return caller(inv.caller_key, project=inv.root)


def _run_start(inv, args: argparse.Namespace) -> Reply:
    who = _caller_of(inv)
    try:
        return Reply.of(*env.start(inv.root, args.subject, who))
    except ValueError as error:  # a malformed subject: a usage error, exit 2
        raise Refuse(str(error)) from error


def _run_stop(inv, args: argparse.Namespace) -> Reply:
    del args  # no argument (D13)
    who = _caller_of(inv)
    # A set VSDD_CALLER is the hook's rewrite: the call is a tool call wherever
    # its marker can be read or not (pruned, looked for under another -C root,
    # already read), so it never takes the stop-everything branch (refinement
    # of D3; the same fact require_person reads).
    terminal = inv.stdin_tty and inv.caller_key is None
    return Reply.of(*env.stop(inv.root, who, terminal=terminal))


COMMANDS = (
    Command(
        words=("env", "start"),
        summary=_SUMMARY_START,
        run=_run_start,
        configure=_configure_start,
        needs="initialised",
    ),
    Command(
        words=("env", "stop"),
        summary=_SUMMARY_STOP,
        run=_run_stop,
        configure=_configure_stop,
        needs="initialised",
    ),
)
