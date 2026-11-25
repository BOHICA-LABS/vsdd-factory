"""The step record store (plan records.md 7): the path and the added fields;
ended run, over cap, bad identity and holdout path refuse; the same agent
replaces its own result, another agent is refused; the list is ordered and
corrupt files raise."""

from __future__ import annotations

import pytest

from .state_fixtures import make_project
from vsdd.state import runs, steps
from vsdd.state.store import StoreCorrupt, StoreRefused


def _result(run_id, subject="S-9.01", name="implement", attempt=1, workflow="story", **extra):
    step = {"run": run_id, "workflow": workflow, "name": name, "subject": subject, "attempt": attempt, "method": "tdd"}
    step.update(extra)
    return {"step": step, "status": "done", "summary": "ok", "head": "abc", "data": {}}


def test_store_step(tmp_path):
    root = make_project(tmp_path)
    run = runs.run_open(root, workflow="story", cycle="product-1", session="s", subject="S-9.01")
    stored = steps.store_step(root, _result(run["id"]), "builder", "head-abc", agent_id="ag-1")
    assert stored["agent_type"] == "builder"
    assert stored["worktree_head"] == "head-abc"
    assert stored["agent_id"] == "ag-1"
    path = (
        root
        / ".factory"
        / "local"
        / "runs"
        / run["id"]
        / "steps"
        / "story"
        / "S-9.01"
        / "implement.1.json"
    )
    assert path.is_file()
    key = steps.step_key("story", "S-9.01", "implement", 1)
    assert steps.step_read(root, run["id"], key)["summary"] == "ok"
    assert steps.run_has_steps(root, run["id"])

    # An empty subject becomes the directory _ (decision D2).
    empty = runs.run_open(root, workflow="spec", cycle="product-1", session="s", subject="")
    steps.store_step(root, _result(empty["id"], subject="", workflow="spec"), "clerk", "", agent_id="ag-1")
    assert (root / ".factory" / "local" / "runs" / empty["id"] / "steps" / "spec" / "_").is_dir()

    # An ended run stores nothing.
    runs.run_end(root, empty["id"])
    with pytest.raises(StoreRefused) as exc:
        steps.store_step(root, _result(empty["id"], subject="", workflow="spec"), "clerk", "", agent_id="ag-1")
    assert exc.value.code == "run_not_live"

    # Over the cap.
    fat = _result(run["id"], name="fat")
    fat["data"] = {"blob": "x" * 70_000}
    with pytest.raises(StoreRefused) as exc:
        steps.store_step(root, fat, "builder", "", agent_id="a")
    assert exc.value.code == "over_cap"

    # Bad identity: no attempt, no run, no name.
    with pytest.raises(StoreRefused) as exc:
        steps.store_step(root, {"status": "done", "summary": "", "head": "", "data": {}}, "b", "")
    assert exc.value.code == "bad_identity"
    broken = _result(run["id"], name="broken")
    broken["step"].pop("attempt")
    with pytest.raises(StoreRefused) as exc:
        steps.store_step(root, broken, "b", "", agent_id="a")
    assert exc.value.code == "bad_identity"

    # A finding located under .factory/holdout/ needs the evaluation file,
    # unbuilt; the rule reads the finding's location, not any string anywhere.
    walled = _result(run["id"], name="scenario-reviewer")
    walled["data"] = {"findings": [{"key": "F-1", "location": {"path": ".factory/holdout/HS-001.md"}}]}
    with pytest.raises(StoreRefused) as exc:
        steps.store_step(root, walled, "reviewer", "", agent_id="a")
    assert exc.value.code == "walled_not_built"

    # The state-relative spelling is walled too, and dot segments are
    # normalised before the wall test: they do not carry a path out
    # (S4R-rules-25).
    for path in ("holdout/HS-002.md", ".factory/./holdout/HS-003.md"):
        walled2 = _result(run["id"], name="scenario-reviewer")
        walled2["data"] = {"findings": [{"key": "F-2", "location": {"path": path}}]}
        with pytest.raises(StoreRefused) as exc:
            steps.store_step(root, walled2, "reviewer", "", agent_id="a")
        assert exc.value.code == "walled_not_built", path

    # A summary that merely mentions the wall is not a walled finding: the
    # step stores, or a review mentioning the wall would wedge the step.
    mention = _result(run["id"], name="mentions-wall")
    mention["summary"] = "checked .factory/holdout/ and found nothing"
    mention["data"] = {"note": "holdout/ is empty"}
    stored_mention = steps.store_step(root, mention, "reviewer", "", agent_id="a")
    assert stored_mention["summary"] == "checked .factory/holdout/ and found nothing"

    # The kept schema check: a result that fails step-record refuses before
    # anything is written.
    bad_schema = _result(run["id"], name="bad-status")
    bad_schema["status"] = "hardly"
    target = (
        root / ".factory" / "local" / "runs" / run["id"] / "steps" / "story" / "S-9.01"
    )
    before = sorted(p.name for p in target.rglob("*.json")) if target.is_dir() else []
    with pytest.raises(StoreRefused) as exc:
        steps.store_step(root, bad_schema, "builder", "", agent_id="a")
    assert exc.value.code == "schema_invalid"
    after = sorted(p.name for p in target.rglob("*.json")) if target.is_dir() else []
    assert before == after, "a schema-refused step writes no file"


def test_step_collision(tmp_path):
    root = make_project(tmp_path)
    run = runs.run_open(root, workflow="story", cycle="product-1", session="s", subject="S-9.01")
    steps.store_step(root, _result(run["id"]), "builder", "h", agent_id="ag-1")
    # Another agent refused (decision D4).
    with pytest.raises(StoreRefused) as exc:
        steps.store_step(root, _result(run["id"]), "builder", "h", agent_id="ag-2")
    assert exc.value.code == "step_exists"
    # The same agent replaces its result.
    revised = _result(run["id"], name="implement")
    revised["summary"] = "revised after the platform refused it"
    replaced = steps.store_step(root, revised, "builder", "h", agent_id="ag-1")
    assert replaced["summary"] == "revised after the platform refused it"
    # Identical content replaces when the hook's own additions match too —
    # the whole result: artifacts and grounded included, not only the step
    # object and summary. This is the no-agent rewrite's path: both writes
    # carry an empty agent id.
    full = _result(run["id"], name="full-key")
    full["artifacts"] = [".factory/stories/E-1/S-1.01.md"]
    full["grounded"] = True
    full["data"] = {"note": "checked"}
    stored_full = steps.store_step(root, full, "reviewer", "h", agent_id="")
    assert stored_full["agent_id"] == ""
    again = steps.store_step(root, dict(full), "reviewer", "h", agent_id="")
    assert again["summary"] == "ok"
    # One differing field among those is a different result: refused.
    drifted = dict(full)
    drifted["grounded"] = False
    with pytest.raises(StoreRefused) as exc:
        steps.store_step(root, drifted, "reviewer", "h", agent_id="")
    assert exc.value.code == "step_exists"


def test_store_step_unknown_run_is_run_not_live(tmp_path):
    """An unknown run is not a live run: the promised store_step code."""
    root = make_project(tmp_path)
    with pytest.raises(StoreRefused) as exc:
        steps.store_step(root, _result("r-19700101-0000-zz"), "builder", "", agent_id="a")
    assert exc.value.code == "run_not_live"


def test_steps_list(tmp_path):
    root = make_project(tmp_path)
    late = runs.run_open(root, workflow="story", cycle="c", session="s", subject="S-9.01", now="2026-11-24T10:00:00Z")
    early = runs.run_open(root, workflow="spec", cycle="c", session="s", subject="", now="2026-11-24T09:00:00Z")
    steps.store_step(root, _result(late["id"]), "b", "", agent_id="a")
    steps.store_step(root, _result(early["id"], subject="", workflow="spec"), "b", "", agent_id="a")

    listed = steps.steps_list(root)
    assert [r["step"]["workflow"] for r in listed] == ["spec", "story"]

    one_run = steps.steps_list(root, late["id"])
    assert [r["step"]["name"] for r in one_run] == ["implement"]
    assert [r["step"]["workflow"] for r in steps.steps_list(root, late["id"], workflow="spec")] == []

    # The key is the prelude's spelling.
    key = steps.step_key("story", "S-9.01", "implement", 1)
    assert steps.step_read(root, late["id"], key)["step"]["attempt"] == 1
    assert steps.step_read(root, late["id"], "story/S-9.01/implement.9") is None

    # A corrupt record raises; nothing is silently dropped.
    victim = (
        root
        / ".factory"
        / "local"
        / "runs"
        / late["id"]
        / "steps"
        / "story"
        / "S-9.01"
        / "broken.1.json"
    )
    victim.write_text("{not json", encoding="utf-8")
    with pytest.raises(StoreCorrupt):
        steps.steps_list(root, late["id"])
