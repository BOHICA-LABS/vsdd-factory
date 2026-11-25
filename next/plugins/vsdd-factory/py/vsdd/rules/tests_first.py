"""The tests-first rule (rules/tests-first.md; plan rules.md 3.2).

Delivery-order gate H5: in a story, product source is written only once a
passing red record exists, the test writer's ``source:before-red`` grant is
spent once it does, and the tests frozen at the red gate stay frozen for
every other caller. One module, ``check`` and nothing else; the state modules
import inside ``check`` (decision D1), so the runner's import stays cheap and
a missing state module is the rule's own crash.
"""

from __future__ import annotations

import os
from pathlib import Path

from ..hooks.writes import write_targets, write_unknown

__all__: list[str] = []

_FILE_TOOLS = ("Edit", "Write", "MultiEdit", "NotebookEdit")


def check(ctx, cfg):
    """Every path the call writes, held to the evidence the story has. The
    per-story lookups (the kind's red gate, the red record) are taken once
    per story: forty written paths are not forty record reads
    (S4R-rules-19). A write the parse cannot read — a writer behind an
    opaque segment — is judged as the edit it could be, where a frozen test
    or product source could be the target (S4R-rules-7)."""
    facts: dict[str, tuple] = {}
    for path in _targets(ctx):
        block = _one_path(ctx, path, facts)
        if block is not None:
            return block
    if ctx.tool == "Bash":
        command = ctx.command
        if command is not None:
            for segment in command.segments:
                if not write_targets(segment) and write_unknown(segment):
                    block = _unreadable(ctx, segment, facts)
                    if block is not None:
                        return block
    return None


def _targets(ctx):
    """The absolute paths this call writes: the file tool's target, or every
    shell write of every segment (decision D5)."""
    if ctx.tool != "Bash":
        target = ctx.target
        return (target,) if target is not None else ()
    command = ctx.command
    if command is None:
        return ()
    found: list[Path] = []
    for segment in command.segments:
        for path in write_targets(segment):
            found.append(Path(path))
    return found


def _unreadable(ctx, segment, facts):
    """The verdict for a write whose target the parse cannot give: the edit
    it could be, judged at the segment's own directory (S4R-rules-7). A
    segment whose text holds a cd — a subshell, a ``sh -c`` — runs somewhere
    its own directory does not name, so its write is judged in an unknown
    place: any story of the governing project may be the target
    (step4-rules-fix-3-C1-9)."""
    story = ctx.story_at(segment.where)
    if not story:
        if not _unstuck(segment):
            return None  # the rule applies inside a story only
        story = _frozen_story(ctx, facts)
        if not story:
            return None
    known = facts.get(story)
    if known is None:
        known = (_has_red_gate(ctx, story), _red_record(ctx, story))
        facts[story] = known
    has_red_gate, record = known
    if not has_red_gate:
        return None
    where = segment.where or "a place the command's cd chooses"
    if record is None:
        if "source:before-red" in _grants(ctx):
            return None
        return _block(
            "no_red_evidence",
            f"a write the parse cannot read (in {where}) could make product "
            f"source in story {story}, and no passing red record exists yet.",
            f"Run vsdd gate red --story {story}; run the write by itself, "
            "plainly, so the rule can read it.",
        )
    if "tests:frozen" in _grants(ctx):
        return None
    return _block(
        "red_test_frozen",
        f"a write the parse cannot read (in {where}) could change a test "
        f"frozen by the red record of story {story}; the red gate measured "
        "these tests, and another builder changing them voids the evidence.",
        "Run the write by itself, plainly, so the rule can read it; return "
        "blocked with data.test_defects to change a frozen test.",
    )


def _unstuck(segment) -> bool:
    """Whether the segment's own text moves it elsewhere: a ``cd``,
    ``pushd`` or ``popd`` word, or a directory the parse cannot give at
    all (step4-rules-fix-3-C1-9)."""
    if not segment.where:
        return True
    return any(
        word in ("cd", "pushd", "popd") for word in (segment.text or "").split()
    )


def _frozen_story(ctx, facts):
    """A story of the governing project whose passing red record froze
    tests: the story a write in an unknown place could hit, taken once per
    call (step4-rules-fix-3-C1-9)."""
    known = facts.get(__frozen_key)
    if known is None:
        known = _frozen_stories(ctx)
        facts[__frozen_key] = known
    return known[0] if known else None


__frozen_key = " the stories of the project with frozen tests"


def _frozen_stories(ctx) -> tuple[str, ...]:
    from ..state import gate_evidence
    from ..state.store import root_of

    project = ctx.project
    if project is None:
        return ()
    try:
        base = root_of(project) / ".factory" / "local" / "evidence"
        if not base.is_dir():
            return ()
        found = []
        for directory in sorted(base.iterdir()):
            if not directory.is_dir():
                continue
            try:
                record = gate_evidence.evidence(project, "red", directory.name)
            except Exception:
                continue
            if record and record.get("passed") and record.get("frozen"):
                found.append(directory.name)
        return tuple(found)
    except Exception:
        return ()


def _one_path(ctx, path: Path, facts: dict | None = None):
    """The verdict for one written path, at its first ``no``; ``None`` passes."""
    story = ctx.story_at(path.parent)
    if not story:
        return None  # the rule applies inside a story only
    if facts is None:
        facts = {}
    known = facts.get(story)
    if known is None:
        known = (_has_red_gate(ctx, story), _red_record(ctx, story))
        facts[story] = known
    has_red_gate, record = known
    if not has_red_gate:
        return None  # the story's kind has no red gate
    if _config_invalid(ctx):
        # The path classes fell back to empty with the configuration: an
        # unjudged write inside a story worktree is not a pass
        # (S4R-rules-10).
        return _block(
            "config_invalid",
            f"{path} in story {story} cannot be classified ({ctx.project.config_error}); "
            "the configuration fell back to defaults with no path classes.",
            "Fix .factory/factory.toml; the rule blocks until the file parses.",
        )
    classes = ctx.classes_of(path)
    if "source" not in classes and "tests" not in classes:
        return None

    if "source" in classes:
        before_red = "source:before-red" in _grants(ctx)
        if record is None:
            if not before_red:
                return _block(
                    "no_red_evidence",
                    f"{path} in story {story} is product source, and no passing red "
                    "record for the story exists yet.",
                    f"Run vsdd gate red --story {story}; write source once it passes.",
                )
        elif before_red:
            return _block(
                "red_already_recorded",
                f"{path} in story {story} is product source, and a passing red record "
                "already exists: the before-red grant is spent.",
                f"Run vsdd gate red --story {story} again after the change; the "
                "implementer writes source from here on.",
            )
        return None

    return _frozen_block(ctx, path, story, record)


def _config_invalid(ctx) -> bool:
    """Whether the governing project's ``factory.toml`` did not parse, so the
    path classes are the empty defaults."""
    project = ctx.project
    if project is None:
        return False
    try:
        return project.config_error is not None
    except Exception:
        return False


def _frozen_block(ctx, path: Path, story: str, record):
    """A frozen test stays frozen for every caller without ``tests:frozen``."""
    if record is None or "tests:frozen" in _grants(ctx):
        return None
    if not _is_frozen(path, record):
        return None
    return _block(
        "red_test_frozen",
        f"{path} is frozen by the red record of story {story}; the red gate measured "
        "these tests, and another builder changing them voids the evidence.",
        f"Return blocked again, with data.test_defects naming {path}.",
    )


def _is_frozen(path: Path, record) -> bool:
    """The path, literal or resolved, relative to its own worktree root
    (POSIX), against the record's frozen set. Each form is measured against
    the root its own spelling gives: a symlinked parent (macOS ``/tmp``,
    a linked home) leaves the resolved form in a different spelling of the
    same worktree, and one form's failure is not the other's
    (S4R-rules-2)."""
    frozen = {
        entry.get("file", "")
        for entry in (record.get("frozen") or [])
        if isinstance(entry, dict)
    }
    if not frozen:
        return False
    from ..hooks import gitfs

    relatives = []
    for form in (path, Path(os.path.realpath(path))):
        root = gitfs.worktree_root(form)
        if root is None:
            continue
        try:
            relatives.append(form.relative_to(root).as_posix())
        except ValueError:
            continue  # this form sits in no worktree root; the other may
    return any(relative in frozen for relative in relatives)


def _grants(ctx) -> tuple[str, ...]:
    """The caller's write grants: what the wall table allows this row."""
    write = ctx.row.get("write", [])
    if isinstance(write, str):
        return (write,)
    return tuple(write)


def _has_red_gate(ctx, story: str) -> bool:
    """Whether the story's kind has a red gate. A kind that cannot be read
    counts as having one, and the fault surfaces at the red gate instead of
    the rule crashing (decision D4)."""
    from ..state import story_unit

    try:
        kind = story_unit.story_kind(ctx.project, story)
    except Exception:
        return True
    if not kind:
        return True
    return story_unit.kind_has_red_gate(kind)


def _red_record(ctx, story: str):
    """The passing red record for the story, on any commit of its branch
    since its base. A corrupt record raises: the rule fails closed."""
    from ..state import gate_evidence

    return gate_evidence.evidence(ctx.project, "red", story)


def _block(code: str, reason: str, fix: str):
    from ..hooks.model import Block

    return Block(code, reason, fix)
