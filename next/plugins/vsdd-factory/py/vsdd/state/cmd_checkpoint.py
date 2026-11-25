"""The ``vsdd checkpoint`` command (state-core.md 3.1, rule 2 of ORDER.md).

The body runs in :mod:`vsdd.state.checkpoint`; this module holds the
``Command`` with its parser, checks the returned object against the shipped
``checkpoint-result.schema.json``, and raises ``Refuse`` for a violation.
"""

from __future__ import annotations

from ..cli.dispatch import Command, Refuse, Reply
from .store import schema_of
from . import checkpoint as _checkpoint

_SUMMARY = "last step of every workflow: close what the evidence supports, release the lock, return next"


def _configure(parser) -> None:
    parser.add_argument("workflow", help="the workflow that ran ('story' is built)")
    parser.add_argument("--run", required=True, help="the run to close")
    parser.add_argument("--subject", default="", help="must equal the run record's (plan decision D1)")
    parser.add_argument("--outcome", default="", help="'failed' closes the run as failed; any status word")


def _run(inv, args) -> Reply:
    output, code = _checkpoint.run_checkpoint(inv, args)
    problems = _violations(output)
    if problems:
        raise Refuse("the checkpoint object fails its schema: " + "; ".join(problems))
    return Reply.of(output, code)


def _violations(output: dict) -> list[str]:
    from ..schema import validate
    from .store import _registry

    return [
        str(problem)
        for problem in validate(output, schema_of("checkpoint-result"), _registry())[:5]
    ]


COMMANDS = (
    Command(
        words=("checkpoint",),
        summary=_SUMMARY,
        run=_run,
        needs="initialised",
        output="checkpoint-result",
        failure={"status": "failed", "next": {"kind": "human", "command": "", "args": {}}},
        configure=_configure,
    ),
)
