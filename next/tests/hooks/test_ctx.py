"""The context: payload targets, project resolution, rows (plan 7,
test_ctx.py)."""

from __future__ import annotations

import os

from tests.hooks import runner_helpers as helpers
from tests.hooks.test_runner import base_env
from vsdd.hooks.model import ContextUnavailable, Ctx
from vsdd import project as vsdd_project


def make_ctx(project, payload_overrides=None, env_extra=None, tool="Bash", command="echo"):
    payload = helpers.payload("PreToolUse/bash-main.json")
    payload["cwd"] = str(project)
    payload["tool_name"] = tool
    if tool == "Bash":
        payload["tool_input"] = {"command": command}
    payload.update(payload_overrides or {})
    return Ctx(payload, base_env(project, env_extra), "gate", str(project))


def test_payload_target_and_where(tmp_path):
    project = helpers.make_project(tmp_path)
    (project / "src").mkdir()
    (project / "src" / "lib.rs").write_text("fn main() {}\n")

    edit = make_ctx(project, tool="Edit", command=None,
                    payload_overrides={"tool_input": {"file_path": str(project / "src" / "lib.rs")}})
    assert edit.target == project / "src" / "lib.rs"
    assert edit.where == str(project / "src")

    # A relative path resolves against the payload cwd, and the literal keeps
    # the unresolved spelling:
    (project / "src" / "sub").mkdir()
    rel = make_ctx(project, tool="Edit",
                   payload_overrides={
                       "cwd": str(project / "src"),
                       "tool_input": {"file_path": "sub/x.rs"},
                   })
    assert rel.target_literal == project / "src" / "sub" / "x.rs"
    assert rel.where == str(project / "src" / "sub")

    # Grep names no path: the target is the payload cwd.
    grep = make_ctx(project, tool="Grep", payload_overrides={"tool_input": {"pattern": "x"}})
    assert grep.target == project
    assert grep.where == str(project)

    # Literal versus resolved: a symlink's target is resolved.
    link = project / "src" / "link.rs"
    if not link.exists():
        link.symlink_to(project / "src" / "lib.rs")
    symlinked = make_ctx(project, tool="Edit",
                         payload_overrides={"tool_input": {"file_path": str(link)}})
    assert symlinked.target_literal == link
    assert symlinked.target == project / "src" / "lib.rs"


def test_project_resolution(tmp_path):
    project = helpers.make_project(tmp_path, story="S-1.02")
    inside = make_ctx(project, payload_overrides={
        "tool_input": {"file_path": str(project / ".worktrees" / "S-1.02" / "x.rs")},
        "tool_name": "Edit",
    })
    resolved = inside.project
    assert resolved.root == project
    assert resolved.worktree.kind == "story"
    assert inside.story == "S-1.02"

    # An outside path falls back to the session's project, a real
    # vsdd.project.Project:
    outside_tree = tmp_path / "somewhere-else"
    outside_tree.mkdir(exist_ok=True)
    outside = Ctx(helpers.payload("PreToolUse/edit-main.json") | {
        "cwd": str(outside_tree),
        "tool_input": {"file_path": str(outside_tree / "y.txt")},
    }, base_env(project), "gate", str(project))
    resolved = outside.project
    assert isinstance(resolved, vsdd_project.Project)
    assert resolved.root == project
    assert outside.story is None


def test_project_resolution_spikes(tmp_path):
    spike = helpers.make_project(tmp_path, story="spike-probe")
    ctx = Ctx(helpers.payload("PreToolUse/edit-main.json") | {
        "cwd": str(spike / ".worktrees" / "spike-probe"),
        "tool_input": {"file_path": str(spike / ".worktrees" / "spike-probe" / "n.md")},
        "tool_name": "Edit",
    }, base_env(spike), "gate", str(spike))
    assert ctx.project.worktree.kind == "spike"
    assert ctx.in_spike


def test_state_at_home_adopts_nothing(tmp_path, monkeypatch):
    project = helpers.make_project(tmp_path)
    home = tmp_path / "home"
    (home / ".factory").mkdir(parents=True)
    (home / ".factory" / "factory.toml").write_text("schema = 1\n")
    outside_tree = home / "work"
    outside_tree.mkdir()

    env = base_env(project, {"HOME": str(home)})
    outside = Ctx(helpers.payload("PreToolUse/edit-main.json") | {
        "cwd": str(outside_tree),
        "tool_input": {"file_path": str(outside_tree / "y.txt")},
        "tool_name": "Edit",
    }, env, "gate", str(project))
    assert outside.project.root == project  # never the home-rooted state


def test_rows(tmp_path):
    project = helpers.make_project(tmp_path)
    ctx = make_ctx(project)
    assert ctx.row.name == "main"  # empty agent_type is the main session
    step = Ctx(helpers.payload("PreToolUse/bash-vsdd.json"), base_env(project),
               "gate", str(project))
    assert step.row.name == "other"  # workflow-subagent is other
    stranger = make_ctx(project, payload_overrides={"agent_type": "vsdd-factory:nobody"})
    assert stranger.row.name == "other"
    agent = make_ctx(project, payload_overrides={"agent_type": "vsdd-factory:implementer"})
    assert agent.row.name == "vsdd-factory:implementer"
    spike_agent = make_ctx(project, payload_overrides={"agent_type": "vsdd-factory:spike-runner"})
    assert spike_agent.row.spike is True  # the spike row


def test_new_content_raises_and_classes_do_not(tmp_path):
    project = helpers.make_project(tmp_path)
    ctx = make_ctx(project)
    raised = None
    try:
        ctx.new_content
    except ContextUnavailable as exc:
        raised = exc
    assert raised is not None
    # classes is built by hook-runner-b: a Bash call names no file, so the
    # answer is an empty tuple, never a raise.
    assert ctx.classes == ()


def test_ctx_agrees_with_the_shared_root_table(tmp_path, monkeypatch):
    """The runner's project resolution is the shared find-root table
    (SL1-48): whatever the shell and ``find_project_root`` agree on, the
    context resolves the same, with the session's project as the fallback."""
    from ..root_cases import build_table

    sandbox = tmp_path / "root-table"
    session = helpers.make_project(tmp_path / "session")
    for case in build_table(sandbox):
        env = base_env(session, {"HOME": str(case.home), "VSDD_FACTORY_ROOT": str(session)})
        ctx = Ctx(helpers.payload("PreToolUse/edit-main.json") | {
            "cwd": str(case.start),
            "tool_input": {"file_path": str(case.start / "f.txt")},
            "tool_name": "Edit",
        }, env, "gate", str(session))
        resolved = ctx.project
        if case.expected is not None:
            assert resolved.root == case.expected, case.name
        else:
            assert resolved.root == session, case.name  # the session's fallback
