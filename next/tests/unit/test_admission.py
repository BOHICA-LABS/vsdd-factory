"""Story admission (plan survey.md 3.4, section 7; decisions D4, D5)."""

from __future__ import annotations

import json

import pytest

from . import story_ready
from ._repo import make_project
from .story_ready import make_project as make_story_project

from vsdd.state import admission, worktrees
from vsdd.state.cycles import closure_read, closure_update
from vsdd.state.locks import lock_entries, lock_scope, run_begin
from vsdd.state.runs import run_end, run_open, run_read
from vsdd.state.store import git as run_git, write_record

STORY = "S-9.01"
BRANCH = "story/S-9.01"


def _begin(root, session=story_ready.SESSION, subject=STORY):
    return run_begin(
        root,
        scope=lock_scope(root),
        workflow="story",
        subject=subject,
        session=session,
        cycle="c-1",
    )


def test_admissible_open_wave_later_wave_dropped(tmp_path):
    root = make_story_project(tmp_path)
    closure_update(root, "c-1", "plan", "", lambda record: record["waves"].update({"w2": ["S-9.02"]}))

    assert admission.admissible(root, "c-1", STORY) == (True, "")
    ok, detail = admission.admissible(root, "c-1", "S-9.02")
    assert (ok, "not in the open wave" in detail) == (False, True)
    ok, _ = admission.admissible(root, "c-1", "S-9.99")
    assert ok is False

    # Closing w1 opens w2: the story of the later wave is admitted then.
    closure_update(root, "c-1", "wave", "w1", lambda record: record.update({"state": "closed"}))
    assert admission.admissible(root, "c-1", "S-9.02") == (True, "")


def test_admit_writes_closure_run_branch_worktree(tmp_path):
    root = make_story_project(tmp_path)
    begun = _begin(root)
    unit = admission.admit(root, cycle="c-1", story=STORY, run=begun["run"]["id"], admitted_by=begun["run"]["id"])

    closure = closure_read(root, "c-1", "story", STORY)
    assert closure["state"] == "open"
    assert closure["position"] == "ready"
    assert closure["admitted_by"] == begun["run"]["id"]
    assert closure["delivery"]["branch"] == "develop"
    assert closure["delivery"]["base"] == run_git(root, "rev-parse", "develop")
    assert worktrees.story_path(root, STORY).is_dir()
    assert run_read(root, begun["run"]["id"])["stories"] == [STORY]
    assert unit["id"] == STORY
    assert unit["position"] == "ready"
    # Finding D33-21: the tracked closure carries no machine-local path.
    assert str(root) not in json.dumps(closure)


def test_admit_base_is_target_head_at_creation(tmp_path):
    root = make_story_project(tmp_path)
    begun = _begin(root)
    admission.admit(root, cycle="c-1", story=STORY, run=begun["run"]["id"], admitted_by=begun["run"]["id"])
    base = closure_read(root, "c-1", "story", STORY)["delivery"]["base"]
    assert base == run_git(root, "rev-parse", "develop")

    # A second story is cut from the head as it is now, not the plan's base.
    run_git(root, "commit", "-q", "--allow-empty", "-m", "advance")
    story_two = "S-9.02"
    closure_update(root, "c-1", "plan", "", lambda record: record["waves"].update({"w1": [STORY, story_two]}))
    story_ready._story_file(root, story_two, cycle="c-1")
    from vsdd.state.commits import flush

    flush(root, "second story")
    unit_two = admission.admit(root, cycle="c-1", story=story_two, run=begun["run"]["id"], admitted_by=begun["run"]["id"])
    assert unit_two["base"] == run_git(root, "rev-parse", "develop")
    assert unit_two["base"] != base


def test_admit_again_same_unit_new_run(tmp_path):
    root = make_story_project(tmp_path)
    first = _begin(root)
    unit_one = admission.admit(root, cycle="c-1", story=STORY, run=first["run"]["id"], admitted_by=first["run"]["id"])

    # A new run in the same project: the aged lock is taken over, the story
    # joins the new run and the unit is the same (findings P3REQ-07, P3OPS-07).
    _age_lock(root)
    second = run_begin(
        root,
        scope=lock_scope(root),
        workflow="story",
        subject=STORY,
        session="sess-third-aabb",
        cycle="c-1",
    )
    assert second["entered"] == "took_over"
    unit_two = admission.admit(root, cycle="c-1", story=STORY, run=second["run"]["id"], admitted_by=second["run"]["id"])
    assert run_read(root, second["run"]["id"])["stories"] == [STORY]
    assert unit_two["id"] == unit_one["id"]
    assert unit_two["base"] == unit_one["base"]
    assert unit_two["branch"] == unit_one["branch"]
    assert run_read(root, first["run"]["id"])["ended"]


def _age_lock(root) -> None:
    entries = lock_entries(root)
    for entry in entries:
        entry["seen"] = "2026-01-01T00:00:00Z"
    write_record(root / ".factory" / "local" / "lock.json", {"locks": entries}, cap=None, schema="lock")


def test_admit_completes_after_crash(tmp_path):
    root = make_story_project(tmp_path)
    begun = _begin(root)
    run_id = begun["run"]["id"]
    admission.admit(root, cycle="c-1", story=STORY, run=run_id, admitted_by=run_id)
    recorded = closure_read(root, "c-1", "story", STORY)["delivery"]["base"]

    # A killed call leaves the branch but no worktree: the next call completes.
    worktrees.remove_story(root, STORY)
    assert not worktrees.story_path(root, STORY).exists()
    admission.admit(root, cycle="c-1", story=STORY, run=run_id, admitted_by=run_id)
    assert worktrees.story_path(root, STORY).is_dir()
    assert closure_read(root, "c-1", "story", STORY)["delivery"]["base"] == recorded

    # A killed call that left no branch either: the head is read and recorded
    # again (decision D5), and the branch is cut there.
    run_git(root, "commit", "-q", "--allow-empty", "-m", "moved on")
    worktrees.remove_story(root, STORY)
    run_git(root, "branch", "-D", BRANCH)
    admission.admit(root, cycle="c-1", story=STORY, run=run_id, admitted_by=run_id)
    fresh = closure_read(root, "c-1", "story", STORY)["delivery"]["base"]
    assert fresh == run_git(root, "rev-parse", "develop")
    assert worktrees.verify_story(root, STORY, BRANCH, fresh)


def test_admit_requires_live_run_holding_lock(tmp_path):
    root = make_story_project(tmp_path)
    with pytest.raises(admission.AdmissionError) as unknown:
        admission.admit(root, cycle="c-1", story=STORY, run="r-missing", admitted_by="r-missing")
    assert unknown.value.reason == "run_unknown"
    assert closure_read(root, "c-1", "story", STORY) is None
    assert not worktrees.story_path(root, STORY).exists()

    # A live run that holds no lock is refused too, and writes nothing.
    run_id = run_open(root, workflow="story", cycle="c-1", session=story_ready.SESSION, subject=STORY)["id"]
    with pytest.raises(admission.AdmissionError) as no_lock:
        admission.admit(root, cycle="c-1", story=STORY, run=run_id, admitted_by=run_id)
    assert no_lock.value.reason == "not_holder"
    assert closure_read(root, "c-1", "story", STORY) is None
    assert not worktrees.story_path(root, STORY).exists()


def test_admit_rejects_bad_story(tmp_path):
    root = make_story_project(tmp_path)
    begun = _begin(root)
    run_id = begun["run"]["id"]

    with pytest.raises(admission.AdmissionError) as missing:
        admission.admit(root, cycle="c-1", story="S-9.42", run=run_id, admitted_by=run_id)
    assert missing.value.reason == "missing_story"

    for field, value, reason in (
        ("status", "dropped", "not_draft"),
        ("kind", "emergency", "unknown_kind"),
        ("criticality", "urgent", "bad_criticality"),
    ):
        rel = story_ready._story_file(root, "S-9.50", cycle="c-1")
        text = (root / rel).read_text(encoding="utf-8").replace(
            {"status": "status: draft", "kind": "kind: fix", "criticality": "criticality: medium"}[field],
            f"{field}: {value}",
        )
        (root / rel).write_text(text, encoding="utf-8")
        with pytest.raises(admission.AdmissionError) as bad:
            admission.admit(root, cycle="c-1", story="S-9.50", run=run_id, admitted_by=run_id)
        assert bad.value.reason == reason
        (root / rel).unlink()

    # An id the file does not carry is refused after the file exists.
    rel = story_ready._story_file(root, STORY, cycle="c-1")
    (root / rel).write_text(
        (root / rel).read_text(encoding="utf-8").replace(f"id: {STORY}", "id: S-9.02"),
        encoding="utf-8",
    )
    with pytest.raises(admission.AdmissionError) as bad_id:
        admission.admit(root, cycle="c-1", story=STORY, run=run_id, admitted_by=run_id)
    assert bad_id.value.reason == "bad_id"


def test_admit_delivered_creates_nothing(tmp_path):
    root = make_story_project(tmp_path)
    begun = _begin(root)
    run_id = begun["run"]["id"]
    admission.admit(root, cycle="c-1", story=STORY, run=run_id, admitted_by=run_id)
    closure_update(
        root,
        "c-1",
        "story",
        STORY,
        lambda record: record.setdefault("delivery", {}).update({"merged_sha": "a" * 40}),
    )
    worktrees.remove_story(root, STORY)
    unit = admission.admit(root, cycle="c-1", story=STORY, run=run_id, admitted_by=run_id)
    assert unit["position"] == "delivered"
    assert not worktrees.story_path(root, STORY).exists()  # creates nothing


def test_require_story_checks_run_membership(tmp_path):
    """The membership check is the one every gate and delivery repeats: a
    story absent from the run given is refused while the closure, base and
    worktree all exist (finding S3R-survey-6, P3OPS-07)."""
    root = make_story_project(tmp_path)
    begun = _begin(root)
    run_id = begun["run"]["id"]
    admission.admit(root, cycle="c-1", story=STORY, run=run_id, admitted_by=run_id)

    # A second live run that holds nothing: the story is not in it.
    stranger = run_open(
        root, workflow="story", cycle="c-1", session="sess-stranger-beef", subject=STORY
    )["id"]
    with pytest.raises(admission.AdmissionError) as not_member:
        admission.require_story(root, stranger, STORY)
    assert not_member.value.reason == "story_not_in_run"
    closure = closure_read(root, "c-1", "story", STORY)
    assert closure is not None and closure["delivery"]["base"]
    assert worktrees.story_path(root, STORY).is_dir()


def test_admit_after_by_hand_removal(tmp_path):
    """A worktree directory removed outside git does not wedge admission: the
    library prunes the stale registration before it adds (finding S3R-survey-3)."""
    import shutil

    root = make_story_project(tmp_path)
    begun = _begin(root)
    run_id = begun["run"]["id"]
    admission.admit(root, cycle="c-1", story=STORY, run=run_id, admitted_by=run_id)
    shutil.rmtree(worktrees.story_path(root, STORY))
    admission.admit(root, cycle="c-1", story=STORY, run=run_id, admitted_by=run_id)
    assert worktrees.story_path(root, STORY).is_dir()
    base = closure_read(root, "c-1", "story", STORY)["delivery"]["base"]
    assert worktrees.verify_story(root, STORY, BRANCH, base)


def test_require_story_refusals(tmp_path):
    root = make_story_project(tmp_path)
    stranger = run_open(root, workflow="story", cycle="c-1", session="s", subject="S-9.01")["id"]
    with pytest.raises(admission.AdmissionError):
        admission.require_story(root, stranger, STORY)  # not in the run

    begun = _begin(root)
    run_id = begun["run"]["id"]
    admission.admit(root, cycle="c-1", story=STORY, run=run_id, admitted_by=run_id)
    checked = admission.require_story(root, run_id, STORY)
    assert checked["cycle"] == "c-1"
    assert checked["branch"] == BRANCH
    assert checked["target"] == "develop"
    assert checked["base"] == run_git(root, "rev-parse", "develop")
    assert checked["head"] == run_git(root, "rev-parse", "HEAD")

    run_end(root, run_id)
    with pytest.raises(admission.AdmissionError) as ended:
        admission.require_story(root, run_id, STORY)
    assert ended.value.reason == "run_ended"

    # A fresh run and admission, for the worktree and history cases.
    _age_lock(root)
    fresh = run_begin(
        root,
        scope=lock_scope(root),
        workflow="story",
        subject=STORY,
        session="sess-another-beef",
        cycle="c-1",
    )
    assert fresh["entered"] == "took_over"
    admission.admit(root, cycle="c-1", story=STORY, run=fresh["run"]["id"], admitted_by=fresh["run"]["id"])
    assert admission.require_story(root, fresh["run"]["id"], STORY)["head"]

    worktrees.remove_story(root, STORY)
    with pytest.raises(worktrees.WorktreeError) as gone:
        admission.require_story(root, fresh["run"]["id"], STORY)
    assert gone.value.code == "missing"

    # Recreate the worktree, then claim a base the history does not contain.
    admission.admit(root, cycle="c-1", story=STORY, run=fresh["run"]["id"], admitted_by=fresh["run"]["id"])
    closure_update(
        root,
        "c-1",
        "story",
        STORY,
        lambda record: record.setdefault("delivery", {}).update({"base": "0" * 40}),
    )
    with pytest.raises(worktrees.WorktreeError) as rewritten:
        admission.require_story(root, fresh["run"]["id"], STORY)
    assert rewritten.value.code == "not_descendant"


def test_record_base_is_the_single_writer(tmp_path):
    root = make_story_project(tmp_path)
    begun = _begin(root)
    run_id = begun["run"]["id"]
    admission.admit(root, cycle="c-1", story=STORY, run=run_id, admitted_by=run_id)
    admission.record_base(root, "c-1", STORY, "develop", "abc456")
    closure = closure_read(root, "c-1", "story", STORY)
    assert closure["delivery"]["branch"] == "develop"
    assert closure["delivery"]["base"] == "abc456"
