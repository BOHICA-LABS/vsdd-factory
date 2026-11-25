"""The red gate (plan gates.md section 7; every result checked against
gate-result)."""

import json
import sys
import time
from pathlib import Path

from tests.unit import gate_project
from tests.unit.gate_project import STORY, run_gate
from vsdd.schema import validate
from vsdd.state.store import _registry, schema_of

IMPL = gate_project.IMPLEMENTER
SOURCE_CLASS_TEST = '''"""A test file inside the product's source tree."""

import unittest


class TestInsideSource(unittest.TestCase):
    def test_inside_source(self):
        raise AssertionError("this test is classed source and does not count")
'''


def assert_gate_shape(result: dict) -> None:
    problems = validate(result, schema_of("gate-result"), _registry())
    assert not problems, "; ".join(str(one) for one in problems[:5])


def red(tmp_path, *, tests: str | None = None, story_kind: str | None = None):
    """A project with the story committed and red measured."""
    root, run = gate_project.make_project(tmp_path)
    if story_kind is not None:
        _set_kind(root, story_kind)
    gate_project.red_commit(root, tests=tests)
    env = gate_project.env_with(root, IMPL)
    code, result, err = run_gate(root, "gate", "red", "--story", STORY, "--run", run, env=env)
    assert_gate_shape(result)
    return root, run, code, result, env


def _set_kind(root, kind: str) -> None:
    """The story file's kind, edited in place: the gate reads the file, and
    the .factory state directory is not tracked on the integration branch."""
    story_file = next((Path(root) / ".factory" / "stories").glob(f"*/{STORY}.md"))
    text = story_file.read_text().replace("kind: fix", f"kind: {kind}", 1)
    story_file.write_text(text)


def test_red_passes_and_freezes(tmp_path):
    root, run, code, result, _env = red(tmp_path)
    assert code == 0 and result["passed"] is True
    assert result["gate"] == "red" and result["sha"]
    record = json.loads(Path(result["evidence"]).read_text())
    assert record["passed"] is True
    frozen = [entry["file"] for entry in record["frozen"]]
    assert frozen == ["tests/test_blade.py"]
    assert sorted(record["tests"]) == [
        "tests.test_blade.TestSharpen::test_ac1_sharpen_is_linenoise",
        "tests.test_blade.TestSharpen::test_sharpen_zero",
        "tests.test_blade.TestSharpen::test_sharpens_by_one",
    ]
    assert record["criteria"] == {"AC-1": [
        "tests.test_blade.TestSharpen::test_ac1_sharpen_is_linenoise"]}
    report = Path(result["evidence"]).with_suffix(".xml")
    assert report.is_file() and "testcase" in report.read_text()
    base = Path(root) / ".factory" / "local" / "evidence" / "_base"
    assert list(base.glob("*.xml")), "the base report is kept"


def test_base_report_measured_once(tmp_path):
    root, run, _code, _result, _env = red(tmp_path)
    gate_project.commit_story(root, {"tally/more.py": "VALUE = 1\n"}, "an unrelated commit")
    env = gate_project.env_with(root, IMPL)
    code, again, _err = run_gate(root, "gate", "red", "--story", STORY, "--run", run, env=env)
    assert code == 0 and again["passed"] is True, again["failures"]
    record_log = Path(again["evidence"]).with_suffix(".log").read_text()
    assert "# base report reused" in record_log
    _ = _result


def test_unparsable_base_report_is_measured_again(tmp_path):
    """A kept base report that does not parse (a kill mid-write) is deleted
    and the base measured again — never read silently as an empty set."""
    from vsdd.gates.junit import read_report

    root, run, _code, _result, _env = red(tmp_path)
    base_dir = Path(root) / ".factory" / "local" / "evidence" / "_base"
    kept = next(base_dir.glob("*.xml"))
    kept.write_text("<testsuites><broken>", encoding="utf-8")
    gate_project.commit_story(root, {"tally/more.py": "VALUE = 2\n"}, "another commit")
    env = gate_project.env_with(root, IMPL)
    code2, again, _err = run_gate(root, "gate", "red", "--story", STORY, "--run", run, env=env)
    assert code2 == 0 and again["passed"] is True, again["failures"]
    record_log = Path(again["evidence"]).with_suffix(".log").read_text()
    assert "# base report reused" not in record_log, "the base was measured again"
    read_report(next(base_dir.glob("*.xml")))  # the kept report parses again


def test_retention_reads_the_state_setting(tmp_path):
    """Base reports older than ``state.retain_runs_days`` go when a new one
    is written; the job's own configuration is the source (D7)."""
    import os
    import time

    root, run, _code, _result, _env = red(tmp_path)
    from tests.unit._repo import set_keys

    set_keys(root, "state", retain_runs_days=1)
    base_dir = Path(root) / ".factory" / "local" / "evidence" / "_base"
    aged = base_dir / "aaaaaaaaaaaa.xml"  # an old report beside the kept one
    aged.write_text("<testsuites/>", encoding="utf-8")
    old = time.time() - 3 * 86400
    os.utime(aged, (old, old))
    kept = next(one for one in base_dir.glob("*.xml") if one != aged)
    kept.unlink()  # so a new base report is written, and retention with it
    gate_project.commit_story(root, {"tally/more.py": "VALUE = 3\n"}, "third commit")
    env = gate_project.env_with(root, IMPL)
    code2, again, _err = run_gate(root, "gate", "red", "--story", STORY, "--run", run, env=env)
    assert code2 == 0 and again["passed"] is True, again["failures"]
    assert not aged.exists(), "the aged report is gone at one retained day"
    assert next(base_dir.glob("*.xml")), "the fresh kept report is there"


def test_stub_error_counts_as_red(tmp_path):
    stub_errors = gate_project.NEW_TESTS.replace(
        "self.assertEqual(sharpen(1), 2)", "raise NotImplementedError('stub')"
    ).replace("self.assertEqual(sharpen(0), 1)", "pass").replace(
        "self.assertEqual(sharpen(3), 4)", "raise NotImplementedError('stub')"
    )
    root, run, code, result, _env = red(tmp_path, tests=stub_errors)
    _ = root, run
    assert code == 0 and result["passed"] is True, result["failures"]
    assert result["metrics"]["red"] == 2


def test_harness_error_fails(tmp_path):
    broken = gate_project.NEW_TESTS.replace(
        "from tally.blade import sharpen", "from tally.missing_module import sharpen"
    )
    root, run, code, result, _env = red(tmp_path, tests=broken)
    _ = root, run
    assert code == 1 and result["passed"] is False
    assert any(
        "build or harness error in" in one and "ImportError" in one
        for one in result["failures"]
    ), result["failures"]


def test_half_rule_and_exemptions(tmp_path):
    three_tests = (
        "import unittest\n\n\nclass TestHalf(unittest.TestCase):\n"
        "    def test_one_red(self):\n        assert False\n"
        "    def test_two_passes(self):\n        assert True\n"
        "    def test_three_passes(self):\n        assert True\n"
    )
    root, run, code, result, _env = red(tmp_path, tests=three_tests)
    _ = root
    assert code == 1 and result["passed"] is False
    assert any("fewer than half" in one for one in result["failures"])
    # exemptions alone never pass: two wiring tests and no red test
    exempt_only = (
        "import unittest\n\n\nclass TestWires(unittest.TestCase):\n"
        "    def test_wiring_one(self):\n        assert True\n"
        "    def test_GREEN_BY_DESIGN_two(self):\n        assert True\n"
    )
    root2, run2, code2, result2, _env2 = red(tmp_path / "2", tests=exempt_only)
    _ = run2
    assert code2 == 1 and result2["passed"] is False
    assert any("no red test" in one or "no new tests" in one for one in result2["failures"])
    _ = result2["metrics"]
    # a red exempt test does not feed the half rule: 0 of 1 non-exempt is red
    mixed = (
        "import unittest\n\n\nclass TestMixed(unittest.TestCase):\n"
        "    def test_plain_passes(self):\n        assert True\n"
        "    def test_wiring_broken(self):\n        assert False\n"
    )
    root3, run3, code3, result3, _env3 = red(tmp_path / "3", tests=mixed)
    _ = root3, run3
    assert code3 == 1 and result3["passed"] is False
    assert any("fewer than half" in one for one in result3["failures"])
    assert result3["metrics"]["red"] == 1 and result3["metrics"]["exempt"] == 1


def test_no_new_tests_fails(tmp_path):
    root, run, code, result, _env = red(tmp_path, tests=""""placeholder"\n""")
    _ = root, run
    assert code == 1 and result["passed"] is False
    assert any("no new tests" in one for one in result["failures"])


def test_source_class_test_does_not_count(tmp_path):
    """A test in a source-class file does not count as new (D8, finding
    S4R-gates-15). The sample runner discovers tally/ as well, so the case
    really reaches the report — the filter, not the runner, drops it."""
    root, run = gate_project.make_project(tmp_path)
    gate_project.commit_story(
        root,
        {
            "tally/blade.py": gate_project.BLADE_WITH_STUB,
            "tests/test_blade.py": gate_project.NEW_TESTS,
            "tally/test_in_src.py": SOURCE_CLASS_TEST,
        },
        "red commit",
    )
    env = gate_project.env_with(root, IMPL)
    code, result, _err = run_gate(
        root, "gate", "red", "--story", STORY, "--run", run, env=env
    )
    assert code == 0 and result["passed"] is True
    report = Path(result["evidence"]).with_suffix(".xml").read_text()
    assert 'file="tally/test_in_src.py"' in report, "the case must reach the report"
    assert result["metrics"]["new"] == 3  # the source-class test does not count
    record = json.loads(Path(result["evidence"]).read_text())
    assert not any("test_in_src" in one for one in record["tests"])
    assert "tally/test_in_src.py" not in [entry["file"] for entry in record["frozen"]]


def test_dirty_worktree_writes_nothing(tmp_path):
    root, run = gate_project.make_project(tmp_path)
    gate_project.red_commit(root)
    tree = gate_project.worktree_of(root)
    (tree / "tally" / "touched.py").write_text("TOUCHED = 1\n")
    (tree / "tests" / "untracked_new.py").write_text("UNT = 1\n")
    (tree / "build").mkdir(exist_ok=True)
    (tree / "build" / "junit.xml").write_text("<testsuites/>")  # the report alone is not dirt
    env = gate_project.env_with(root, IMPL)
    code, result, _err = run_gate(
        root, "gate", "red", "--story", STORY, "--run", run, env=env
    )
    assert code == 1 and result["passed"] is False and result["ok"] is True
    names = " ".join(result["failures"])
    assert "uncommitted changes" in names and "touched.py" in names
    assert "untracked_new.py" in names and "junit.xml" not in names
    assert not (Path(root) / ".factory" / "local" / "evidence" / STORY).exists()


def test_passing_record_returned_without_measuring(tmp_path):
    root, run, code, result, _env = red(tmp_path)
    assert "changed_files" not in result, "changed_files is a green field (3.1)"
    evidence = Path(result["evidence"])
    stamp = evidence.stat().st_mtime_ns
    time.sleep(1.1)  # a whole second, so a new record could not look the same
    env = gate_project.env_with(root, IMPL)
    code2, again, _err = run_gate(
        root, "gate", "red", "--story", STORY, "--run", run, env=env
    )
    assert code2 == 0 and again["passed"] is True
    assert Path(again["evidence"]).stat().st_mtime_ns == stamp
    assert again["metrics"] == {}
    # the cached result keeps the red shape: changed_files is green's alone
    assert "changed_files" not in again


def test_failing_record_measured_again(tmp_path):
    root, run, code, result, _env = red(tmp_path, tests=""""placeholder"\n""")
    _ = code, result
    evidence_dir = Path(root) / ".factory" / "local" / "evidence" / STORY
    first = sorted(p.name for p in evidence_dir.glob("red.*.json"))
    assert first, "a failing red records its measurement"
    # now the story grows real failing tests: the gate measures again and passes
    gate_project.commit_story(
        root, {"tests/test_blade.py": gate_project.NEW_TESTS}, "the real tests"
    )
    env = gate_project.env_with(root, IMPL)
    code2, again, _err = run_gate(
        root, "gate", "red", "--story", STORY, "--run", run, env=env
    )
    assert code2 == 0 and again["passed"] is True, again["failures"]
    second = sorted(p.name for p in evidence_dir.glob("red.*.json"))
    assert second != first or json.loads((evidence_dir / second[0]).read_text())["passed"]


def test_no_report_is_loud(tmp_path):
    root, run = gate_project.make_project(tmp_path)
    gate_project.red_commit(root)
    import subprocess

    done = subprocess.run(
        ["git", "commit", "-q", "--allow-empty", "-m", "x"],
        cwd=gate_project.worktree_of(root),
        capture_output=True,
        text=True,
    )
    assert done.returncode == 0
    set_test_cmd = f"{sys.executable} -c 'pass'"  # writes no report
    _apply_test_cmd(root, set_test_cmd)
    env = gate_project.env_with(root, IMPL)
    code, result, _err = run_gate(
        root, "gate", "red", "--story", STORY, "--run", run, env=env
    )
    assert code == 1 and result["passed"] is False and result["ok"] is True
    assert any("wrote no JUnit report" in one for one in result["failures"])


def test_stale_report_is_not_read(tmp_path):
    root, run = gate_project.make_project(tmp_path)
    gate_project.red_commit(root)
    tree = gate_project.worktree_of(root)
    stale = tree / "build" / "junit.xml"
    stale.parent.mkdir(exist_ok=True)
    stale.write_text(
        '<testsuites><testsuite name="ghost">'
        '<testcase classname="tests.ghost" name="test_ghost_passes"/>'
        "</testsuite></testsuites>",
        encoding="utf-8",
    )
    env = gate_project.env_with(root, IMPL)
    code, result, _err = run_gate(
        root, "gate", "red", "--story", STORY, "--run", run, env=env
    )
    assert code == 0 and result["passed"] is True, result["failures"]
    record = json.loads(Path(result["evidence"]).read_text())
    assert "tests.ghost::test_ghost_passes" not in record["tests"]
    assert "tests.ghost::test_ghost_passes" not in record["criteria"].get("AC-1", [True])


def test_kind_without_red_gate_fails_loudly(tmp_path):
    root, run, code, result, _env = red(tmp_path, story_kind="chore")
    _ = root, run
    assert code == 1 and result["ok"] is True and result["passed"] is False
    assert any("story kind chore has no red gate" in one for one in result["failures"])
    assert not list((Path(root) / ".factory" / "local" / "evidence" / STORY).glob("red.*.json"))


def test_story_not_in_run_refused(tmp_path):
    root, run = gate_project.make_project(tmp_path)
    gate_project.red_commit(root)
    env = gate_project.env_with(root, IMPL)
    code, result, _err = run_gate(
        root, "gate", "red", "--story", STORY, "--run", "r-nope-00", env=env
    )
    assert code == 1 and result["ok"] is False and result["passed"] is False
    assert any("admission refused" in one for one in result["failures"])
    assert not (Path(root) / ".factory" / "local" / "evidence" / STORY).exists()


def test_head_moved_writes_nothing(tmp_path):
    root, run = gate_project.make_project(tmp_path)
    mover = f"{sys.executable} tools/junit_runner.py; git commit -q --allow-empty -m move"
    _apply_test_cmd(root, mover)
    gate_project.red_commit(root)
    env = gate_project.env_with(root, IMPL)
    code, result, _err = run_gate(
        root, "gate", "red", "--story", STORY, "--run", run, env=env
    )
    assert code == 1 and result["passed"] is False
    assert any("head moved" in one for one in result["failures"])
    assert not (Path(root) / ".factory" / "local" / "evidence" / STORY).exists()


def test_record_over_cap_is_loud(tmp_path):
    many = ["import unittest", "", "", "class TestMany(unittest.TestCase):"]
    for n in range(1, 241):
        many.append(f"    def test_ac1_case_{n}(self):\n        assert False\n")
    root, run, code, result, _env = red(tmp_path, tests="\n".join(many))
    _ = root, run
    assert code == 1 and result["passed"] is False
    assert any("over its cap" in one and "split the story" in one
               for one in result["failures"])
    records = list((Path(root) / ".factory" / "local" / "evidence" / STORY).glob("red.*.json"))
    assert records == [], "nothing is cut: an over-cap record writes nothing"


def _apply_test_cmd(root, command: str) -> None:
    from tests.unit._repo import set_keys

    set_keys(root, "commands", test=command)
