#!/usr/bin/env python3
"""Run the tally test suite and write a JUnit XML report.

Standard library only, so the end-to-end test tiers never need a package
download (a PyPI fetch is unavailable in the sandboxed shell). Writes
build/junit.xml and exits 0 when the suite passes, 1 when it does not, so
either gate can read the per-test results afterwards. The report is written
after every run: a test module that fails to import — however it fails,
a missing module, a syntax error, a name error — becomes an ImportError case
in it, a plain test function is collected beside the TestCase classes, and a
function the runner cannot run as written is an error case, so the report
never quietly loses a test the story committed.
"""

import importlib
import inspect
import sys
import time
import traceback
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
TESTS_DIR = PROJECT_ROOT / "tests"
REPORT_PATH = PROJECT_ROOT / "build" / "junit.xml"

OUTCOMES = {"failure", "error", "skipped"}


def _record_type(err) -> str:
    """The exception class name of a failure or error, for the `type` attribute."""
    try:
        first = err[0]
    except (TypeError, IndexError):
        return ""
    return getattr(first, "__name__", "") if first is not None else ""


def file_of(test) -> str:
    """The test's file, relative to the project root: the longest dotted
    prefix of the test id that names a file under the project root. Empty
    when no prefix does."""
    parts = test.id().split(".")
    for cut in range(len(parts), 0, -1):
        candidate = Path(*parts[:cut]).with_suffix(".py")
        if (PROJECT_ROOT / candidate).is_file():
            return candidate.as_posix()
    return ""


class RecordingResult(unittest.TextTestResult):
    """A TextTestResult that remembers every case for the XML report."""

    def __init__(self, stream, descriptions, verbosity):
        super().__init__(stream, descriptions, verbosity)
        self.records = []
        self._started_at = 0.0

    def startTest(self, test):
        super().startTest(test)
        self._started_at = time.monotonic()

    def _record(self, test, outcome, detail="", err=None):
        file = getattr(test, "junit_file", None)
        if file is None:
            file = file_of(test)
        self.records.append(
            {
                "id": test.id(),
                "seconds": time.monotonic() - self._started_at,
                "outcome": outcome,
                "detail": detail,
                "file": file,
                "type": _record_type(err) if err is not None else "",
            }
        )

    def addSuccess(self, test):
        super().addSuccess(test)
        self._record(test, "passed")

    def addFailure(self, test, err):
        super().addFailure(test, err)
        self._record(test, "failure", self._exc_info_to_string(err, test), err=err)

    def addError(self, test, err):
        super().addError(test, err)
        self._record(test, "error", self._exc_info_to_string(err, test), err=err)

    def addSkip(self, test, reason):
        super().addSkip(test, reason)
        self._record(test, "skipped", reason)

    def addExpectedFailure(self, test, err):
        super().addExpectedFailure(test, err)
        self._record(test, "passed", self._exc_info_to_string(err, test))

    def addUnexpectedSuccess(self, test):
        super().addUnexpectedSuccess(test)
        self._record(test, "failure", "test was expected to fail and passed")


def write_report(records, seconds):
    suites = ET.Element("testsuites")
    suite = ET.SubElement(
        suites,
        "testsuite",
        {
            "name": "tally",
            "tests": str(len(records)),
            "failures": str(sum(r["outcome"] == "failure" for r in records)),
            "errors": str(sum(r["outcome"] == "error" for r in records)),
            "skipped": str(sum(r["outcome"] == "skipped" for r in records)),
            "time": f"{seconds:.3f}",
        },
    )
    for record in records:
        parts = record["id"].rsplit(".", 1)
        attributes = {"classname": parts[0], "name": parts[1], "time": f"{record['seconds']:.6f}"}
        if record.get("file"):
            attributes["file"] = record["file"]
        case = ET.SubElement(suite, "testcase", attributes)
        if record["outcome"] in OUTCOMES:
            detail = record["detail"].strip() or record["outcome"]
            node_attributes = {"message": detail.splitlines()[0][:400]}
            if record["outcome"] in ("failure", "error") and record.get("type"):
                node_attributes["type"] = record["type"]
            node = ET.SubElement(case, record["outcome"], node_attributes)
            node.text = detail
    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    tree = ET.ElementTree(suites)
    tree.write(REPORT_PATH, encoding="utf-8", xml_declaration=True)
    with open(REPORT_PATH, "ab") as report:
        report.write(b"\n")


def _dotted(path) -> str:
    """The module name of a test file under the project root."""
    path = Path(path)
    return ".".join(path.with_suffix("").relative_to(PROJECT_ROOT).parts)


def _modules(paths):
    """The dotted names of the test files in ``paths``, files only."""
    return [_dotted(path) for path in sorted(paths) if path.is_file()]


def _without_args(func) -> bool:
    """Whether the function can be called with no arguments."""
    try:
        signature = inspect.signature(func)
    except (TypeError, ValueError):  # a callable with no readable signature
        return False
    try:
        signature.bind()
    except TypeError:
        return False
    return True


class _FunctionCase(unittest.FunctionTestCase):
    """A plain test function as a case with the identity of a class-based
    test of the same module: ``<module>.<name>``, and the module's file
    through :func:`file_of`, so the file mapping and the criterion rule work
    for it unchanged. (Some interpreters give FunctionTestCase an id of the
    function's name alone, so the id is carried here.)"""

    def __init__(self, run, module_name, name):
        super().__init__(run)
        self._module_name = module_name
        self._case_name = name

    def id(self):
        return f"{self._module_name}.{self._case_name}"


def _as_case(module_name, name, run):
    run.__name__ = name
    run.__module__ = module_name
    return _FunctionCase(run, module_name, name)


def _non_synchronous(member) -> bool:
    """Whether calling ``member`` returns something other than ``None`` that
    a caller would have to await or iterate: a coroutine, an async generator
    or a generator function. FunctionTestCase calls the function and throws
    the answer away, so the case would pass with the body never run
    (step4-gates-fix-3-C1-1)."""
    return bool(
        inspect.iscoroutinefunction(member)
        or inspect.isasyncgenfunction(member)
        or inspect.isgeneratorfunction(member)
    )


def function_cases(module):
    """Every module-level ``test_`` function the module defines, as cases.
    Discovery collects TestCase classes only, so a plain test function would
    vanish from the report: the base set would under-count and the red gate
    would see no failing test for it (S4B-gates-1). A function the runner
    cannot run as written — required parameters, or a coroutine, async
    generator or generator function whose answer would never be awaited or
    iterated — is reported as an error case: never dropped, never a silent
    pass. A module with a ``load_tests`` hook decides for itself."""
    if hasattr(module, "load_tests"):
        return []
    cases = []
    for name, member in sorted(vars(module).items()):
        if not name.startswith("test_") or not inspect.isfunction(member):
            continue
        if getattr(member, "__module__", "") != module.__name__:
            continue  # imported, not defined here: its own module collects it
        if _without_args(member) and not _non_synchronous(member):
            cases.append(_as_case(module.__name__, name, member))
        elif _non_synchronous(member):

            def error(member=member, module_name=module.__name__):
                raise TypeError(
                    f"{module_name}.{member.__name__}: the runner runs plain "
                    "synchronous test functions with no parameters"
                )

            cases.append(_as_case(module.__name__, name, error))
        else:

            def error(member=member, module_name=module.__name__):
                raise TypeError(
                    f"{module_name}.{member.__name__}: the runner runs unittest "
                    "cases and plain test functions with no parameters"
                )

            cases.append(_as_case(module.__name__, name, error))
    return cases


def _function_cases_of(name):
    """The plain test functions of one test module, or nothing when the
    module does not import — its import error is in the report already."""
    try:
        module = importlib.import_module(name)
    except Exception:  # any import failure keeps its error case in the report
        return []
    return function_cases(module)


def _import_error_case(name, detail):
    """One ImportError case for a source-tree test module that died at
    import: the twin of the case discovery gives such a module over
    ``tests/``, whatever the failure — a syntax error or a NameError, not
    only a missing module. Like that case it carries no ``file``: nothing of
    the module's own tests is known, and a file would class the case out of
    the red gate's new tests instead of naming the build or harness error
    (S4R-gates-C2-1)."""
    def error(detail=f"Failed to import test module: {name}\n{detail}"):
        raise ImportError(detail)
    case = _as_case(name, "import_error", error)
    case.junit_file = ""  # no file attribute: discovery gives none either
    return case


def build_suite(loader):
    """The tests/ tree, plus every test_*.py beside the product's own
    package: a test a story commits inside its source tree reaches the
    report too, with its ``file``, so the gates see it (VSA-12).

    A test module that fails to import is never fatal to the report: over
    ``tests/`` discovery turns the import error into an ImportError test
    case, and a source-tree module that dies at import — a syntax error or
    a NameError, not only a missing module, since ``loadTestsFromName``
    catches ImportError alone — contributes one ImportError case of its
    own, so ``build/junit.xml`` is written after every run and names the
    error (S4R-gates-C2-1). Discovery collects TestCase classes only, so
    every module-level ``test_`` function joins as well, in both trees."""
    suite = loader.discover(str(TESTS_DIR), top_level_dir=str(PROJECT_ROOT))
    for name in _modules(PROJECT_ROOT.glob("*/test_*.py")):
        if name.startswith("tests."):
            continue  # discovery covers tests/ already
        try:
            loaded = loader.loadTestsFromName(name)
        except Exception:  # not every import failure is an ImportError
            suite.addTest(_import_error_case(name, traceback.format_exc()))
        else:
            suite.addTests(loaded)
        suite.addTests(_function_cases_of(name))
    for name in _modules(TESTS_DIR.rglob("test*.py")):
        suite.addTests(_function_cases_of(name))
    return suite


def main():
    sys.path.insert(0, str(PROJECT_ROOT))
    loader = unittest.TestLoader()
    suite = build_suite(loader)
    started = time.monotonic()
    result = unittest.TextTestRunner(resultclass=RecordingResult, verbosity=1).run(suite)
    elapsed = time.monotonic() - started
    write_report(result.records, elapsed)
    print(f"JUnit report written to {REPORT_PATH}")
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main())
