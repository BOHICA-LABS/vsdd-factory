"""The ``vsdd survey`` command (state-core.md 3.1, rule 2 of ORDER.md: a
relayed command prints every field of its schema on every exit).

The body runs in :mod:`vsdd.state.survey`; this module holds the ``Command``
with its parser, checks the returned object against the shipped
``survey.schema.json``, and raises ``Refuse`` for a violation.
"""

from __future__ import annotations

from ..cli.dispatch import Command, Refuse, Reply
from .store import schema_of
from . import survey as _survey

_SUMMARY = "first step of every workflow: check preconditions, open a run, take the lock (story is built)"


def _configure(parser) -> None:
    parser.add_argument("workflow", help="the workflow to start ('story' is built)")
    parser.add_argument("--subject", default="", help="the unit id (a story id)")
    parser.add_argument("--repo", default=None, help="not built yet")
    parser.add_argument("--cycle", default=None, help="the cycle to run in; must be the one open cycle")
    parser.add_argument("--request-uri", dest="request_uri", default=None, help="not built yet")
    parser.add_argument("--guidance-uri", dest="guidance_uri", default=None, help="not built yet")
    parser.add_argument("--question-uri", dest="question_uri", default=None, help="not built yet")


def _run(inv, args) -> Reply:
    output, code = _survey.run_survey(inv, args)
    problems = _violations(output)
    if problems:
        raise Refuse("the survey object fails its schema: " + "; ".join(problems))
    return Reply.of(output, code)


def _violations(output: dict) -> list[str]:
    from ..schema import validate
    from .store import _registry

    return [str(problem) for problem in validate(output, schema_of("survey"), _registry())[:5]]


COMMANDS = (
    Command(
        words=("survey",),
        summary=_SUMMARY,
        run=_run,
        needs="initialised",
        output="survey",
        failure={"next": {"kind": "human", "command": "", "args": {}}},
        configure=_configure,
    ),
)
