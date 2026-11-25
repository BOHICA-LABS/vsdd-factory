"""The re-freeze (plan gates.md D10, D11; finding P3SEAM-15, AU-S2-12)."""

import json
import subprocess
import sys
from pathlib import Path

from tests.unit import gate_project
from tests.unit.gate_project import STORY, run_gate
from tests.unit.test_gate_red import SOURCE_CLASS_TEST, assert_gate_shape

TW = gate_project.TEST_WRITER
IMPL = gate_project.IMPLEMENTER


def _git(tree: Path, *args: str) -> None:
    done = subprocess.run(["git", *args], cwd=tree, capture_output=True, text=True)
    assert done.returncode == 0, done.stderr

CHANGED_TESTS = gate_project.NEW_TESTS.replace(
    "self.assertEqual(sharpen(1), 2)", "self.assertEqual(sharpen(1), 3)"
)


def setup_red(tmp_path, *, name: str = "proj"):
    """A project whose plain red gate passed."""
    root, run = gate_project.make_project(Path(tmp_path) / name)
    gate_project.red_commit(root)
    env = gate_project.env_with(root, gate_project.IMPLEMENTER)
    code, red, _err = run_gate(root, "gate", "red", "--story", STORY, "--run", run, env=env)
    assert code == 0 and red["passed"] is True, red["failures"]
    return root, run


def refreeze(root, run, env):
    code, result, _err = run_gate(
        root, "gate", "red", "--story", STORY, "--run", run, "--refreeze", env=env
    )
    assert_gate_shape(result)
    return code, result


def test_refreeze_nothing_changed_passes_without_suite(tmp_path):
    root, run = setup_red(tmp_path)
    tw_env = gate_project.env_with(root, TW)
    # a source-only commit moves the head: no frozen file changed
    gate_project.commit_story(root, {"tally/blade.py": gate_project.BLADE_FIXED}, "the fix")
    code, result = refreeze(root, run, tw_env)
    assert code == 0 and result["passed"] is True, result["failures"]
    record = json.loads(Path(result["evidence"]).read_text())
    assert record["refreeze"] is True and record["gate"] == "red"
    assert record["sha"] == result["sha"]  # written at the head
    log = Path(result["evidence"]).with_suffix(".log").read_text()
    assert "the suite does not run" in log
    # the earlier tests and criteria stand
    assert record["tests"] and record["criteria"]["AC-1"]
    frozen = {entry["file"] for entry in record["frozen"]}
    assert frozen == {"tests/test_blade.py"}


def test_refreeze_same_head_returns_the_record(tmp_path):
    root, run = setup_red(tmp_path)
    tw_env = gate_project.env_with(root, TW)
    _code, first = refreeze(root, run, tw_env)
    evidence = Path(first["evidence"])
    stamp = evidence.stat().st_mtime_ns
    _code, again = refreeze(root, run, tw_env)
    assert again["passed"] is True
    assert Path(again["evidence"]).stat().st_mtime_ns == stamp
    assert again["metrics"] == {}


def test_refreeze_records_changed_and_added(tmp_path):
    root, run = setup_red(tmp_path)
    tw_env = gate_project.env_with(root, TW)
    # the test-writer changes the frozen test and adds another file of tests
    added = (
        "import unittest\n\n\nclass TestAdded(unittest.TestCase):\n"
        "    def test_added_red(self):\n        assert False\n"
        "    def test_ac1_added_cover(self):\n        assert False\n"
    )
    gate_project.commit_story(
        root,
        {"tests/test_blade.py": CHANGED_TESTS, "tests/test_added.py": added},
        "the test-writer's fix",
    )
    code, result = refreeze(root, run, tw_env)
    assert code == 0 and result["passed"] is True, result["failures"]
    record = json.loads(Path(result["evidence"]).read_text())
    files = sorted(entry["file"] for entry in record["frozen"])
    assert files == ["tests/test_added.py", "tests/test_blade.py"]
    assert "tests.test_added.TestAdded::test_added_red" in record["tests"]
    assert record["criteria"]["AC-1"]
    log = Path(result["evidence"]).with_suffix(".log").read_text()
    assert "the suite does not run" not in log


def test_refreeze_accepts_a_failing_assertion(tmp_path):
    root, run = setup_red(tmp_path)
    tw_env = gate_project.env_with(root, TW)
    gate_project.commit_story(
        root, {"tests/test_blade.py": CHANGED_TESTS}, "the test-writer's fix"
    )
    code, result = refreeze(root, run, tw_env)
    assert code == 0 and result["passed"] is True, result["failures"]
    record = json.loads(Path(result["evidence"]).read_text())
    assert record["tests"]  # the new set, failing on assertions included


def test_refreeze_refuses_removed_errored_skipped(tmp_path):
    root, run = setup_red(tmp_path)
    tw_env = gate_project.env_with(root, TW)
    evidence_dir = Path(root) / ".factory" / "local" / "evidence" / STORY

    # removed: the frozen test file disappears from the story's diff
    tree = gate_project.worktree_of(root)
    _git(tree, "rm", "-q", "tests/test_blade.py")
    _git(tree, "commit", "-qm", "the frozen file is gone")
    _ = tw_env
    code, result = refreeze(root, run, tw_env)
    assert code == 1 and result["passed"] is False and result["ok"] is True
    assert any("frozen test file removed" in one for one in result["failures"])
    # the earlier record stands
    records = list(evidence_dir.glob("red.*.json"))
    assert len(records) == 1
    earlier = json.loads(records[0].read_text())
    assert earlier["passed"] is True and not earlier.get("refreeze")

    # errored: the changed frozen test breaks its import, so it cannot run
    root2, run2 = setup_red(tmp_path, name="proj2")
    tw_env2 = gate_project.env_with(root2, TW)
    errored = gate_project.NEW_TESTS.replace(
        "from tally.blade import sharpen", "from tally.no_such_module import sharpen"
    )
    gate_project.commit_story(root2, {"tests/test_blade.py": errored}, "broken import")
    code2, result2 = refreeze(root2, run2, tw_env2)
    assert code2 == 1 and result2["passed"] is False
    names2 = " | ".join(result2["failures"])
    assert "build or harness error" in names2 or "did not run" in names2, result2["failures"]

    # skipped: a changed frozen test that is skipped fails the re-freeze
    root3, run3 = setup_red(tmp_path, name="proj3")
    tw_env3 = gate_project.env_with(root3, TW)
    skipped = gate_project.NEW_TESTS.replace(
        "    def test_sharpens_by_one(self):",
        "    @unittest.skip('later')\n    def test_sharpens_by_one(self):",
    )
    gate_project.commit_story(root3, {"tests/test_blade.py": skipped}, "skipping a frozen test")
    code3, result3 = refreeze(root3, run3, tw_env3)
    assert code3 == 1 and result3["passed"] is False
    assert any("skipped" in one for one in result3["failures"])


def test_refreeze_needs_a_red_record(tmp_path):
    root, run = gate_project.make_project(tmp_path)
    gate_project.commit_story(
        root, {"tests/test_fresh.py": "def test_only():\n    assert False\n"}, "tests, no red"
    )
    tw_env = gate_project.env_with(root, TW)
    code, result = refreeze(root, run, tw_env)
    assert code == 1 and result["ok"] is True and result["passed"] is False
    assert any("no red record" in one for one in result["failures"])
    assert not (Path(root) / ".factory" / "local" / "evidence" / STORY).exists()


def test_refreeze_only_for_the_test_writer(tmp_path):
    root, run = setup_red(tmp_path)
    # the implementer's marker: no grant
    impl_env = gate_project.env_with(root, IMPL)
    code, result = refreeze(root, run, impl_env)
    assert code == 1 and result["ok"] is False and result["passed"] is False
    assert any("may not re-freeze" in one for one in result["failures"])
    # no marker at all: refused, naming the one-segment form
    plain_env = {"HOME": str(Path(root).parent)}
    code2, result2 = refreeze(root, run, plain_env)
    assert code2 == 1 and result2["ok"] is False
    assert any("single-segment" in one for one in result2["failures"])
    # the test-writer's marker is the grant
    tw_env = gate_project.env_with(root, TW)
    code3, result3 = refreeze(root, run, tw_env)
    assert code3 == 0 and result3["passed"] is True


def test_refreeze_ignores_unchanged_frozen_errors(tmp_path):
    """D10: every earlier identity must still be reported; the skip and
    harness checks belong to the changed files alone. An unchanged frozen
    test broken by half-done code does not refuse the re-freeze."""
    other = (
        "import unittest\n\n\nclass TestOther(unittest.TestCase):\n"
        "    def test_needs_module(self):\n        import tally.helper\n\n"
        "        assert True\n"
        "    def test_other_red(self):\n        assert False\n"
    )
    root, run = gate_project.make_project(Path(tmp_path) / "proj")
    gate_project.commit_story(
        root,
        {
            "tally/blade.py": gate_project.BLADE_WITH_STUB,
            "tests/test_blade.py": gate_project.NEW_TESTS,
            "tests/test_other.py": other,
            "tally/helper.py": "VALUE = 1\n",
        },
        "the story's first commit",
    )
    env = gate_project.env_with(root, gate_project.IMPLEMENTER)
    code, red, _err = run_gate(root, "gate", "red", "--story", STORY, "--run", run, env=env)
    assert code == 0 and red["passed"] is True, red["failures"]
    # the implementer's half-done work removes the module the unchanged
    # test loads; the test-writer's edit touches only the other frozen file
    tree = gate_project.worktree_of(root)
    _git(tree, "rm", "-q", "tally/helper.py")
    _git(tree, "commit", "-qm", "half-done: the helper goes")
    gate_project.commit_story(
        root, {"tests/test_blade.py": gate_project.NEW_TESTS + "\n# touched\n"}, "a cosmetic edit"
    )
    tw_env = gate_project.env_with(root, TW)
    code2, result = refreeze(root, run, tw_env)
    assert code2 == 0 and result["passed"] is True, result["failures"]
    record = json.loads(Path(result["evidence"]).read_text())
    assert record["refreeze"] is True


def test_refreeze_names_an_error_that_has_no_type(tmp_path):
    """An earlier test reported with an <error> of no type at all is a
    harness error named 'unknown' — the worker never crashes (D9, D10)."""
    root, run = setup_red(tmp_path)
    writer = (
        "import pathlib\n"
        "pathlib.Path('build').mkdir(exist_ok=True)\n"
        "pathlib.Path('build/junit.xml').write_text(\n"
        "    \"<?xml version='1.0'?><testsuites><testsuite name='s'>\"\n"
        "    \"<testcase classname='tests.test_blade.TestSharpen' "
        "name='test_sharpens_by_one' file='tests/test_blade.py'>\"\n"
        "    \"<error>boom</error></testcase>\"\n"
        "    \"<testcase classname='tests.test_blade.TestSharpen' "
        "name='test_sharpen_zero' file='tests/test_blade.py'/>\"\n"
        "    \"<testcase classname='tests.test_blade.TestSharpen' "
        "name='test_ac1_sharpen_is_linenoise' file='tests/test_blade.py'/>\"\n"
        "    \"</testsuite></testsuites>\")\n"
        "print('written')\n"
    )
    gate_project.commit_story(root, {"tools/error_report.py": writer}, "the report writer")
    from tests.unit._repo import set_keys

    set_keys(root, "commands", test=f"{sys.executable} tools/error_report.py")
    gate_project.commit_story(
        root, {"tests/test_blade.py": gate_project.NEW_TESTS + "\n# touched\n"}, "the edit"
    )
    tw_env = gate_project.env_with(root, TW)
    code, result = refreeze(root, run, tw_env)
    assert code == 1 and result["ok"] is True and result["passed"] is False
    names = " | ".join(result["failures"])
    assert "build or harness error in tests.test_blade.TestSharpen::test_sharpens_by_one" in names
    assert ": unknown" in names
    assert_gate_shape(result)


def test_refreeze_over_cap_is_loud(tmp_path):
    """An over-cap re-freeze record fails out loud with the count of its
    tests — the worker never dies of a missing metrics key (D15)."""
    many = ["import unittest", "", "", "class TestMany(unittest.TestCase):"]
    for n in range(1, 241):
        many.append(f"    def test_ac1_case_{n}(self):\n        assert False\n")
    root, run = setup_red(tmp_path)
    gate_project.commit_story(
        root, {"tests/test_many.py": "\n".join(many)}, "many more tests"
    )
    tw_env = gate_project.env_with(root, TW)
    code, result = refreeze(root, run, tw_env)
    assert code == 1 and result["ok"] is True and result["passed"] is False
    assert any("over its cap" in one and "split the story" in one
               for one in result["failures"])
    assert_gate_shape(result)


def test_refreeze_drops_source_class_tests(tmp_path):
    """Red and the re-freeze agree on which tests are new: a test in a
    source-class file counts in neither (S4R-gates-15). The sample runner
    discovers tally/ as well, so the case really reaches both reports —
    the shared filter, not the runner, drops it."""
    root, run = gate_project.make_project(Path(tmp_path) / "proj")
    gate_project.commit_story(
        root,
        {
            "tally/blade.py": gate_project.BLADE_WITH_STUB,
            "tests/test_blade.py": gate_project.NEW_TESTS,
            "tally/test_in_src.py": SOURCE_CLASS_TEST,
        },
        "the story's first commit",
    )
    env = gate_project.env_with(root, gate_project.IMPLEMENTER)
    code, red, _err = run_gate(root, "gate", "red", "--story", STORY, "--run", run, env=env)
    assert code == 0 and red["passed"] is True, red["failures"]
    red_report = Path(red["evidence"]).with_suffix(".xml").read_text()
    assert 'file="tally/test_in_src.py"' in red_report, "the case must reach the report"
    assert red["metrics"]["new"] == 3
    earlier = json.loads(Path(red["evidence"]).read_text())
    assert not any("test_in_src" in one for one in earlier["tests"])
    gate_project.commit_story(
        root, {"tests/test_blade.py": gate_project.NEW_TESTS + "\n# touched\n"}, "a cosmetic edit"
    )
    tw_env = gate_project.env_with(root, TW)
    code2, result = refreeze(root, run, tw_env)
    assert code2 == 0 and result["passed"] is True, result["failures"]
    refreeze_report = Path(result["evidence"]).with_suffix(".xml").read_text()
    assert 'file="tally/test_in_src.py"' in refreeze_report, "the case must reach the report"
    record = json.loads(Path(result["evidence"]).read_text())
    assert sorted(record["tests"]) == sorted(earlier["tests"])
    assert not any("test_in_src" in one for one in record["tests"])
