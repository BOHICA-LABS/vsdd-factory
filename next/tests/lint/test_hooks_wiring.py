"""The wiring lint (B4b; plan 7, tests/lint/test_hooks_wiring.py): hooks.json
is current, its shape holds, the generator refuses what it must, the locked
rows agree with the loader, and the self-guarding command behaves."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

from tests.hooks import runner_helpers as helpers
from vsdd.hooks import registry
from vsdd.hooks.model import RuleEntry

NEXT = helpers.NEXT
HOOKS_JSON = NEXT / "plugins" / "vsdd-factory" / "hooks" / "hooks.json"
DESIGN_MATCHER = (
    "Bash|Edit|Write|MultiEdit|NotebookEdit|Read|Grep|Glob|Agent|Task|"
    "StructuredOutput|Workflow|WebFetch|WebSearch|"
    "mcp__(plugin_vsdd-factory_)?playwright__browser_(navigate|navigate_back|tabs|file_upload)|"
    "mcp__(plugin_vsdd-factory_)?(perplexity|tavily|context7)__.*"
)


def build_check():
    return subprocess.run(
        [sys.executable, str(NEXT / "tools" / "build.py"), "--check"],
        capture_output=True, text=True, cwd=str(NEXT), timeout=120,
    )


def _copy_tree(tmp: Path) -> Path:
    """A minimal `next/`-shaped tree for the drift check: build.py and
    everything it reads, and nothing that another unit may be rewriting."""
    copy = tmp / "next"
    (copy / "tools").mkdir(parents=True)
    (copy / "workflow-lib").mkdir()
    (copy / "tests" / "lint").mkdir(parents=True)
    (copy / "tests" / "workflows").mkdir(parents=True)
    shutil.copyfile(NEXT / "tools" / "build.py", copy / "tools" / "build.py")
    shutil.copytree(NEXT / "workflow-lib", copy / "workflow-lib", dirs_exist_ok=True)
    shutil.copyfile(NEXT / "tests" / "lint" / "roster.toml", copy / "tests" / "lint" / "roster.toml")
    shutil.copytree(NEXT / "tests" / "workflows" / "toys", copy / "tests" / "workflows" / "toys")
    ignore = shutil.ignore_patterns("__pycache__")
    shutil.copytree(
        NEXT / "plugins" / "vsdd-factory", copy / "plugins" / "vsdd-factory", ignore=ignore
    )
    return copy


def test_current_and_drift(tmp_path):
    """The checked-in file is current; a changed byte makes --check exit 1.
    The drift is simulated in a temporary copy of the tree, so a killed run
    never leaves the checked-in file drifted."""
    copy = _copy_tree(tmp_path)
    drifted_json = copy / "plugins" / "vsdd-factory" / "hooks" / "hooks.json"

    def check():
        return subprocess.run(
            [sys.executable, str(copy / "tools" / "build.py"), "--check"],
            capture_output=True, text=True, cwd=str(copy), timeout=120,
        )

    result = check()
    assert result.returncode == 0, result.stdout + result.stderr

    text = drifted_json.read_text()
    drifted_json.write_text(text + " ")
    result = check()
    assert result.returncode == 1
    assert "DRIFT plugins/vsdd-factory/hooks/hooks.json" in result.stdout

    # Restoring the copy's file clears the drift; the checked-in file itself
    # was never touched.
    drifted_json.write_text(text)
    assert check().returncode == 0
    assert build_check().returncode == 0


def test_shape():
    document = json.loads(HOOKS_JSON.read_text())
    assert set(document) == {"hooks"}
    hooks = document["hooks"]

    # At this depth: the duties, and the lock liveness of hook-runner-b:
    # SessionEnd wired in notify mode beside the two PreToolUse atoms.
    assert set(hooks) == {"SessionStart", "PreToolUse", "SessionEnd"}

    for event, wires in hooks.items():
        for wire in wires:
            for handler in wire["hooks"]:
                timeout = handler.get("timeout", 0)
                assert isinstance(timeout, int) and timeout <= 60
                if registry._EVENT_BY_NAME[event].mode == "gate":
                    assert handler.get("onFailure") == "block"

    pre_tool = hooks["PreToolUse"][0]
    # The marker and liveness atoms, plus the three rule rows', in TOOL_ATOMS order.
    assert pre_tool["matcher"] == "Bash|Edit|Write|MultiEdit|NotebookEdit|StructuredOutput"
    assert pre_tool["hooks"][0]["timeout"] == 10
    assert pre_tool["hooks"][0]["onFailure"] == "block"

    session = hooks["SessionStart"][0]
    assert "matcher" not in session  # every source
    assert session["hooks"][0]["timeout"] == 15

    # The liveness release: SessionEnd reaches every source, in notify mode,
    # with no timeout key — the platform's budget governs it (HD4).
    session_end = hooks["SessionEnd"][0]
    assert "matcher" not in session_end
    end_handler = session_end["hooks"][0]
    assert "timeout" not in end_handler
    assert "onFailure" not in end_handler
    assert "async" not in end_handler
    assert end_handler["command"] == registry.guard_command("notify")

    # HD4's deadlines:
    expected = {
        "PreToolUse": 4.0, "UserPromptExpansion": 4.0, "SubagentStop": 4.0,
        "SessionStart": 8.0, "PreCompact": 25.0, "SubagentStart": 1.0,
        "SessionEnd": 1.0,
    }
    for event in registry.EVENTS:
        assert event.deadline == expected[event.name], event.name
    # SessionEnd carries no timeout key: the platform's budget governs it.
    session_end = next(e for e in registry.EVENTS if e.name == "SessionEnd")
    assert session_end.timeout is None

    # The atoms are declared, not split out of the joined matcher: sixteen of
    # them, each compiling, the MCP ones selecting their tools.
    assert len(registry.TOOL_ATOMS) == 16
    for atom in registry.TOOL_ATOMS:
        re.compile(atom)  # a broken atom would raise here
    assert "|".join(registry.TOOL_ATOMS) == DESIGN_MATCHER
    assert registry.matches(registry.TOOL_ATOMS[15], "mcp__tavily__search")
    assert registry.matches(registry.TOOL_ATOMS[15], "mcp__context7__resolve-library-id")
    assert registry.matches(registry.TOOL_ATOMS[14], "mcp__playwright__browser_navigate")
    assert registry.matches(registry.TOOL_ATOMS[14], "mcp__plugin_vsdd-factory_playwright__browser_tabs")
    assert not registry.matches(registry.TOOL_ATOMS[15], "mcp__otherserver__tool")


def test_wiring_generated_from_a_fixture_table():
    """Every event the table names is wired with its own matcher: the fixed
    matcher of every event but PreToolUse, the named atoms for PreToolUse,
    every atom when a PreToolUse row names none, and ``async`` where the row
    says so (HD2). Reachability is checked with re.fullmatch, so a matcher
    that only contains an atom as a substring is not enough."""
    entries = (
        RuleEntry(id="a-gate", on=(("PreToolUse", ("Bash", "Read")),), kind="gate", fail="closed"),
        RuleEntry(id="all-tools", on=(("PreToolUse", ()),), kind="recorder", fail="open"),
        RuleEntry(id="prompts", on=(("UserPromptExpansion", ()),), kind="gate", fail="closed"),
        RuleEntry(id="starter", on=(("SubagentStart", ()),), kind="recorder", fail="open"),
        RuleEntry(id="stopper", on=(("SubagentStop", ()),), kind="recorder", fail="open"),
        RuleEntry(id="compactor", on=(("PreCompact", ()),), kind="recorder", fail="open"),
        RuleEntry(id="ender", on=(("SessionEnd", ()),), kind="recorder", fail="open"),
    )
    document = registry.wiring(entries)["hooks"]

    assert set(document) == {
        "SessionStart", "PreToolUse", "UserPromptExpansion",
        "SubagentStart", "SubagentStop", "PreCompact", "SessionEnd",
    }
    # The fixed matchers, event for event:
    assert document["UserPromptExpansion"][0]["matcher"] == "^vsdd-factory:decide$"
    assert document["SubagentStart"][0]["matcher"] == "^vsdd-factory:|^workflow-subagent$"
    assert document["SubagentStop"][0]["matcher"] == "^vsdd-factory:|^workflow-subagent$"
    # The async event is wired with async:
    assert document["SubagentStart"][0]["hooks"][0]["async"] is True
    assert "async" not in document["SubagentStop"][0]["hooks"][0]
    # Every session source, no matcher on SessionStart:
    assert "matcher" not in json.loads(registry.hooks_json(entries))["hooks"]["SessionStart"][0]
    # The every-tool PreToolUse row pulls the whole atom list, in order:
    assert document["PreToolUse"][0]["matcher"] == DESIGN_MATCHER

    # SessionEnd is the one event with neither matcher nor timeout: every
    # source reaches it, and the platform's own budget governs it (the table's
    # timeout is None, and no timeout key may be wired). Notify mode: no
    # onFailure, and the notify guard's command.
    end_wire = document["SessionEnd"][0]
    assert "matcher" not in end_wire
    end_handler = end_wire["hooks"][0]
    assert "timeout" not in end_handler
    assert "onFailure" not in end_handler
    assert "async" not in end_handler
    assert end_handler["command"] == registry.guard_command("notify")

    # With only the atom-naming row: its atoms plus the duties', in
    # TOOL_ATOMS order.
    named = registry.wiring(entries[:1])["hooks"]["PreToolUse"][0]["matcher"]
    assert named == "Bash|Read|StructuredOutput"

    # Reachability: for every (event, atom) a rule names, some real tool name
    # both the rule's atom and the wired matcher select whole.
    samples = (
        "Bash", "Read", "Edit", "Write", "Task", "StructuredOutput",
        "mcp__tavily__search", "mcp__playwright__browser_navigate",
    )
    for entry in entries:
        for event, atoms in entry.on:
            wires = document[event]
            if not atoms:
                # Named no tools: reachable on every call of the event, which
                # the wire serves whatever its matcher.
                assert wires
                continue
            matcher = wires[0]["matcher"]
            for atom in atoms:
                assert any(
                    re.fullmatch(atom, tool) and re.fullmatch(matcher, tool)
                    for tool in samples
                ), (entry.id, atom, matcher)


def test_every_tool_row_gets_every_atom():
    """Without the every-tool row, the matcher is the named atoms plus the
    core duties' atoms, never the whole atom list."""
    solo = RuleEntry(id="solo", on=(("PreToolUse", ("Bash",)),), kind="gate", fail="closed")
    matcher = registry.wiring((solo,))["hooks"]["PreToolUse"][0]["matcher"]
    assert matcher == "Bash|StructuredOutput"  # the row's atom and the duties'
    assert re.fullmatch(matcher, "Bash")
    assert re.fullmatch(matcher, "StructuredOutput")
    assert not re.fullmatch(matcher, "Edit")


def test_generator_refusals():
    """Async under a gate rule; an unknown tool; an unknown event — against
    the real event table, not a patched one."""
    import pytest

    # SubagentStart is the async event: a gate rule there is refused (the
    # platform ignores decisions from background hooks).
    async_gate = RuleEntry(
        id="late-gate", on=(("SubagentStart", ()),), kind="gate", fail="closed",
    )
    with pytest.raises(registry.WiringError) as excused:
        registry.wiring((async_gate,))
    assert "SubagentStart" in str(excused.value)

    # No shipped event is a gate and async at once:
    for event in registry.EVENTS:
        assert not (event.is_async and event.mode == "gate"), event.name

    unknown_tool = RuleEntry(
        id="x", on=(("PreToolUse", ("NoSuchTool",)),), kind="gate", fail="closed",
    )
    with pytest.raises(registry.WiringError):
        registry.wiring((unknown_tool,))

    unknown_event = RuleEntry(id="x", on=(("Nonsense", ()),), kind="recorder", fail="open")
    with pytest.raises(registry.WiringError):
        registry.wiring((unknown_event,))


def test_locked_rows_match_config():
    """HD1: a locked rule is locked where it is declared. The runner's rows
    and config._LOCKED_HOOK_RULES state one set; the lint checks the two
    agree (plan 3.2: a locked rule always gets block)."""
    from vsdd import config

    shipped = {entry.id for entry in registry.RULES}
    locked = {entry.id for entry in registry.RULES if entry.locked}
    declared = set(config._LOCKED_HOOK_RULES)
    assert locked <= declared, "a row marked locked that the loader does not lock"
    assert shipped & declared <= locked, "a locked rule shipped without the lock in its row"


def test_all_names_exist():
    """Every ``__all__`` name of the hooks package exists (plan 3.7: the
    declaration and the object are read as one thing)."""
    import vsdd.hooks
    import vsdd.hooks.caller
    import vsdd.hooks.emit  # noqa: F401
    import vsdd.hooks.gitfs
    import vsdd.hooks.main
    import vsdd.hooks.model
    import vsdd.hooks.policy
    import vsdd.hooks.registry
    import vsdd.hooks.shell

    modules = [
        vsdd.hooks,
        vsdd.hooks.caller,
        vsdd.hooks.emit,
        vsdd.hooks.gitfs,
        vsdd.hooks.main,
        vsdd.hooks.model,
        vsdd.hooks.policy,
        vsdd.hooks.registry,
        vsdd.hooks.shell,
    ]
    for module in modules:
        names = getattr(module, "__all__", None)
        if names is None:
            continue
        for name in names:
            assert hasattr(module, name), f"{module.__name__}.__all__ names missing {name!r}"


def test_guard(tmp_path):
    """The self-guarding command: loud in a factory project, silent elsewhere,
    the down-briefing in brief, and its own launcher's code when present. The
    body runs directly under each interpreter, not only through /bin/sh."""
    import subprocess

    project = helpers.make_project(tmp_path)
    plain = tmp_path / "plain"
    plain.mkdir()
    gone = tmp_path / "gone-plugin"
    fake = tmp_path / "fake-plugin"
    (fake / "py").mkdir(parents=True)
    (fake / "py" / "launch.sh").write_text("#!/bin/sh\nexit 7\n")
    os.chmod(fake / "py" / "launch.sh", 0o755)

    for interpreter in ("sh", "bash", "dash"):
        # Launcher missing, factory project: exit 2 with the reinstall message.
        result = subprocess.run(
            [interpreter, "-c", registry.guard_body("gate")],
            capture_output=True, text=True, timeout=30,
            env=dict(os.environ, CLAUDE_PROJECT_DIR=str(project),
                     CLAUDE_PLUGIN_ROOT=str(gone)),
        )
        assert result.returncode == 2, (interpreter, result.stderr)
        assert "Reinstall the plugin" in result.stderr

        # Launcher missing, any other project: silent 0 (D24).
        result = subprocess.run(
            [interpreter, "-c", registry.guard_body("gate")],
            capture_output=True, text=True, timeout=30,
            env=dict(os.environ, CLAUDE_PROJECT_DIR=str(plain),
                     CLAUDE_PLUGIN_ROOT=str(gone)),
        )
        assert result.returncode == 0 and result.stderr == "" and result.stdout == ""

        # brief: the fixed down-briefing as JSON with systemMessage.
        result = subprocess.run(
            [interpreter, "-c", registry.guard_body("brief")],
            capture_output=True, text=True, timeout=30,
            env=dict(os.environ, CLAUDE_PROJECT_DIR=str(project),
                     CLAUDE_PLUGIN_ROOT=str(gone)),
        )
        assert result.returncode == 0
        rendered = json.loads(result.stdout)
        assert rendered["systemMessage"]
        assert "missing" in rendered["hookSpecificOutput"]["additionalContext"]

        # notify: exit 0, silent.
        result = subprocess.run(
            [interpreter, "-c", registry.guard_body("notify")],
            capture_output=True, text=True, timeout=30,
            env=dict(os.environ, CLAUDE_PROJECT_DIR=str(project),
                     CLAUDE_PLUGIN_ROOT=str(gone)),
        )
        assert result.returncode == 0 and result.stdout == "" and result.stderr == ""

        # Launcher present: its code comes back.
        result = subprocess.run(
            [interpreter, "-c", registry.guard_body("gate")],
            capture_output=True, text=True, timeout=30,
            env=dict(os.environ, CLAUDE_PROJECT_DIR=str(project),
                     CLAUDE_PLUGIN_ROOT=str(fake)),
        )
        assert result.returncode == 7, (interpreter, result.stderr)

    # The wrapped command agrees with the body under sh: one wrapper, no
    # surprise from the quoting.
    result = subprocess.run(
        ["sh", "-c", registry.guard_command("gate")],
        capture_output=True, text=True, timeout=30,
        env=dict(os.environ, CLAUDE_PROJECT_DIR=str(project),
                 CLAUDE_PLUGIN_ROOT=str(gone)),
    )
    assert result.returncode == 2 and "Reinstall the plugin" in result.stderr

    # A trailing slash on HOME, state at home, and a relative start adopt
    # nothing (HD12, SL1-40):
    home = tmp_path / "home"
    (home / ".factory").mkdir(parents=True)
    (home / ".factory" / "factory.toml").write_text("schema = 1\n")
    inside = home / "work"
    inside.mkdir()
    guard = registry.guard_body("gate")

    result = subprocess.run(
        ["sh", "-c", guard], capture_output=True, text=True, timeout=30,
        env=dict(os.environ, CLAUDE_PROJECT_DIR=str(inside),
                 CLAUDE_PLUGIN_ROOT=str(gone), HOME=str(home) + "/"),
    )
    assert result.returncode == 0 and result.stderr == ""

    result = subprocess.run(
        ["sh", "-c", guard], capture_output=True, text=True, timeout=30,
        env=dict(os.environ, CLAUDE_PROJECT_DIR="relative/path",
                 CLAUDE_PLUGIN_ROOT=str(gone)),
    )
    assert result.returncode == 0 and result.stderr == ""
