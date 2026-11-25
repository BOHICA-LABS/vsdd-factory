"""``vsdd decide`` (commands/decide.md; plan records.md section 3).

``vsdd decide <id> <choice> [key=value ...] [-- note]`` — parsed from the raw
tokens by the one grammar. Settles by writing the decision record and nothing
else; exit 0 settled, 1 refused by ``settle_decision``, 2 for bad grammar and
for a caller that is not a person. It logs ``decision.recorded`` or
``decision.refused`` as session ``terminal``; the stores log nothing.
"""

from __future__ import annotations

from ..cli.dispatch import Command, Reply, Refuse
from ..events import EventLog, events_dir_for
from .decisions import parse_decide, settle_decision
from .store import DecisionRefused, StoreError


def _log(log: EventLog, type_: str, **fields) -> None:
    """Log, but never raise: the event is the record's witness, and a log
    failure after the settle must not crash the command."""
    try:
        log.log(type_, **fields)
    except Exception:
        pass


def _run(inv, args) -> Reply:
    inv.require_person("decide")
    log = EventLog(events_dir_for(inv.root), "terminal")
    tokens = list(args.tokens)
    try:
        ident, choice, pairs, note = parse_decide(tokens)
    except StoreError as exc:
        ident = tokens[0] if tokens else ""
        choice = tokens[1] if len(tokens) > 1 else ""
        _log(log, "decision.refused", decision=ident, choice=choice, code=exc.code)
        raise Refuse(f"decide: {exc}") from exc
    try:
        record = settle_decision(inv.root, ident, choice, pairs, note, session_id="")
    except DecisionRefused as exc:
        _log(log, "decision.refused", decision=ident, choice=choice, code=exc.code)
        return Reply(
            ok=False,
            data={
                "decision": None,
                "error": exc.code,
                "message": str(exc),
                "choices": exc.choices,
            },
        )
    except StoreError as exc:
        _log(log, "decision.refused", decision=ident, choice=choice, code=exc.code)
        return Reply(
            ok=False,
            data={"decision": None, "error": exc.code, "message": str(exc), "choices": []},
        )
    _log(log, "decision.recorded", decision=record.get("id", ""), choice=choice)
    return Reply(data={"decision": record, "error": "", "message": "", "choices": []})


COMMANDS = (
    Command(
        words=("decide",),
        summary="settle a decision: decide <id> <choice> [key=value ...] [-- note]",
        run=_run,
        needs="mounted",
        raw=True,
    ),
)
