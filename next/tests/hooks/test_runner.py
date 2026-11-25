"""The runner core: selection, strength, failure, deadline, notices, the
environment switches (plan 7, test_runner.py)."""

from __future__ import annotations

import json
import os
import re
import sys
import uuid
from pathlib import Path

import pytest

from tests.hooks import runner_helpers as helpers
from vsdd import events as vsdd_events
from vsdd.hooks import main as hook_main
from vsdd.hooks import registry
from vsdd.hooks.model import RuleEntry

FIXTURES = "tests.hooks.fixture_rules"


def base_env(project: Path, extra: dict | None = None) -> dict:
    environment = {
        "VSDD_FACTORY_ROOT": str(project),
        "VSDD_EVENTS_DIR": str(project / "events"),
        "HOME": os.environ["HOME"],
    }
    environment.update(extra or {})
    return environment


def bash_call(project: Path, command: str, **extra) -> dict:
    payload = helpers.payload("PreToolUse/bash-main.json")
    payload["cwd"] = str(project)
    payload["tool_input"] = {"command": command}
    payload.update(extra)
    return payload


def read_events(project: Path) -> list[dict]:
    day = sorted((project / "events").glob("*.jsonl"))
    if not day:
        return []
    return [json.loads(line) for line in day[-1].read_text().splitlines() if line.strip()]


def write_events(project: Path, count: int, rule: str, type_: str = "rule.error") -> None:
    log = vsdd_events.EventLog(project / "events", "seeder")
    for _ in range(count):
        log.log(type_, "error", rule=rule)


def entry(identifier: str, module: str, **overrides) -> RuleEntry:
    fields = dict(
        id=identifier,
        on=(("PreToolUse", ("Bash",)),),
        kind="gate",
        fail="closed",
        module=module,
    )
    fields.update(overrides)
    return RuleEntry(**fields)


BLOCKING = entry("probe", f"{FIXTURES}.blocking")
ADVISING = entry("probe", f"{FIXTURES}.advising", kind="advisor")
CRASHING_OPEN = entry("probe", f"{FIXTURES}.crashing", fail="open")
CRASHING_CLOSED = entry("probe", f"{FIXTURES}.crashing")
MISSING_MODULE = entry("probe", f"{FIXTURES}.{uuid.uuid4().hex}")


class TestSelect:
    def test_table_order_and_atoms(self):
        a = entry("a-rule", f"{FIXTURES}.blocking", on=(("PreToolUse", ("Bash",)),))
        b = entry("b-rule", f"{FIXTURES}.advising", on=(("PreToolUse", ()),))
        c = entry("c-rule", f"{FIXTURES}.blocking", on=(("SessionStart", ()),))
        chosen = registry.select("PreToolUse", "Bash", (a, b, c))
        assert [rule.id for rule in chosen] == ["a-rule", "b-rule"]

    def test_atom_matches_whole_name(self):
        partial = entry("partial", f"{FIXTURES}.blocking", on=(("PreToolUse", ("Ba",)),))
        exact = entry("exact", f"{FIXTURES}.advising", on=(("PreToolUse", ("Bash",)),))
        assert registry.select("PreToolUse", "Bash", (partial, exact)) == [exact]

    def test_unselected_rule_never_imports(self, tmp_path):
        sleeper = "tests.hooks.fixture_rules.sleeping"
        absent = entry("absent", f"{FIXTURES}.nonexistent_module_xyz")
        unused = entry("unused", sleeper, on=(("SessionStart", ()),))
        payload = bash_call(tmp_path, "echo hi")
        code, out, err = helpers.run_hook(payload, "gate", entries=(absent, unused),
                                          env=base_env(tmp_path))
        assert code == 10  # the missing-module rule crashed and blocked
        assert "tests.hooks.fixture_rules.sleeping" not in sys.modules


def test_all_rules_run_after_a_block(tmp_path):
    a = entry("a-rule", f"{FIXTURES}.blocking")
    b = entry("b-rule", f"{FIXTURES}.blocking")
    code, out, err = helpers.run_hook(bash_call(tmp_path, "echo hi"), "gate",
                                      entries=(a, b), env=base_env(tmp_path))
    assert code == 10
    assert err.count("Why:") == 2, err
    assert "a-rule" in err and "b-rule" in err


class TestAppliesAndStrength:
    def test_quarantine_needs_two_faults(self, tmp_path, monkeypatch):
        project = helpers.make_project(tmp_path)
        rule = entry("flaky", f"{FIXTURES}.crashing")
        env = base_env(project, {"VSDD_FACTORY_HOOKS_QUARANTINE": "flaky"})

        write_events(project, 1, "flaky")
        code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                          entries=(rule,), env=env)
        assert code == 10  # one error is too few: the rule runs, and crashes

        write_events(project, 2, "flaky")
        code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                          entries=(rule,), env=env)
        assert code == 0, err  # quarantined: the rule is skipped entirely

    def test_strength_off_skips(self, tmp_path):
        project = helpers.make_project(tmp_path)
        (project / ".factory" / "factory.toml").write_text(
            'schema = 1\n[hooks.rules.probe]\nstrength = "off"\n', encoding="utf-8"
        )
        rule = entry("probe", f"{FIXTURES}.blocking")
        code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                          entries=(rule,), env=base_env(project))
        assert code == 0, err

    def test_spike_exempt_rule_skipped_inside_a_spike(self, tmp_path):
        """HD5, in order: a spike_exempt rule does not run when the call works
        in a spike worktree, and is in force outside one."""
        project = helpers.make_project(tmp_path, story="spike-probe")
        exempt = entry("probe", f"{FIXTURES}.blocking", spike_exempt=True)
        inside = bash_call(project, "echo hi")
        inside["cwd"] = str(project / ".worktrees" / "spike-probe")
        code, out, err = helpers.run_hook(inside, "gate", entries=(exempt,),
                                          env=base_env(project))
        assert code == 0, err  # exempted in the spike

        code, out, err = helpers.run_hook(bash_call(project, "echo hi"), "gate",
                                          entries=(exempt,), env=base_env(project))
        assert code == 10, err  # in force outside the spike

    def test_advise_converts_block_and_logs_would_block(self, tmp_path):
        project = helpers.make_project(tmp_path)
        (project / ".factory" / "factory.toml").write_text(
            'schema = 1\n[hooks.rules.probe]\nstrength = "advise"\n', encoding="utf-8"
        )
        rule = entry("probe", f"{FIXTURES}.strength_probe")
        code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                          entries=(rule,), env=base_env(project))
        assert code == 0, err
        assert "advice [probe/strength_block]" in out, out
        events = read_events(project)
        assert "rule.would_block" in [event["type"] for event in events]
        would_block = [event for event in events if event["type"] == "rule.would_block"][0]
        # The design's fields: an id of the block that would have fired, and
        # the target-or-command facts (tables/event-types.toml).
        assert re.fullmatch(r"b-[0-9a-f]{6}", would_block["id"]), would_block
        assert would_block["command"] == "echo"

    def test_faulty_rule_table_resets_that_rule_only(self, tmp_path):
        project = helpers.make_project(tmp_path)
        (project / ".factory" / "factory.toml").write_text(
            'schema = 1\n'
            '[hooks.rules.probe]\nstrength = "sometimes"\n'
            '[hooks.rules.other-rule]\nstrength = "off"\n',
            encoding="utf-8",
        )
        rule = entry("probe", f"{FIXTURES}.strength_probe")
        other = entry("other-rule", f"{FIXTURES}.blocking")
        code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                          entries=(rule, other), env=base_env(project))
        # probe's faulty table reset to its defaults (block, so it blocks);
        # the other rule's own valid table still holds: a per-rule fault is
        # local to the rule whose table is broken.
        assert code == 10, err
        assert "probe/strength_block" in err, err
        assert "other-rule" not in err  # strength off honoured, not reset

    def test_locked_rule_always_blocks(self, tmp_path):
        """Plan 3.2: a locked rule always gets block, whatever its table says
        — the runner enforces it, not only the configuration loader."""
        project = helpers.make_project(tmp_path)
        locked = entry("probe", f"{FIXTURES}.blocking", locked=True)
        for strength in ("off", "advise"):
            (project / ".factory" / "factory.toml").write_text(
                f'schema = 1\n[hooks.rules.probe]\nstrength = "{strength}"\n',
                encoding="utf-8",
            )
            code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                              entries=(locked,), env=base_env(project))
            assert code == 10, (strength, err)  # locked: the relaxation is ignored
            assert "fixture_block" in err

        # An unlocked rule at the same table is relaxed as declared (off):
        unlocked = entry("probe", f"{FIXTURES}.blocking")
        (project / ".factory" / "factory.toml").write_text(
            'schema = 1\n[hooks.rules.probe]\nstrength = "off"\n', encoding="utf-8"
        )
        code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                          entries=(unlocked,), env=base_env(project))
        assert (code, out, err) == (0, "", "")


def test_rule_crash(tmp_path):
    project = helpers.make_project(tmp_path)
    closed = entry("probe", f"{FIXTURES}.crashing")
    code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                      entries=(closed,), env=base_env(project))
    assert code == 10
    assert "rule_error" in err and "ZeroDivisionError" in err, err
    assert "VSDD_FACTORY_HOOKS_QUARANTINE=probe" in err, err  # the quarantine line
    assert "probe" in err

    code, out, err = helpers.run_hook(bash_call(project, "echo"), "notify",
                                      entries=(closed,), env=base_env(project))
    assert code == 0 and err == ""  # only gate blocks

    opener = entry("probe", f"{FIXTURES}.crashing", fail="open")
    code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                      entries=(opener,), env=base_env(project))
    assert code == 0 and err == ""  # fail-open is logged, not raised
    types = [event["type"] for event in read_events(project)]
    assert "rule.error" in types


def test_deadline(tmp_path, monkeypatch):
    project = helpers.make_project(tmp_path)
    original = registry.EVENTS
    shortened = tuple(
        spec if spec.name != "PreToolUse" else
        type(spec)("PreToolUse", "gate", spec.matcher, spec.timeout, 0.5, spec.on_failure)
        for spec in original
    )
    monkeypatch.setattr(registry, "EVENTS", shortened)
    sleeper = entry("slowpoke", f"{FIXTURES}.sleeping")
    code, out, err = helpers.run_hook(
        bash_call(project, "echo", tool_input={"command": "echo", "sleep": "3"}),
        "gate",
        entries=(sleeper,),
        env=base_env(project),
    )
    assert code == 10, err
    assert "runner_timeout" in err and "slowpoke" in err, err


def test_deadline_in_brief_and_notify(tmp_path, monkeypatch):
    """HD3 outside gate mode: the deadline is logged everywhere, but blocks
    only in gate — brief prints the fixed fallback, notify exits 0 silently.
    Brief is SessionStart's own mode, so the fallback context has a channel."""
    project = helpers.make_project(tmp_path)
    original = registry.EVENTS
    shortened = tuple(
        spec if spec.name != "SessionStart" else
        type(spec)(spec.name, spec.mode, spec.matcher, spec.timeout, 0.5, spec.on_failure,
                   spec.is_async)
        for spec in original
    )
    monkeypatch.setattr(registry, "EVENTS", shortened)
    sleeper = entry("slowpoke", f"{FIXTURES}.sleeping", on=(("SessionStart", ()),))
    payload = helpers.payload("SessionStart/startup.json") | {
        "tool_input": {"command": "echo", "sleep": "3"}
    }

    code, out, err = helpers.run_hook(payload, "notify", entries=(sleeper,),
                                      env=base_env(project))
    assert (code, out, err)[:2] == (0, ""), (code, out, err)  # no block, no exit 10
    types = [event["type"] for event in read_events(project)]
    assert "rule.timeout" in types  # logged in every mode

    code, out, err = helpers.run_hook(payload, "brief", entries=(sleeper,),
                                      env=base_env(project))
    assert code == 0, err
    rendered = json.loads(out)
    assert hook_main.FALLBACK_TEXT in rendered["hookSpecificOutput"]["additionalContext"]
    assert rendered["systemMessage"]


def test_crash_in_brief_prints_the_fallback(tmp_path):
    """HD3: a crashing briefing rule is never a silent session start — the
    fixed fallback line is printed, and the exit stays 0 (also in notify)."""
    project = helpers.make_project(tmp_path)
    crasher = entry("probe", f"{FIXTURES}.crashing", kind="advisor",
                    on=(("SessionStart", ()),))
    code, out, err = helpers.run_hook(
        helpers.payload("SessionStart/startup.json"), "brief", entries=(crasher,),
        env=base_env(project),
    )
    assert code == 0, err
    rendered = json.loads(out)
    assert hook_main.FALLBACK_TEXT in rendered["hookSpecificOutput"]["additionalContext"]
    assert rendered["systemMessage"]

    code, out, err = helpers.run_hook(
        helpers.payload("SessionStart/startup.json"), "notify", entries=(crasher,),
        env=base_env(project),
    )
    assert (code, out, err)[:2] == (0, ""), (code, out, err)
    types = [event["type"] for event in read_events(project)]
    assert "rule.error" in types  # logged in every mode


class TestDegradedInputs:
    def test_unwritable_log_still_prints_the_block(self, tmp_path):
        project = helpers.make_project(tmp_path)
        logfile = project / "not-a-dir"
        logfile.write_text("i am a file", encoding="utf-8")
        env = base_env(project, {"VSDD_EVENTS_DIR": str(logfile)})
        code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                          entries=(BLOCKING,), env=env)
        assert code == 10 and "fixture_block" in err  # the block survives

    def test_broken_configuration_relaxes_nothing(self, tmp_path):
        project = helpers.make_project(tmp_path)
        (project / ".factory" / "factory.toml").write_text("schema = 999\n[oops", encoding="utf-8")
        code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                          entries=(BLOCKING,), env=base_env(project))
        assert code == 10 and "fixture_block" in err

    def test_missing_policy_table_is_policy_invalid(self, tmp_path, monkeypatch):
        from vsdd.hooks import policy as policy_module

        project = helpers.make_project(tmp_path)

        def broken(name):
            raise OSError("table vanished")

        monkeypatch.setattr(policy_module, "_load_table", broken)
        monkeypatch.setattr(policy_module, "_policy", None)
        reader = entry("probe", f"{FIXTURES}.policy_reading")
        code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                          entries=(reader,), env=base_env(project))
        assert code == 10 and "policy_invalid" in err, err


class TestSessionNotices:
    def test_four_notices(self, tmp_path, monkeypatch):
        from vsdd.hooks import policy as policy_module

        sick = helpers.make_project(tmp_path)
        # One rule's table is faulty (per-rule problem, the file still parses)
        # and one rule id is not in the registry; both are configuration
        # faults and belong in the briefing.
        (sick / ".factory" / "factory.toml").write_text(
            'schema = 1\n[hooks.rules.weird]\nstrength = "wrong"\n'
            '[hooks.rules.soft-rule]\nstrength = "off"\n',
            encoding="utf-8",
        )
        write_events(sick, 2, "flaky")
        env = base_env(sick, {"VSDD_FACTORY_HOOKS_QUARANTINE": "flaky"})

        def broken(name):
            raise OSError("table vanished")

        monkeypatch.setattr(policy_module, "_load_table", broken)
        monkeypatch.setattr(policy_module, "_policy", None)

        code, out, err = helpers.run_hook(
            helpers.payload("SessionStart/startup.json"), "brief", env=env
        )
        assert code == 0
        rendered = json.loads(out)
        context = rendered["hookSpecificOutput"]["additionalContext"]
        assert "configuration is invalid" in context
        assert "weird" in context  # the rule whose table is faulty
        assert "unknown rules: soft-rule" in context  # a typoed id is a fault
        assert "policy table is invalid" in context
        assert "quarantine is in force" in context
        assert rendered["systemMessage"]

    def test_registered_rules_are_announced_as_relaxations(self, tmp_path, monkeypatch):
        """Only registered rules are announced as relaxations; a known id
        below full strength reads as the working relaxation it is."""
        project = helpers.make_project(tmp_path)
        (project / ".factory" / "factory.toml").write_text(
            'schema = 1\n[hooks.rules.soft-rule]\nstrength = "off"\n'
            '[hooks.rules.typoo]\nstrength = "advise"\n',
            encoding="utf-8",
        )
        soft = entry("soft-rule", f"{FIXTURES}.advising", kind="advisor")
        monkeypatch.setattr(registry, "RULES", (soft,))
        code, out, err = helpers.run_hook(
            helpers.payload("SessionStart/startup.json"), "brief", env=base_env(project)
        )
        assert code == 0, err
        rendered = json.loads(out)
        context = rendered["hookSpecificOutput"]["additionalContext"]
        # Only registered rules are relaxations; the unknown id is a fault.
        # The whole relaxation sentence is asserted, so an unfiltered line
        # that also lists the unknown id cannot sneak past:
        assert "vsdd-factory: rules below full strength this session: soft-rule (off)." in context
        assert "typoo (advise)" not in context
        assert "unknown rules: typoo" in context

    def test_none_on_a_healthy_project(self, tmp_path):
        healthy = helpers.make_project(tmp_path)
        code, out, err = helpers.run_hook(
            helpers.payload("SessionStart/startup.json"), "brief", env=base_env(healthy)
        )
        assert code == 0 and out == ""  # none on a healthy project

    def test_config_problem_alone_announces(self, tmp_path):
        project = helpers.make_project(tmp_path)
        (project / ".factory" / "factory.toml").write_text(
            'schema = 1\n[hooks.rules.weird]\nstrength = "wrong"\n', encoding="utf-8"
        )
        code, out, err = helpers.run_hook(
            helpers.payload("SessionStart/startup.json"), "brief", env=base_env(project)
        )
        assert code == 0 and "configuration is invalid" in out


def test_bad_input_blocks_via_launcher(tmp_path):
    """Garbage on stdin in gate mode exits 2 with runner_bad_input — the loud
    failure a gate or check must never switch off without."""
    from tests.hooks._helpers import INTERPRETERS

    project = helpers.make_project(tmp_path)
    launch_sh = helpers.PLUGIN / "py" / "launch.sh"
    for interpreter in INTERPRETERS.values():
        result = helpers.launch(
            interpreter, launch_sh, ["hook", "gate"], project,
            stdin_text="garbage{",
            env={"VSDD_EVENTS_DIR": str(project / "events")},
        )
        assert result.returncode == 2, (interpreter, result.stderr)
        assert "runner_bad_input" in result.stderr, result.stderr


def test_bad_input_logged_in_gate_only(tmp_path):
    """HD11: in gate mode the runner_bad_input block is logged as rule.block;
    in notify and brief nothing blocks, and the fault is on the record as a
    non-block event of the runner (the failure table: logged in notify). The
    events directory the call names is used (VSDD_EVENTS_DIR honoured)."""
    project = helpers.make_project(tmp_path)
    events_dir = project / "events"  # not the default .factory/local/events

    def events_of():
        days = sorted(events_dir.glob("*.jsonl"))
        if not days:
            return []
        return [json.loads(line) for line in days[-1].read_text().splitlines() if line.strip()]

    code, out, err = helpers.run_hook("garbage{", "notify", env=base_env(project))
    assert (code, out, err)[:2] == (0, "")
    notify_events = events_of()
    assert [event["type"] for event in notify_events] == ["rule.error"]
    assert notify_events[0]["rule"] == "runner"
    assert notify_events[0]["code"] == "runner_bad_input"
    assert notify_events[0]["session_id"] == "unknown"

    code, out, err = helpers.run_hook("garbage{", "brief", env=base_env(project))
    assert code == 0 and hook_main.FALLBACK_TEXT in out
    brief_events = events_of()
    assert [event["type"] for event in brief_events] == ["rule.error", "rule.error"]

    code, out, err = helpers.run_hook("garbage{", "gate", env=base_env(project))
    assert code == 10 and "runner_bad_input" in err
    day = sorted(events_dir.glob("*.jsonl"))
    assert day  # the override directory, not .factory/local/events
    events = events_of()
    assert [event["type"] for event in events] == ["rule.error", "rule.error", "rule.block"]
    assert events[-1]["code"] == "runner_bad_input"
    assert events[-1]["severity"] == "block"  # the block, gate only

    default_days = sorted((project / ".factory" / "local" / "events").glob("*.jsonl"))
    assert not any(
        "rule.block" in line for day in default_days for line in day.read_text().splitlines()
    )


def test_import_crash_blocks_via_launcher(tmp_path):
    """A broken emit/import exits 2 in gate, 0 in notify, through the real
    launcher (a rule module that cannot import is that rule's error)."""
    from tests.hooks._helpers import INTERPRETERS

    launch, plugin = helpers.launch_copy(tmp_path, rules=[
        (
            {
                "id": "probe",
                "on": (("PreToolUse", ("Bash",)),),
                "kind": "gate",
                "fail": "closed",
                "module": "vsdd.rules.probe_missing_module",
            },
            None,  # no source: the module file does not exist
        ),
    ])
    project = helpers.make_project(tmp_path / "work")
    for interpreter in INTERPRETERS.values():
        gate = helpers.launch(interpreter, launch, ["hook", "gate"], project,
                              stdin_text=json.dumps(bash_call(project, "echo hi")),
                              env={"VSDD_EVENTS_DIR": str(project / "events")})
        assert gate.returncode == 2, (interpreter, gate.stderr)
        assert "rule_error" in gate.stderr, gate.stderr

        notify = helpers.launch(interpreter, launch, ["hook", "notify"], project,
                                stdin_text=json.dumps(bash_call(project, "echo hi")),
                                env={"VSDD_EVENTS_DIR": str(project / "events")})
        assert notify.returncode == 0, (interpreter, notify.stderr)


def test_broken_emit_via_launcher(tmp_path):
    """Plan 7's own example: a broken ``emit.py`` (not a missing rule module)
    is the runner's own crash. Through the real launcher: exit 2 in gate, 0 in
    notify, and the down briefing in brief — under sh, bash and dash."""
    from tests.hooks._helpers import INTERPRETERS

    launch, plugin = helpers.launch_copy(tmp_path)
    (plugin / "py" / "vsdd" / "hooks" / "emit.py").write_text(
        'raise RuntimeError("broken emit for the launcher test")\n', encoding="utf-8"
    )
    project = helpers.make_project(tmp_path / "work")
    gate_payload = json.dumps(bash_call(project, "echo hi"))
    brief_payload = json.dumps(helpers.payload("SessionStart/startup.json"))
    for interpreter in INTERPRETERS.values():
        gate = helpers.launch(interpreter, launch, ["hook", "gate"], project,
                              stdin_text=gate_payload,
                              env={"VSDD_EVENTS_DIR": str(project / "events")})
        assert gate.returncode == 2, (interpreter, gate.stderr)
        assert "launch_rc_" in gate.stderr, gate.stderr  # the runner's own crash

        notify = helpers.launch(interpreter, launch, ["hook", "notify"], project,
                                stdin_text=gate_payload,
                                env={"VSDD_EVENTS_DIR": str(project / "events")})
        assert notify.returncode == 0, (interpreter, notify.stderr)

        brief = helpers.launch(interpreter, launch, ["hook", "brief"], project,
                               stdin_text=brief_payload,
                               env={"VSDD_EVENTS_DIR": str(project / "events")})
        assert brief.returncode == 0, (interpreter, brief.stderr)
        rendered = json.loads(brief.stdout)
        assert "the factory gates are down" in rendered["systemMessage"], brief.stdout


def test_root_at_home_is_inert(tmp_path):
    env = base_env(tmp_path)
    env["VSDD_FACTORY_ROOT"] = os.environ["HOME"]
    code, out, err = helpers.run_hook(bash_call(tmp_path, "echo"), "gate", env=env)
    assert (code, out, err) == (0, "", "")

    env = base_env(tmp_path)
    env["VSDD_FACTORY_ROOT"] = "/"
    code, out, err = helpers.run_hook(bash_call(tmp_path, "echo"), "gate", env=env)
    assert (code, out, err) == (0, "", "")


class TestEnvSwitches:
    def test_debug_logs_runner_call(self, tmp_path):
        project = helpers.make_project(tmp_path)
        env = base_env(project, {"VSDD_FACTORY_HOOKS_DEBUG": "1"})
        code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                          entries=(BLOCKING,), env=env)
        types = [event["type"] for event in read_events(project)]
        assert "runner.call" in types

    def test_slow_call_logged(self, tmp_path, monkeypatch):
        project = helpers.make_project(tmp_path)
        original = registry.EVENTS
        shortened = tuple(
            spec if spec.name != "PreToolUse" else
            type(spec)("PreToolUse", "gate", spec.matcher, spec.timeout, 5.0, spec.on_failure)
            for spec in original
        )
        monkeypatch.setattr(registry, "EVENTS", shortened)
        env = base_env(project, {"VSDD_FACTORY_HOOKS_DEBUG": "1"})
        sleeper = entry("slowpoke", f"{FIXTURES}.sleeping")
        helpers.run_hook(
            bash_call(project, "echo", tool_input={"command": "echo", "sleep": "0.35"}),
            "gate",
            entries=(sleeper,),
            env=env,
        )
        events = read_events(project)
        slow = [event for event in events if event["type"] == "runner.slow"]
        assert slow and slow[0]["ms"] >= 300

    def test_slow_call_logged_without_debug(self, tmp_path, monkeypatch):
        """runner.slow is a governed event type of its own: a call over 300 ms
        is logged whoever is debugging (tables/event-types.toml)."""
        project = helpers.make_project(tmp_path)
        original = registry.EVENTS
        shortened = tuple(
            spec if spec.name != "PreToolUse" else
            type(spec)("PreToolUse", "gate", spec.matcher, spec.timeout, 5.0, spec.on_failure)
            for spec in original
        )
        monkeypatch.setattr(registry, "EVENTS", shortened)
        env = base_env(project)  # no VSDD_FACTORY_HOOKS_DEBUG
        sleeper = entry("slowpoke", f"{FIXTURES}.sleeping")
        helpers.run_hook(
            bash_call(project, "echo", tool_input={"command": "echo", "sleep": "0.35"}),
            "gate",
            entries=(sleeper,),
            env=env,
        )
        slow = [event for event in read_events(project) if event["type"] == "runner.slow"]
        assert slow and slow[0]["ms"] >= 300
        assert "runner.call" not in [event["type"] for event in read_events(project)]

    def test_capture_writes_substituted_payload(self, tmp_path):
        project = helpers.make_project(tmp_path)
        capture = tmp_path / "captured"
        env = base_env(project, {"VSDD_FACTORY_HOOKS_CAPTURE": str(capture)})
        helpers.run_hook(bash_call(project, "echo hi"), "gate", env=env)
        helpers.run_hook(bash_call(project, "echo ho"), "gate", env=env)
        stored = sorted((capture / "PreToolUse").glob("Bash-*.json"))
        assert len(stored) == 2  # one file per capture, not one per tool
        text = stored[-1].read_text()
        assert str(project) not in text and "{root}" in text
        meta = json.loads((capture / "_meta.json").read_text())
        assert meta["version"] and meta["captured_at"]  # stamped, not anonymous
