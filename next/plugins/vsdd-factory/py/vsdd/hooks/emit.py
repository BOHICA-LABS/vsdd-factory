"""The only writer of a hook's stdout, stderr and exit code
(plan 3.4; [[topic:rule-model]], [[topic:hook-failure]]).

``finish`` runs once per call, over ``(rule id, outcome)`` pairs. Any ``Block``
prints with its own id, is logged, and exits 10; the launcher maps 10 to the
exit 2 Claude Code blocks on. A ``PreToolUse`` without a block carries the
advice, the rewrite's ``updatedInput`` and the paired permission decision in
one ``hookSpecificOutput`` object. The pairing is not decorative: the
platform applies a rewrite whether or not a decision travels with it, so a
rewrite never leaves the runner without its decision — the output step
refuses one, as the rewriting rule's crash (:class:`DecisionlessRewrite`).
``Context`` rides ``additionalContext`` on
the session events and plain stdout on ``PreCompact``, and is dropped
elsewhere. With nothing to say, the runner prints nothing.
"""

from __future__ import annotations

import json
import secrets
import sys

from .model import (
    Advise,
    Block,
    Context,
    Permit,
    Rewrite,
    EVENT_REWRITE_DISCARDED,
    EVENT_RULE_ADVISE,
    EVENT_RULE_BLOCK,
    EVENT_RULE_ERROR,
)

__all__ = ["DecisionlessRewrite", "finish", "block_lines", "new_block_id"]

#: The runner's own block codes (plan 3.4): a rule id of ``runner`` for the
#: ones the runner itself raises, and the failing rule's id for the rest.
RUNNER = "runner"


class DecisionlessRewrite(RuntimeError):
    """A rewrite reached the output step with no permission decision to
    travel beside it (PR4C-04, HD7). The platform applies ``updatedInput``
    whether or not a decision is sent with it, so a decisionless rewrite is
    not advice — it silently changes what runs. The runner refuses to emit
    one: this is the rewriting rule's crash, raised loudly, with the rule's
    ``rule.error`` logged first."""


_CONTEXT_EVENTS = ("SessionStart", "UserPromptExpansion", "SubagentStart")
_PLAIN_STDOUT_EVENT = "PreCompact"

_REASON_PREFIX = "vsdd-factory hook:"


def new_block_id() -> str:
    """One block id: ``b-`` and six hex characters."""
    return "b-" + secrets.token_hex(3)


def block_lines(tool: str, rule: str, code: str, reason: str, fix: str, block_id: str) -> list[str]:
    """The fixed four-line block message ([[topic:rule-model]])."""
    head = f"vsdd-factory blocked this {tool or 'tool'} call. [{rule}/{code} {block_id}]"
    lines = [head, f"Why: {reason}"]
    if fix:
        lines.append(f"Instead: {fix}")
    lines.append(f"More: vsdd explain {block_id}")
    return lines


def context_target_fields(ctx) -> dict:
    """``target or command``, and the where-facts the design's rule events
    carry: the design's example line has `story` and `cwd` too."""
    fields: dict = {}
    target = None
    try:
        target = ctx.target
    except Exception:
        target = None
    if target is not None:
        fields["target"] = str(target)
    elif ctx.tool == "Bash":
        command = ctx.tool_input.get("command")
        if isinstance(command, str):
            fields["command"] = command
    try:
        story = ctx.story
        if story:
            fields["story"] = story
    except Exception:
        pass  # a story lookup must not take the log line down with it
    if ctx.cwd:
        fields["cwd"] = ctx.cwd
    return fields


def _log_block(ctx, rule: str, outcome: Block, block_id: str) -> None:
    ctx.log(
        EVENT_RULE_BLOCK,
        "block",
        id=block_id,
        rule=rule,
        code=outcome.code,
        reason=outcome.reason,
        fix=outcome.fix,
        **context_target_fields(ctx),
    )


def finish(ctx, outcomes, mode: str) -> int:
    """Emit one call's outcomes; return the exit code, 0 or 10."""
    blocks = [(rule, outcome) for rule, outcome in outcomes if isinstance(outcome, Block)]
    advises = [(rule, outcome) for rule, outcome in outcomes if isinstance(outcome, Advise)]
    contexts = [(rule, outcome) for rule, outcome in outcomes if isinstance(outcome, Context)]
    permits = [(rule, outcome) for rule, outcome in outcomes if isinstance(outcome, Permit)]

    if blocks:
        for rule, block in blocks:
            block_id = new_block_id()
            for line in block_lines(ctx.tool, rule, block.code, block.reason, block.fix, block_id):
                print(line, file=sys.stderr)
            _log_block(ctx, rule, block, block_id)
        return 10

    rewrites = [(rule, outcome) for rule, outcome in outcomes if isinstance(outcome, Rewrite)]
    decision = _pre_tool_use_decision(ctx, advises, permits, rewrites)
    _deliver(ctx, advises, contexts, decision, rewrites, mode)
    return 0


def _pre_tool_use_decision(ctx, advises, permits, rewrites):
    """The permission decision of a ``PreToolUse`` call (HD7, P3COV-07).

    With a rewrite, ``allow`` when a ``Permit`` stands, the caller is an agent
    or a workflow step (``agent_id`` set), the tool is ``StructuredOutput``, or
    the permission mode is automatic; otherwise ``ask`` — the main session in a
    mode that prompts — because an ``ask`` paired with a rewrite never runs
    unattended and an ``allow`` for the main session would widen D4. With no
    rewrite, ``allow`` needs a ``Permit``, and never rides advice. The reason
    is the chosen rewrite's own. A second rewrite is dropped, and logged, so
    two rewriting rules cannot silently eat each other's replacement.
    """
    if ctx.event != "PreToolUse":
        return None
    permitted = bool(permits) and not advises
    if rewrites:
        for rule, dropped in rewrites[1:]:
            ctx.log(
                EVENT_REWRITE_DISCARDED,
                "warn",
                rule=rule,
                reason=dropped.reason,
                detail="only the first Rewrite of a call is sent",
            )
        rewrite = rewrites[0][1]
        allowed = (
            permitted
            or bool(ctx.agent_id)
            or ctx.tool == "StructuredOutput"
            or ctx.permission_mode in ("auto", "bypassPermissions")
        )
        return (
            "allow" if allowed else "ask",
            f"{_REASON_PREFIX} {rewrite.reason}",
            rewrite.tool_input,
        )
    if permitted:
        reason = f"{_REASON_PREFIX} {permits[0][1].reason}"
        return ("allow", reason, None)
    return None


def _deliver(ctx, advises, contexts, decision, rewrites, mode: str) -> None:
    """Advice and context to the channel the event carries them on. A
    ``Context`` is dropped on ``PreToolUse`` (plan 3.4): only advice, the
    decision and the rewrite ride ``hookSpecificOutput`` there."""
    if rewrites and decision is None:
        # PR4C-04: the platform applies a rewrite whether or not a permission
        # decision travels with it, so a decisionless rewrite is not advice —
        # it silently changes what runs. The output step refuses to emit one
        # (HD7: every rewrite leaves paired with a decision): the rewriting
        # rule's crash, logged and raised.
        rule, rewrite = rewrites[0]
        exc = DecisionlessRewrite(
            f"the {rule} rule returned a rewrite with no permission decision; "
            "the platform applies updatedInput with or without one, so the "
            "runner never sends a decisionless rewrite"
        )
        try:
            ctx.log(
                EVENT_RULE_ERROR,
                "error",
                rule=rule,
                error=f"{type(exc).__name__}: {exc}",
                fail="open",
            )
        except Exception:
            pass  # the event log cannot take the decision down with it
        raise exc
    advice_lines = []
    for rule, advise in advises:
        advice_lines.append(f"vsdd-factory advice [{rule}/{advise.code}]: {advise.message}")
        ctx.log(EVENT_RULE_ADVISE, "info", id=new_block_id(), rule=rule, code=advise.code,
                reason=advise.message, **context_target_fields(ctx))

    context_text = "\n".join(context.text for _, context in contexts if context.text)
    system_text = "; ".join(dict.fromkeys(c.system for _, c in contexts if c.system))

    if ctx.event == _PLAIN_STDOUT_EVENT:
        plain = advice_lines + ([context_text] if context_text else [])
        if plain:
            print("\n".join(plain), file=sys.stdout)
        return
    if ctx.event not in _CONTEXT_EVENTS and ctx.event != "PreToolUse":
        return  # dropped elsewhere

    carried_context = [] if ctx.event == "PreToolUse" else (
        [context_text] if context_text else []
    )
    additional = advice_lines + carried_context
    hspecific: dict = {"hookEventName": ctx.event}
    if additional:
        hspecific["additionalContext"] = "\n".join(additional)
    if decision is not None:
        outcome_kind, reason, tool_input = decision
        hspecific["permissionDecision"] = outcome_kind
        hspecific["permissionDecisionReason"] = reason
        if tool_input is not None:
            hspecific["updatedInput"] = tool_input
    if not hspecific or set(hspecific) == {"hookEventName"}:
        return  # with nothing to say, the runner prints nothing
    output: dict = {"hookSpecificOutput": hspecific}
    if system_text:
        output["systemMessage"] = system_text
    print(json.dumps(output, ensure_ascii=False), file=sys.stdout)
