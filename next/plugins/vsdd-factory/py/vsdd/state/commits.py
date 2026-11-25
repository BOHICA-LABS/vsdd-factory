"""Flush: the one writer of state commits (topics/state-commits.md; plan
records.md section 3, decision D11).

Called from five places and nowhere else. It takes ``flush.lock`` (a second
flush waits up to 5 seconds and then leaves the work to the holder), checks
the mount, validates every changed file — artifacts with S1's
``check_paths``, the records themselves by schema and cap, ``factory.toml``
by the loader — stages deletions and commits with the hooks path emptied and
signing off, so the product's own hooks never run in the state tree. It
never touches a product branch, a product file or the person's index, and it
raises for nothing: every failure comes back as a result.

A flush called from a sandboxed Bash call cannot write under
``.git/worktrees/`` (the probe of 2026-11-24: Operation not permitted), so a
git failure whose cause is the sandbox comes back in ``error`` naming it; a
flush run from a hook is not sandboxed.
"""

from __future__ import annotations

from pathlib import Path

from .store import StoreBusy, StoreCorrupt, read_record, root_of

__all__ = ["flush"]  # the facade lists only state-api names

FLUSH_WAIT = 5.0

#: State paths are passed to ``git add`` / ``git rm --cached`` / ``git commit
#: --only`` as pathspecs, and git reads glob characters (``* ? [ ]``) and a
#: leading ``:`` as patterns and pathspec magic. Records carry person-chosen
#: names, so the pathspecs are literal: a valid ``[ab].md`` must not drag a
#: rejected neighbour ``a.md`` into the commit (finding
#: step3-records-fix-3-C1-1).
#:
#: Literal mode is forced on with the other three global pathspec settings
#: forced off (git reads ``0`` as off): with a person's
#: ``GIT_ICASE_PATHSPECS=1`` or ``GIT_GLOB_PATHSPECS=1`` inherited, literal
#: alone makes git exit 128 — the 'literal' pathspec setting is incompatible
#: with all other global pathspec settings — and every flush with changes
#: would fail (finding step3-records-fix-3-C2-1). The env this dict leaves
#: alone cannot bite the commands flush runs: the repository-locating
#: ``GIT_DIR`` / ``GIT_WORK_TREE`` / ``GIT_INDEX_FILE`` (and the deeper object
#: and common-dir keys) are stripped for every git call in :mod:`gitrun`;
#: ``GIT_CONFIG_*`` and the author and committer variables change only the
#: commit's metadata, which is the person's identity by design.
_LITERAL_PATHSPECS = {
    "GIT_LITERAL_PATHSPECS": "1",
    "GIT_GLOB_PATHSPECS": "0",
    "GIT_NOGLOB_PATHSPECS": "0",
    "GIT_ICASE_PATHSPECS": "0",
}

_SANDBOX_MARKS = ("operation not permitted", "sandbox")

_CLEAN = {"ok": True, "state": "clean", "commit": "", "files": [], "rejected": [], "error": ""}


def flush(root, reason: str, *, run: str = "") -> dict:
    """Commit the tracked state files that changed. ``ok`` is false for
    ``partial``, ``failed`` and a non-empty ``rejected``. States:
    ``committed``, ``clean``, ``partial``, ``busy``, ``failed``. Flush never
    raises (PR4C-08): every failure, seen or surprise, comes back as a
    result."""
    from .gitrun import GitError
    from .store import locked

    root = root_of(root)
    try:
        with locked(root, "flush.lock", wait=FLUSH_WAIT):
            return _flush_locked(root, reason, run=run)
    except StoreBusy:
        return {**_CLEAN, "state": "busy", "error": "another flush is running; it does the work"}
    except GitError as exc:
        return _failure(str(exc), exc.stderr)
    except Exception as exc:  # a surprise is a state failure, never a raise
        return _failure(f"flush failed unexpectedly: {exc!r}", "")


def _flush_locked(root: Path, reason: str, *, run: str) -> dict:
    from ..config import ConfigError, load as load_config
    from .artifacts import changed_paths, check_paths
    from .gitrun import GitError, git as run_git
    from .mount import mounted

    base = root / ".factory"
    if not mounted(root):
        return {
            **_CLEAN,
            "ok": False,
            "state": "partial",
            "error": "the state directory is not mounted on the state branch",
        }
    try:
        changed = changed_paths(root)
        # Deletions too (D11): a record deleted from the tree is staged and
        # committed like any other change. And the index's own leftovers (a
        # killed flush staged work it never committed), whose both sides a
        # staged rename carries — status pairs them into one R record, so the
        # index diff, not the status, is what lists them.
        changed = sorted(
            set(changed) | set(_deleted_paths(root)) | set(_staged_paths(root))
        )
    except GitError as exc:
        return _failure(str(exc), exc.stderr)
    if not changed:
        return dict(_CLEAN)

    try:
        config = load_config(base / "factory.toml")
        cfg = getattr(config, "data", config)
    except ConfigError as exc:
        return {
            **_CLEAN,
            "ok": False,
            "state": "failed",
            "rejected": [{"path": ".factory/factory.toml", "problems": [str(exc)]}],
            "error": "factory.toml does not load",
        }

    problems_by_path: dict[str, list[str]] = {}
    # Only the changed files are checked (PL3-3), and never a deleted one: a
    # deletion has no text to judge, and capping it would stat a missing file.
    present = [rel for rel in changed if (root / rel).exists()]
    for problem in check_paths(root, present, config=cfg, references=False):
        if problem.severity == "error":
            problems_by_path.setdefault(problem.rel, []).append(f"{problem.code}: {problem.message}")
    for rel in present:
        name = rel[len(".factory/") :]
        detail = _record_problem(root, name)
        if detail is None and _record_folders(name):
            detail = [
                f"no_validator: {name} is a tracked record file no validator claims; "
                "register it in vsdd.state.commits.record_kinds (topics/state-commits.md)"
            ]
        if detail:
            problems_by_path.setdefault(rel, []).extend(detail)

    rejected = [
        {"path": rel, "problems": problems_by_path[rel]}
        for rel in changed
        if rel in problems_by_path
    ]
    valid = [rel for rel in changed if rel not in problems_by_path]
    if not valid:
        return {
            **_CLEAN,
            "ok": False,
            "state": "partial",
            "rejected": rejected,
            "error": "nothing committed; every changed file was refused",
        }
    try:
        trailer_run = run or _live_lock_run(root)
        message = _message(reason, trailer_run, _integration_head(run_git, root))
        state_rels = [rel[len(".factory/") :] for rel in valid]
        # A path gone from the worktree is staged with rm --cached: add -A
        # refuses a path that is in neither the worktree nor the index, which
        # a deletion staged by a killed flush (or by an earlier flush) is.
        existing = [rel for rel in state_rels if (base / rel).exists()]
        gone = [rel for rel in state_rels if not (base / rel).exists()]
        for start in range(0, len(existing), 40):
            chunk = existing[start : start + 40]
            run_git(base, "add", "-A", "--", *chunk, env=_LITERAL_PATHSPECS)
        if gone:
            for start in range(0, len(gone), 40):
                chunk = gone[start : start + 40]
                run_git(
                    base, "rm", "--cached", "-q", "--ignore-unmatch", "--", *chunk,
                    env=_LITERAL_PATHSPECS,
                )
        # A new file staged by a killed flush and deleted since ('AD') is gone
        # from index and HEAD alike once staging ran, and commit --only refuses
        # a pathspec no file matches. Commit what staging left that differs
        # from HEAD — with renames off, so a delete-plus-add pair of a renamed
        # record lists both its paths and the deletion is committed with the
        # addition (S3R-records-C2-1); with rename detection git prints only
        # the new name and the old path would stay staged, wedging every
        # later flush.
        differing = set(
            run_git(base, "diff", "--cached", "--name-only", "--no-renames", "-z").split("\0")
        )
        commit_rels = [rel for rel in state_rels if rel in differing]
        if not commit_rels:
            # Staging cancelled every valid change; only the rejected files,
            # unstaged, can stand between the tree and clean.
            return {
                **_CLEAN,
                "ok": not rejected,
                "state": "partial" if rejected else "clean",
                "rejected": rejected,
                "error": "" if not rejected else "some changed files stay uncommitted",
            }
        # git config exits 1 when the key is unset; that is the answer, not
        # an error (the no-identity fallback must be reachable).
        identity = run_git(base, "config", "user.email", check=False).strip()
        args = [
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--only",
            "-m",
            message,
            "--",
            *commit_rels,
        ]
        if not identity:
            args = ["-c", "user.name=vsdd-factory", "-c", "user.email=vsdd-factory@example.invalid", *args]
        run_git(base, *args, env=_LITERAL_PATHSPECS)
        commit = run_git(base, "rev-parse", "HEAD")
        return {
            "ok": not rejected,
            "state": "partial" if rejected else "committed",
            "commit": commit,
            "files": commit_rels,
            "rejected": rejected,
            "error": "" if not rejected else "some changed files stay uncommitted",
        }
    except GitError as exc:
        return _failure(str(exc), exc.stderr)


def _deleted_paths(root: Path) -> list[str]:
    """Tracked files deleted from the state worktree, as project-relative
    paths: the ``D`` records of ``git status``, which :func:`changed_paths`
    leaves out. A deletion is staged with ``rm --cached`` and committed like
    any other change (decision D11)."""
    from .gitrun import git as run_git

    raw = run_git(root / ".factory", "status", "--porcelain=v1", "-z", "-uall")
    records = raw.split("\0")
    deleted: list[str] = []
    index = 0
    while index < len(records):
        record = records[index]
        index += 1
        if not record:
            continue
        flag, name = record[:2], record[3:]
        if flag[0] in ("R", "C") and index < len(records):
            index += 1  # the paired original path follows each R/C record
        if "D" in flag and "?" not in flag:
            deleted.append(".factory/" + name)
    return deleted


def _staged_paths(root: Path) -> list[str]:
    """Paths the index already carries but HEAD does not, as project-relative
    paths: the work a killed flush staged and never committed. Status pairs a
    staged rename into one ``R`` record, which the two status readers pass
    over, so the index itself is read — with renames off, which lists the old
    and the new path of a rename both. Spaces and non-ASCII names come back
    whole: ``-z`` output is NUL-separated and never C-quoted."""
    from .gitrun import git as run_git

    raw = run_git(root / ".factory", "diff", "--cached", "--name-only", "--no-renames", "-z")
    return [".factory/" + name for name in raw.split("\0") if name]


def _failure(message: str, stderr: str) -> dict:
    return {
        **_CLEAN,
        "ok": False,
        "state": "failed",
        "error": _cause(message, stderr),
    }


def _cause(message: str, stderr: str) -> str:
    """Name the sandbox when git's failure is the Bash sandbox's write ban."""
    detail = f"{message} {stderr}".lower()
    if any(mark in detail for mark in _SANDBOX_MARKS):
        return (
            f"{message} (the Bash sandbox refuses writes under .factory/.git; run flush "
            "from a hook, which is not sandboxed, or outside the sandbox)"
        )
    return message


def record_kinds() -> list[dict]:
    """The tracked record kinds flush validates, one row per kind — the table
    beside the artifact registry (topics/state-commits.md). Each row carries
    ``kind`` (a name for the test), ``claim`` (a predicate on the
    state-relative path), ``schema`` (a shipped schema name or ``None``),
    ``cap`` (a byte cap or ``None``) and ``parse`` (whether the file must
    parse as a JSON record).

    A new tracked record kind must register its row here in the same step as
    it first lands in the state tree: flush refuses a tracked file under the
    record folders that no row claims, naming it, so a kind added later
    cannot ride into a commit unchecked (finding S3B-records-2)."""
    from .cycles import CLOSURE_CAP
    from .decisions import PACKET_CAP, RECORD_CAP

    def under(name: str, marker: str) -> bool:
        return name.startswith("cycles/") and marker in name

    return [
        {
            "kind": "cycle-record",
            "claim": lambda name: under(name, "/") and name.endswith("/cycle.json"),
            "schema": "cycle-record",
            "cap": None,
            "parse": True,
        },
        {
            "kind": "closure-record",
            "claim": lambda name: under(name, "/units/") and name.endswith(".json"),
            "schema": "closure-record",
            "cap": CLOSURE_CAP,
            "parse": True,
        },
        {
            "kind": "decision-record",
            "claim": lambda name: under(name, "/decisions/") and name.endswith(".json"),
            "schema": "decision-record",
            "cap": RECORD_CAP,
            "parse": True,
        },
        {
            "kind": "decision-packet",
            "claim": lambda name: under(name, "/decisions/") and name.endswith(".md"),
            "schema": None,
            "cap": PACKET_CAP,
            "parse": False,
        },
        {
            "kind": "notes",
            "claim": lambda name: name == "notes.json",
            "schema": None,
            "cap": None,
            "parse": True,
        },
    ]


def _record_folders(name: str) -> bool:
    """Whether ``name`` lies in a record folder whose tracked files flush
    refuses to commit unchecked: the cycles tree and the notes file. A file
    outside them is the artifact registry's or the loader's to judge."""
    return name == "notes.json" or name.startswith("cycles/")


def _claiming_row(name: str) -> dict | None:
    """The row of :func:`record_kinds` that claims ``name``, if any."""
    for row in record_kinds():
        if row["claim"](name):
            return row
    return None


def _record_problem(root: Path, name: str) -> list[str] | None:
    """Problems of a tracked record file, judged by the row that claims it;
    ``None`` when no validator claims the file. The caller turns ``None`` on
    a record-folder file into a refusal naming the file (finding
    S3B-records-2). Every other changed file — the artifacts ``check_paths``
    already judged, the tracked ``.gitignore``, the machine records under
    ``local/`` — is left alone here; ``factory.toml`` is the loader's."""
    row = _claiming_row(name)
    if row is None:
        return None
    problems: list[str] = []
    record = None
    if row["parse"]:
        try:
            record = read_record(root / ".factory" / name)
        except StoreCorrupt as exc:
            return [f"corrupt: {exc}"]
    if record is not None and row["schema"] is not None:
        problems.extend(_schema_problems(record, row["schema"]))
    if row["cap"]:
        problems.extend(_cap_problems(root, name, row["cap"]))
    return problems


def _cap_problems(root: Path, name: str, cap: int) -> list[str]:
    size = (root / ".factory" / name).stat().st_size
    if size > cap:
        return [f"over_cap: {name} is {size} bytes; the cap is {cap}"]
    return []


def _schema_problems(record: dict, schema: str) -> list[str]:
    from ..schema import validate
    from .store import schema_of

    return [str(error) for error in validate(record, schema_of(schema))[:5]]


def _live_lock_run(root: Path) -> str:
    """The one live lock's run, for the ``Run:`` trailer. Flush never raises,
    so a corrupt lock.json or run.json is no live run here, not an error."""
    from .locks import lock_entries
    from .store import StoreCorrupt

    try:
        entries = lock_entries(root)
    except StoreCorrupt:
        return ""
    for entry in entries:
        try:
            record = read_record(
                root / ".factory" / "local" / "runs" / entry.get("run", "") / "run.json"
            )
        except StoreCorrupt:
            continue
        if record is not None and record.get("ended", "") == "":
            return entry.get("run", "")
    return ""


def _integration_head(run_git, root: Path) -> str:
    from ..config import ConfigError, load as load_config
    from .gitrun import GitError

    try:
        config = load_config(root / ".factory" / "factory.toml")
        data = getattr(config, "data", config)
        branches = (data.get("branches") or {}) if isinstance(data, dict) else {}
        integration = branches.get("integration", "develop")
        return run_git(root, "rev-parse", "--short=12", integration)
    except (GitError, ConfigError):
        return "-"


def _message(reason: str, trailer_run: str, integration_head: str) -> str:
    lines = [f"factory: {reason}", "", f"Run: {trailer_run or '-'}"]
    lines.append(f"Code-Head: {integration_head or '-'}")
    return "\n".join(lines)
