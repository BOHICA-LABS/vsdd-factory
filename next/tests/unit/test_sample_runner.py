"""The sample's JUnit runner never loses a test (S4R-gates-C2-1,
S4B-gates-1): an import-broken test module — however it fails to import —
is an ImportError case in the report, a plain ``test_`` function is
collected beside the TestCase classes in both trees, and a test function
the runner cannot run as written (required parameters, a coroutine or a
generator) is reported as an error rather than dropped or passed
silently. The gate tests' project builder (gate_project) asserts the
configuration it builds, so a regression there is loud (ORCH-5)."""

import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

from tests.unit import gate_project
from tests.unit.gate_project import STORY, run_gate
from tests.unit.test_gate_red import assert_gate_shape
from vsdd.gates.junit import criteria_of, read_report

IMPL = gate_project.IMPLEMENTER
SAMPLE = Path(__file__).resolve().parents[2] / "tests" / "e2e" / "sample-project"


def _copy_project(tmp_path) -> Path:
    """The sample, less its ``.factory``, built under tmp_path — never inside
    the repository."""
    project = Path(tmp_path) / "sample-run"
    project.mkdir(parents=True)
    for child in sorted(SAMPLE.iterdir()):
        if child.name == ".factory":
            continue
        target = project / child.name
        if child.is_dir():
            shutil.copytree(child, target)
        else:
            shutil.copy2(child, target)
    return project


def _run_runner(project: Path):
    return subprocess.run(
        [sys.executable, "tools/junit_runner.py"],
        cwd=project,
        capture_output=True,
        text=True,
        timeout=120,
    )


def test_module_level_functions_reach_the_report(tmp_path):
    """A module-level failing ``test_`` function appears in the report as a
    failure with its file and its criterion, like a class-based test."""
    project = _copy_project(tmp_path)
    (project / "tests" / "test_funcs.py").write_text(
        "def test_plain_ok():\n"
        "    return None\n"
        "\n"
        "\n"
        "def test_ac1_extra_red():\n"
        "    raise AssertionError('red function')\n",
        encoding="utf-8",
    )
    done = _run_runner(project)
    cases = read_report(project / "build" / "junit.xml")
    plain = cases["tests.test_funcs::test_plain_ok"]
    assert plain.passed and plain.file_attr == "tests/test_funcs.py"
    red = cases["tests.test_funcs::test_ac1_extra_red"]
    assert red.failed and red.file_attr == "tests/test_funcs.py"
    assert criteria_of(red.name) == "AC-1"


def test_parameter_function_is_reported_as_error(tmp_path):
    """A module-level function with required parameters cannot run: it is
    reported as an error that says what the runner runs, never dropped."""
    project = _copy_project(tmp_path)
    (project / "tests" / "test_params.py").write_text(
        "def test_takes_arguments(x):\n    return x\n", encoding="utf-8"
    )
    done = _run_runner(project)
    assert done.returncode == 1, done.stderr
    cases = read_report(project / "build" / "junit.xml")
    case = cases["tests.test_params::test_takes_arguments"]
    assert case.harness_error and "TypeError" in case.error_types
    assert "no parameters" in (project / "build" / "junit.xml").read_text()


def test_async_and_generator_functions_are_reported_as_errors(tmp_path):
    """A coroutine, async generator or generator test function would be
    called and its answer thrown away: a pass with the body never run. The
    runner reports such a function as an error instead
    (step4-gates-fix-3-C1-1)."""
    project = _copy_project(tmp_path)
    (project / "tests" / "test_async.py").write_text(
        "async def test_ac1_coroutine_red():\n"
        "    raise AssertionError('red coroutine')\n"
        "\n"
        "\n"
        "async def test_ac1_async_gen_red():\n"
        "    yield 1\n"
        "    raise AssertionError('red async generator')\n"
        "\n"
        "\n"
        "def test_ac1_generator_red():\n"
        "    yield 1\n"
        "    raise AssertionError('red generator')\n",
        encoding="utf-8",
    )
    done = _run_runner(project)
    assert done.returncode == 1, done.stderr
    cases = read_report(project / "build" / "junit.xml")
    for name in ("test_ac1_coroutine_red", "test_ac1_async_gen_red", "test_ac1_generator_red"):
        case = cases[f"tests.test_async::{name}"]
        assert case.harness_error and "TypeError" in case.error_types, case
        assert not case.passed, case
    report = (project / "build" / "junit.xml").read_text()
    assert "plain synchronous test functions" in report


def test_source_tree_function_reaches_the_report(tmp_path):
    """A module-level failing ``test_`` function in a source-tree test
    module is collected like one in ``tests/``: failure, file, criterion
    (S4B-gates-1, the ``*/test_*.py`` half of build_suite)."""
    project = _copy_project(tmp_path)
    (project / "tally" / "test_fn.py").write_text(
        "def test_ac1_src_red():\n"
        "    raise AssertionError('red function in the source tree')\n",
        encoding="utf-8",
    )
    done = _run_runner(project)
    cases = read_report(project / "build" / "junit.xml")
    red = cases["tally.test_fn::test_ac1_src_red"]
    assert done.returncode == 1, done.stderr
    assert red.failed and not red.passed
    assert red.file_attr == "tally/test_fn.py"
    assert criteria_of(red.name) == "AC-1"


def test_import_broken_source_module_still_writes_the_report(tmp_path):
    """An import-broken tally/test_*.py costs a traceback, not the report
    (S4R-gates-C2-1): the report is written and names the ImportError, and
    the tests that do run are still in it."""
    project = _copy_project(tmp_path)
    (project / "tally" / "test_broken.py").write_text(
        "from tally.absent_module import thing\n", encoding="utf-8"
    )
    done = _run_runner(project)
    assert done.returncode == 1
    assert "Failed to import test module" in done.stderr, done.stderr
    cases = read_report(project / "build" / "junit.xml")
    assert any("ImportError" in one.error_types for one in cases.values()), sorted(cases)
    assert any(one.identity.startswith("tests.test_stats") for one in cases.values())


def test_syntax_broken_source_module_still_writes_the_report(tmp_path):
    """An import failure that is not an ImportError — a syntax error here —
    still costs a traceback in the report, not the report itself
    (S4R-gates-C2-1): ``loadTestsFromName`` catches ImportError alone, so
    build_suite owes the case itself."""
    project = _copy_project(tmp_path)
    (project / "tally" / "test_syn.py").write_text("def test_x(:\n", encoding="utf-8")
    done = _run_runner(project)
    assert done.returncode == 1
    cases = read_report(project / "build" / "junit.xml")
    case = cases["tally.test_syn::import_error"]
    assert "ImportError" in case.error_types and case.harness_error
    # no file attribute, as discovery gives: the identity names the module,
    # and the gate counts a file-less case as a build or harness error
    assert case.file_attr == ""
    report = (project / "build" / "junit.xml").read_text()
    assert "SyntaxError" in report and "Failed to import test module" in report
    assert any(one.identity.startswith("tests.test_stats") for one in cases.values())


def test_name_error_at_import_names_the_error(tmp_path):
    """A module whose body raises a NameError at import time: the report is
    written, with an ImportError case carrying the original traceback
    (S4R-gates-C2-1)."""
    project = _copy_project(tmp_path)
    (project / "tally" / "test_ne.py").write_text("an_undefined_name\n", encoding="utf-8")
    done = _run_runner(project)
    assert done.returncode == 1
    cases = read_report(project / "build" / "junit.xml")
    case = cases["tally.test_ne::import_error"]
    assert "ImportError" in case.error_types and case.harness_error
    assert case.file_attr == ""  # the twin of the case discovery gives
    report = (project / "build" / "junit.xml").read_text()
    assert "NameError" in report and "Failed to import test module" in report
    assert any(one.identity.startswith("tests.test_stats") for one in cases.values())


def test_import_broken_tests_module_names_the_error(tmp_path):
    """The same promise over ``tests/``: a broken module there is an
    ImportError case beside the tests that run."""
    project = _copy_project(tmp_path)
    (project / "tests" / "test_broken.py").write_text(
        "import not_importable_anywhere\n", encoding="utf-8"
    )
    done = _run_runner(project)
    assert done.returncode == 1
    assert "Failed to import test module" in done.stderr, done.stderr
    cases = read_report(project / "build" / "junit.xml")
    broken = [one for one in cases.values() if "ImportError" in one.error_types]
    assert broken and any("test_broken" in one.identity for one in broken)


def test_make_project_asserts_the_sample_configuration(tmp_path, monkeypatch):
    """The project the gate tests build is the sample the sample's own
    configuration describes; when the builder drifts from it, the mismatch
    is an AssertionError naming the value, not a misread red gate (ORCH-5)."""
    from tests.unit._repo import set_keys
    from tests.unit import story_ready

    real_make_project = story_ready.make_project

    def drifted(tmp_path, **kwargs):
        root = real_make_project(tmp_path, **kwargs)
        set_keys(root, "paths", source=["wrong/**"])
        return root

    monkeypatch.setattr(story_ready, "make_project", drifted)
    with pytest.raises(AssertionError) as raised:
        gate_project.make_project(tmp_path)
    message = str(raised.value)
    assert "paths.source" in message and "'wrong/**'" in message, message


def test_red_counts_a_module_level_function(tmp_path):
    """Through the real red gate: a module-level failing ``test_ac1_*``
    function in tests/ is a new red test that covers AC-1, and its file is
    frozen like any class-based test's (S4B-gates-1)."""
    root, run = gate_project.make_project(tmp_path)
    gate_project.commit_story(
        root,
        {
            "tally/blade.py": gate_project.BLADE_WITH_STUB,
            "tests/test_blade.py": gate_project.NEW_TESTS,
            "tests/test_extra.py": (
                "def test_ac1_extra_red():\n"
                "    raise AssertionError('red function')\n"
            ),
        },
        "red commit",
    )
    env = gate_project.env_with(root, IMPL)
    code, result, _err = run_gate(root, "gate", "red", "--story", STORY, "--run", run, env=env)
    assert_gate_shape(result)
    assert code == 0 and result["passed"] is True, result["failures"]
    assert result["metrics"]["new"] == 4, result["metrics"]
    record = json.loads(Path(result["evidence"]).read_text())
    assert "tests.test_extra::test_ac1_extra_red" in record["criteria"]["AC-1"]
    assert "tests/test_extra.py" in [entry["file"] for entry in record["frozen"]]


def test_red_survives_an_import_broken_source_module(tmp_path):
    """Through the real red gate: an import-broken tally/test_*.py does not
    cost the report — the gate names the ImportError instead of claiming
    there is no JUnit report (S4R-gates-C2-1)."""
    root, run = gate_project.make_project(tmp_path)
    gate_project.commit_story(
        root,
        {
            "tally/blade.py": gate_project.BLADE_WITH_STUB,
            "tests/test_blade.py": gate_project.NEW_TESTS,
            "tally/test_broken.py": "from tally.absent_module import thing\n",
        },
        "red commit",
    )
    env = gate_project.env_with(root, IMPL)
    code, result, _err = run_gate(root, "gate", "red", "--story", STORY, "--run", run, env=env)
    assert_gate_shape(result)
    assert code == 1 and result["passed"] is False
    assert any(
        "build or harness error" in one and "ImportError" in one
        for one in result["failures"]
    ), result["failures"]
    assert not any("wrote no JUnit report" in one for one in result["failures"])
    cases = read_report(gate_project.worktree_of(root) / "build" / "junit.xml")
    assert any("ImportError" in one.error_types for one in cases.values())


def test_red_survives_source_modules_that_die_at_import(tmp_path):
    """Through the real red gate: a source-tree test module that dies at
    import with anything — a syntax error, a NameError — leaves the report
    standing, and the gate names the build or harness error instead of
    claiming there is no JUnit report (S4R-gates-C2-1)."""
    root, run = gate_project.make_project(tmp_path)
    gate_project.commit_story(
        root,
        {
            "tally/blade.py": gate_project.BLADE_WITH_STUB,
            "tests/test_blade.py": gate_project.NEW_TESTS,
            "tally/test_syn.py": "def test_x(:\n",
            "tally/test_ne.py": "an_undefined_name\n",
        },
        "red commit",
    )
    env = gate_project.env_with(root, IMPL)
    code, result, _err = run_gate(root, "gate", "red", "--story", STORY, "--run", run, env=env)
    assert_gate_shape(result)
    assert code == 1 and result["passed"] is False
    failures = "\n".join(result["failures"])
    assert "build or harness error in tally.test_syn::import_error: ImportError" in failures, (
        result["failures"]
    )
    assert "build or harness error in tally.test_ne::import_error: ImportError" in failures, (
        result["failures"]
    )
    assert "wrote no JUnit report" not in failures
    cases = read_report(gate_project.worktree_of(root) / "build" / "junit.xml")
    assert "tally.test_syn::import_error" in cases and "tally.test_ne::import_error" in cases
    assert all("ImportError" in cases[one].error_types for one in ("tally.test_syn::import_error",
                                                                   "tally.test_ne::import_error"))
