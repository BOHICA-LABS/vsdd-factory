"""The survey command body (plan survey.md section 7; D2, D3, D7, D11)."""

from __future__ import annotations

import json

from . import story_ready
from .story_ready import make_project as make_story_project

from vsdd.state import steps
from vsdd.state.artifacts import changed_paths
from vsdd.state.cycles import closure_update
from vsdd.state.locks import lock_entries
from vsdd.state.runs import run_read, runs_list
from vsdd.state.store import write_record
from vsdd.state.worktrees import remove_story, story_path

STORY = "S-9.01"
OTHER = "sess-other-8888"


def _env(root, session=story_ready.SESSION):
    key = story_ready.write_caller(root, session_id=session)
    from pathlib import Path

    home = Path(root).parent  # the walk to the project root stops below home
    return {"HOME": str(home), "VSDD_CALLER": key}


def _survey(root, environ, subject=STORY):
    code, out, err = story_ready.run_vsdd(
        root, "survey", "story", "--subject", subject, "--json", environ=environ
    )
    data = json.loads(out) if out else {}
    return code, data, err


def test_survey_story_ready(tmp_path):
    root = make_story_project(tmp_path)
    code, data, err = _survey(root, _env(root))
    assert code == 0, err
    assert data["ok"] is True and data["error"] == ""
    run = run_read(root, data["run_id"])
    assert run["workflow"] == "story" and run["subject"] == STORY and run["cycle"] == "c-1"
    entries = lock_entries(root)
    assert any(entry["session"] == story_ready.SESSION for entry in entries)
    assert any(entry["run"] == data["run_id"] for entry in entries)
    assert story_path(root, STORY).is_dir()
    unit = data["units"][0]
    assert unit["id"] == STORY and unit["position"] == "ready"
    assert data["scope"] == "full"  # the sample cycle's baseline is empty
    assert data["limits"]["agents"]
    # A story the open wave admits is admitted by the plan, not by the run
    # (a run id means an out-of-plan admission through gate plan --story).
    from vsdd.state.cycles import closure_read

    closure = closure_read(root, "c-1", "story", STORY)
    assert closure["admitted_by"] == "plan"


def test_survey_without_marker_takes_nothing(tmp_path):
    root = make_story_project(tmp_path)
    code, data, _err = _survey(root, {"HOME": str(tmp_path)})  # no VSDD_CALLER
    assert code == 1 and data["ok"] is False
    assert "no caller marker" in data["error"]
    assert data["run_id"] == "" and data["units"] == []
    assert not (root / ".factory" / "local" / "runs").exists()
    assert not (root / ".factory" / "local" / "lock.json").exists()
    assert not story_path(root, STORY).exists()


def test_survey_unmet_lists_all_takes_no_lock(tmp_path):
    from ._repo import set_keys

    root = make_story_project(tmp_path)
    set_keys(root, "commands", test="", test_report="")
    code, data, _err = _survey(root, _env(root), subject="S-9.77")
    assert code == 1 and data["ok"] is False
    assert "commands_configured" in data["error"]
    assert "story_admitted" in data["error"]
    assert not (root / ".factory" / "local" / "lock.json").exists()
    assert data["next"]["kind"]  # the route's answer stands even on a refusal


def test_survey_lock_held_by_other_session(tmp_path):
    root = make_story_project(tmp_path)
    code, first, _err = _survey(root, _env(root))
    assert code == 0
    code, second, _err = _survey(root, _env(root, session=OTHER))
    assert code == 1 and second["ok"] is False
    for needle in ("story", STORY, first["run_id"], story_ready.SESSION, "seen"):
        assert needle in second["error"]


def test_reentry_before_first_step_same_run(tmp_path):
    root = make_story_project(tmp_path)
    code_one, first, _err = _survey(root, _env(root))
    code_two, second, _err = _survey(root, _env(root))
    assert code_one == code_two == 0
    assert second["run_id"] == first["run_id"]
    assert run_read(root, first["run_id"])["ended"] == ""


def test_reentry_after_first_step_takes_over(tmp_path):
    root = make_story_project(tmp_path)
    _code, first, _err = _survey(root, _env(root))
    head = "a" * 40
    steps.store_step(
        root,
        {
            "step": {
                "run": first["run_id"],
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
    _code, second, _err = _survey(root, _env(root))
    assert second["run_id"] != first["run_id"]
    ended = run_read(root, first["run_id"])
    assert ended["ended"]


def test_stale_lock_taken_over_old_run_ended(tmp_path):
    root = make_story_project(tmp_path)
    _code, first, _err = _survey(root, _env(root))
    entries = lock_entries(root)
    for entry in entries:
        entry["seen"] = "2026-01-01T00:00:00Z"
    write_record(root / ".factory" / "local" / "lock.json", {"locks": entries}, cap=None, schema="lock")
    code, second, _err = _survey(root, _env(root, session="sess-fresh-beef"))
    assert code == 0
    assert second["run_id"] != first["run_id"]
    assert run_read(root, first["run_id"])["ended"]


def test_survey_records_unit_in_run(tmp_path):
    root = make_story_project(tmp_path)
    _code, data, _err = _survey(root, _env(root))
    assert run_read(root, data["run_id"])["stories"] == [STORY]


def test_survey_flushes_settled_decision(tmp_path):
    root = make_story_project(tmp_path)
    from vsdd.state.decisions import decision_open

    decision_open(root, cycle="c-1", kind="finding", packet="# A finding waits")
    assert changed_paths(root)  # the decision record is pending
    code, data, err = _survey(root, _env(root))
    assert code == 0, err
    pending = changed_paths(root)
    assert pending == [".factory/cycles/c-1/units/story.S-9.01.json"]  # the decision is in; the closure this survey wrote is the survey's own late write


def test_survey_limits_config_tier_override(tmp_path):
    root = make_story_project(tmp_path)
    environ = _env(root)
    code, data, _err = _survey(root, {**environ, "VSDD_TEST_TIER": "glm-5:high"})
    assert code == 0
    assert data["config"]["tier_override"] == {"model": "glm-5", "effort": "high"}
    _code, plain, _err = _survey(root, _env(root))
    assert plain["config"]["tier_override"] is None
    _code, bare, _err = _survey(root, {**_env(root), "VSDD_TEST_TIER": "glm-5"})
    assert bare["config"]["tier_override"] == {"model": "glm-5", "effort": "low"}


def test_survey_releases_lock_when_admission_fails(tmp_path, monkeypatch):
    root = make_story_project(tmp_path)
    from vsdd.state import worktrees

    def refused(root, story, branch, base):
        raise worktrees.WorktreeError("git", "the branch refuses")

    monkeypatch.setattr(worktrees, "create_story", refused)
    code, data, _err = _survey(root, _env(root))
    assert code == 1 and data["ok"] is False
    assert "could not be admitted" in data["error"]
    assert data["run_id"] == ""  # exit 1 names no run
    assert lock_entries(root) == []  # the lock is released first, which ends the run
    runs = runs_list(root)
    assert runs and runs[-1]["ended"]


def test_survey_releases_lock_when_the_answer_fails(tmp_path, monkeypatch):
    """The build-fault handler releases the lock too: the story stands
    admitted, but building the answer dies — exit 1, ok false, the lock
    released, the run ended, and no run named (finding S3R-survey-C1-1)."""
    root = make_story_project(tmp_path)
    from vsdd.state import survey as survey_body

    def dead(_root, _cycle):
        raise ValueError("the cycle record is unreadable")

    monkeypatch.setattr(survey_body, "cycle_record_baseline", dead)
    code, data, _err = _survey(root, _env(root))
    assert code == 1 and data["ok"] is False
    assert "could not build the story unit" in data["error"]
    assert "the cycle record is unreadable" in data["error"]
    assert data["run_id"] == "" and data["units"] == []  # exit 1 names no run
    assert lock_entries(root) == []  # the lock is released
    runs = runs_list(root)
    assert runs and runs[-1]["ended"]  # the release ended the run


def test_survey_failed_flush_takes_no_lock(tmp_path, monkeypatch):
    root = make_story_project(tmp_path)
    from vsdd.state import commits

    monkeypatch.setattr(
        commits, "flush", lambda root, reason, run="": {
            "ok": False, "state": "failed", "commit": "", "files": [],
            "rejected": [], "error": "the sandbox refuses",
        }
    )
    code, data, _err = _survey(root, _env(root))
    assert code == 1 and data["ok"] is False
    assert "the sandbox refuses" in data["error"]
    assert lock_entries(root) == []
    assert data["run_id"] == ""


def test_survey_unbuilt_is_exit_2_with_whole_object(tmp_path):
    root = make_story_project(tmp_path)
    code, out, _err = story_ready.run_vsdd(root, "survey", "feature", "--json", environ={"HOME": str(tmp_path)})
    assert code == 2
    data = json.loads(out)
    for field in (
        "ok", "error", "run_id", "workflow", "subject", "cycle", "scope",
        "project_dir", "plugin_root", "head", "preconditions", "done", "units",
        "config", "limits", "spikes_used", "rounds_used", "decisions", "waived", "next",
    ):
        assert field in data
    assert data["run_id"] == "" and data["units"] == []

    # A missing --subject for story is a usage refusal, not an unmet
    # precondition (finding S3R-survey-2).
    code, out, err = story_ready.run_vsdd(root, "survey", "story", "--json", environ={"HOME": str(tmp_path)})
    assert code == 2
    assert "--subject" in err

    # A hub layout is not built yet (D3), before the marker is read.
    from ._repo import set_keys

    hub = make_story_project(tmp_path / "hub")
    set_keys(hub, "project", layout="hub")
    code, out, err = story_ready.run_vsdd(
        hub, "survey", "story", "--subject", STORY, "--json", environ={"HOME": str(tmp_path)}
    )
    assert code == 2
    assert "hub" in err

    # A hotfix cycle is not built yet: its stories are cut from
    # hotfix/<version>, not from the integration head.
    from vsdd.state.cycles import cycle_read, cycle_write

    hot = make_story_project(tmp_path / "hot")
    record = cycle_read(hot, "c-1")
    record["kind"] = "hotfix"
    cycle_write(hot, record)
    code, out, err = story_ready.run_vsdd(
        hot, "survey", "story", "--subject", STORY, "--json", environ={"HOME": str(tmp_path)}
    )
    assert code == 2
    assert "hotfix" in err

    # --cycle with no cycle open is a refusal, not an IndexError.
    none_open = make_story_project(tmp_path / "nocycle")
    import shutil

    shutil.rmtree(none_open / ".factory" / "cycles" / "c-1")
    code, out, err = story_ready.run_vsdd(
        none_open, "survey", "story", "--subject", STORY, "--cycle", "c-1", "--json",
        environ={"HOME": str(tmp_path)},
    )
    assert code == 2
    assert "no cycle is open" in err


def test_survey_delivered_story(tmp_path):
    root = make_story_project(tmp_path)
    code, first, _err = _survey(root, _env(root))
    assert code == 0
    closure_update(
        root,
        "c-1",
        "story",
        STORY,
        lambda record: record.setdefault("delivery", {}).update({"merged_sha": "a" * 40}),
    )
    remove_story(root, STORY)
    code, second, _err = _survey(root, _env(root))
    assert code == 0
    unit = second["units"][0]
    assert unit["position"] == "delivered"
    assert not story_path(root, STORY).exists()  # admission creates nothing
    # Same session, no steps: the survey re-enters the same run, and the
    # story stands recorded in it (topics/run-lock.md).
    assert second["run_id"] == first["run_id"]
    assert run_read(root, second["run_id"])["stories"] == [STORY]


def test_cycle_option_must_name_the_open_cycle(tmp_path):
    root = make_story_project(tmp_path)
    code, out, err = story_ready.run_vsdd(
        root, "survey", "story", "--subject", STORY, "--cycle", "c-2", "--json",
        environ=_env(root),
    )
    assert code == 2
    assert "the one open cycle" in err
