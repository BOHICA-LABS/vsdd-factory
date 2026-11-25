"""The checkpoint command body (plan survey.md section 7; D1, D8, D9, D18,
D19)."""

from __future__ import annotations

import json

from . import story_ready
from .story_ready import make_project as make_story_project

from vsdd.state.artifacts import changed_paths
from vsdd.state.cycles import closure_read, closure_update
from vsdd.state.gate_evidence import evidence_write
from vsdd.state.locks import lock_entries
from vsdd.state.runs import run_read
from vsdd.state.steps import store_step
from vsdd.state.worktrees import remove_story, story_path

STORY = "S-9.01"


def _survey(root, session=story_ready.SESSION):
    key = story_ready.write_caller(root, session_id=session)
    from pathlib import Path

    environ = {"HOME": str(Path(root).parent), "VSDD_CALLER": key}
    code, out, err = story_ready.run_vsdd(
        root, "survey", "story", "--subject", STORY, "--json", environ=environ
    )
    assert code == 0, err
    return json.loads(out)


def _checkpoint(root, run_id, *extra, session=story_ready.SESSION):
    key = story_ready.write_caller(root, session_id=session)
    from pathlib import Path

    environ = {"HOME": str(Path(root).parent), "VSDD_CALLER": key}
    code, out, err = story_ready.run_vsdd(
        root, "checkpoint", "story", "--run", run_id, *extra, "--json", environ=environ
    )
    data = json.loads(out) if out else {}
    return code, data, err


def _step(root, run_id, head):
    store_step(
        root,
        {
            "step": {
                "run": run_id,
                "workflow": "story",
                "name": "tests",
                "subject": STORY,
                "attempt": 1,
                "method": "regression-test",
            },
            "status": "done",
            "summary": "the failing tests are written",
            "head": head,
            "data": {},
        },
        "test-writer",
        head,
    )


def test_checkpoint_closes_delivered_story(tmp_path):
    root = make_story_project(tmp_path)
    survey = _survey(root)
    run_id = survey["run_id"]
    head = "a" * 40
    _step(root, run_id, head)
    closure_update(
        root,
        "c-1",
        "story",
        STORY,
        lambda record: record.setdefault("delivery", {}).update({"merged_sha": head, "head": head}),
    )
    remove_story(root, STORY)

    code, data, _err = _checkpoint(root, run_id, "--subject", STORY)
    assert code == 0 and data["ok"] is True and data["error"] == ""
    assert data["status"] == "done" and data["progress"] is True
    assert data["next"] == {"kind": "workflow", "command": "wave", "args": {}}
    assert data["decisions"] == []

    closure = closure_read(root, "c-1", "story", STORY)
    assert closure["state"] == "closed" and closure["position"] == "delivered"
    assert closure["stages"]["deliver"]["state"] == "done"
    assert closure["stages"]["deliver"]["merged_sha"] == head
    # The close mark is the checkpoint's ISO stamp, as every closed stamp in
    # the record family is (finding S3R-survey-19).
    assert isinstance(closure["closed"], str) and closure["closed"].endswith("Z")
    record = run_read(root, run_id)
    assert record["ended"] and record["agents"] == 1  # D18: the run's step records
    assert lock_entries(root) == []
    assert changed_paths(root) == []  # the checkpoint's flush committed the closure
    # Finding D33-21: the tracked closure carries no machine-local path.
    assert str(root) not in json.dumps(closure)


def test_checkpoint_undelivered_is_partial(tmp_path):
    root = make_story_project(tmp_path)
    survey = _survey(root)
    run_id = survey["run_id"]
    head = story_head(root)
    _red(root, head)

    code, data, _err = _checkpoint(root, run_id)
    assert code == 0 and data["ok"] is True
    assert data["status"] == "partial" and data["progress"] is True
    closure = closure_read(root, "c-1", "story", STORY)
    assert closure["stages"]["tests"]["state"] == "done"  # the position's stage (D9)
    assert closure["stages"]["tests"]["head"] == head
    assert run_read(root, run_id)["ended"]


def story_head(root):
    from vsdd.state.store import git as run_git

    return run_git(story_path(root, STORY), "rev-parse", "HEAD")


def _red(root, sha):
    return evidence_write(
        root,
        {
            "gate": "red",
            "subject": STORY,
            "sha": sha,
            "passed": True,
            "frozen": [{"file": "tests/test_x.py", "sha": "f" * 64}],
            "tests": ["tests/test_x.py::test_x"],
            "criteria": {"AC-1": ["tests/test_x.py::test_x"]},
        },
    )


def test_checkpoint_outcome_values(tmp_path):
    root = make_story_project(tmp_path)
    survey = _survey(root)
    run_id = survey["run_id"]
    code, data, _err = _checkpoint(root, run_id, "--outcome", "failed")
    assert code == 0 and data["status"] == "failed" and data["ok"] is True

    root2 = make_story_project(tmp_path / "inert")
    survey2 = _survey(root2)
    code2, data2, _err = _checkpoint(root2, survey2["run_id"], "--outcome", "done")
    assert code2 == 0 and data2["status"] == "partial"  # done changes nothing: the status is computed

    root3 = make_story_project(tmp_path / "banana")
    survey3 = _survey(root3)
    code3, data3, _err = _checkpoint(root3, survey3["run_id"], "--outcome", "banana")
    assert code3 == 2
    assert data3["status"] == "failed" and data3["ok"] is False  # the whole object, every time


def test_checkpoint_subject_must_match_run(tmp_path):
    root = make_story_project(tmp_path)
    survey = _survey(root)
    run_id = survey["run_id"]
    code, data, err = _checkpoint(root, run_id, "--subject", "S-9.99")
    assert code == 2
    assert "S-9.99" in err
    assert run_read(root, run_id)["ended"] == ""  # nothing was written
    assert lock_entries(root)
    closure = closure_read(root, "c-1", "story", STORY)
    assert closure is not None
    assert closure["state"] == "open"  # the checkpoint changed nothing
    assert "deliver" not in closure["stages"]


def test_checkpoint_twice_is_harmless(tmp_path):
    root = make_story_project(tmp_path)
    survey = _survey(root)
    run_id = survey["run_id"]
    code_one, first, _err = _checkpoint(root, run_id)
    code_two, second, _err = _checkpoint(root, run_id)
    assert code_one == code_two == 0
    assert first["status"] == second["status"] == "partial"
    assert second["progress"] is False  # an ended run writes nothing (D8)
    assert second["ok"] is True


def test_checkpoint_releases_lock_when_flush_fails(tmp_path, monkeypatch):
    root = make_story_project(tmp_path)
    survey = _survey(root)
    run_id = survey["run_id"]
    from vsdd.state import commits

    monkeypatch.setattr(
        commits, "flush", lambda root, reason, run="": {
            "ok": False, "state": "failed", "commit": "", "files": [],
            "rejected": [], "error": "disk full",
        }
    )
    code, data, _err = _checkpoint(root, run_id)
    assert code == 1 and data["ok"] is False
    assert "disk full" in data["error"]
    assert lock_entries(root) == []  # the lock is still released
    assert run_read(root, run_id)["ended"]


def test_checkpoint_broken_mount_still_releases(tmp_path):
    from vsdd.state.store import git as run_git

    root = make_story_project(tmp_path)
    survey = _survey(root)
    run_id = survey["run_id"]
    run_git(root / ".factory", "checkout", "-q", "--detach", "HEAD")  # the mount breaks
    code, data, _err = _checkpoint(root, run_id)
    assert code == 1 and data["ok"] is False
    assert "mounted" in data["error"]
    assert lock_entries(root) == []  # a broken mount must not strand a lock
    assert run_read(root, run_id)["ended"]


def test_checkpoint_unknown_run(tmp_path):
    root = make_story_project(tmp_path)
    code, data, err = _checkpoint(root, "r-nowhere")
    assert code == 2
    assert data["status"] == "failed" and data["ok"] is False
    assert "r-nowhere" in err


def test_checkpoint_stops_products(tmp_path, monkeypatch):
    root = make_story_project(tmp_path)
    survey = _survey(root)
    run_id = survey["run_id"]
    stopped = []
    from vsdd.gates import jobs

    monkeypatch.setattr(jobs, "stop_products", lambda root, **kw: stopped.append(kw) or [])
    code, data, _err = _checkpoint(root, run_id)
    assert code == 0
    assert stopped == [{"subject": STORY}]


def test_checkpoint_corrupt_closure_still_releases(tmp_path):
    """A closure that does not parse is a fault in the answer, not an escape:
    ok false, exit 1, the lock empty and the run ended (S3R-survey-1)."""
    root = make_story_project(tmp_path)
    survey = _survey(root)
    run_id = survey["run_id"]
    path = root / ".factory" / "cycles" / "c-1" / "units" / "story.S-9.01.json"
    path.write_text("{ not json", encoding="utf-8")
    code, data, _err = _checkpoint(root, run_id)
    assert code == 1 and data["ok"] is False
    assert "S-9.01" in data["error"]
    assert lock_entries(root) == []
    assert run_read(root, run_id)["ended"]


def test_checkpoint_busy_store_still_releases(tmp_path, monkeypatch):
    """A StoreBusy out of the closure write rides in the same try: the
    release in the finally still ends the run (S3R-survey-1)."""
    from vsdd.state import cycles
    from vsdd.state.store import StoreBusy

    root = make_story_project(tmp_path)
    survey = _survey(root)
    run_id = survey["run_id"]

    def busy(project, cycle, workflow, subject, mutate):
        raise StoreBusy("busy", "flush.lock is held by another process")

    monkeypatch.setattr(cycles, "closure_update", busy)
    code, data, _err = _checkpoint(root, run_id)
    assert code == 1 and data["ok"] is False
    assert "flush.lock" in data["error"]
    assert lock_entries(root) == []
    assert run_read(root, run_id)["ended"]


def test_checkpoint_run_without_stories_is_partial(tmp_path):
    """all([]) is not done: a run with no stories is partial (S3R-survey-14)."""
    from vsdd.state.runs import run_open

    root = make_story_project(tmp_path)
    run_id = run_open(
        root, workflow="story", cycle="c-1", session=story_ready.SESSION, subject=STORY
    )["id"]
    code, data, _err = _checkpoint(root, run_id)
    assert code == 0 and data["ok"] is True
    assert data["status"] == "partial"
    assert run_read(root, run_id)["ended"]


def test_checkpoint_ended_run_without_stories_is_partial(tmp_path):
    """The same for a run whose record is already ended (D8's read path)."""
    from vsdd.state.runs import run_end, run_open

    root = make_story_project(tmp_path)
    run_id = run_open(
        root, workflow="story", cycle="c-1", session=story_ready.SESSION, subject=STORY
    )["id"]
    run_end(root, run_id)
    code, data, _err = _checkpoint(root, run_id)
    assert code == 0 and data["ok"] is True
    assert data["status"] == "partial" and data["progress"] is False


def test_checkpoint_unusable_run_id_is_a_refusal(tmp_path):
    """checkpoint --run ../x is the unknown-run refusal, not an internal
    error (finding S3R-survey-12)."""
    root = make_story_project(tmp_path)
    code, data, err = _checkpoint(root, "../x")
    assert code == 2
    assert "no run" in err
    assert data["status"] == "failed" and data["ok"] is False
