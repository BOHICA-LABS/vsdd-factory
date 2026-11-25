"""The emitter: block format, and what each outcome prints where (plan 7,
test_emit.py)."""

from __future__ import annotations

import json
import os
from pathlib import Path

from tests.hooks import runner_helpers as helpers
from tests.hooks.test_runner import base_env, bash_call, read_events
from vsdd.hooks.model import RuleEntry


def rule(module: str, **overrides) -> RuleEntry:
    fields = dict(
        id="probe",
        on=(("PreToolUse", ("Bash",)),),
        kind="gate",
        fail="closed",
        module=module,
    )
    fields.update(overrides)
    return RuleEntry(**fields)


BLOCKING = rule("tests.hooks.fixture_rules.blocking")
ADVISING = rule("tests.hooks.fixture_rules.advising", kind="advisor")
PERMITTING = rule("tests.hooks.fixture_rules.permitting", kind="advisor")
REWRITING = rule("tests.hooks.fixture_rules.rewriting", rewrites=True)
NOT_REWRITING = rule("tests.hooks.fixture_rules.rewriting")


def test_block_format(tmp_path):
    project = helpers.make_project(tmp_path)
    code, out, err = helpers.run_hook(bash_call(project, "echo hi"), "gate",
                                      entries=(BLOCKING,), env=base_env(project))
    lines = err.splitlines()
    assert code == 10
    assert len(lines) == 4, err  # the fixed four lines
    assert lines[0].startswith("vsdd-factory blocked this Bash call. [probe/fixture_block ")
    assert lines[1].startswith("Why: ")
    assert lines[2].startswith("Instead: ")
    assert lines[3].startswith("More: vsdd explain b-")
    # One id on stderr and in the event:
    block_id = lines[0].rsplit(" ", 1)[1].rstrip("]")
    events = read_events(project)
    block = [event for event in events if event["type"] == "rule.block"]
    assert len(block) == 1 and block[0]["id"] == block_id
    # The design's fields are on the event:
    assert block[0]["rule"] == "probe"
    assert block[0]["code"] == "fixture_block"
    assert block[0]["command"] == "echo hi"
    assert block[0]["severity"] == "block"
    assert block[0]["agent_type"] == ""  # the main session, filled by ctx.log
    assert block[0]["event"] == "PreToolUse"
    assert block[0]["tool"] == "Bash"
    assert block[0]["cwd"] == str(project)  # the where-facts of the example line


def test_stdout_by_outcome(tmp_path):
    project = helpers.make_project(tmp_path)

    code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                      entries=(ADVISING,), env=base_env(project))
    rendered = json.loads(out)["hookSpecificOutput"]
    assert code == 0
    assert "vsdd-factory advice [probe/fixture_advice]" in rendered["additionalContext"]
    assert "permissionDecision" not in rendered  # advice is not permission

    permitter = rule("tests.hooks.fixture_rules.permitting", kind="advisor")
    code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                      entries=(permitter,), env=base_env(project))
    rendered = json.loads(out)["hookSpecificOutput"]
    assert rendered["permissionDecision"] == "allow"
    assert rendered["permissionDecisionReason"].startswith("vsdd-factory hook:")

    # A permit only stands when no rule advised:
    code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                      entries=(permitter, ADVISING), env=base_env(project))
    rendered = json.loads(out)["hookSpecificOutput"]
    assert "permissionDecision" not in rendered
    assert "fixture_advice" in rendered["additionalContext"]

    # Context, per event — and PreToolUse carries none of it (plan 3.4):
    context = rule("tests.hooks.fixture_rules.contextual", kind="recorder")
    code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                      entries=(context,), env=base_env(project))
    assert (code, out) == (0, "")  # dropped on PreToolUse: no additionalContext, no systemMessage

    contextual = rule("tests.hooks.fixture_rules.contextual", kind="recorder",
                      on=(("SessionStart", ()),))
    code, out, err = helpers.run_hook(helpers.payload("SessionStart/startup.json"),
                                      "brief", entries=(contextual,), env=base_env(project))
    full = json.loads(out)
    assert full["hookSpecificOutput"]["additionalContext"] == "the fixture context"
    assert full["systemMessage"] == "vsdd-factory: fixture system line"

    # A rewrite from a rule whose row sets rewrites is the updatedInput:
    code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                      entries=(REWRITING,), env=base_env(project))
    rendered = json.loads(out)["hookSpecificOutput"]
    assert rendered["updatedInput"]["fixture"] is True
    # The main session in the default mode is asked (HD7):
    assert rendered["permissionDecision"] == "ask"


def test_context_channel_by_event(tmp_path):
    project = helpers.make_project(tmp_path)
    context = rule("tests.hooks.fixture_rules.contextual", kind="recorder",
                   on=(("SessionStart", ()),))
    code, out, err = helpers.run_hook(helpers.payload("SessionStart/startup.json"),
                                      "brief", entries=(context,), env=base_env(project))
    rendered = json.loads(out)
    assert rendered["hookSpecificOutput"]["additionalContext"] == "the fixture context"

    context_compact = rule("tests.hooks.fixture_rules.contextual", kind="recorder",
                           on=(("PreCompact", ()),))
    code, out, err = helpers.run_hook(helpers.payload("PreCompact/manual.json"),
                                      "notify", entries=(context_compact,),
                                      env=base_env(project))
    assert "the fixture context" in out  # plain stdout on PreCompact
    assert "hookSpecificOutput" not in out

    context_stop = rule("tests.hooks.fixture_rules.contextual", kind="recorder",
                        on=(("SubagentStop", ()),))
    code, out, err = helpers.run_hook(helpers.payload("SubagentStop/step.json"),
                                      "notify", entries=(context_stop,),
                                      env=base_env(project))
    assert out == ""  # dropped elsewhere


def test_rewrite_from_unflagged_rule_is_its_crash(tmp_path):
    project = helpers.make_project(tmp_path)
    code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                      entries=(NOT_REWRITING,), env=base_env(project))
    assert code == 10
    assert "rule_error" in err, err
    assert "Rewrite" in err


def test_rewrite_from_unflagged_rule_is_its_crash_at_advise(tmp_path):
    """HD10 at every strength: the rewrites check runs before the strength
    branch, so an unflagged rule's Rewrite is its crash at advise too — not
    an advice-relaxed rewrite that slips through."""
    project = helpers.make_project(tmp_path)
    (project / ".factory" / "factory.toml").write_text(
        'schema = 1\n[hooks.rules.probe]\nstrength = "advise"\n', encoding="utf-8"
    )
    code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                      entries=(NOT_REWRITING,), env=base_env(project))
    assert code == 10, err
    assert "rule_error" in err, err
    assert "Rewrite" in err
    assert out == ""  # no updatedInput rode the crash


def test_decision_reason_is_the_chosen_rewrite_reason(tmp_path):
    """HD10: the reason sent beside permissionDecision is the chosen
    rewrite's own, prefixed — never a fixed sentence that hides which rule
    rewrote the call."""
    project = helpers.make_project(tmp_path)
    agent = bash_call(project, "echo hi")
    agent["agent_id"] = "agent-1"
    code, out, err = helpers.run_hook(agent, "gate",
                                      entries=(REWRITING,), env=base_env(project))
    rendered = json.loads(out)["hookSpecificOutput"]
    assert rendered["permissionDecisionReason"] == "vsdd-factory hook: the fixture rewrite"


def test_second_rewrite_dropped_and_logged(tmp_path):
    """HD10: one rewrite leaves a call; a second rewriting rule's rewrite is
    dropped, and the drop is logged."""
    project = helpers.make_project(tmp_path)
    other = rule("tests.hooks.fixture_rules.rewriting_other", rewrites=True)
    agent = bash_call(project, "echo hi")
    agent["agent_id"] = "agent-1"
    code, out, err = helpers.run_hook(agent, "gate", entries=(REWRITING, other),
                                      env=base_env(project))
    assert code == 0, err
    rendered = json.loads(out)["hookSpecificOutput"]
    assert rendered["updatedInput"]["fixture"] is True  # the first rewrite wins
    events = read_events(project)
    types = [event["type"] for event in events]
    assert "rewrite.discarded" in types
    dropped = [event for event in events if event["type"] == "rewrite.discarded"]
    assert dropped[0]["reason"] == "the second rewrite"
