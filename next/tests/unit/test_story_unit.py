"""The story unit and the position ladder (plan survey.md 3.5, section 7;
decisions D6, D14, D15, D16)."""

from __future__ import annotations

import pytest

from . import story_ready
from .story_ready import make_project as make_story_project

from vsdd.state import story_unit, worktrees
from vsdd.state.cycles import closure_update
from vsdd.state.gate_evidence import evidence_write
from vsdd.state.store import git as run_git

STORY = "S-9.01"


def _head(root) -> str:
    return run_git(worktrees.story_path(root, STORY), "rev-parse", "HEAD")


def _survey(root, tmp_path):
    key = story_ready.write_caller(root)
    code, out, err = story_ready.run_vsdd(
        root, "survey", "story", "--subject", STORY, "--json",
        environ={"HOME": str(tmp_path), "VSDD_CALLER": key},
    )
    assert code == 0, err
    import json

    return json.loads(out)


def test_unit_fields_and_kind_defaults(tmp_path):
    root = make_story_project(tmp_path)
    survey = _survey(root, tmp_path)
    unit = survey["units"][0]
    # The kind row of a fix (plan decision D14, the shipped story-kind rows).
    assert unit["tests_agent"] == "test-writer"
    assert unit["tests_method"] == "regression-test"
    assert unit["impl_agent"] == "implementer"
    assert unit["impl_method"] == "fix-findings"
    assert unit["red_gate"] is True
    assert unit["id"] == STORY
    assert unit["kind"] == "fix"
    assert unit["criticality"] == "medium"
    assert unit["bcs"] == ["BC-1.00.001"]
    assert unit["acs"] == ["AC-1"]
    assert unit["demo_required"] is False
    assert unit["waived"] == [] and unit["guidance"] == "" and unit["deferred"] == []
    assert unit["blocked"] == {"kind": "", "decision": "", "open": False}
    assert unit["worktree"] == unit["tree"] == str(root / ".worktrees" / STORY)

    # A story's own methods win over the kind row's (decision D14).
    story_path = root / ".factory" / "stories" / "E-9" / "S-9.02.md"
    story_path.write_text(
        "---\nid: S-9.02\ntitle: Own methods\nstatus: draft\ncycle: c-1\nkind: fix\n"
        "priority: P1\ncriticality: low\ncontracts: [BC-1.00.001]\n"
        "tests_method: ui-tests\nimpl_method: careful-fix\n---\n\n"
        "## Acceptance criteria\n\n### AC-1 (BC-1.00.001)\n",
        encoding="utf-8",
    )
    from vsdd.state.story_unit import build

    own = build(root, "c-1", "S-9.02")
    assert own["tests_method"] == "ui-tests"
    assert own["impl_method"] == "careful-fix"

    # A kind with no tests stage carries no tests method, whatever the story says.
    story_path = root / ".factory" / "stories" / "E-9" / "S-9.03.md"
    story_path.write_text(
        "---\nid: S-9.03\ntitle: Facade\nstatus: draft\ncycle: c-1\nkind: facade\n"
        "priority: P2\ncriticality: low\ncontracts: [BC-1.00.001]\n---\n\n"
        "## Acceptance criteria\n\n### AC-1 (BC-1.00.001)\n",
        encoding="utf-8",
    )
    facade = build(root, "c-1", "S-9.03")
    assert facade["tests_agent"] == ""
    assert facade["tests_method"] == ""
    assert facade["impl_agent"] == "implementer"
    assert facade["impl_method"] == "service-clone"
    assert facade["red_gate"] is False


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
        log="the failing run\n",
    )


def _green(root, sha):
    return evidence_write(
        root,
        {"gate": "green", "subject": STORY, "sha": sha, "passed": True, "patch_id": "p" * 40},
        log="the passing run\n",
    )


def test_position_ladder(tmp_path):
    root = make_story_project(tmp_path)
    _survey(root, tmp_path)  # admits the story: position ready
    from vsdd.state.story_unit import position

    assert position(root, "c-1", STORY) == "ready"
    head = _head(root)
    _red(root, head)
    assert position(root, "c-1", STORY) == "red"
    green = _green(root, head)
    assert position(root, "c-1", STORY) == "green"

    # Reviewed: a converged verdict in the closure that covers the head.
    closure_update(
        root,
        "c-1",
        "story",
        STORY,
        lambda record: record.update(
            {
                "verdict": {"outcome": "converged", "passes": 2, "head": head,
                            "patch_id": green["patch_id"]},
                "delivery": {
                    **(record.get("delivery") or {}),
                    "head": head,
                    "patch_id": green["patch_id"],
                },
            }
        ),
    )
    # The sample story needs no demo, so evidenced follows at once.
    assert position(root, "c-1", STORY) == "evidenced"

    # A story whose demo the criteria owe stays reviewed until the demo
    # evidence covers the head.
    from vsdd.state.admission import admit

    closure_update(root, "c-1", "plan", "", lambda record: record["waves"].update({"w1": [STORY, "S-9.02"]}))
    story_path = root / ".factory" / "stories" / "E-9" / "S-9.02.md"
    story_path.write_text(
        "---\nid: S-9.02\ntitle: Demoed\nstatus: draft\ncycle: c-1\nkind: fix\n"
        "priority: P1\ncriticality: high\ncontracts: [BC-1.00.001]\ndemo: true\n---\n\n"
        "## Acceptance criteria\n\n### AC-1 (BC-1.00.001)\n",
        encoding="utf-8",
    )
    run_id = _survey(root, tmp_path)["run_id"]
    admit(root, cycle="c-1", story="S-9.02", run=run_id, admitted_by=run_id)
    other = run_git(root / ".worktrees" / "S-9.02", "rev-parse", "HEAD")
    _red_other = evidence_write(
        root,
        {
            "gate": "red",
            "subject": "S-9.02",
            "sha": other,
            "passed": True,
            "frozen": [{"file": "tests/test_y.py", "sha": "e" * 64}],
            "tests": ["tests/test_y.py::test_y"],
            "criteria": {"AC-1": ["tests/test_y.py::test_y"]},
        },
    )
    _green_other = evidence_write(
        root, {"gate": "green", "subject": "S-9.02", "sha": other, "passed": True, "patch_id": "q" * 40}
    )
    closure_update(
        root,
        "c-1",
        "story",
        "S-9.02",
        lambda record: record.update(
            {
                "verdict": {"outcome": "converged", "passes": 1, "head": other,
                            "patch_id": _green_other["patch_id"]},
                "delivery": {
                    **(record.get("delivery") or {}),
                    "head": other,
                    "patch_id": _green_other["patch_id"],
                },
            }
        ),
    )
    assert position(root, "c-1", "S-9.02") == "reviewed"
    evidence_write(root, {"gate": "demo", "subject": "S-9.02", "sha": other, "passed": True})
    assert position(root, "c-1", "S-9.02") == "evidenced"


def test_position_delivered_without_worktree(tmp_path):
    root = make_story_project(tmp_path)
    _survey(root, tmp_path)
    closure_update(
        root,
        "c-1",
        "story",
        STORY,
        lambda record: record.setdefault("delivery", {}).update({"merged_sha": "a" * 40}),
    )
    worktrees.remove_story(root, STORY)
    # Delivery removes the worktree; the position is delivered before any
    # evidence is read (decision D6).
    from vsdd.state.story_unit import position

    assert position(root, "c-1", STORY) == "delivered"
    unit = story_unit.build(root, "c-1", STORY)
    assert unit["position"] == "delivered"


def test_position_falls_back_after_new_commit(tmp_path):
    root = make_story_project(tmp_path)
    _survey(root, tmp_path)
    from vsdd.state.story_unit import position

    head = _head(root)
    _red(root, head)
    _green(root, head)
    assert position(root, "c-1", STORY) == "green"
    # A commit added after the position was reached moves the story back.
    tree = worktrees.story_path(root, STORY)
    (tree / "new.txt").write_text("later work", encoding="utf-8")
    run_git(tree, "add", "-A")
    run_git(tree, "commit", "-qm", "repair")
    assert position(root, "c-1", STORY) == "red"  # red covers, green does not


def test_position_survives_new_run(tmp_path):
    root = make_story_project(tmp_path)
    _survey(root, tmp_path)
    from vsdd.state.story_unit import position

    head = _head(root)
    _red(root, head)
    _green(root, head)
    assert position(root, "c-1", STORY) == "green"
    # The evidence of any run is read: ending the run and opening another
    # changes nothing (finding P3OPS-08).
    from vsdd.state.locks import lock_release
    from vsdd.state.runs import run_open

    lock_release(root, force=True)
    run_open(root, workflow="story", cycle="c-1", session="sess-next-abcd", subject=STORY)
    assert position(root, "c-1", STORY) == "green"


def test_position_reviewed_from_closure_verdict(tmp_path):
    """Reviewed is granted only by a converged closure verdict that covers the
    head; a verdict that does not cover grants nothing (plan survey.md
    section 7; D6)."""
    root = make_story_project(tmp_path)
    _survey(root, tmp_path)  # admits S-9.01
    closure_update(root, "c-1", "plan", "", lambda record: record["waves"].update({"w1": [STORY, "S-9.02"]}))
    rel = story_ready._story_file(root, "S-9.02", cycle="c-1")
    text = (root / rel).read_text(encoding="utf-8").replace("demo: false", "demo: true")
    (root / rel).write_text(text, encoding="utf-8")
    from vsdd.state.admission import admit
    from vsdd.state.story_unit import position

    run_id = _survey(root, tmp_path)["run_id"]
    admit(root, cycle="c-1", story="S-9.02", run=run_id, admitted_by=run_id)
    other = run_git(root / ".worktrees" / "S-9.02", "rev-parse", "HEAD")
    evidence_write(
        root,
        {
            "gate": "red",
            "subject": "S-9.02",
            "sha": other,
            "passed": True,
            "frozen": [{"file": "tests/test_y.py", "sha": "e" * 64}],
            "tests": ["tests/test_y.py::test_y"],
            "criteria": {"AC-1": ["tests/test_y.py::test_y"]},
        },
    )
    green = evidence_write(
        root, {"gate": "green", "subject": "S-9.02", "sha": other, "passed": True, "patch_id": "q" * 40}
    )
    closure_update(
        root,
        "c-1",
        "story",
        "S-9.02",
        lambda record: record.update(
            {
                "verdict": {"outcome": "converged", "passes": 1, "head": other,
                            "patch_id": green["patch_id"]},
                "delivery": {
                    **(record.get("delivery") or {}),
                    "head": other,
                    "patch_id": green["patch_id"],
                },
            }
        ),
    )
    assert position(root, "c-1", "S-9.02") == "reviewed"

    # A verdict that does not cover the head: a commit the verdict never saw
    # is covered by the newer green evidence, but not by the verdict.
    tree = worktrees.story_path(root, "S-9.02")
    (tree / "late.txt").write_text("later", encoding="utf-8")
    run_git(tree, "add", "-A")
    run_git(tree, "commit", "-qm", "late")
    late = run_git(tree, "rev-parse", "HEAD")
    evidence_write(
        root, {"gate": "green", "subject": "S-9.02", "sha": late, "passed": True, "patch_id": "r" * 40}
    )
    assert position(root, "c-1", "S-9.02") == "green"


def test_kind_without_red_gate_starts_at_red(tmp_path):
    """D6: a kind with no red gate is at red at once, without red evidence."""
    root = make_story_project(tmp_path)
    closure_update(root, "c-1", "plan", "", lambda record: record["waves"].update({"w1": [STORY, "S-9.02"]}))
    story = root / ".factory" / "stories" / "E-9" / "S-9.02.md"
    story.write_text(
        "---\nid: S-9.02\ntitle: Facade\nstatus: draft\ncycle: c-1\nkind: facade\n"
        "priority: P2\ncriticality: low\ncontracts: [BC-1.00.001]\ndemo: false\n---\n\n"
        "## Acceptance criteria\n\n### AC-1 (BC-1.00.001)\n",
        encoding="utf-8",
    )
    from vsdd.state.admission import admit
    from vsdd.state.story_unit import position, story_kind

    run_id = _survey(root, tmp_path)["run_id"]
    admit(root, cycle="c-1", story="S-9.02", run=run_id, admitted_by=run_id)
    assert story_kind(root, "S-9.02") == "facade"
    assert position(root, "c-1", "S-9.02") == "red"


def test_corrupt_evidence_rises(tmp_path):
    """A corrupt evidence record is a fault, not silence: position() refuses
    and the survey reports it (finding S3R-survey-13)."""
    import pytest

    root = make_story_project(tmp_path)
    _survey(root, tmp_path)
    head = _head(root)
    _red(root, head)
    _green(root, head)
    from vsdd.state.gate_evidence import evidence_dir
    from vsdd.state.store import StoreCorrupt
    from vsdd.state.story_unit import position

    for path in evidence_dir(root, STORY).glob("green.*.json"):
        path.write_text("{ not json", encoding="utf-8")
    with pytest.raises(StoreCorrupt):
        position(root, "c-1", STORY)

    # Through the survey: a fault at exit 1 with the words, not an internal
    # error at exit 2 and not a silently lowered position.
    key = story_ready.write_caller(root)
    from pathlib import Path

    code, out, err = story_ready.run_vsdd(
        root, "survey", "story", "--subject", STORY, "--json",
        environ={"HOME": str(Path(root).parent), "VSDD_CALLER": key},
    )
    assert code == 1, err
    import json

    data = json.loads(out)
    assert data["ok"] is False and "does not parse" in data["error"]
    assert data["units"] == []


def test_kind_has_red_gate_matches_policy(tmp_path):
    from vsdd.policy import load as load_policy

    for row in load_policy("artifacts")["story-kind"]:
        assert story_unit.kind_has_red_gate(row["id"]) is bool(row["red_gate"]), row["id"]
    assert story_unit.kind_has_red_gate("no-such-kind") is False


def test_diff_patch_and_test_output_from_green(tmp_path):
    root = make_story_project(tmp_path)
    survey = _survey(root, tmp_path)
    unit = survey["units"][0]
    assert unit["diff_patch"] == ""
    assert unit["test_output"] == ""
    head = _head(root)
    _green(root, head)
    unit = story_unit.build(root, "c-1", STORY)
    assert unit["diff_patch"] == ""
    from vsdd.state.gate_evidence import evidence_dir

    log = evidence_dir(root, STORY) / f"green.{head[:12]}.log"
    log.write_text("1 passed\n", encoding="utf-8")
    unit = story_unit.build(root, "c-1", STORY)
    assert unit["test_output"] == str(log)
