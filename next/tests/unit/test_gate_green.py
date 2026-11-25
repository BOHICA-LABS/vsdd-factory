"""The green gate (plan gates.md D12, D13, D17; findings P3BLD-16, AU-P2-07)."""

import json
import subprocess
from pathlib import Path

from tests.unit import gate_project
from tests.unit.gate_project import STORY, run_gate
from tests.unit.test_gate_red import assert_gate_shape
from vsdd.state.cycles import cycle_read, cycle_write

IMPL = gate_project.IMPLEMENTER


def green_ready(tmp_path, *, name: str = "proj", lint: str = "", format_check: str = "",
                test_cmd: str | None = None):
    """A project whose red gate passed and whose source is fixed, so green
    can pass. ``test_cmd`` (and ``lint``/``format_check``) are swapped in
    after red, so the red gate always measures with the real runner.
    Returns (root, run, implementer-env)."""
    root, run = gate_project.make_project(Path(tmp_path) / name)
    gate_project.red_commit(root)
    env = gate_project.env_with(root, IMPL)
    code, red, _err = run_gate(root, "gate", "red", "--story", STORY, "--run", run, env=env)
    assert code == 0 and red["passed"] is True, red["failures"]
    gate_project.commit_story(root, {"tally/blade.py": gate_project.BLADE_FIXED}, "the fix")
    if test_cmd or lint or format_check:
        from tests.unit._repo import set_keys

        keys: dict = {}
        if test_cmd:
            keys["test"] = test_cmd
        if lint:
            keys["lint"] = lint
        if format_check:
            keys["format_check"] = format_check
        set_keys(root, "commands", **keys)
    return root, run, env, red


def sys_executable() -> str:
    import sys

    return sys.executable


def _edit_story_frontmatter(root, kind: str | None = None, **extra: str) -> None:
    story_file = next((Path(root) / ".factory" / "stories").glob(f"*/{STORY}.md"))
    text = story_file.read_text()
    if kind is not None:
        text = text.replace("kind: fix", f"kind: {kind}", 1)
    for key, value in extra.items():
        text = text.replace("demo: false", f"demo: false\n{key}: {value}", 1)
    story_file.write_text(text)


def _git(tree, *args: str) -> None:
    done = subprocess.run(["git", *args], cwd=tree, capture_output=True, text=True)
    assert done.returncode == 0, done.stderr


def test_green_passes_and_writes_patch(tmp_path):
    root, run, env, _red = green_ready(tmp_path)
    code, result, _err = run_gate(root, "gate", "green", "--story", STORY, "--run", run, env=env)
    assert_gate_shape(result)
    assert code == 0 and result["passed"] is True, result["failures"]
    patch = Path(result["diff_patch"])
    assert patch.is_file() and patch.name == f"diff.{result['sha'][:12]}.patch"
    text = patch.read_text()
    assert "sharpen" in text  # the base-to-head patch
    assert Path(result["test_output"]).is_file()
    assert result["flaky"] == []
    record = json.loads(Path(result["evidence"]).read_text())
    assert record["passed"] is True and record["flaky"] == []
    assert Path(result["evidence"]).with_suffix(".xml").is_file()
    # 7 base stats tests, the story's 3, and tests/test_module.py's one plain
    # test function, which the runner reports since S4B-gates-1
    assert result["metrics"]["tests"] == 11
    assert result["metrics"]["skipped"] == ["build", "lint", "format_check"]


def test_green_record_fields_are_declared_in_the_schema(tmp_path):
    """Plan 3.3: the record's fields are evidence-record's — the shipped
    schema declares every field the green gate writes, ``diff_patch``
    included (finding S4R-gates-C1-2)."""
    root, run, env, _red = green_ready(tmp_path)
    code, result, _err = run_gate(root, "gate", "green", "--story", STORY, "--run", run, env=env)
    assert code == 0 and result["passed"] is True, result["failures"]
    record = json.loads(Path(result["evidence"]).read_text())
    assert record["diff_patch"]
    schema_path = (
        Path(__file__).resolve().parents[2]
        / "plugins" / "vsdd-factory" / "schemas" / "evidence-record.schema.json"
    )
    properties = json.loads(schema_path.read_text())["properties"]
    undeclared = sorted(set(record) - set(properties))
    assert not undeclared, f"fields the schema does not declare: {undeclared}"


def test_green_needs_red_record(tmp_path):
    root, run = gate_project.make_project(Path(tmp_path) / "p")
    gate_project.red_commit(root)
    gate_project.commit_story(root, {"tally/blade.py": gate_project.BLADE_FIXED}, "the fix")
    env = gate_project.env_with(root, IMPL)
    code, result, _err = run_gate(root, "gate", "green", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["passed"] is False and result["ok"] is True
    assert any("no red record" in one for one in result["failures"])
    record = json.loads(Path(result["evidence"]).read_text())
    assert record["passed"] is False  # the failing green record is written


def test_frozen_test_changed_fails(tmp_path):
    root, run, env, _red = green_ready(tmp_path)
    # a new commit touches the frozen test file: green must refuse
    gate_project.commit_story(
        root, {"tests/test_blade.py": gate_project.NEW_TESTS + "\n# touched\n"}, "touch"
    )
    code, result, _err = run_gate(root, "gate", "green", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["passed"] is False
    assert any("frozen test changed: tests/test_blade.py" in one for one in result["failures"])


def test_stub_marker_fails(tmp_path):
    root, run, env, _red = green_ready(tmp_path)
    # the stub marker sits in a source file of the story's own commit
    gate_project.commit_story(
        root,
        {"tally/note.py": "# VSDD-STUB: leftover\n"},
        "a forgotten stub",
    )
    code, result, _err = run_gate(root, "gate", "green", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["passed"] is False
    assert any("the stub marker remains in tally/note.py" in one
               for one in result["failures"])


def test_failing_suite_lint_format_each_named(tmp_path):
    failing = f"{sys_executable()} -c 'import sys; sys.exit(3)'"
    root, run, env, _red = green_ready(
        tmp_path, lint=f"{sys_executable()} -c 'import sys; sys.exit(1)'",
        format_check=failing,
    )
    # the test command writes a report whose one case failed, then exits 1
    writer = (
        "import pathlib, sys\n"
        "pathlib.Path('build').mkdir(exist_ok=True)\n"
        "pathlib.Path('build/junit.xml').write_text(\n"
        "    \"<?xml version='1.0'?><testsuites><testsuite name='s'>\"\n"
        "    \"<testcase classname='t.one' name='two'><failure message='m'>x</failure>\"\n"
        "    \"</testcase></testsuite></testsuites>\")\n"
        "sys.exit(1)\n"
    )
    gate_project.commit_story(root, {"tools/write_report.py": writer}, "the report writer")
    from tests.unit._repo import set_keys

    set_keys(root, "commands", test=f"{sys_executable()} tools/write_report.py")
    code, result, _err = run_gate(root, "gate", "green", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["passed"] is False
    names = " | ".join(result["failures"])
    assert "commands.lint failed (exit 1)" in names
    assert "commands.format_check failed" in names
    assert "commands.test failed (exit 1)" in names
    assert "failed: t.one::two" in names  # a failed case in the report is named


def test_frozen_test_missing_from_report_fails(tmp_path):
    short_report = (
        f"{sys_executable()} -c \""
        "import pathlib; pathlib.Path('build').mkdir(exist_ok=True); "
        "pathlib.Path('build/junit.xml').write_text("
        "'<testsuites><testsuite name=\"s\"><testcase classname=\"tests.other\" "
        "name=\"one\"/></testsuite></testsuites>'); "
        "print('written')\""
    )
    root, run, env, _red = green_ready(tmp_path, test_cmd=short_report)
    code, result, _err = run_gate(root, "gate", "green", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["passed"] is False
    assert any("frozen test did not run: tests.test_blade.TestSharpen::test_sharpens_by_one"
               in one for one in result["failures"])


def test_exit_zero_without_report_fails(tmp_path):
    root, run, env, _red = green_ready(
        tmp_path, test_cmd=f"{sys_executable()} -c 'pass'"
    )
    code, result, _err = run_gate(root, "gate", "green", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["passed"] is False
    assert any("wrote no JUnit report" in one for one in result["failures"])


def test_since_narrows_patch_and_lists_changed_files(tmp_path):
    root, run, env, _red = green_ready(tmp_path)
    head = _git_rev(root)
    base = _merge_base(root)
    code, result, _err = run_gate(
        root, "gate", "green", "--story", STORY, "--run", run,
        "--since", base, env=env,
    )
    assert code == 0 and result["passed"] is True, result["failures"]
    assert result["changed_files"] == ["tally/blade.py", "tests/test_blade.py"]
    ranged = Path(result["diff_patch"])
    assert ranged.name == f"diff.{base[:12]}-{head[:12]}.patch"
    _ = head


def test_since_patch_leaves_whole_patch(tmp_path):
    root, run, env, _red = green_ready(tmp_path)
    base = _merge_base(root)
    head_before = _git_rev(root)
    code, result, _err = run_gate(
        root, "gate", "green", "--story", STORY, "--run", run, "--since", base, env=env
    )
    assert code == 0 and result["passed"] is True
    assert Path(result["diff_patch"]).name == f"diff.{base[:12]}-{head_before[:12]}.patch"
    whole = Path(result["diff_patch"]).parent / f"diff.{head_before[:12]}.patch"
    assert whole.is_file(), "the whole base-to-head patch is still there"
    assert "sharpen" in whole.read_text()
    record = json.loads(Path(result["evidence"]).read_text())
    assert record["sha"] == head_before
    assert record["patch_id"]
    assert record["diff_patch"] == str(whole), "the record carries the whole patch (3.3)"


def test_since_not_an_ancestor_refused(tmp_path):
    root, run, env, _red = green_ready(tmp_path)
    stranger = _commit_on_develop(root)
    head = _git_rev(root)
    code, result, _err = run_gate(
        root, "gate", "green", "--story", STORY, "--run", run, "--since", stranger, env=env
    )
    assert code == 1 and result["ok"] is False and result["passed"] is False
    assert any("not an ancestor" in one for one in result["failures"])
    assert result["sha"] == head, "the story resolved, so the head is named"


def test_report_refusal_carries_the_head(tmp_path):
    """A `test_report` that is not a junit path is a refusal, and the story
    resolved: sha is the head, not '' (plan gates.md 3.1)."""
    root, run, env, _red = green_ready(tmp_path)
    from tests.unit._repo import set_keys

    set_keys(root, "commands", test_report="report.xml")
    head = _git_rev(root)
    code, result, _err = run_gate(root, "gate", "green", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["ok"] is False and result["passed"] is False
    assert result["sha"] == head


def test_baseline_failure_line_appears_once(tmp_path):
    root, run, env, _red = green_ready(tmp_path)
    cycle = cycle_read(root, "c-1")
    cycle["baseline"] = "some baseline id"
    cycle_write(root, cycle)
    gate_project.commit_story(root, {"tally/note.py": "# VSDD-STUB: leftover\n"}, "a stub")
    code, result, _err = run_gate(root, "gate", "green", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["passed"] is False
    lines = [one for one in result["failures"]
             if one.startswith("the cycle has a baseline")]
    assert len(lines) == 1, result["failures"]


def test_cached_green_rewrites_patch(tmp_path):
    root, run, env, _red = green_ready(tmp_path)
    code, first, _err = run_gate(
        root, "gate", "green", "--story", STORY, "--run", run, env=env
    )
    assert code == 0 and first["passed"] is True
    Path(first["diff_patch"]).unlink()
    code2, again, _err = run_gate(
        root, "gate", "green", "--story", STORY, "--run", run, env=env
    )
    assert code2 == 0 and again["passed"] is True
    assert Path(again["diff_patch"]).is_file(), "the cached result rewrites the patch"
    assert again["changed_files"] == [], "a cached green without --since names no changed files"


def test_green_xml_lost_is_measured_again(tmp_path):
    """D4: the cached green record also needs its report at the head; a lost
    .xml is a cue to measure again, not a pass forever."""
    root, run, env, _red = green_ready(tmp_path)
    code, first, _err = run_gate(
        root, "gate", "green", "--story", STORY, "--run", run, env=env
    )
    assert code == 0 and first["passed"] is True
    report = Path(first["evidence"]).with_suffix(".xml")
    report.unlink()
    code2, again, _err = run_gate(
        root, "gate", "green", "--story", STORY, "--run", run, env=env
    )
    assert code2 == 0 and again["passed"] is True, again["failures"]
    assert again["metrics"] != {}, "the gate measured again"
    assert Path(again["evidence"]).with_suffix(".xml").is_file(), "the report is written again"


def test_baseline_named_is_strict_and_says_so(tmp_path):
    root, run, env, _red = green_ready(tmp_path)
    cycle = cycle_read(root, "c-1")
    cycle["baseline"] = "some baseline id"
    cycle_write(root, cycle)
    code, result, _err = run_gate(root, "gate", "green", "--story", STORY, "--run", run, env=env)
    assert code == 0 and result["passed"] is True
    assert result["metrics"]["baseline"] == "not applied"


def test_docs_and_ui_checks_required(tmp_path):
    root, run, env, _red = green_ready(tmp_path)
    # the story becomes a docs story with ui: true after red; neither check is configured
    _edit_story_frontmatter(root, "docs", ui="true")
    code, result, _err = run_gate(root, "gate", "green", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["passed"] is False
    names = " | ".join(result["failures"])
    assert "commands.ui_check is not configured" in names
    assert "commands.docs_check" in names


def test_job_failure_keeps_gate_shape(tmp_path):
    """A job-layer failure keeps the gate shape: its ``error`` is copied into
    ``failures`` (D3), so an agent always has a line to act on."""
    from vsdd.gates import gatelib

    result = gatelib.normalize(
        "green", STORY, "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b",
        {"gate": "green", "ok": False,
         "error": "the job lost its worker twice and kept dying; log: x"},
    )
    assert result["ok"] is False and result["passed"] is False
    assert result["failures"] == [
        "the job lost its worker twice and kept dying; log: x"
    ]
    assert result["error"] == result["failures"][0]
    assert result["sha"] == "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b"
    assert_gate_shape(result)


def _merge_base(root) -> str:
    tree = gate_project.worktree_of(root)
    done = subprocess.run(
        ["git", "merge-base", "develop", "HEAD"], cwd=tree, capture_output=True, text=True
    )
    assert done.returncode == 0, done.stderr
    return done.stdout.strip()


def _git_rev(root) -> str:
    tree = gate_project.worktree_of(root)
    done = subprocess.run(["git", "rev-parse", "HEAD"], cwd=tree, capture_output=True, text=True)
    assert done.returncode == 0, done.stderr
    return done.stdout.strip()


def _commit_on_develop(root) -> str:
    scratch = Path(root) / "tally" / "scratch.txt"
    scratch.write_text("on develop\n", encoding="utf-8")
    _git(root, "add", "-A")
    _git(root, "commit", "-qm", "develop moves on")
    done = subprocess.run(["git", "rev-parse", "HEAD"], cwd=root, capture_output=True, text=True)
    return done.stdout.strip()
