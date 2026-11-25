"""The demo gate (commands/gate-demo.md): records a story's demo evidence, or
builds the criterion-to-test table when no recording is required (D14). One
gate, one module: the records' registry imports ``gate_<name>.py`` by file
name and calls what it registers here. Direct: never pending."""

from __future__ import annotations

import re
from pathlib import Path

from ..policy import load as load_policy
from ..state.gate_evidence import evidence, evidence_dir, evidence_read, evidence_write
from ..state.store import StoreRefused, root_of, safe_segment
from ..state.story_unit import kind_has_red_gate, story_kind
from .gatelib import (
    Refused,
    WorkFailed,
    _failed,
    not_attempted,
    record_path,
    resolve_story,
    result,
)
from .junit import read_report

__all__ = ["measure"]


def _caps() -> tuple[int, int, int, int]:
    """The caps of the ``[demo-evidence]`` row of the artifact registry —
    the one source (plan section 4): script, transcript and screenshot from
    its ``cap_note``, the story total from ``max_bytes``."""
    row = load_policy("artifacts").get("demo-evidence", {})
    note = str(row.get("cap_note", ""))

    def named(word: str) -> int:
        found = re.search(rf"{word} ([\d,]+)", note)
        return int(found.group(1).replace(",", "")) if found else 0

    return named("script"), named("transcript"), named("screenshot"), int(row.get("max_bytes", 0))


#: Script, transcript, screenshot, and the story total: 8,000; 20,000;
#: 300,000; 1,500,000 — as the ``[demo-evidence]`` row holds them.
SCRIPT_CAP, TRANSCRIPT_CAP, SCREENSHOT_CAP, STORY_CAP = _caps()

_SCRIPT_SUFFIXES = (".sh", ".tape", ".spec.ts")

_AC_NAME_RE = re.compile(r"AC-(\d+)")
_STORY_HEADING_RE = re.compile(r"^(AC-\d+)\b")


def measure(project, subject: str, run: str, *, wait: int | None = None, retry: str = "") -> dict:
    """The demo gate, directly (plan gates.md 3.4, 3.5): ``wait`` and
    ``retry`` are the registry contract's and are ignored — demo is never
    pending and never a job. It returns the passing record for the head when
    there is one, else a measurement — the table when the story's ``demo``
    is false."""
    from ..state.admission import AdmissionError
    from ..state.worktrees import WorktreeError

    del wait, retry  # direct: taken for the registry contract, never used
    root = root_of(project)
    gate = "demo"
    try:
        spec = resolve_story(root, subject, run)
    except (Refused, AdmissionError, WorktreeError) as exc:
        message = getattr(exc, "message", str(exc))
        return not_attempted(gate, subject, message)
    head = spec["head"]
    cached = evidence_read(root, subject, gate, head)
    if cached is not None and cached.get("passed"):
        # A passing record for the head is returned unmeasured, so a later
        # failing measurement never replaces it (plan gates.md 3.3).
        return result(
            gate, subject, head, passed=True, failures=[],
            evidence_path=record_path(root, subject, gate, head),
        )
    try:
        record, metrics, table, failures = _demo(root, spec)
    except WorkFailed as exc:
        return result(
            gate, subject, head, passed=False,
            failures=list(exc.args[0]), metrics=exc.metrics,
        )
    outputs = "\n".join(table)
    try:
        written = evidence_write(root, record, log=outputs)
    except StoreRefused as exc:
        return not_attempted(
            gate, subject, f"the evidence record was refused: {exc.code}: {exc}", sha=head
        )
    return result(
        gate,
        subject,
        head,
        passed=bool(written.get("passed")),
        failures=failures,
        metrics=metrics,
        evidence_path=record_path(root, subject, gate, head),
    )


def _demo(root, spec: dict):
    story, head = spec["story"], spec["head"]
    from ..state.artifacts import ArtifactError, load_artifact

    try:
        meta = load_artifact(root, story).frontmatter
    except ArtifactError:
        meta = {}
    criteria_of_story = _story_criteria(root, story)
    metrics: dict = {"mode": "table", "uncovered": [], "recorded": False}
    green = evidence(root, "green", story)
    if green is None:
        raise _failed(["no green evidence covers the head"], metrics)
    if not meta.get("demo"):
        return _table_record(root, spec, green, criteria_of_story, metrics)
    metrics["mode"] = "recorded"
    metrics["recorded"] = True
    failures = _recording_checks(root, spec, criteria_of_story)
    if failures:
        raise _failed(failures, metrics)
    record = {
        "gate": "demo",
        "subject": story,
        "sha": head,
        "passed": True,
        "patch_id": _patch_of(spec),
    }
    return record, metrics, ["# demo evidence recorded"], []


def _story_criteria(root, story: str) -> list[str]:
    """The story's ``### AC-<N>`` headings, in file order (D14: the criteria
    the story actually states, test or not)."""
    from ..state.artifacts import ArtifactError, load_artifact

    try:
        body = load_artifact(root, story).body
    except ArtifactError:
        return []
    found: list[str] = []
    for line in body.splitlines():
        stripped = line.strip()
        level = len(stripped) - len(stripped.lstrip("#"))
        text = stripped.lstrip("#").strip()
        if level != 3 or not text.startswith("AC-"):
            continue
        match = _STORY_HEADING_RE.match(text)
        if match and match.group(1) not in found:
            found.append(match.group(1))
    return found


def _patch_of(spec: dict) -> str:
    from ..state.gate_evidence import patch_id

    return patch_id(Path(spec["worktree"]), spec["base"], spec["head"])


def _table_record(root, spec: dict, green: dict, criteria_of_story: list[str], metrics: dict):
    """The criterion-to-test table for a story with no user-visible criteria:
    the rows come from the red record's ``criteria`` map, each with the
    test's result in the latest green report; a criterion of the story with
    no test is listed and does not fail (D14)."""
    story, head = spec["story"], spec["head"]
    red = None
    if kind_has_red_gate(story_kind(root, story)):
        red = evidence(root, "red", story)
        if red is None:
            raise _failed(["no red record"], metrics)
    criteria = (red or {}).get("criteria", {})
    cases = _green_cases(root, spec, green)
    table = ["# criterion: test -> result"]
    failures: list[str] = []
    for criterion in sorted(criteria):
        identities = criteria[criterion]
        if not identities:
            continue
        for identity in identities:
            case = cases.get(identity)
            outcome = "passed" if case is not None and case.passed else "not passed"
            table.append(f"{criterion}: {identity} -> {outcome}")
            if case is None or not case.passed:
                failures.append(
                    f"the test of {criterion} is not passed in the green report: {identity}"
                )
    # A criterion the story states with no test is uncovered: named in the
    # table and in the metrics, never a failure (D14).
    tested = {criterion for criterion, ids in criteria.items() if ids}
    uncovered = sorted(one for one in criteria_of_story if one not in tested)
    metrics["uncovered"] = uncovered
    for criterion in uncovered:
        table.append(f"{criterion}: (no test) -> uncovered")
    record = {
        "gate": "demo",
        "subject": story,
        "sha": head,
        "passed": not failures,
        "patch_id": _patch_of(spec),
    }
    return record, metrics, table + [f"# {one}" for one in failures], failures


def _green_cases(root, spec: dict, green: dict) -> dict:
    """The report of the latest green evidence, parsed from the ``.xml``
    beside the record; empty when the copy is missing (a failure — the demo
    table's results come from it)."""
    sha = str(green.get("sha", ""))
    if not sha:
        raise _failed(["no green evidence covers the head"], {})
    path = evidence_dir(root, spec["story"]) / f"green.{sha[:12]}.xml"
    if not path.is_file():
        raise _failed([f"the green report copy is missing: {path}"], {})
    return read_report(path)


def _recording_checks(root, spec: dict, criteria_of_story: list[str]) -> list[str]:
    """The recorded-demo checks (D14): ``report.md`` not empty; at least one
    criterion of the story with one script and a transcript; every
    ``AC-<k>`` file names a criterion of the story, and every screenshot is
    an ``AC-<k>.png`` of one; sizes within the row's caps."""
    directory = Path(root) / ".factory" / "demo" / safe_segment(spec["cycle"]) / safe_segment(spec["story"])
    failures: list[str] = []
    if not directory.is_dir():
        return [f"the demo evidence directory is missing: {directory}"]
    report = directory / "report.md"
    if not report.is_file() or report.stat().st_size == 0:
        failures.append("the demo report.md is empty or missing")
    names = sorted(one.name for one in directory.iterdir() if one.is_file() and one.name != "report.md")
    known = set(criteria_of_story)
    criteria: dict[str, dict] = {}

    def criterion_of(stem: str) -> str:
        match = _AC_NAME_RE.fullmatch(stem)
        return f"AC-{match.group(1)}" if match else ""

    total = report.stat().st_size if report.is_file() else 0
    for name in names:
        path = directory / name
        size = path.stat().st_size
        total += size
        if name.endswith(".png"):
            if size > SCREENSHOT_CAP:
                failures.append(f"{name} is {size} bytes, over the {SCREENSHOT_CAP}-byte screenshot cap")
            if criterion_of(name[: -len(".png")]) not in known:
                failures.append(f"a demo file does not name a criterion: {name}")
            continue
        suffix = next((one for one in _SCRIPT_SUFFIXES if name.endswith(one)), None)
        is_script = suffix is not None
        if not is_script:
            suffix = ".txt" if name.endswith(".txt") else None
        if suffix is None:
            failures.append(f"a demo file does not name a criterion: {name}")
            continue
        criterion = criterion_of(name[: -len(suffix)])
        if not criterion:
            failures.append(f"a demo file does not name a criterion: {name}")
            continue
        if criterion not in known:
            failures.append(f"a demo file names a criterion the story lacks: {name}")
            continue
        cap, kind = ((SCRIPT_CAP, "script") if is_script else (TRANSCRIPT_CAP, "transcript"))
        if size > cap:
            failures.append(f"{name} is {size} bytes, over the {cap}-byte {kind} cap")
        entry = criteria.setdefault(criterion, {"script": False, "transcript": False})
        entry["script" if is_script else "transcript"] = True
    if total > STORY_CAP:
        failures.append(f"the demo evidence is {total} bytes, over the {STORY_CAP}-byte story cap")
    if not any(one["script"] and one["transcript"] for one in criteria.values()):
        failures.append("no criterion has one script and a transcript")
    return failures


from ..state.gate_evidence import register_gate  # noqa: E402 - the registry

register_gate("demo", measure)
