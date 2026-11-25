"""One test per built route row, and the loud guard (plan survey.md D12,
section 7; rows 1, 2 and 12 to 16 of tables/routes.toml)."""

from __future__ import annotations

from . import story_ready
from ._repo import set_keys
from .story_ready import make_project as make_story_project

from vsdd.state.cycles import closure_update
from vsdd.state.decisions import decision_open
from vsdd.state.locks import lock_scope, run_begin
from vsdd.state.route import next_step

STORY = "S-9.01"
NOT_BUILT = "this project state needs a route row that is not built yet"




def test_row_not_initialised(tmp_path):
    bare = tmp_path / "empty"
    bare.mkdir()
    next_move = next_step(bare)
    assert next_move["kind"] == "human"
    assert next_move["command"] == "/vsdd-factory:setup"
    assert "not initialised" in next_move["args"]["reason"]


def test_row_live_lock(tmp_path):
    root = make_story_project(tmp_path)
    run_begin(
        root,
        scope=lock_scope(root),
        workflow="story",
        subject=STORY,
        session="sess-live-lock1",
        cycle="c-1",
    )
    next_move = next_step(root)
    assert next_move["kind"] == "human"
    assert next_move["command"] == ""
    assert "run lock" in next_move["args"]["reason"]


def test_row_spec(tmp_path):
    root = make_story_project(tmp_path)
    # The approval story_ready ran is dropped: an open spec decision replaces
    # the approved one, which sends the project to spec.
    decision_open(root, cycle="c-1", kind="spec", packet="# Revised specification")
    next_move = next_step(root)
    assert next_move == {"kind": "workflow", "command": "spec", "args": {}}


def test_row_missing_commands(tmp_path):
    root = make_story_project(tmp_path)
    set_keys(root, "commands", test="")
    next_move = next_step(root)
    assert next_move["kind"] == "human"
    assert next_move["command"] == ""
    assert "factory.toml" in next_move["args"]["reason"]


def test_row_plan(tmp_path):
    root = make_story_project(tmp_path)
    # No plan closure yet: the plan unit is not closed.
    import shutil

    shutil.rmtree(root / ".factory" / "cycles" / "c-1" / "units")
    next_move = next_step(root)
    assert next_move == {"kind": "workflow", "command": "plan", "args": {}}


def test_row_waiting_on_decisions(tmp_path):
    root = make_story_project(tmp_path)
    decision_open(
        root,
        cycle="c-1",
        kind="blocked",
        subject=STORY,
        reason="environment",
        packet="# Blocked on the environment",
    )
    next_move = next_step(root)
    assert next_move["kind"] == "human"
    assert next_move["command"] == ""
    assert "blocked.S-9.01" in next_move["args"]["reason"]
    # D10: the ids are listed in args.decisions, not only in the reason text.
    ids = [qualified.split("@", 1)[0] for qualified in next_move["args"]["decisions"]]
    assert ids == ["blocked.S-9.01"]


def test_row_waiting_counts_only_runnable_stories_as_runnable(tmp_path):
    """A story whose dependency is undelivered is not 'could run': the
    runnable test is preconditions.check with hooks_alive=None, so the
    decisions row still fires while another story waits (S3R-survey-10)."""
    root = make_story_project(tmp_path)
    closure_update(root, "c-1", "plan", "", lambda record: record["waves"].update({"w1": [STORY, "S-9.02"]}))
    rel = story_ready._story_file(root, "S-9.02", cycle="c-1")
    text = (root / rel).read_text(encoding="utf-8").replace(
        "contracts: [BC-1.00.001]", "contracts: [BC-1.00.001]\ndepends_on: [S-9.99]"
    )
    (root / rel).write_text(text, encoding="utf-8")
    decision_open(
        root, cycle="c-1", kind="blocked", subject=STORY,
        reason="environment", packet="# Blocked on the environment",
    )
    next_move = next_step(root)
    assert next_move["kind"] == "human"
    assert "blocked.S-9.01" in next_move["args"]["reason"]


def test_row_wave(tmp_path):
    root = make_story_project(tmp_path)
    next_move = next_step(root)
    assert next_move == {"kind": "workflow", "command": "wave", "args": {}}


def test_unbuilt_state_is_loud(tmp_path):
    import shutil

    # No cycle open: the brief and spec decisions went with it, so neither
    # the spec row nor the plan row can match, and the guard speaks.
    root = make_story_project(tmp_path / "no-cycle")
    shutil.rmtree(root / ".factory" / "cycles" / "c-1")
    next_move = next_step(root)
    assert next_move == {"kind": "human", "command": "", "args": {"reason": NOT_BUILT}}

    # A queued request.
    root = make_story_project(tmp_path / "queued")
    requests = root / ".factory" / "requests"
    requests.mkdir(parents=True, exist_ok=True)
    (requests / "CR-001.md").write_text("A change request\n", encoding="utf-8")
    assert next_step(root)["args"]["reason"] == NOT_BUILT

    # A hub layout.
    root = make_story_project(tmp_path / "hub")
    set_keys(root, "project", layout="hub")
    assert next_step(root)["args"]["reason"] == NOT_BUILT

    # Every wave closed.
    root = make_story_project(tmp_path / "closed")
    closure_update(root, "c-1", "wave", "w1", lambda record: record.update({"state": "closed"}))
    assert next_step(root)["args"]["reason"] == NOT_BUILT
