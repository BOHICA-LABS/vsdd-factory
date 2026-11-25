"""The ``vsdd gate red`` / ``gate green`` / ``gate demo`` commands (plan
gates.md 3.2, D1, D11).

The command is the cheap half of the gate: it parses and checks the caller
(``--refreeze`` is the test-writer's alone), resolves the story and the key,
and hands the work to :func:`vsdd.gates.gatelib.measure` — which attaches to
the gate's own job. Red and green are job commands; demo returns directly,
accepting ``--wait`` and ignoring it."""

from __future__ import annotations

from ..cli.dispatch import Command, Refuse, Reply
from . import gatelib
from .gate_demo import measure as _demo_measure

_SUMMARIES = {
    "red": "measure that the story's new tests fail on assertions; freeze them (--refreeze re-freezes)",
    "green": "measure build, suite, lint and format on the head; write the diff patch (--since narrows)",
    "demo": "record the story's demo evidence, or build the criterion-to-test table",
}

def _story_id(text: str) -> str:
    from ..project import classify_worktree

    if not text:
        raise Refuse("--story needs a story id")
    classified = classify_worktree(text)
    if classified is None or classified.kind != "story":
        raise Refuse(f"{text!r} is not a story id")
    return text


def _configure(gate: str):
    def configure(parser) -> None:
        parser.add_argument("--story", required=True, help="the story id (S-<n>.<n>)")
        parser.add_argument("--run", required=True, help="the run the caller works in")
        if gate == "red":
            parser.add_argument(
                "--refreeze",
                action="store_true",
                help="record changed frozen tests; the test-writer's call alone",
            )
        if gate == "green":
            parser.add_argument(
                "--since", default="", help="a commit between the base and the head"
            )
    return configure


def _run(gate: str):
    def run(inv, args) -> Reply:
        story = _story_id(getattr(args, "story", ""))
        marker = _marker_of(inv)
        session_id = marker.get("session_id") if marker else None
        result: dict | None = None
        if gate == "red" and getattr(args, "refreeze", False):
            refusal = _refreeze_refusal(inv, marker)
            if refusal:
                # Not attempted (plan gates.md 3.1): ok false, exit 1, no job.
                result = gatelib.not_attempted("red", story, refusal)
        if result is None:
            if gate == "demo":
                result = _demo_measure(inv.root, story, args.run)
            else:
                options: dict = {"wait": inv.wait, "retry": inv.retry_command(),
                                 "session_id": session_id}
                if gate == "red":
                    options["refreeze"] = bool(args.refreeze)
                if gate == "green":
                    options["since"] = getattr(args, "since", "") or ""
                result = gatelib.measure(inv.root, story, args.run, gate=gate, **options)
        result.pop("run", None)
        if result.get("state") != "pending":
            _validate(result)
        if result.get("state") == "pending" or (result.get("ok") and result.get("passed")):
            return Reply.of(result, 0)
        return Reply.of(result, 1)

    return run


def _marker_of(inv) -> dict | None:
    if not inv.caller_key:
        return None
    from ..state.callers import caller

    return caller(inv.caller_key, project=inv.root)


def _validate(result: dict) -> None:
    from ..schema import validate
    from ..state.store import _registry, schema_of

    problems = validate(result, schema_of("gate-result"), _registry())
    if problems:
        raise Refuse("the gate result fails its schema: "
                     + "; ".join(str(one) for one in problems[:5]))


def _refreeze_refusal(inv, marker: dict | None) -> str:
    """Why the caller may not re-freeze, or ``""`` (D11): only an agent whose
    ``walls.toml`` row holds ``refreeze`` — the test-writer. The marker
    exists only when the whole raw command text was one plain ``vsdd`` call,
    so a call inside ``cd x && ...`` arrives unmarked and is refused,
    naming that."""
    from ..policy import load as load_policy

    if marker is None:
        return ("gate red --refreeze needs a caller marker: it is honoured only as the whole "
                "single-segment command of a test-writer's own call "
                "(vsdd gate red --refreeze ...), never inside cd x && ... or another "
                "compound command")
    agent_type = marker.get("agent_type", "")
    row = load_policy("walls").get(agent_type)
    groups = (row or {}).get("vsdd", []) if isinstance(row, dict) else []
    if "refreeze" not in groups:
        return (f"the caller {agent_type or '(main session)'} may not re-freeze; "
                "only the test-writer may")
    return ""


COMMANDS = (
    Command(
        words=("gate", "red"),
        summary=_SUMMARIES["red"],
        run=_run("red"),
        configure=_configure("red"),
        needs="mounted",
        output="gate-result",
        failure={"gate": "red"},
    ),
    Command(
        words=("gate", "green"),
        summary=_SUMMARIES["green"],
        run=_run("green"),
        configure=_configure("green"),
        needs="mounted",
        output="gate-result",
        failure={"gate": "green"},
    ),
    Command(
        words=("gate", "demo"),
        summary=_SUMMARIES["demo"],
        run=_run("demo"),
        configure=_configure("demo"),
        needs="mounted",
        output="gate-result",
        failure={"gate": "demo"},
    ),
)
