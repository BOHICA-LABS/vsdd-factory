"""Reading a JUnit report (plan gates.md D8; findings VSA-12)."""

from pathlib import Path

import pytest

from vsdd.gates.junit import (
    NOT_IMPLEMENTED_TYPES,
    Case,
    JUnitError,
    criteria_of,
    file_from_classname,
    read_report,
)

FIXTURES = Path(__file__).resolve().parent / "fixtures" / "junit"


def test_reads_cases_and_outcomes():
    cases = read_report(FIXTURES / "simple.xml")
    assert set(cases) == {
        "tests.test_calc::test_add",
        "tests.test_calc::test_wrong",
        "tests.test_calc::test_later",
    }
    passed = cases["tests.test_calc::test_add"]
    assert passed.passed and not failed_or_errored(passed)
    wrong = cases["tests.test_calc::test_wrong"]
    assert wrong.failed and wrong.red
    later = cases["tests.test_calc::test_later"]
    assert later.skipped and not later.red and not later.passed


def failed_or_errored(case: Case) -> bool:
    return case.failed or bool(case.error_types)


def test_not_implemented_types():
    assert NOT_IMPLEMENTED_TYPES == frozenset(
        {"NotImplementedError", "NotImplementedException", "UnsupportedOperationException"}
    )
    cases = read_report(FIXTURES / "not_implemented.xml")
    assert cases["tests.test_stub::test_stub_raises"].red
    assert cases["tests.test_stub::test_java_style"].red
    crash = cases["tests.test_stub::test_crash"]
    assert crash.harness_error and not crash.red
    silent = cases["tests.test_stub::test_no_type"]
    assert silent.harness_error  # an error with no type at all


def test_file_from_attribute_then_classname():
    cases = read_report(FIXTURES / "files.xml")
    tests_files = [
        "tests/deep/test_other.py",
        "tests/test_stats.py",
        "src/parser/tests/test_x.py",
    ]
    assert cases["tests.deep.test_nested::test_attr"].file_attr == "tests/deep/test_other.py"
    # the classname rule: tests.test_stats.TestMean names tests/test_stats.py
    assert file_from_classname("tests.test_stats.TestMean", tests_files) == "tests/test_stats.py"
    # longest prefix that ends the extension-less path of exactly one file
    assert (
        file_from_classname("parser.tests.test_x.TestX", tests_files)
        == "src/parser/tests/test_x.py"
    )
    # two candidates give "" (VSA-12): both files end with the same prefix
    two = ["tests/test_stats.py", "pkg/tests/test_stats.py"]
    assert file_from_classname("tests.test_stats.TestMean", two) == ""


def test_criteria_need_a_non_digit():
    cases = read_report(FIXTURES / "criteria.xml")
    assert criteria_of(cases["tests.test_c::test_ac1_first"].name) == "AC-1"
    assert criteria_of(cases["tests.test_c::test_ac12_second"].name) == "AC-12"
    assert criteria_of(cases["tests.test_c::test_AC2_upper"].name) == "AC-2"
    assert criteria_of(cases["tests.test_c::test_no_criterion_here"].name) == ""


def test_repeated_identity_failed_if_any_occurrence_is():
    cases = read_report(FIXTURES / "repeated.xml")
    twice = cases["tests.test_r::test_twice"]
    assert twice.failed and twice.red
    skipped = cases["tests.test_r::test_skip_only"]
    assert skipped.skipped and not skipped.red


def test_broken_report_raises():
    with pytest.raises(JUnitError):
        read_report(FIXTURES / "broken.xml")
    with pytest.raises(JUnitError):
        read_report(FIXTURES / "no-such-report.xml")


def test_sample_runner_writes_file_and_type(tmp_path):
    """The e2e sample runner writes `file` on each testcase and `type` on a
    failure or error (finding VSA-12), read from a run of the runner itself.
    The copy is built under tmp_path, never inside the repository."""
    import shutil
    import subprocess
    import sys

    sample = Path(__file__).resolve().parents[2] / "tests" / "e2e" / "sample-project"
    project = tmp_path / "sample-run"
    project.mkdir(parents=True)
    for child in sorted(sample.iterdir()):
        if child.name == ".factory":
            continue
        target = project / child.name
        if child.is_dir():
            shutil.copytree(child, target)
        else:
            shutil.copy2(child, target)
    broken = project / "tests" / "test_broken.py"
    broken.write_text(
        "import unittest\n\n\nclass TestBroken(unittest.TestCase):\n"
        "    def test_breaks(self):\n        raise ZeroDivisionError('boom')\n",
        encoding="utf-8",
    )
    done = subprocess.run(
        [sys.executable, "tools/junit_runner.py"],
        cwd=project,
        capture_output=True,
        text=True,
        timeout=120,
    )
    assert done.returncode == 1, done.stderr
    cases = read_report(project / "build" / "junit.xml")
    entry = cases["tests.test_broken.TestBroken::test_breaks"]
    assert entry.file_attr == "tests/test_broken.py"
    assert "ZeroDivisionError" in entry.error_types
