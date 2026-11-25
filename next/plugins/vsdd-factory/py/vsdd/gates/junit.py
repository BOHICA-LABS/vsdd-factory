"""Reading a JUnit report: identities, outcomes, files and criteria (plan
gates.md D8; commands/gate-red.md "What the gate reads").

``read_report`` collapses a report to one :class:`Case` per test identity
(``<classname>::<name>``); a repeated identity is failed if any occurrence
is. A ``<skipped>`` case is neither red nor passed. The file of a test is its
``file`` attribute when the report carries one, else the longest prefix of
the classname that names exactly one ``tests``-class file (finding VSA-12);
a criterion is covered when the test's name contains ``ac<N>`` followed by a
non-digit, compared without case.
"""

from __future__ import annotations

import re
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from pathlib import Path

__all__ = ["NOT_IMPLEMENTED_TYPES", "Case", "JUnitError", "criteria_of", "file_from_classname", "read_report"]

#: An ``<error>`` whose type is one of these is the expected red of a stub
#: that raises; any other type (or no type) is a build or harness error.
NOT_IMPLEMENTED_TYPES = frozenset(
    {"NotImplementedError", "NotImplementedException", "UnsupportedOperationException"}
)

_CRITERION_RE = re.compile(r"ac(\d+)(?!\d)", re.IGNORECASE)


class JUnitError(Exception):
    """A report that does not parse, or a report file that is not there."""


@dataclass
class Case:
    """One test identity as the report reports it, occurrences collapsed."""

    identity: str  # <classname>::<name>
    classname: str
    name: str
    failed: bool = False  # any occurrence holds a <failure>
    errored: bool = False  # any occurrence holds an <error>
    error_types: list[str] = field(default_factory=list)  # <error type="...">
    skipped: bool = False  # any occurrence holds a <skipped>
    file_attr: str = ""  # the `file` attribute, as the report carries it

    @property
    def red(self) -> bool:
        """A failure, or an error of a not-implemented type (a stub)."""
        if self.failed:
            return True
        return any(one in NOT_IMPLEMENTED_TYPES for one in self.error_types)

    @property
    def harness_error(self) -> bool:
        """An error whose type is absent or not a not-implemented type."""
        if self.failed or not self.errored:
            return False
        return not any(one in NOT_IMPLEMENTED_TYPES for one in self.error_types)

    @property
    def passed(self) -> bool:
        return not (self.failed or self.errored or self.skipped)

    @property
    def exempt(self) -> bool:
        lowered = self.name.lower()
        return "wiring" in lowered or "green_by_design" in lowered


def read_report(path) -> dict[str, Case]:
    """Every ``<testcase>`` of the report, by identity; raises
    :class:`JUnitError` when the file is missing or does not parse."""
    try:
        tree = ET.parse(Path(path))
    except (ET.ParseError, OSError) as exc:
        raise JUnitError(f"{path}: the JUnit report does not parse: {exc}") from exc
    cases: dict[str, Case] = {}
    for element in tree.iter("testcase"):
        classname = element.get("classname", "")
        name = element.get("name", "")
        identity = f"{classname}::{name}"
        case = cases.setdefault(identity, Case(identity=identity, classname=classname, name=name))
        if element.find("failure") is not None:
            case.failed = True
        error = element.find("error")
        if error is not None:
            case.errored = True
            error_type = error.get("type", "")
            if error_type and error_type not in case.error_types:
                case.error_types.append(error_type)
        if element.find("skipped") is not None:
            case.skipped = True
        file_attr = element.get("file", "")
        if file_attr and not case.file_attr:
            case.file_attr = file_attr
    return cases


def file_from_classname(classname: str, tests_files) -> str:
    """The file of a test with no ``file`` attribute: the longest prefix of
    the classname (``.`` and ``::`` read as ``/``) that equals, or ends, the
    extension-less path of exactly one ``tests``-class file. The file's own
    path, extension included, is returned. Two candidates name no file
    (VSA-12); so does no match."""
    files = list(tests_files)
    path = classname.replace("::", "/").replace(".", "/")
    parts = path.split("/")
    for cut in range(len(parts), 0, -1):
        prefix = "/".join(parts[:cut])
        matches = [
            one
            for one in files
            if Path(one).with_suffix("").as_posix() == prefix
            or Path(one).with_suffix("").as_posix().endswith("/" + prefix)
        ]
        if len(matches) == 1:
            return matches[0]
        if matches:
            return ""
    return ""


def criteria_of(name: str) -> str:
    """``AC-<N>`` when the test's name contains ``ac<N>`` with no digit
    after it (``test_ac2_rejects_empty`` covers AC-2, and ``ac12`` does not
    cover AC-1); ``""`` when none."""
    found = _CRITERION_RE.search(name)
    return f"AC-{found.group(1)}" if found else ""
