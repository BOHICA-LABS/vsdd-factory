"""Outside a factory project the hooks are inert ([[topic:hooks-inert]]):
nothing blocked, nothing written (ORCH-6). Before the runner decided this
once at the top, the first rules registered for Bash crashed on a ``None``
project and blocked every call with a rule error."""

from __future__ import annotations

import json

from tests.hooks import runner_helpers as helpers
from tests.hooks._helpers import INTERPRETERS
from tests.hooks.test_runner import base_env, bash_call

PLUGIN = helpers.PLUGIN
LAUNCH_SH = PLUGIN / "py" / "launch.sh"


def _git_payload(outside) -> dict:
    """A guarded-looking git command: blocked wherever the rule judges it."""
    return bash_call(outside, "git push --force origin main")


def _edit_payload(outside) -> dict:
    return {
        "hook_event_name": "PreToolUse",
        "tool_name": "Edit",
        "session_id": "sess_inert",
        "cwd": str(outside),
        "tool_input": {"file_path": str(outside / "src" / "a.py")},
    }


def _step_payload(outside) -> dict:
    """A step result with a full identity: stored when the rule runs."""
    return {
        "hook_event_name": "PreToolUse",
        "tool_name": "StructuredOutput",
        "session_id": "sess_inert",
        "agent_id": "wf-1",
        "agent_type": "workflow-subagent",
        "cwd": str(outside),
        "tool_input": {
            "step": {
                "run": "r-inert",
                "workflow": "wave",
                "subject": "W-1",
                "name": "implement",
                "attempt": 1,
                "method": "tdd",
            },
            "status": "done",
            "summary": "the step went",
            "head": "",
            "data": {},
        },
    }


def test_call_outside_a_project_is_inert_through_the_launcher(tmp_path):
    outside = tmp_path / "plain"
    outside.mkdir()
    for payload in (_git_payload(outside), _edit_payload(outside),
                    _step_payload(outside)):
        for interpreter in INTERPRETERS.values():
            result = helpers.launch(
                interpreter, LAUNCH_SH, ["hook", "gate"], outside,
                stdin_text=json.dumps(payload),
            )
            assert result.returncode == 0, (interpreter, result.stderr)
            assert result.stdout == "", (interpreter, result.stdout)
            assert result.stderr == "", (interpreter, result.stderr)
    assert not (outside / ".factory").exists()  # and no file appears


def test_call_outside_a_project_is_inert_in_process(tmp_path):
    """In process, with the runner's root pointing at a directory in no
    factory project, every rule-selecting call passes with exit 0, no output
    and no log write: decided once, before any rule is selected (ORCH-6)."""
    outside = tmp_path / "plain"
    outside.mkdir()
    for payload in (_git_payload(outside), _edit_payload(outside),
                    _step_payload(outside)):
        env = base_env(outside)
        code, out, err = helpers.run_hook(payload, "gate", env=env)
        assert (code, out, err) == (0, "", ""), payload["tool_name"]
    assert not (outside / ".factory").exists()
    assert not (outside / "events").exists()  # no log write either


def test_blocked_command_still_blocks_inside_a_project(tmp_path):
    """The control beside the inertness: the same git command inside a
    factory project is blocked, so the assertions above bite."""
    project = tmp_path / "proj"
    (project / ".factory").mkdir(parents=True)
    (project / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
    code, out, err = helpers.run_hook(_git_payload(project), "gate",
                                      env=base_env(project))
    assert code == 10, err
    assert "push_to_guarded" in err, err
