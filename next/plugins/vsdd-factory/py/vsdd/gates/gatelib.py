"""What the three story gates share (plan gates.md 3.1, D1, D3-D8, D17).

Not named ``gate_*``: the records' registry imports those by file name. Here
live the shapes (the result builder, the failure caps), the paths (the report
path, the evidence and tmp directories), the clean-worktree rule
(``dirty_files``, the one copy), the measure path the commands and the
registered functions both take, and the work bodies the job workers run.
"""

from __future__ import annotations

import hashlib
import os
import time
from pathlib import Path

from ..state import worktrees as _worktrees
from ..state.gate_evidence import evidence_dir, evidence_read, evidence_write, patch_id
from ..state.store import StoreRefused, locked, root_of, safe_segment, write_via_tmp
from ..state.worktrees import WorktreeError
from . import jobs
from .junit import JUnitError, file_from_classname, read_report

__all__ = ["STUB"]

#: The stub marker the test-writer puts on every stub it writes and the green
#: gate searches for in ``source``-class files (finding P3SEAM-16). Spliced
#: so this source file does not itself carry the marker.
STUB = "VSDD-" + "STUB"

FAILURES_MAX = 20
FAILURE_LENGTH = 400

_REPORT_PREFIX = "junit:"


# ---------------------------------------------------------------------------
# Shapes


def result(
    gate: str,
    subject: str,
    sha: str,
    *,
    passed: bool,
    failures,
    metrics=None,
    evidence_path: str = "",
    diff_patch: str = "",
    test_output: str = "",
    flaky=None,
    changed_files=None,
) -> dict:
    """A gate result: ``ok`` true, at least one failure line when not passed
    (D3), ``error`` equal to the first failure."""
    failures = capped_failures([one for one in failures if one])
    built = {
        "ok": True,
        "gate": gate,
        "subject": subject,
        "passed": bool(passed),
        "sha": sha,
        "evidence": evidence_path,
        "diff_patch": diff_patch,
        "test_output": test_output,
        "failures": failures,
        "flaky": list(flaky or []),
        "metrics": dict(metrics or {}),
    }
    if changed_files is not None:
        built["changed_files"] = list(changed_files)
    if not passed and failures:
        built["error"] = failures[0]
    return built


def not_attempted(gate: str, subject: str, reason: str, sha: str = "") -> dict:
    """The gate was not attempted: ``ok`` false, exit 1, one failure line."""
    return {
        "ok": False,
        "gate": gate,
        "subject": subject,
        "passed": False,
        "sha": sha,
        "evidence": "",
        "failures": capped_failures([reason]),
        "error": reason,
        "metrics": {},
    }


def normalize(gate: str, subject: str, sha: str, output: dict) -> dict:
    """A job-layer failure (limit, crash, lost twice) keeps the gate shape
    (D3): the job's ``error`` is copied into ``failures``. A pending object
    or an ordinary result passes as it stands."""
    if output.get("state") == "pending" or output.get("ok"):
        return output
    failures = list(output.get("failures") or ())
    error = str(output.get("error") or "the gate job failed")
    if not failures:
        failures = [error]
    return {
        "ok": False,
        "gate": output.get("gate", gate),
        "subject": output.get("subject", subject),
        "passed": False,
        "sha": output.get("sha") or sha,
        "evidence": "",
        "failures": capped_failures(failures),
        "error": failures[0],
        "metrics": dict(output.get("metrics") or {}),
    }


def capped_failures(lines) -> list[str]:
    """At most 20 lines of 400 characters; beyond that, one final
    ``and <n> more``."""
    lines = [str(one)[:FAILURE_LENGTH] for one in lines if one]
    if len(lines) > FAILURES_MAX - 1:
        more = len(lines) - (FAILURES_MAX - 1)
        lines = lines[: FAILURES_MAX - 1] + [f"and {more} more"]
    return lines


def record_path(project, subject: str, gate: str, sha: str) -> str:
    """The absolute path of that gate and commit's evidence record."""
    target = evidence_dir(project, subject) / f"{safe_segment(gate)}.{str(sha)[:12]}.json"
    return str(target if target.is_absolute() else os.path.abspath(target))


def absolute(path) -> str:
    return str(path if Path(path).is_absolute() else os.path.abspath(path))


# ---------------------------------------------------------------------------
# Configuration and the clean-worktree rule


def report_relative(config) -> str:
    """The JUnit report path inside the worktree, from ``commands.test_report``
    (``junit:<path>``); ``""`` when unset or not a ``junit:`` path."""
    raw = config.command("test_report")
    if not raw.startswith(_REPORT_PREFIX):
        return ""
    relative = raw[len(_REPORT_PREFIX):].strip()
    if not relative or os.path.isabs(relative) or ".." in Path(relative).parts:
        return ""
    return Path(relative).as_posix()


def dirty_files(tree, config) -> list[str]:
    """The worktree's uncommitted files (D5): ``git status --porcelain -z
    --untracked-files=all``, the JUnit report path excepted — it is the one
    untracked file a gate may write. The one copy; delivery's clean check
    calls it too."""
    from ..state import gitrun

    raw = gitrun.git(tree, "status", "--porcelain", "-z", "--untracked-files=all")
    report = report_relative(config) if config is not None else ""
    paths: list[str] = []
    for entry in raw.split("\0"):
        if len(entry) < 4 or entry[2] != " ":
            continue  # the second path of a rename pair, or a malformed line
        status, path = entry[:2], entry[3:]
        if report and path == report and set(status) <= {"?", "!"}:
            continue  # an untracked (or ignored-marked) report file alone is not dirt
        if path:
            paths.append(path)
    return sorted(set(paths))


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(65536), b""):
            digest.update(block)
    return digest.hexdigest()


def tracked_of_class(tree, config, wanted: str) -> list[str]:
    """Every tracked file of one product class in ``tree``, worktree-relative."""
    from .. import project as project_mod
    from ..state import gitrun

    out: list[str] = []
    for path in sorted({one for one in gitrun.git(tree, "ls-files", "-z").split("\0") if one}):
        if project_mod.path_class(Path(tree) / path, root=Path(tree), config=config) == wanted:
            out.append(path)
    return out


def frozen_set(tree, config, base: str, head: str) -> tuple[list[dict], list[str]]:
    """The files the diff from ``base`` to ``head`` adds or changes that are of
    class ``tests``, with the SHA-256 of each at ``head``; and the
    extension-less paths of every tracked ``tests``-class file, for the
    classname-to-file rule (VSA-12)."""
    from .. import project as project_mod
    from ..state import gitrun

    frozen: list[dict] = []
    names_raw = gitrun.git(tree, "diff", "--name-only", "-z", base, head)
    for path in sorted({one for one in names_raw.split("\0") if one}):
        absolute = Path(tree) / path
        if not absolute.is_file():
            continue
        if project_mod.path_class(absolute, root=Path(tree), config=config) == "tests":
            frozen.append({"file": path, "sha": sha256_of(absolute)})
    tests_files = tracked_of_class(tree, config, "tests")
    return frozen, tests_files


def stub_files(tree, config) -> list[str]:
    """Tracked ``source``-class files at the head that hold the stub marker
    (finding P3SEAM-16)."""
    found: list[str] = []
    for path in tracked_of_class(tree, config, "source"):
        try:
            text = (Path(tree) / path).read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        if STUB in text:
            found.append(path)
    return found


# ---------------------------------------------------------------------------
# The measure path: taken by the commands and by the registered functions


class Refused(Exception):
    """The gate cannot be attempted; the message is the one failure line."""


def resolve_since(tree, since: str, head: str) -> str:
    """``--since`` resolved to a commit that is an ancestor of the head
    (D17); raises :class:`Refused` for anything else."""
    from ..state import gitrun

    if not since:
        return ""
    try:
        resolved = gitrun.git(tree, "rev-parse", "--verify", f"{since}^{{commit}}").strip()
    except gitrun.GitError as exc:
        raise Refused(f"--since {since!r} names no commit: {exc}") from exc
    if not _worktrees.is_ancestor(tree, resolved, head):
        raise Refused(f"--since {since[:12]} is not an ancestor of the head {head[:12]}")
    return resolved


def resolve_story(root, subject: str, run: str) -> dict:
    """``require_story`` plus the configuration, raising :class:`Refused` for
    anything a caller can cause."""
    from ..config import load
    from ..state.admission import AdmissionError, require_story

    try:
        spec = require_story(root, run, subject)
    except (AdmissionError, WorktreeError) as exc:
        raise Refused(f"admission refused: {exc.message}") from exc
    spec["story"] = subject
    spec["config"] = load(root / ".factory" / "factory.toml")
    return spec


def measure(
    project,
    subject: str,
    run: str,
    *,
    gate: str,
    wait: int | None = None,
    retry: str = "",
    refreeze: bool = False,
    since: str = "",
    session_id: str | None = None,
) -> dict:
    """The one path to a gate result (D1): resolve the story, refuse what a
    caller caused, return a cached record, attach to the gate's job. Taken by
    the command, by the registry function and by S3's second part; it never
    measures in the caller's process, and raises for nothing a caller causes."""
    root = root_of(project)
    spec: dict | None = None
    try:
        spec = resolve_story(root, subject, run)
        if gate == "red":
            from ..state.story_unit import kind_has_red_gate, story_kind

            if not kind_has_red_gate(story_kind(root, subject)):
                # D16: a red gate, or a re-freeze, for a story kind with none
                # fails out loud — measured-and-failed (ok true), no record.
                return result(
                    gate, subject, spec["head"], passed=False,
                    failures=[f"story kind {story_kind(root, subject) or '(none)'} "
                              "has no red gate"],
                )
        _require_report(spec["config"])
        since_full = resolve_since(spec["worktree"], since, spec["head"]) if since else ""
    except Refused as exc:
        # The story resolved: the head is named even on a refusal (plan
        # gates.md 3.1 — sha is "" only when the story could not be resolved).
        return not_attempted(gate, subject, str(exc), sha=spec["head"] if spec else "")
    tree, head = spec["worktree"], spec["head"]
    dirty = dirty_files(tree, spec["config"])
    if dirty:
        return _dirty_result(gate, subject, head, dirty)
    cached = _cached(root, spec, gate, since_full=since_full)
    if cached is not None:
        return cached
    command_words = _command_words(gate, refreeze=refreeze, since_full=since_full)
    job = jobs.JobSpec(
        command=command_words,
        subject=subject,
        commit=head,
        work={"red": "vsdd.gates.gate_red:work", "green": "vsdd.gates.gate_green:work"}[gate],
        args={
            "gate": gate,
            "story": subject,
            "run": run,
            "head": head,
            "base": spec["base"],
            "cycle": spec["cycle"],
            "since": since_full,
            "refreeze": bool(refreeze),
        },
        error_shape={"gate": gate},
        session_id=session_id,
    )
    if not retry:
        retry = f"vsdd {command_words} --story {subject} --run {run}"
    try:
        outcome = jobs.attach(root, job, wait=wait, retry=retry)
    except ValueError as exc:  # a malformed spec: nothing written
        return not_attempted(gate, subject, f"the gate job cannot start: {exc}", sha=head)
    if outcome.pending:
        return dict(outcome.output)
    output = _serve_own_run(root, job, outcome.output, run, wait=wait, retry=retry)
    return normalize(gate, subject, head, output)


def _require_report(config) -> str:
    if not config.command("test"):
        raise Refused("commands.test is not configured; the gate cannot measure")
    report = report_relative(config)
    if not report:
        raise Refused("commands.test_report must be junit:<path inside the worktree>")
    return report


def _dirty_result(gate: str, subject: str, head: str, dirty: list[str]) -> dict:
    lines = ["uncommitted changes: commit them, then run the gate again"] + sorted(dirty)
    return result(gate, subject, head, passed=False, failures=lines)


def _command_words(gate: str, *, refreeze: bool, since_full: str) -> str:
    words = f"gate {gate}"
    if gate == "red" and refreeze:
        words += " --refreeze"
    if gate == "green" and since_full:
        words += f" --since {since_full}"
    return words


def _cached(project, spec: dict, gate: str, *, since_full: str = "") -> dict | None:
    """The passing record for exactly this head (D4), never one found by
    patch id. Green writes the whole patch again on every call, and with
    ``--since`` the range patch too; the result's ``diff_patch`` is the range
    patch with ``--since``, the whole patch without it."""
    subject, head = spec["story"], spec["head"]
    record = evidence_read(project, subject, gate, head)
    if record is None or not record.get("passed"):
        return None
    diff_patch = ""
    test_output = ""
    changed_files: list[str] = []
    if gate == "green":
        from .diffpatch import write_patch

        if not (evidence_dir(project, subject) / f"green.{head[:12]}.xml").is_file():
            # D4: green's cached record also needs its report at this head;
            # a lost .xml is measured again.
            return None
        diff_patch = write_patch(project, subject, spec["base"], head, tree=spec["worktree"])
        if since_full:
            diff_patch = write_patch(
                project, subject, spec["base"], head, tree=spec["worktree"], since=since_full
            )
            from ..state.verdicts import changed_files as changed_files_of

            computed = changed_files_of(project, since_full, head)
            if computed is None:
                return None
            changed_files = computed
        test_output = absolute(evidence_dir(project, subject) / f"green.{head[:12]}.log")
    return result(
        gate,
        subject,
        head,
        passed=True,
        failures=[],
        evidence_path=record_path(project, subject, gate, head),
        diff_patch=diff_patch,
        test_output=test_output,
        flaky=record.get("flaky") or [],
        # changed_files is a green field (plan gates.md 3.1): a cached red
        # keeps the red shape, like the freshly measured result.
        changed_files=changed_files if gate == "green" else None,
    )


def _serve_own_run(root, job: jobs.JobSpec, output: dict, run: str, *, wait, retry) -> dict:
    """A result made for another run is not served (D1, PL4-16): attach again,
    once, then strip the private ``run`` key either way."""
    served = output
    if isinstance(served, dict) and served.get("run") and run and served["run"] != run:
        outcome = jobs.attach(root, job, wait=wait, retry=retry)
        served = outcome.output
    if isinstance(served, dict):
        served.pop("run", None)
    return served


# ---------------------------------------------------------------------------
# The work bodies


class WorkFailed(Exception):
    """Ends the work with a measured failure: ``ok`` true, ``passed`` false,
    no record written."""


def _failed(lines, metrics=None) -> WorkFailed:
    failure = WorkFailed(tuple(lines))
    failure.metrics = dict(metrics or {})
    return failure


def _not_ok(gate: str, subject: str, sha: str, reason: str) -> dict:
    """A measured-but-not-answerable outcome: ``ok`` false, exit 1."""
    return not_attempted(gate, subject, reason, sha=sha)


def work_context(ctx: jobs.JobContext) -> dict:
    """The worker's body (D1: the worker repeats the checks and measures),
    under the one-measurer lock of the story worktree (D6). A WorktreeError —
    a git failure, the sandbox wording included, ORDER rule 7 — and a
    :class:`StoreBusy` rise: the job layer's error line carries their
    message."""
    args = ctx.args
    try:
        spec = resolve_story(ctx.root, args["story"], args["run"])
    except Refused as exc:
        return _with_run(args, not_attempted(args["gate"], args["story"], str(exc)))
    with locked(ctx.root, f"gate-{args['story']}.lock", wait=max(ctx.remaining(), 0.0)):
        try:
            output = _measured(ctx, spec)
        except WorkFailed as exc:
            output = result(
                args["gate"], args["story"], spec["head"], passed=False,
                failures=tuple(exc.args[0]), metrics=exc.metrics,
            )
        return _with_run(args, output)


def _with_run(args: dict, output: dict) -> dict:
    """Carry the private ``run`` key in the worker's result (D1): the command
    and the registry function serve no result made for another run."""
    output = dict(output)
    if args.get("run"):
        output["run"] = args["run"]
    return output


def _measured(ctx: jobs.JobContext, spec: dict) -> dict:
    gate = ctx.args["gate"]
    tree, head = Path(spec["worktree"]), spec["head"]
    report_rel = _require_report(spec["config"])
    dirty = dirty_files(tree, spec["config"])
    if dirty:
        raise _failed(_dirty_result(gate, spec["story"], head, dirty)["failures"])
    cached = _cached(ctx.root, spec, gate, since_full=ctx.args.get("since", ""))
    if cached is not None:
        return cached
    started = time.monotonic()
    outputs: list[str] = []
    if gate == "red":
        if ctx.args.get("refreeze"):
            record, metrics = _refreeze(ctx, spec, report_rel, outputs)
            failures: list[str] = []
        else:
            record, metrics, failures = _red(ctx, spec, report_rel, outputs)
    else:
        record, metrics, failures = _green(ctx, spec, report_rel, outputs)
    metrics["seconds"] = round(time.monotonic() - started, 3)
    moved = _worktrees.head(tree)
    if moved != head:
        # After measuring, a head that moved is a failure with no record (D5).
        raise _failed(
            [f"the head moved while the gate ran ({moved[:12]}): run the gate again"],
            metrics,
        )
    log = "\n".join(outputs)
    extra: dict = {}
    baseline = metrics.pop("_baseline", "")
    if gate == "green":
        from .diffpatch import write_patch

        # D4: every call writes the whole base-to-head patch again, and with
        # --since the range patch too; the result's diff_patch is the range
        # patch with --since, the whole patch without it (D17, PL4-6). The
        # record itself always carries the whole patch (plan gates.md 3.3),
        # so it is written before the record is.
        record["diff_patch"] = write_patch(ctx.root, spec["story"], spec["base"], head, tree=tree)
    try:
        evidence_write(ctx.root, record, log=log, report=tree / report_rel)
    except StoreRefused as exc:
        if exc.code == "over_cap" and gate == "red":
            raise _failed(
                [f"the red record is over its cap ({len(record['tests'])} new tests): "
                 "split the story"],
                metrics,
            ) from exc
        raise _failed([f"the evidence record was refused: {exc.code}: {exc}"], metrics) from exc
    if gate == "green":
        extra["diff_patch"] = record["diff_patch"]
        extra["changed_files"] = []
        if ctx.args.get("since"):
            from .diffpatch import write_patch

            extra["diff_patch"] = write_patch(
                ctx.root, spec["story"], spec["base"], head, tree=tree,
                since=ctx.args["since"],
            )
            from ..state.verdicts import changed_files as changed_files_of

            computed = changed_files_of(ctx.root, ctx.args["since"], head)
            if computed is None:
                return _not_ok(gate, spec["story"], head,
                               f"the files changed since {ctx.args['since'][:12]} "
                               "cannot be listed")
            extra["changed_files"] = computed
        extra["test_output"] = absolute(
            evidence_dir(ctx.root, spec["story"]) / f"green.{head[:12]}.log"
        )
        if baseline and not record.get("passed"):
            failures.append("the cycle has a baseline, and the comparison is not built: "
                            "every failure counts")
    built = result(
        gate,
        spec["story"],
        head,
        passed=bool(record.get("passed")),
        failures=failures,
        metrics=metrics,
        evidence_path=record_path(ctx.root, spec["story"], gate, head),
        flaky=record.get("flaky") or [],
    )
    built.update(extra)
    return built


def _run_command(ctx, name: str, command: str, cwd: Path) -> str:
    """One configured command inside the job; its output lands in
    ``.factory/local/tmp/<job key>.<name>.out`` (D6) and is returned."""
    output = ctx.root / ".factory" / "local" / "tmp" / f"{ctx.key}.{name}.out"
    ctx.run(command, cwd=cwd, output=output)
    try:
        return output.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def _head_report(ctx, spec: dict, report_rel: str, outputs: list[str]):
    """Delete the report, run the test command, read it (D7: never an old
    report). Raises :class:`WorkFailed` when no readable report is there."""
    report = Path(spec["worktree"]) / report_rel
    try:
        report.unlink()
    except FileNotFoundError:
        pass
    outputs.append(_run_command(ctx, "test", spec["config"].command("test"), spec["worktree"]))
    try:
        return read_report(report)
    except JUnitError as exc:
        raise _failed(
            [f"the test command wrote no JUnit report at {report_rel} ({exc})"]) from None


def _base_cases(ctx, spec: dict, report_rel: str, outputs: list[str]) -> dict:
    """The base set: the test command's report at the base commit, kept as
    ``evidence_dir(project, "_base")/<base12>.xml`` and so measured once (D7).
    No report at the base means an empty base set and nothing kept. A kept
    report that does not parse (a kill mid-write, two stories cut from one
    base measured at once) is deleted and the base measured again — never
    read as an empty set."""
    base = spec["base"]
    directory = evidence_dir(ctx.root, "_base")
    kept = directory / f"{base[:12]}.xml"
    if kept.is_file():
        try:
            cases = read_report(kept)
        except JUnitError:
            try:
                kept.unlink()
            except OSError:
                pass
        else:
            outputs.append(f"# base report reused from local/evidence/_base/{base[:12]}.xml")
            return cases
    name = f"base-{ctx.key}"  # D6: the base report runs in _tmp-base-<job key>
    with _worktrees.detached(ctx.root, base, name) as tmp:
        report = Path(tmp) / report_rel
        try:
            report.unlink()
        except FileNotFoundError:
            pass
        outputs.append(_run_command(ctx, "base", spec["config"].command("test"), Path(tmp)))
        if not report.is_file():
            outputs.append("# no report at the base; the base set is empty")
            return {}
        directory.mkdir(parents=True, exist_ok=True)
        _retain_old_base_reports(directory)
        write_via_tmp(kept, report.read_bytes())
        return _read_or_empty(kept)


def _read_or_empty(path) -> dict:
    """The report just measured at the base; unusable bytes are an empty set
    (D7: no usable report at the base, nothing kept)."""
    try:
        return read_report(path)
    except JUnitError:
        return {}


def _retain_old_base_reports(directory) -> None:
    """Base reports older than ``state.retain_runs_days`` are deleted when a
    new one is written (D7). The job's own configuration is read; anything
    unreadable falls back to the default of 30 days, out loud in neither
    direction."""
    from ..config import ConfigError, load

    root = directory.parent.parent.parent.parent  # evidence/_base sits four under the project
    try:
        config = load(root / ".factory" / "factory.toml")
        days = int(config.get("state", "retain_runs_days"))
    except ConfigError:
        days = 30
    deadline = time.time() - days * 86400
    for path in directory.glob("*.xml"):
        try:
            if path.stat().st_mtime < deadline:
                path.unlink()
        except OSError:
            continue


# ----------------------------------------------------------------- red -----


def _red(ctx, spec: dict, report_rel: str, outputs: list[str]):
    """The red rule (D9): new tests that fail on assertions and not on build
    errors, at least half of the non-exempt ones. A passing measurement
    freezes the new tests; a failing one is recorded with no frozen set."""
    tree, base, head = Path(spec["worktree"]), spec["base"], spec["head"]
    config = spec["config"]
    head_cases = _head_report(ctx, spec, report_rel, outputs)
    base_identities = set(_base_cases(ctx, spec, report_rel, outputs))
    frozen, tests_files = frozen_set(tree, config, base, head)

    counted = _new_cases(tree, config, head_cases, base_identities, tests_files)
    metrics = {"new": len(counted), "red": 0, "exempt": 0, "base_tests": len(base_identities),
               "no_file": sum(1 for _, file in counted if not file), "seconds": 0.0}
    red = [case for case, _ in counted if case.red]
    harness = [(case, _) for case, _ in counted if case.harness_error]
    exempt = sum(1 for case, _ in counted if case.exempt)
    non_exempt = sum(1 for case, _ in counted if not case.exempt)
    metrics["red"] = len(red)
    metrics["exempt"] = exempt
    failures: list[str] = []
    if not counted:
        failures.append("no new tests")
    for case, _ in harness:
        failures.append(
            f"build or harness error in {case.identity}: "
            f"{case.error_types[0] if case.error_types else 'unknown'}"
        )
    # The half rule over the non-exempt tests alone: a red exempt test does
    # not make a non-exempt passing test acceptable (D9).
    red_non_exempt = sum(1 for case, _ in counted if case.red and not case.exempt)
    if red_non_exempt * 2 < non_exempt:
        failures.append(
            f"fewer than half of the non-exempt new tests are red "
            f"({red_non_exempt} of {non_exempt})"
        )
    if counted and not red:
        failures.append("no red test")
    record = {
        "gate": "red",
        "subject": spec["story"],
        "sha": head,
        "passed": not failures,
        "patch_id": patch_id(tree, base, head),
    }
    if not failures:
        record["frozen"] = frozen
        record["tests"] = [case.identity for case, _ in counted]
        record["criteria"] = _criteria_map(counted)
    return record, metrics, failures


def _refreeze(ctx, spec: dict, report_rel: str, outputs: list[str]):
    """The re-freeze (D10): the frozen set computed afresh from the base to
    the head; a file the earlier record froze that is gone is a refusal, and
    a refused re-freeze writes no record, so the earlier one stands. With no
    file changed or added the record is written at the head with the earlier
    tests and criteria, and the suite does not run. Otherwise the suite runs
    once: each earlier identity must still exist and either pass or fail on
    an assertion (a skipped changed test fails), and each new test in a
    changed or unknown file must pass or be red in D9's sense."""
    from ..state.gate_evidence import evidence

    root, tree = ctx.root, Path(spec["worktree"])
    base, head, story = spec["base"], spec["head"], spec["story"]
    red = evidence(root, "red", story)
    if red is None:
        raise _failed(["no red record"], {"changed": 0, "seconds": 0.0})
    frozen, tests_files = frozen_set(tree, spec["config"], base, head)
    fresh = {entry["file"]: entry["sha"] for entry in frozen}
    earlier = {entry["file"]: entry["sha"] for entry in red.get("frozen", [])}
    changed = [one for one in fresh if one in earlier and fresh[one] != earlier[one]]
    added = [one for one in fresh if one not in earlier]
    metrics = {"changed": len(changed), "added": len(added), "seconds": 0.0}
    gone = [one for one in earlier if one not in fresh]
    if gone:
        raise _failed([f"frozen test file removed: {one}" for one in gone], metrics)
    counted: list[tuple] = []
    if changed or added:
        head_cases = _head_report(ctx, spec, report_rel, outputs)
        base_identities = set(_base_cases(ctx, spec, report_rel, outputs))
        counted = _new_cases(tree, spec["config"], head_cases, base_identities, tests_files)
        # D10: every earlier identity must still be reported; the skip and
        # harness checks belong to the changed or added files alone (and to
        # tests with no file), so an unchanged test broken by half-done code
        # does not refuse the re-freeze.
        checked: set = set(changed) | set(added)
        failures: list[str] = []
        for identity in red.get("tests", []):
            case = head_cases.get(identity)
            if case is None:
                failures.append(f"frozen test did not run: {identity}")
                continue
            file = _file_of(case, tree, tests_files)
            if file and file not in checked:
                continue
            if case.skipped:
                failures.append(f"frozen test is skipped: {identity}")
            elif case.harness_error:
                types = case.error_types
                failures.append(
                    f"build or harness error in {identity}: {types[0] if types else 'unknown'}"
                )
        for case, file in counted:
            if file and file not in checked:  # a changed or unknown file: D10
                continue
            if case.passed or case.red:
                continue
            failures.append(f"the new test {case.identity} neither passes nor is red")
        if failures:
            raise _failed(failures, metrics)
    record = {
        "gate": "red",
        "subject": story,
        "sha": head,
        "passed": True,
        "refreeze": True,
        "patch_id": patch_id(tree, base, head),
        "frozen": [{"file": one, "sha": fresh[one]} for one in sorted(fresh)],
        "tests": [case.identity for case, _ in counted] if counted else list(red.get("tests", [])),
        "criteria": _criteria_map(counted) if counted else dict(red.get("criteria", {})),
    }
    if not (changed or added):
        outputs.append("# nothing changed since the latest red record; the suite does not run")
    return record, metrics


def _file_of(case, tree: Path, tests_files: list[str]) -> str:
    """The file of a test: its ``file`` attribute made relative to the
    worktree, else the classname rule (D8)."""
    raw = case.file_attr
    if raw:
        path = Path(raw)
        if path.is_absolute():
            try:
                path = path.relative_to(tree)
            except ValueError:
                return ""
        return path.as_posix()
    return file_from_classname(case.classname, tests_files)


def _new_cases(tree, config, head_cases: dict, base_identities: set, tests_files: list[str]):
    """The head's new tests that red and the re-freeze count with (D8, D9):
    the identities the base report lacks, and none of a source-class file —
    the one copy, so the two gates agree on which tests are new."""
    from .. import project as project_mod

    counted: list[tuple] = []
    for identity, case in head_cases.items():
        if identity in base_identities:
            continue
        file = _file_of(case, tree, tests_files)
        if file and project_mod.path_class(tree / file, root=tree, config=config) == "source":
            continue  # a test in a source-class file does not count
        counted.append((case, file))
    return counted


def _criteria_map(counted) -> dict:
    from .junit import criteria_of

    mapping: dict[str, list[str]] = {}
    for case, _ in counted:
        criterion = criteria_of(case.name)
        if criterion:
            mapping.setdefault(criterion, []).append(case.identity)
    return mapping


# --------------------------------------------------------------- green -----


def _green(ctx, spec: dict, report_rel: str, outputs: list[str]):
    """The green rule (D12, D13): the static checks first — if one fails, no
    command runs and the failing record is written — then the configured
    commands. The suite passes on exit 0 and a report with no failed case."""
    root, tree = ctx.root, Path(spec["worktree"])
    story, base, head = spec["story"], spec["base"], spec["head"]
    config = spec["config"]
    metrics: dict = {"tests": 0, "failed": 0, "skipped": [], "seconds": 0.0, "baseline": ""}
    static: list[str] = []
    red_record = None
    from ..state.gate_evidence import evidence
    from ..state.story_unit import kind_has_red_gate, story_kind

    if kind_has_red_gate(story_kind(root, story)):
        red_record = evidence(root, "red", story)
        if red_record is None:
            static.append("no red record")
    if red_record is not None:
        for entry in red_record.get("frozen", []):
            file = entry.get("file", "")
            current = _hash_at(tree, file)
            if current is None or current != entry.get("sha"):
                static.append(f"frozen test changed: {file}")
    for file in stub_files(tree, config):
        static.append(f"the stub marker remains in {file}")
    static.extend(_required_check_failures(root, story, config))
    failures: list[str] = list(static)
    passed = False
    if not static:
        failures = _green_commands(ctx, spec, report_rel, outputs, metrics, red_record)
        passed = not failures
    record = {
        "gate": "green",
        "subject": story,
        "sha": head,
        "passed": passed,
        "patch_id": patch_id(tree, base, head),
        "flaky": [],
    }
    from ..state.cycles import cycle_read

    baseline = (cycle_read(root, spec["cycle"]) or {}).get("baseline", "")
    if baseline:
        metrics["baseline"] = "not applied"
    metrics["_baseline"] = baseline
    return record, metrics, failures


def _green_commands(ctx, spec, report_rel, outputs, metrics, red_record) -> list[str]:
    """Every configured command runs; each failure is named (D12). The suite
    passes on exit 0 and a report with no failed case."""
    tree = Path(spec["worktree"])
    config = spec["config"]
    report = tree / report_rel
    failures: list[str] = []
    codes: dict[str, int] = {}
    try:
        report.unlink()  # never an old report (D7)
    except FileNotFoundError:
        pass
    for name, command in (
        ("build", config.command("build")),
        ("test", config.command("test")),
        ("lint", config.command("lint")),
        ("format_check", config.command("format_check")),
        ("ui_check", config.command("ui_check")),
        ("docs_check", config.command("docs_check")),
    ):
        if not command:
            if name in ("build", "lint", "format_check"):
                metrics["skipped"].append(name)
            continue
        codes[name] = _coded_command(ctx, name, command, tree, outputs)
        if codes[name] != 0:
            failures.append(f"commands.{name} failed (exit {codes[name]})")
    cases: dict = {}
    try:
        cases = read_report(report)
    except JUnitError as exc:
        failures.append(f"the test command wrote no JUnit report at {report_rel} ({exc})")
    failed_cases = [case for case in cases.values() if case.failed or case.harness_error]
    metrics["tests"] = len(cases)
    metrics["failed"] = len(failed_cases)
    for case in failed_cases:
        failures.append(f"failed: {case.identity}")
    if red_record is not None:
        for identity in red_record.get("tests", []):
            case = cases.get(identity)
            if case is None:
                failures.append(f"frozen test did not run: {identity}")
            elif not case.passed:
                failures.append(f"frozen test did not pass: {identity}")
    return failures


def _coded_command(ctx, name: str, command: str, tree: Path, outputs: list[str]) -> int:
    """One configured command; its output is collected for the record's
    ``.log``, its exit code returned."""
    output = ctx.root / ".factory" / "local" / "tmp" / f"{ctx.key}.{name}.out"
    outcome = ctx.run(command, cwd=tree, output=output)
    try:
        outputs.append(output.read_text(encoding="utf-8", errors="replace"))
    except OSError:
        outputs.append("")
    return outcome.returncode


def _required_check_failures(root, story: str, config) -> list[str]:
    """A story with ``ui`` also needs ``ui_check``; a ``docs`` story needs
    ``docs_check`` as well as the others (finding AU-P2-07, D12)."""
    from ..state.artifacts import ArtifactError, load_artifact

    try:
        meta = load_artifact(root, story).frontmatter
    except ArtifactError:
        return []
    lines: list[str] = []
    if meta.get("ui") and not config.command("ui_check"):
        lines.append("the story has ui, and commands.ui_check is not configured")
    if str(meta.get("kind", "")) == "docs" and not config.command("docs_check"):
        lines.append("a docs story needs commands.docs_check")
    return lines


def _hash_at(tree: Path, file: str) -> str | None:
    path = Path(tree) / file
    if not path.is_file():
        return None
    try:
        return sha256_of(path)
    except OSError:
        return None
