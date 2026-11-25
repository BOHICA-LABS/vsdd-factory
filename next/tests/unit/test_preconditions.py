"""One test per precondition predicate (plan survey.md section 7; D2, D12)."""

from __future__ import annotations

from pathlib import Path

from . import story_ready
from .story_ready import make_project as make_story_project

from vsdd.state import preconditions
from vsdd.state.cycles import closure_update

STORY = "S-9.01"
ROWS = {row["name"]: row for row in []}


def _names(rows):
    return [row["name"] for row in rows]


def test_project_ready(tmp_path):
    root = make_story_project(tmp_path)
    rows = preconditions.check(root, "story", subject=STORY, cycle="c-1", hooks_alive=None)
    ready = next(row for row in rows if row["name"] == "project_ready")
    assert ready["met"] is True and ready["detail"] == ""
    bare = Path(tmp_path) / "nowhere"
    bare.mkdir()
    rows = preconditions.check(bare, "story", hooks_alive=None)
    ready = next(row for row in rows if row["name"] == "project_ready")
    assert ready["met"] is False and "detail" in ready


def test_hooks_alive(tmp_path):
    root = make_story_project(tmp_path)
    rows = preconditions.check(root, "story", subject=STORY, cycle="c-1", hooks_alive=True)
    alive = next(row for row in rows if row["name"] == "hooks_alive")
    assert alive["met"] is True
    rows = preconditions.check(root, "story", subject=STORY, cycle="c-1", hooks_alive=False)
    alive = next(row for row in rows if row["name"] == "hooks_alive")
    assert alive["met"] is False and "no caller marker" in alive["detail"]


def test_check_without_hooks_row(tmp_path):
    """The route's "could run" form: hooks_alive=None leaves the row out."""
    root = make_story_project(tmp_path)
    rows = preconditions.check(root, "story", subject=STORY, cycle="c-1", hooks_alive=None)
    assert "hooks_alive" not in _names(rows)


def test_spec_approved(tmp_path):
    root = make_story_project(tmp_path)
    rows = preconditions.check(root, "story", subject=STORY, cycle="c-1", hooks_alive=None)
    spec = next(row for row in rows if row["name"] == "spec_approved")
    assert spec["met"] is True and spec["detail"] == ""


def test_spec_stale_after_edit(tmp_path):
    root = make_story_project(tmp_path)
    # A file in the specification set, added after the approval: the digest
    # no longer matches, and the precondition shows records' stale detail
    # (finding P3OPS-10).
    prd = root / ".factory" / "specs" / "product" / "prd.md"
    prd.parent.mkdir(parents=True, exist_ok=True)
    prd.write_text("---\ntitle: Product requirements\nstatus: draft\n---\n\nBody.\n", encoding="utf-8")
    rows = preconditions.check(root, "story", subject=STORY, cycle="c-1", hooks_alive=None)
    spec = next(row for row in rows if row["name"] == "spec_approved")
    assert spec["met"] is False and spec["detail"] == "stale"


def test_story_admitted(tmp_path):
    root = make_story_project(tmp_path)
    rows = preconditions.check(root, "story", subject=STORY, cycle="c-1", hooks_alive=None)
    admitted = next(row for row in rows if row["name"] == "story_admitted")
    assert admitted["met"] is True
    rows = preconditions.check(root, "story", subject="S-9.77", cycle="c-1", hooks_alive=None)
    admitted = next(row for row in rows if row["name"] == "story_admitted")
    assert admitted["met"] is False and admitted["detail"]


def test_dependencies_delivered(tmp_path):
    root = make_story_project(tmp_path)
    rel = story_ready._story_file(root, STORY, cycle="c-1")
    text = (root / rel).read_text(encoding="utf-8").replace(
        "contracts: [BC-1.00.001]", "contracts: [BC-1.00.001]\ndepends_on: [S-9.02]"
    )
    (root / rel).write_text(text, encoding="utf-8")
    rows = preconditions.check(root, "story", subject=STORY, cycle="c-1", hooks_alive=None)
    deps = next(row for row in rows if row["name"] == "dependencies_delivered")
    assert deps["met"] is False and "S-9.02" in deps["detail"]

    closure_update(
        root,
        "c-1",
        "story",
        "S-9.02",
        lambda record: record.setdefault("delivery", {}).update({"merged_sha": "b" * 40}),
    )
    rows = preconditions.check(root, "story", subject=STORY, cycle="c-1", hooks_alive=None)
    deps = next(row for row in rows if row["name"] == "dependencies_delivered")
    assert deps["met"] is True


def test_commands_configured(tmp_path):
    from ._repo import set_keys

    root = make_story_project(tmp_path)
    set_keys(root, "commands", test="", test_report="report.xml")
    rows = preconditions.check(root, "story", subject=STORY, cycle="c-1", hooks_alive=None)
    commands = next(row for row in rows if row["name"] == "commands_configured")
    assert commands["met"] is False and "test" in commands["detail"]
    set_keys(root, "commands", test="pytest -q", test_report="report.xml")
    rows = preconditions.check(root, "story", subject=STORY, cycle="c-1", hooks_alive=None)
    commands = next(row for row in rows if row["name"] == "commands_configured")
    assert commands["met"] is True


def test_other_workflow_gets_the_common_rows_only(tmp_path):
    root = make_story_project(tmp_path)
    rows = preconditions.check(root, "wave", hooks_alive=True)
    assert _names(rows) == ["project_ready", "hooks_alive"]


def test_no_open_cycle_names_it_and_skips_the_reads(tmp_path):
    """With no open cycle the cycle-scoped rows say so plainly; a story with
    dependencies does not crash the check (finding S3R-survey-12)."""
    root = make_story_project(tmp_path)
    rel = story_ready._story_file(root, STORY, cycle="c-1")
    text = (root / rel).read_text(encoding="utf-8").replace(
        "contracts: [BC-1.00.001]", "contracts: [BC-1.00.001]\ndepends_on: [S-9.02]"
    )
    (root / rel).write_text(text, encoding="utf-8")

    rows = preconditions.check(root, "story", subject=STORY, cycle="", hooks_alive=None)
    admitted = next(row for row in rows if row["name"] == "story_admitted")
    deps = next(row for row in rows if row["name"] == "dependencies_delivered")
    assert admitted["met"] is False and admitted["detail"] == "no cycle is open"
    assert deps["met"] is False and deps["detail"] == "no cycle is open"
