#!/usr/bin/env python3
"""Run the tally test suite and write a JUnit XML report.

Standard library only, so the end-to-end test tiers never need a package
download (a PyPI fetch is unavailable in the sandboxed shell). Writes
build/junit.xml and exits 0 when the suite passes, 1 when it does not, so
either gate can read the per-test results afterwards.
"""

import sys
import time
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
TESTS_DIR = PROJECT_ROOT / "tests"
REPORT_PATH = PROJECT_ROOT / "build" / "junit.xml"

OUTCOMES = {"failure", "error", "skipped"}


class RecordingResult(unittest.TextTestResult):
    """A TextTestResult that remembers every case for the XML report."""

    def __init__(self, stream, descriptions, verbosity):
        super().__init__(stream, descriptions, verbosity)
        self.records = []
        self._started_at = 0.0

    def startTest(self, test):
        super().startTest(test)
        self._started_at = time.monotonic()

    def _record(self, test, outcome, detail=""):
        self.records.append(
            {
                "id": test.id(),
                "seconds": time.monotonic() - self._started_at,
                "outcome": outcome,
                "detail": detail,
            }
        )

    def addSuccess(self, test):
        super().addSuccess(test)
        self._record(test, "passed")

    def addFailure(self, test, err):
        super().addFailure(test, err)
        self._record(test, "failure", self._exc_info_to_string(err, test))

    def addError(self, test, err):
        super().addError(test, err)
        self._record(test, "error", self._exc_info_to_string(err, test))

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
        case = ET.SubElement(
            suite,
            "testcase",
            {"classname": parts[0], "name": parts[1], "time": f"{record['seconds']:.6f}"},
        )
        if record["outcome"] in OUTCOMES:
            detail = record["detail"].strip() or record["outcome"]
            node = ET.SubElement(
                case, record["outcome"], {"message": detail.splitlines()[0][:400]}
            )
            node.text = detail
    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    tree = ET.ElementTree(suites)
    tree.write(REPORT_PATH, encoding="utf-8", xml_declaration=True)
    with open(REPORT_PATH, "ab") as report:
        report.write(b"\n")


def main():
    sys.path.insert(0, str(PROJECT_ROOT))
    loader = unittest.TestLoader()
    suite = loader.discover(str(TESTS_DIR), top_level_dir=str(PROJECT_ROOT))
    started = time.monotonic()
    result = unittest.TextTestRunner(resultclass=RecordingResult, verbosity=1).run(suite)
    elapsed = time.monotonic() - started
    write_report(result.records, elapsed)
    print(f"JUnit report written to {REPORT_PATH}")
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main())
