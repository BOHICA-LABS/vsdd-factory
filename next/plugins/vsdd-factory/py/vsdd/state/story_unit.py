"""The story unit and the story's position (topics/story-life.md, topics/
story-script.md; plan survey.md 3.5, decisions D6, D14, D15, D16).

The position is computed from records and git, never from what a script
reports: the last position of ``topics/story-life.md`` whose condition holds
for the story and for every position before it. ``delivered`` is read from
the closure alone, before any evidence is touched — delivery removes the
worktree, and the evidence lookup answers ``None`` without one.

The unit carries the fields the story script reads as ``K.*``, resolved from
the story's ``[[story-kind]]`` row: the agents and methods of its tests and
implement stages and whether the kind has a red gate.
"""

from __future__ import annotations

import re
from pathlib import Path

from .store import root_of

__all__ = ["story_kind", "kind_has_red_gate"]  # the facade lists only state-api names

_POSITIONS = ("ready", "red", "green", "reviewed", "evidenced", "delivered")

_AC_RE = re.compile(r"^#{2,6}\s+(AC-\d+)\b")

_REVIEW_RULES_REL = ".factory/specs/architecture/review-rules.md"


def _load_config(root: Path):
    from ..config import load

    return load(root / ".factory" / "factory.toml")


def _story_meta(root: Path, story: str) -> tuple[dict, str]:
    """The story file's frontmatter and its project-relative path; ``({}, "")``
    when there is no story file — the position degrades, admission refuses."""
    from .artifacts import ArtifactError, load_artifact

    try:
        artifact = load_artifact(root, story)
    except ArtifactError:
        return {}, ""
    if artifact.kind != "story":
        return {}, ""
    return dict(artifact.frontmatter), artifact.rel


def _kind_rows() -> list[dict]:
    from ..policy import load as load_policy

    rows = load_policy("artifacts").get("story-kind", [])
    return [row for row in rows if isinstance(row, dict)]


def _kind_row(kind: str) -> dict:
    for row in _kind_rows():
        if row.get("id") == kind:
            return row
    return {}


def story_kind(root, story: str) -> str:
    """The story file's ``kind``; empty without a story file or a kind."""
    meta, _ = _story_meta(Path(root_of(root)), story)
    return str(meta.get("kind", "") or "")


def kind_has_red_gate(kind: str) -> bool:
    """Whether the kind's ``[[story-kind]]`` row carries a red gate. A kind
    with no row reads as False; admission refuses such a kind first."""
    return bool(_kind_row(kind).get("red_gate", False))


def position(root, cycle: str, story: str) -> str:
    """The story's position, by the ladder of ``topics/story-life.md``.

    ``delivered`` whenever the closure holds ``delivery.merged_sha`` (D6):
    delivery removes the worktree and the evidence lookup answers ``None``
    without one. A kind with no red gate is at ``red`` at once, and a story
    whose ``demo`` is false is ``evidenced`` without demo evidence.
    """
    root = root_of(root)
    from .cycles import closure_read

    closure = closure_read(root, cycle, "story", story) or {}
    if (closure.get("delivery") or {}).get("merged_sha"):
        return "delivered"
    from . import worktrees

    tree = worktrees.story_path(root, story)
    base = (closure.get("delivery") or {}).get("base", "")
    if not base or not (tree / ".git").exists():
        return "ready"
    kind = story_kind(root, story)
    if kind_has_red_gate(kind):
        if _gate(root, "red", story) is None:
            return "ready"
    if _gate(root, "green", story) is None:
        return "red"
    if _gate(root, "review", story) is None:
        return "green"
    meta, _ = _story_meta(root, story)
    if meta.get("demo"):
        if _gate(root, "demo", story) is None:
            return "reviewed"
    return "evidenced"


def _gate(root: Path, gate: str, story: str):
    """The gate's evidence for the story, or ``None`` when nothing covers
    the head. Only the absence of evidence is ``None``: a corrupt record or
    a faulty registered gate rises, and the command that computes the
    position reports it (a silent ``None`` would rerun finished stages)."""
    from .gate_evidence import evidence

    return evidence(root, gate, story)


# ------------------------------------------------------------------ build ---


def build(root, cycle: str, story: str) -> dict:
    """The story unit, valid against ``story-unit.schema.json``."""
    root = root_of(root)
    from .cycles import closure_read

    closure = closure_read(root, cycle, "story", story) or {}
    delivery = closure.get("delivery") or {}
    config = _load_config(root)
    from . import worktrees

    branch = worktrees.branch_name(config, story)
    tree = worktrees.story_path(root, story)
    meta, rel = _story_meta(root, story)
    kind = str(meta.get("kind", "") or "")
    row = _kind_row(kind)
    if (tree / ".git").exists():
        from . import gitrun

        head = gitrun.git(tree, "rev-parse", "HEAD")
        dirty = bool(gitrun.git(tree, "status", "--porcelain").strip())
    else:
        head = str(delivery.get("merged_sha", "") or delivery.get("head", ""))
        dirty = False
    verdict = closure.get("verdict") or {}
    passes = verdict.get("passes", 0)
    green = _gate(root, "green", story) or {}
    unit = {
        "id": story,
        "position": position(root, cycle, story),
        "worktree": str(tree),
        "tree": str(tree),
        "branch": branch,
        "base": str(delivery.get("base", "")),
        "head": head,
        "story_path": str(root / rel) if rel else "",
        "bcs": [str(item) for item in (meta.get("contracts") or [])],
        "acs": acs(_body(root, rel)),
        "criticality": str(meta.get("criticality", "")),
        "kind": kind,
        "tests_agent": str(row.get("tests_agent", "") or ""),
        "tests_method": _method(meta, row, "tests_method", "tests_method",
                                has_stage=bool(row.get("tests_agent", ""))),
        "impl_agent": str(row.get("implement_agent", "") or ""),
        "impl_method": _method(meta, row, "impl_method", "implement_method", has_stage=True),
        "red_gate": bool(row.get("red_gate", False)),
        "repo": str(meta.get("repo", "") or ""),
        "security": bool(meta.get("security", False)),
        "ui": bool(meta.get("ui", False)),
        "demo_required": bool(meta.get("demo", False)),
        "waived": [],
        "guidance": "",
        "deferred": [],
        "diff_patch": str(green.get("diff_patch", "") or ""),
        "test_output": _test_output(root, green),
        "review_rules": _review_rules(root),
        "dirty": dirty,
        "open_pr": "",
        "rounds_used": passes if isinstance(passes, int) and not isinstance(passes, bool) else 0,
        "blocked": {"kind": "", "decision": "", "open": False},
        "fixes": [str(meta["source"])] if meta.get("source") else [],
    }
    _validate(unit)
    return unit


def _method(meta: dict, row: dict, story_key: str, row_key: str, *, has_stage: bool) -> str:
    """The story's method override, else the kind row's; empty when the kind
    has no such stage (decision D14)."""
    if not has_stage:
        return ""
    return str(meta.get(story_key) or row.get(row_key, "") or "")


def _body(root: Path, rel: str) -> str:
    if not rel:
        return ""
    try:
        return (root / rel).read_text(encoding="utf-8")
    except OSError:
        return ""


def acs(body: str) -> list[str]:
    """Criterion ids (``AC-1``) read from the ``### AC-N`` headings outside
    code fences, because the verdict compares them as items (decision D15).
    The one reader of its kind: the verdict's ``_story_acs`` calls this, so
    the two cannot drift (S4R-verdicts-7)."""
    found: list[str] = []
    fence = None
    for line in body.splitlines():
        stripped = line.strip()
        if fence is None:
            if stripped.startswith("```") or stripped.startswith("~~~"):
                fence = stripped[:3]
                continue
            match = _AC_RE.match(stripped)
            if match:
                found.append(match.group(1))
        elif stripped.startswith(fence):
            fence = None
    return found


def _test_output(root: Path, green: dict) -> str:
    """The raw output beside the green record that covers the head; empty
    until it does (decision D16)."""
    sha = str(green.get("sha", "")) if green else ""
    if not sha or len(sha) < 12:
        return ""
    from .gate_evidence import evidence_dir

    path = evidence_dir(root, str(green.get("subject", ""))) / f"green.{sha[:12]}.log"
    return str(path) if path.is_file() else ""


def _review_rules(root: Path) -> str:
    path = root / _REVIEW_RULES_REL
    return str(path) if path.is_file() else ""


def _validate(unit: dict) -> None:
    from ..schema import validate
    from .store import schema_of

    problems = validate(unit, schema_of("story-unit"))
    if problems:  # a malformed unit must stop its own builder, loudly
        raise ValueError(
            "story unit fails story-unit.schema.json: "
            + "; ".join(str(problem) for problem in problems[:5])
        )
