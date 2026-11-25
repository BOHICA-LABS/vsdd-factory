"""Story admission as one library function (topics/story-admission.md; plan
survey.md 3.4, decisions D4, D5).

``admit`` is what the survey calls for the story it returns and what the
admission commands (``gate plan --story``, ``story new-fix``) call when they
are built: it checks the story file, requires a live run that holds the lock,
writes the closure, cuts the branch from the target head read at that moment
and creates the worktree. Whatever a killed call left, the next one
completes: the branch and the worktree are made by separate git commands and
each is skipped when it is already there.

``require_story`` is the check every gate and delivery repeats: the story is
in the run given, its worktree is on its branch and descends from the
recorded base.
"""

from __future__ import annotations

from .store import StoreRefused, root_of

__all__: list[str] = []  # the facade lists only state-api names


class AdmissionError(Exception):
    """An admission refusal; ``reason`` is the machine-readable cause."""

    def __init__(self, reason: str, message: str | None = None):
        super().__init__(message or reason)
        self.reason = reason
        self.message = message or reason


# -------------------------------------------------------------- admissible ---


def admissible(root, cycle: str, story: str) -> tuple[bool, str]:
    """The "story is admitted" predicate (decision D4): the story is in the
    open wave — the first wave, in id order, of the plan closure's ``waves``
    whose closure is not ``closed`` — or its own closure carries
    ``admitted_by``. A story in a later wave is not admitted."""
    root = root_of(root)
    from .cycles import closure_read

    closure = closure_read(root, cycle, "story", story)
    if closure is not None and closure.get("admitted_by"):
        return True, ""
    plan = closure_read(root, cycle, "plan", "")
    waves = (plan or {}).get("waves") or {}
    if not isinstance(waves, dict) or not waves:
        return False, "no plan closure places the story in a wave"
    for name in sorted(waves):
        if _wave_closed(root, cycle, str(name)):
            continue
        stories = waves[name]
        if isinstance(stories, list) and story in stories:
            return True, ""
        return False, f"story {story} is not in the open wave {name}"
    return False, f"every wave of {cycle} is closed"


def _wave_closed(root, cycle: str, name: str) -> bool:
    from .cycles import closure_read

    closure = closure_read(root, cycle, "wave", str(name))
    return closure is not None and closure.get("state") == "closed"


def open_wave(root, cycle: str) -> str:
    """The id of the first wave of the plan closure that is not closed; empty
    when every wave is closed or the closure has no waves."""
    root = root_of(root)
    from .cycles import closure_read

    waves = ((closure_read(root, cycle, "plan", "") or {}).get("waves")) or {}
    for name in sorted(waves):
        if not _wave_closed(root, cycle, str(name)):
            return str(name)
    return ""


# --------------------------------------------------------------------- admit ---


def admit(root, *, cycle: str, story: str, run: str, admitted_by: str) -> dict:
    """Admit ``story`` to ``run``: check it, write the closure, create the
    branch and worktree, record it in the run. Returns the story unit.

    Run again it changes nothing, except that it adds the story to a new run
    (findings P3REQ-07, P3OPS-07). For a delivered story it creates nothing.
    Raises :class:`AdmissionError` or :class:`WorktreeError`; whatever a
    failed call left is completed by the next call.
    """
    root = root_of(root)
    from . import worktrees
    from .cycles import closure_read, closure_update
    from .locks import lock_entries
    from .runs import run_admit, run_require
    from .store import now as now_stamp

    _check_story_file(root, story)
    try:
        run_require(root, run)  # live: run_unknown, run_ended
    except StoreRefused as exc:
        raise AdmissionError(exc.code, str(exc)) from exc
    if not any(entry.get("run", "") == run for entry in lock_entries(root)):
        raise AdmissionError(
            "not_holder", f"run {run!r} holds no lock; run the survey again"
        )

    from .story_unit import build

    closure = closure_read(root, cycle, "story", story)
    if closure is not None and (closure.get("delivery") or {}).get("merged_sha"):
        run_admit(root, run, story)  # the story joins the new run; nothing else
        return build(root, cycle, story)

    config = _config(root)
    story_branch = worktrees.branch_name(config, story)
    delivery = (closure or {}).get("delivery") or {}
    base = str(delivery.get("base", ""))
    branch_missing = not worktrees.branch_exists(root, story_branch)
    if closure is None or not base or branch_missing or not delivery.get("branch"):
        target_branch, base = worktrees.target_head(root, config)
        if closure is None:
            stamp = now_stamp()

            def fresh(record: dict) -> None:
                record["position"] = "ready"
                record["admitted_by"] = admitted_by
                record.setdefault("delivery", {})["run"] = run
                record.setdefault("delivery", {})["at"] = stamp

            closure_update(root, cycle, "story", story, fresh)
        record_base(root, cycle, story, target_branch, base)
    worktrees.create_story(root, story, story_branch, base)
    run_admit(root, run, story)
    return build(root, cycle, story)


def _check_story_file(root, story: str) -> None:
    """The story file: exists, ``id`` matches, ``status: draft``, a known
    ``kind``, a valid ``criticality`` (plan survey.md 3.4)."""
    from .artifacts import ArtifactError, load_artifact

    try:
        artifact = load_artifact(root, story)
    except ArtifactError as exc:
        raise AdmissionError("missing_story", str(exc)) from exc
    if artifact.kind != "story":
        raise AdmissionError("missing_story", f"{story!r} is not a story artifact")
    meta = artifact.frontmatter
    ident = str(meta.get("id", "") or artifact.id)
    if ident != story:
        raise AdmissionError("bad_id", f"the story file says id {ident!r}, not {story!r}")
    if str(meta.get("status", "")) != "draft":
        raise AdmissionError("not_draft", f"story {story} is {meta.get('status')!r}, not draft")
    kind = str(meta.get("kind", ""))
    from .story_unit import _kind_row, _kind_rows

    if not kind or not _kind_rows() or not _kind_row(kind):
        raise AdmissionError("unknown_kind", f"{kind!r} is not a known story kind")
    if str(meta.get("criticality", "")) not in _criticalities():
        raise AdmissionError(
            "bad_criticality", f"criticality {meta.get('criticality')!r} is not valid"
        )


def _criticalities() -> list[str]:
    """The criticality values the shipped story frontmatter allows (the
    ``[story.frontmatter]`` row of the artifacts table) — read once here, so
    the enum is not copied into code."""
    from ..policy import load as load_policy

    table = load_policy("artifacts").get("story", {})
    frontmatter = table.get("frontmatter", {}) if isinstance(table, dict) else {}
    allowed = frontmatter.get("criticality", ()) if isinstance(frontmatter, dict) else ()
    return [str(item) for item in allowed] if isinstance(allowed, (list, tuple)) else []


def _config(root):
    from ..config import load

    return load(root / ".factory" / "factory.toml")


def record_base(root, cycle: str, story: str, branch: str, base: str) -> None:
    """The one writer of ``delivery.branch`` and ``delivery.base`` (D5);
    delivery calls it after a rebase (VSA-01)."""
    from .cycles import closure_update

    def mutate(record: dict) -> None:
        delivery = record.setdefault("delivery", {})
        delivery["branch"] = branch
        delivery["base"] = base

    closure_update(root, cycle, "story", story, mutate)


# ---------------------------------------------------------- require_story ---


def require_story(root, run: str, story: str) -> dict:
    """The check of ``commands/story-deliver.md``, first bullet: the story is
    in the live run given, and its worktree verifies against the recorded
    branch and base. Returns ``{cycle, closure, worktree, branch, target,
    base, head}``; raises ``AdmissionError`` or ``WorktreeError``."""
    root = root_of(root)
    from . import worktrees
    from .cycles import closure_read
    from .runs import run_require

    try:
        record = run_require(root, run, story=story)
    except StoreRefused as exc:
        raise AdmissionError(exc.code, str(exc)) from exc
    cycle = record.get("cycle", "")
    closure = closure_read(root, cycle, "story", story)
    if closure is None:
        raise AdmissionError("no_closure", f"story {story} has no closure in {cycle}")
    delivery = closure.get("delivery") or {}
    base = str(delivery.get("base", ""))
    if not base:
        raise AdmissionError("no_base", f"story {story} has no recorded base")
    config = _config(root)
    branch = worktrees.branch_name(config, story)
    tree = worktrees.story_path(root, story)
    head = worktrees.verify_story(root, story, branch, base)
    return {
        "cycle": cycle,
        "closure": closure,
        "worktree": tree,
        "branch": branch,
        "target": str(delivery.get("branch", "")),
        "base": base,
        "head": head,
    }
