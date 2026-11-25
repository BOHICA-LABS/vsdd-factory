"""Ground one review finding in the tree it reviews (plan rules.md 3.5;
PL4-1: this module is H7's, not the verdicts').

:func:`ground` answers one question — does the file a finding cites carry the
lines and the quote it claims? — and never raises, whatever shape the finding
has. ``""`` means grounded, or exempt: a finding of category ``holdout`` (the
tool builds those from scenario scores with no location) and a finding with no
``location.path``, which ``judge()`` already holds advisory.

A path whose first segment is ``.factory`` is read from the project root
(``finding.schema.json``); any other from the reviewed tree the caller gives —
for ``step-results`` that is the unit's tree
(:func:`vsdd.state.worktrees.unit_tree`). The module imports pathlib only, and
nothing of the plugin: the state and hooks layers call it, never the reverse.

The reasons are the finding author's to fix; ``step-results`` shows them one
line per finding, so each names its path as the finding spelled it.
"""

from __future__ import annotations

from pathlib import Path

__all__ = ["ground"]

_QUOTE_WINDOW = 3  # lines of slack each way, per rules/step-results.md


def ground(finding: object, *, tree: Path, project_root: Path) -> str:
    """``""`` when the finding is grounded or exempt, else why it is not."""
    try:
        return _ground(finding, tree=Path(tree), project_root=Path(project_root))
    except Exception:  # never: a malformed finding is not a rule crash
        return ""


def _ground(finding, *, tree: Path, project_root: Path) -> str:
    if not isinstance(finding, dict):
        return ""
    if finding.get("category") == "holdout":
        return ""
    location = finding.get("location")
    if not isinstance(location, dict):
        return ""
    path = location.get("path")
    if not isinstance(path, str) or not path:
        return ""

    base = tree
    if _first_segments(path) == ".factory":
        # The project root answers for a ``.factory`` path — but only while
        # the resolved path stays under its ``.factory/``: ``.factory/../src``
        # names the tree's source, not the state directory's (S4R-rules-20).
        factory_root = (Path(project_root) / ".factory").resolve()
        candidate = (Path(project_root) / path).resolve()
        if factory_root == candidate or factory_root in candidate.parents:
            base = project_root
    if path.startswith("/"):
        return f"path is outside the reviewed tree: {path}"
    resolved = (base / path).resolve()
    root = base.resolve()
    if resolved != root and root not in resolved.parents:
        return f"path is outside the reviewed tree: {path}"
    if not resolved.is_file():
        return f"file not found: {path}"
    try:
        text = resolved.read_text(encoding="utf-8", errors="replace")
    except OSError:
        # An unreadable file is not a grounded one: the catch-all below must
        # not answer "" for it (S4R-rules-20).
        return f"file cannot be read: {path}"

    lines = text.splitlines()
    count = len(lines)

    window = lines
    bounds = _line_bounds(location, count)
    if bounds is not None:
        low, high = bounds
        if not 1 <= low <= high <= count:
            return f"lines {low}-{high} are outside {path} ({count} lines)"
        # The quote is sought in the range widened by three lines each way.
        window = lines[max(low - _QUOTE_WINDOW, 1) - 1 : min(high + _QUOTE_WINDOW, count)]

    quote = location.get("quote")
    if not isinstance(quote, str) or not quote:
        return ""
    haystack = _normalise("\n".join(window))
    if _normalise(quote) not in haystack:
        return f"quote not found in {path}"
    return ""


def _first_segments(path: str) -> str:
    """The path's first segment, as a POSIX spelling: the ``.factory`` test."""
    text = path.replace("\\", "/")
    while text.startswith("./"):
        text = text[2:]
    head, _, rest = text.partition("/")
    return head if rest else ""


def _line_bounds(location: dict, count: int):
    """The 1-based ``(low, high)`` of the finding's line claim. One bound
    stands for both; a bound that is not an integer (or is a boolean) leaves
    the line check out — the quote, if any, is sought in the whole file."""
    start = location.get("line_start")
    end = location.get("line_end")
    start = start if _is_line(start) else None
    end = end if _is_line(end) else None
    if start is None and end is None:
        return None
    if start is None:
        start = end
    if end is None:
        end = start
    return start, end


def _is_line(value) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _normalise(text: str) -> str:
    """Whitespace runs made one space: the quote matches whatever the file's
    own indentation and line breaks are."""
    return " ".join(text.split())
