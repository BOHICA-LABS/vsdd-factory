"""Fail-closed behaviour of the three rules, through the real launcher on a
``launch_copy`` (plan rules.md 3.6, section 7). The loud failure is the point:
a gate or check that switches off without saying so is the worst defect this
project can have."""

from __future__ import annotations

import json
import os
import stat
import subprocess
from pathlib import Path

from tests.hooks import runner_helpers as helpers
from tests.hooks import rule_projects
from tests.hooks.runner_helpers import launch, run_hook
from tests.hooks._helpers import INTERPRETERS
from tests.hooks.test_runner import base_env
from vsdd.hooks import registry

ROWS = [
    {"id": "tests-first",
     "on": (("PreToolUse", ("Edit", "Write", "MultiEdit", "NotebookEdit", "Bash")),),
     "kind": "gate", "fail": "closed", "spike_exempt": True},
    {"id": "merge-guard", "on": (("PreToolUse", ("Bash",)),), "kind": "gate", "fail": "closed"},
    {"id": "step-results",
     "on": (("PreToolUse", ("StructuredOutput",)),),
     "kind": "gate", "fail": "closed", "locked": True, "rewrites": True},
]

EDIT_CALL = ("tool_input.command", "sed -i 's/a/b/' {worktree}/src/lib.rs")
COMMIT_CALL = ("tool_input.command", "git commit --allow-empty -m x")


def build(tmp_path: Path, toml_extra: str = ""):
    """The (project, payload factory) a test drives: a real story project with
    a source tree and one frozen test, and the plugin copy's launcher."""
    launch_sh, plugin = helpers.launch_copy(tmp_path, [(row, None) for row in ROWS])
    p = rule_projects.story_project(tmp_path / "proj")
    rule_projects.freeze(p.root, ["tests/test_sample.py"])
    if toml_extra:
        config = p.root / ".factory" / "factory.toml"
        config.write_text(config.read_text(encoding="utf-8") + toml_extra, encoding="utf-8")

    def run(command: str, interpreter: str = "sh", *, tool: str = "Bash",
            agent_type: str = "vsdd-factory:implementer", payload_override=None,
            cwd=None):
        command = command.replace("{worktree}", str(p.worktree))
        tool_input = {"command": command} if tool == "Bash" else payload_override
        payload = {
            "hook_event_name": "PreToolUse",
            "tool_name": tool,
            "session_id": "sess-fc",
            "agent_type": agent_type,
            "agent_id": "agent-fc",
            "cwd": str(cwd if cwd is not None else p.worktree),
            "tool_input": tool_input,
        }
        environment = dict(os.environ)
        environment["VSDD_EVENTS_DIR"] = str(p.root / "events")
        environment["CLAUDE_PROJECT_DIR"] = str(p.root)
        environment["VSDD_FACTORY_HOOKS_DEBUG"] = "1"
        return subprocess.run(
            [INTERPRETERS[interpreter], str(launch_sh), "hook", "gate"],
            cwd=str(p.root), input=json.dumps(payload), capture_output=True,
            text=True, timeout=120, env=environment,
        )

    def stored(attempt=1):
        from vsdd.state.steps import step_read

        return step_read(p.root, p.run, f"story/{p.story}/implement.{attempt}")

    return p, run, stored, plugin


def test_rows():
    """Each row loads a callable and carries the flags of plan 3.1."""
    flags = {entry.id: entry for entry in registry.RULES}
    for rule_id in ("tests-first", "merge-guard", "step-results"):
        entry = flags[rule_id]
        assert callable(entry.load())
        assert entry.kind == "gate" and entry.fail == "closed"
    assert flags["tests-first"].spike_exempt is True
    assert flags["tests-first"].locked is False
    assert flags["merge-guard"].spike_exempt is False and flags["merge-guard"].locked is False
    assert flags["step-results"].locked is True and flags["step-results"].rewrites is True


def test_import_error_blocks(tmp_path):
    """The loud failure: a rule module that does not import makes the call it
    guards exit 2, naming the rule — every one of the three — and the
    step-results call stores nothing (S4R-rules-26)."""
    broken = "raise RuntimeError('the module is broken')\n"
    for module in ("merge-guard", "tests-first", "step-results"):
        launch_sh, _plugin = helpers.launch_copy(
            tmp_path / module,
            [(row, broken if row["id"] == module else None) for row in ROWS],
        )
        p = rule_projects.story_project(tmp_path / module / "proj")
        if module == "step-results":
            payload = {
                "hook_event_name": "PreToolUse", "tool_name": "StructuredOutput",
                "session_id": "sess-fc", "agent_type": "vsdd-factory:implementer",
                "agent_id": "agent-fc", "cwd": str(p.root),
                "tool_input": rule_projects.step_result(p),
            }
        else:
            payload = {
                "hook_event_name": "PreToolUse", "tool_name": "Bash",
                "session_id": "sess-fc", "agent_type": "vsdd-factory:implementer",
                "cwd": str(p.worktree), "tool_input": {"command": COMMIT_CALL[1]},
            }
        result = launch(INTERPRETERS["sh"], launch_sh, ["hook", "gate"], p.root,
                        stdin_text=json.dumps(payload),
                        env={"VSDD_EVENTS_DIR": str(p.root / "events")})
        assert result.returncode == 2, (module, result.stderr)
        assert "rule_error" in result.stderr, result.stderr
        assert module in result.stderr, result.stderr
        if module == "step-results":
            from vsdd.state.steps import step_read

            assert step_read(p.root, p.run, f"story/{p.story}/implement.1") is None


def test_check_crash_blocks(tmp_path):
    """A ``check`` that raises is the rule's declared failure, exit 2."""
    broken = (
        "def check(ctx, cfg):\n"
        "    raise ValueError('the check exploded')\n"
    )
    launch_sh, plugin = helpers.launch_copy(
        tmp_path, [(row, broken if row["id"] == "tests-first" else None) for row in ROWS]
    )
    p = rule_projects.story_project(tmp_path / "proj")
    payload = {
        "hook_event_name": "PreToolUse", "tool_name": "Edit",
        "session_id": "sess-fc", "agent_type": "vsdd-factory:implementer",
        "cwd": str(p.worktree),
        "tool_input": {"file_path": str(p.worktree / "src" / "lib.rs"),
                       "old_string": "a", "new_string": "b"},
    }
    result = launch(INTERPRETERS["sh"], launch_sh, ["hook", "gate"], p.root,
                    stdin_text=json.dumps(payload),
                    env={"VSDD_EVENTS_DIR": str(p.root / "events")})
    assert result.returncode == 2, result.stderr
    assert "tests-first" in result.stderr and "rule_error" in result.stderr


def test_corrupt_evidence_blocks(tmp_path):
    """A corrupt record never opens the gate: the lookup raises and the rule
    crashes, for a source edit and for a frozen-test edit alike
    (S4R-rules-11)."""
    p, run, stored, _plugin = build(tmp_path)
    rule_projects.red(p)  # a record exists to corrupt
    evidence_dir = p.root / ".factory" / "local" / "evidence" / p.story
    corrupted = sorted(evidence_dir.glob("red.*.json"))
    assert corrupted  # the test is not blind: there was a record to corrupt
    for path in corrupted:
        path.write_text("{not json", encoding="utf-8")
    # A source edit.
    result = run(EDIT_CALL[1])
    assert result.returncode == 2, result.stderr
    assert "tests-first" in result.stderr, result.stderr
    assert "rule_error" in result.stderr, result.stderr
    # And a frozen-test edit, held to the same corrupt record.
    frozen = run("sed -i 's/a/b/' {worktree}/tests/test_sample.py")
    assert frozen.returncode == 2, frozen.stderr
    assert "rule_error" in frozen.stderr, frozen.stderr


def test_missing_policy_blocks(tmp_path):
    """A missing wall table: ``policy_invalid`` for the rule that read it."""
    p, run, stored, plugin = build(tmp_path)
    (plugin / "py" / "vsdd" / "policy" / "walls.toml").unlink()
    # tests-first reads the wall row and the class table: a table fault
    # refuses loudly, never silently.
    result = launch(
        INTERPRETERS["sh"], _launch_sh_of(plugin), ["hook", "gate"], p.root,
        stdin_text=json.dumps({
            "hook_event_name": "PreToolUse", "tool_name": "Edit",
            "session_id": "sess-fc", "agent_type": "vsdd-factory:implementer",
            "cwd": str(p.worktree),
            "tool_input": {"file_path": str(p.worktree / "src" / "lib.rs"),
                           "old_string": "a", "new_string": "b"},
        }),
        env={"VSDD_EVENTS_DIR": str(p.root / "events")},
    )
    assert result.returncode == 2, result.stderr
    assert "policy_invalid" in result.stderr, result.stderr


def _launch_sh_of(plugin: Path) -> Path:
    return plugin / "py" / "launch.sh"


def test_invalid_config_relaxes_nothing(tmp_path):
    """An invalid ``factory.toml``: the defaults hold the gates at full
    strength, with ``develop`` and ``main`` guarded — and tests-first, whose
    path classes fell back to empty, blocks inside the story worktree instead
    of passing unjudged (S4R-rules-10)."""
    p, run, stored, _plugin = build(
        tmp_path, toml_extra="\n[branches]\nintegration = 7\n"
    )
    # The integration branch fell back to develop: still guarded, still full.
    # The commit is judged at the main checkout, which sits on it.
    result = run(COMMIT_CALL[1], cwd=p.root)
    assert result.returncode == 2, result.stderr
    assert "merge-guard/commit_on_guarded" in result.stderr, result.stderr
    # A story-worktree write is judged loudly, not waved through with the
    # empty class table.
    result = run("", tool="Edit", agent_type="vsdd-factory:implementer",
                 payload_override={
                     "file_path": f"{p.worktree}/src/lib.rs",
                     "old_string": "a", "new_string": "b",
                 })
    assert result.returncode == 2, result.stderr
    assert "tests-first/config_invalid" in result.stderr, result.stderr


def test_locked_rule_ignores_strength(tmp_path):
    """``step-results`` is locked: an advise setting does not relax it."""
    p, run, stored, _plugin = build(
        tmp_path, toml_extra='\n[hooks.rules.step-results]\nstrength = "advise"\n'
    )
    payload = {
        "step": {"run": "r-none", "workflow": "story", "subject": p.story,
                 "name": "implement", "attempt": 1, "method": "tdd"},
        "status": "done", "summary": "s", "head": "", "data": {},
    }
    result = run("", tool="StructuredOutput", agent_type="vsdd-factory:implementer",
                 payload_override=payload)
    assert result.returncode == 2, result.stderr
    assert "step-results/run_not_live" in result.stderr, result.stderr


def test_advise_is_announced(tmp_path):
    """Advise strength: the call passes, ``rule.would_block`` is logged, and
    the SessionStart notice names the relaxation."""
    p, run, stored, _plugin = build(
        tmp_path, toml_extra='\n[hooks.rules.tests-first]\nstrength = "advise"\n'
    )
    result = run(EDIT_CALL[1])
    assert result.returncode == 0, result.stderr
    events = _events(p.root, "rule.would_block")
    assert any(entry.get("rule") == "tests-first" and entry.get("code") == "no_red_evidence"
               for entry in events), events

    out, err = _brief(p.root)
    assert "tests-first (advise)" in out, (out, err)


def _events(root: Path, kind: str) -> list[dict]:
    found = []
    for path in sorted((root / "events").rglob("*.jsonl")):
        for line in path.read_text(encoding="utf-8").splitlines():
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(event, dict) and event.get("type") == kind:
                found.append(event)
    return found


def _brief(root: Path) -> tuple[str, str]:
    payload = json.dumps({"hook_event_name": "SessionStart", "session_id": "sess-fc"})
    code, out, err = run_hook(payload, "brief", env=base_env(root))
    assert code == 0, err
    return out, err


def test_wired_event_is_guarded(tmp_path):
    """The launcher missing: the self-guarding command exits 2 in a factory
    project — the wired event's calls cannot pass silently."""
    gone = tmp_path / "gone-plugin"
    gone.mkdir()
    project = helpers.make_project(tmp_path / "proj")
    body = registry.guard_body("gate")
    result = launch(INTERPRETERS["sh"], _fake_launch_sh(tmp_path, body), ["x"], project,
                    stdin_text="", env={"CLAUDE_PLUGIN_ROOT": str(gone)})
    assert result.returncode == 2, (result.returncode, result.stderr)
    assert "Reinstall the plugin" in result.stderr


def _fake_launch_sh(tmp_path: Path, body: str) -> Path:
    fake = tmp_path / "guard.sh"
    fake.write_text(f"#!/bin/sh\n{body}\n", encoding="utf-8")
    fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
    return fake


def test_block_texts_survive_the_wrapper(tmp_path):
    """D16, platform item 40: a block reaches the agent inside the platform's
    wrapper, so its own lines must stand alone inside it — tag, ``Why:`` and
    ``Instead:`` naming the finding, gate or path (S4R-rules-26). The wrapper
    is the one item 40 records: ``PreToolUse:<tool> hook error: [<the hook's
    command line>]: <message>``, a blank line, then the plugin line."""
    p, run, stored, plugin = build(tmp_path)
    command_line = f"{plugin / 'py' / 'launch.sh'} hook gate"

    def wrapped(stderr: str, tool: str = "Bash") -> str:
        """The stderr as the platform delivers it, per platform item 40."""
        return (
            f"PreToolUse:{tool} hook error: [{command_line}]: {stderr}\n"
            "\n"
            f"This hook comes from the vsdd-factory@{plugin} plugin."
        )

    # A merge-guard block, inside the wrapper:
    result = run(COMMIT_CALL[1], cwd=p.root)
    text = wrapped(result.stderr)
    assert "merge-guard/commit_on_guarded" in text
    assert "Why:" in text and "Instead:" in text
    # A tests-first block names the path it judged.
    result = run(EDIT_CALL[1])
    text = wrapped(result.stderr)
    assert "tests-first/no_red_evidence" in text
    assert "src/lib.rs" in text, text
    assert "Why:" in text and "Instead:" in text
    # A step-results refusal names the store's code.
    payload = {
        "step": {"run": "r-none", "workflow": "story", "subject": p.story,
                 "name": "implement", "attempt": 1, "method": "tdd"},
        "status": "done", "summary": "s", "head": "", "data": {},
    }
    result = run("", tool="StructuredOutput", agent_type="vsdd-factory:implementer",
                 payload_override=payload)
    text = wrapped(result.stderr, tool="StructuredOutput")
    assert "step-results/run_not_live" in text
    assert "Why:" in text and "Instead:" in text


def test_store_refusal_stores_nothing_through_launcher(tmp_path):
    """The store refuses: the call is blocked, and no record lands."""
    p, run, stored, _plugin = build(tmp_path)
    payload = {
        "step": {"run": "r-none", "workflow": "story", "subject": p.story,
                 "name": "implement", "attempt": 1, "method": "tdd"},
        "status": "done", "summary": "s", "head": "", "data": {},
    }
    result = run("", tool="StructuredOutput", agent_type="vsdd-factory:implementer",
                 payload_override=payload)
    assert result.returncode == 2, result.stderr
    assert stored() is None
