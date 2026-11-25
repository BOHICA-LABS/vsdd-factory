"""The demo gate (plan gates.md D14, 3.5)."""

import shutil
from pathlib import Path

from tests.unit import gate_project
from tests.unit.gate_project import STORY, run_gate
from tests.unit.test_gate_green import green_ready
from tests.unit.test_gate_red import assert_gate_shape

IMPL = gate_project.IMPLEMENTER


def demo_ready(tmp_path, *, name: str = "proj"):
    root, run, env, _red = green_ready(tmp_path, name=name)
    code, green, _err = run_gate(root, "gate", "green", "--story", STORY, "--run", run, env=env)
    assert code == 0 and green["passed"] is True, green["failures"]
    return root, run, env, green


def test_table_built_without_recording(tmp_path):
    root, run, env, _green = demo_ready(tmp_path)
    code, result, _err = run_gate(root, "gate", "demo", "--story", STORY, "--run", run, env=env)
    assert code == 0 and result["passed"] is True, result["failures"]
    assert result["metrics"]["mode"] == "table" and result["metrics"]["recorded"] is False
    table = Path(result["evidence"]).with_suffix(".log").read_text()
    assert "AC-1: tests.test_blade.TestSharpen::test_ac1_sharpen_is_linenoise -> passed" in table
    record = Path(result["evidence"])
    assert record.is_file()


def test_demo_needs_green_evidence(tmp_path):
    root, run = gate_project.make_project(tmp_path)
    gate_project.red_commit(root)
    env = gate_project.env_with(root, IMPL)
    code, result, _err = run_gate(root, "gate", "demo", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["passed"] is False and result["ok"] is True
    assert any("no green evidence covers the head" in one for one in result["failures"])


def test_demo_missing_report_copy_fails(tmp_path):
    root, run, env, green = demo_ready(tmp_path)
    Path(green["evidence"]).with_suffix(".xml").unlink()
    code, result, _err = run_gate(root, "gate", "demo", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["passed"] is False
    assert any("green report copy is missing" in one for one in result["failures"])
    _ = root


def test_recording_checked(tmp_path):
    root, run, env, _green = demo_ready(tmp_path)
    _set_demo_true(root)
    demo_dir = Path(root) / ".factory" / "demo" / "c-1" / STORY
    _recording(demo_dir, complete=True)
    code, result, _err = run_gate(root, "gate", "demo", "--story", STORY, "--run", run, env=env)
    assert code == 0 and result["passed"] is True, result["failures"]
    assert result["metrics"]["mode"] == "recorded" and result["metrics"]["recorded"] is True
    record = Path(result["evidence"])
    assert record.is_file()

    # each missing part fails
    for dropped, label in (
        ("report.md", "report"),
        ("AC-1.sh", "script"),
        ("AC-1.txt", "transcript"),
    ):
        root2, run2, env2, _g = demo_ready(tmp_path / f"x-{label}", name=label.replace(".", ""))
        _set_demo_true(root2)
        directory = Path(root2) / ".factory" / "demo" / "c-1" / STORY
        _recording(directory, complete=False)
        (directory / dropped).unlink()
        code2, result2, _e = run_gate(
            root2, "gate", "demo", "--story", STORY, "--run", run2, env=env2
        )
        assert code2 == 1 and result2["passed"] is False, (dropped, result2["failures"])


def _set_demo_true(root) -> None:
    """The story's frontmatter gains `demo: true`: the recording mode. The
    recording's `AC-2` files must name a criterion the story states, so the
    second criterion heading is added with the flag."""
    story_file = next((Path(root) / ".factory" / "stories").glob(f"*/{STORY}.md"))
    text = story_file.read_text().replace("demo: false", "demo: true", 1)
    text = text.replace(
        "### AC-1 (BC-1.00.001)",
        "### AC-1 (BC-1.00.001)\n\n### AC-2 (BC-1.00.002)",
        1,
    )
    story_file.write_text(text)


def _recording(directory: Path, *, complete: bool) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "report.md").write_text("# demo\nthe story demonstrably works\n",
                                         encoding="utf-8")
    (directory / "AC-1.sh").write_text("echo demo\n", encoding="utf-8")
    (directory / "AC-1.txt").write_text("$ echo demo\ndemo\n", encoding="utf-8")
    if complete:
        (directory / "AC-2.spec.ts").write_text("// script\n" * 10, encoding="utf-8")
        (directory / "AC-2.txt").write_text("transcript\n", encoding="utf-8")
        (directory / "AC-2.png").write_bytes(b"\x89PNG\r\n\x1a\n" + b"0" * 64)


def test_demo_oversized_parts_fail(tmp_path):
    root, run, env, _green = demo_ready(tmp_path)
    _set_demo_true(root)
    directory = Path(root) / ".factory" / "demo" / "c-1" / STORY
    _recording(directory, complete=True)
    (directory / "AC-1.txt").write_text("x" * 21_000, encoding="utf-8")
    code, result, _err = run_gate(root, "gate", "demo", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["passed"] is False
    assert any("over the 20000-byte transcript cap" in one for one in result["failures"])


def test_demo_file_that_names_no_criterion_fails(tmp_path):
    root, run, env, _green = demo_ready(tmp_path)
    _set_demo_true(root)
    directory = Path(root) / ".factory" / "demo" / "c-1" / STORY
    _recording(directory, complete=True)
    shutil.copy(directory / "AC-1.txt", directory / "loose-notes.txt")
    code, result, _err = run_gate(root, "gate", "demo", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["passed"] is False
    assert any("does not name a criterion" in one for one in result["failures"])


def test_demo_returns_directly(tmp_path):
    """As G4a calls it: never pending, the result straight away, through the
    registry contract's wait and retry (taken and ignored)."""
    from vsdd.state.gate_evidence import registered_gate

    root, run, _env, _green = demo_ready(tmp_path)
    result = registered_gate("demo")(root, STORY, run, wait=0, retry="x")
    assert result["ok"] is True and result["passed"] is True
    assert "state" not in result
    assert result["evidence"]


def test_passing_record_returned_unmeasured(tmp_path):
    """The passing record for the head is returned without measuring, so a
    later failing measurement never replaces it (plan gates.md 3.3)."""
    root, run, env, green = demo_ready(tmp_path)
    code, first, _err = run_gate(root, "gate", "demo", "--story", STORY, "--run", run, env=env)
    assert code == 0 and first["passed"] is True
    stamp = Path(first["evidence"]).stat().st_mtime_ns
    # a measurement now would fail: the green report copy is gone
    Path(green["evidence"]).with_suffix(".xml").unlink()
    code2, again, _err = run_gate(root, "gate", "demo", "--story", STORY, "--run", run, env=env)
    assert code2 == 0 and again["passed"] is True, again["failures"]
    assert Path(again["evidence"]).stat().st_mtime_ns == stamp
    assert again["metrics"] == {}


def test_failing_table_carries_its_failures(tmp_path):
    """A failing table is passed false with its failures and the first one as
    the error (plan gates.md 3.1, D3) — never an empty failure list."""
    root, run, env, green = demo_ready(tmp_path)
    report = Path(green["evidence"]).with_suffix(".xml")
    text = report.read_text()
    marker = 'name="test_ac1_sharpen_is_linenoise"'
    head_, sep, tail = text.partition(marker)
    assert sep, "the AC-1 testcase is in the green report"
    inner, end, rest = tail.partition("/>")
    assert end, "the passed testcase is self-closing"
    report.write_text(head_ + sep + inner + "><failure message='m'>x</failure></testcase>" + rest,
                      encoding="utf-8")
    code, result, _err = run_gate(root, "gate", "demo", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["ok"] is True and result["passed"] is False
    assert result["failures"], "a failing result names at least one failure"
    assert result["error"] == result["failures"][0]
    assert any("not passed in the green report" in one for one in result["failures"])
    assert_gate_shape(result)


def test_demo_refusal_is_not_attempted(tmp_path):
    """A story outside the run is not attempted: ok false, one 'admission
    refused' line, no doubled prefix."""
    root, run = gate_project.make_project(tmp_path)
    gate_project.red_commit(root)
    env = gate_project.env_with(root, IMPL)
    code, result, _err = run_gate(root, "gate", "demo", "--story", STORY, "--run", "r-nope-00", env=env)
    assert code == 1 and result["ok"] is False and result["passed"] is False
    assert len(result["failures"]) == 1
    assert result["failures"][0].count("admission refused") == 1
    assert result["error"] == result["failures"][0]
    assert_gate_shape(result)


def test_over_twenty_bad_demo_files_are_capped(tmp_path):
    """More bad demo files than the failure cap leave a result the
    gate-result schema still holds: twenty lines and one 'and <n> more'."""
    import shutil

    root, run, env, _green = demo_ready(tmp_path)
    _set_demo_true(root)
    directory = Path(root) / ".factory" / "demo" / "c-1" / STORY
    _recording(directory, complete=True)
    for n in range(21):
        shutil.copy(directory / "AC-1.txt", directory / f"loose-{n:02d}.txt")
    code, result, _err = run_gate(root, "gate", "demo", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["passed"] is False
    assert len(result["failures"]) == 20
    assert result["failures"][-1] == "and 2 more"
    assert_gate_shape(result)


def test_uncovered_criterion_is_listed_and_passes(tmp_path):
    """A criterion the story states with no test is uncovered: named in the
    metrics and the table, never a failure (D14)."""
    root, run, env, _green = demo_ready(tmp_path)
    story_file = next((Path(root) / ".factory" / "stories").glob(f"*/{STORY}.md"))
    text = story_file.read_text().replace(
        "### AC-1 (BC-1.00.001)", "### AC-1 (BC-1.00.001)\n\n### AC-2 (BC-1.00.002)", 1
    )
    story_file.write_text(text)
    code, result, _err = run_gate(root, "gate", "demo", "--story", STORY, "--run", run, env=env)
    assert code == 0 and result["passed"] is True, result["failures"]
    assert result["metrics"]["uncovered"] == ["AC-2"]
    table = Path(result["evidence"]).with_suffix(".log").read_text()
    assert "AC-2: (no test) -> uncovered" in table


def test_recording_files_must_name_a_story_criterion(tmp_path):
    """In recorded mode an AC-<k> file of a criterion the story lacks fails,
    and so does a screenshot that is not AC-<k>.png (D14)."""
    root, run, env, _green = demo_ready(tmp_path)
    _set_demo_true(root)
    directory = Path(root) / ".factory" / "demo" / "c-1" / STORY
    _recording(directory, complete=True)
    (directory / "AC-9.txt").write_text("transcript of a criterion the story lacks\n",
                                        encoding="utf-8")
    (directory / "screenshots.png").write_bytes(b"\x89PNG\r\n\x1a\n")
    code, result, _err = run_gate(root, "gate", "demo", "--story", STORY, "--run", run, env=env)
    assert code == 1 and result["passed"] is False
    names = " | ".join(result["failures"])
    assert "a demo file names a criterion the story lacks: AC-9.txt" in names
    assert "a demo file does not name a criterion: screenshots.png" in names


def test_demo_result_is_built_by_the_shared_builder(tmp_path):
    """The demo caps come from the [demo-evidence] policy row, the one
    source (plan section 4) — the constants are no second copy."""
    import re

    from vsdd.gates import gate_demo
    from vsdd.policy import load

    row = load("artifacts")["demo-evidence"]
    assert gate_demo.STORY_CAP == row["max_bytes"]
    note = row["cap_note"]
    for word, cap in (("script", gate_demo.SCRIPT_CAP),
                      ("transcript", gate_demo.TRANSCRIPT_CAP),
                      ("screenshot", gate_demo.SCREENSHOT_CAP)):
        found = re.search(rf"{word} ([\d,]+)", note)
        assert found and cap == int(found.group(1).replace(",", ""))
