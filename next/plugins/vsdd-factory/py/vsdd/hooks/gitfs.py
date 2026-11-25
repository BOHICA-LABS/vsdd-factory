"""Git facts for the runner (plan 3.6; hooks.md 2.4).

Budgets: a rule may start at most one ``git`` subprocess, with a 1.5 s
timeout, and no rule makes a network call. Git facts that can be read from
files — ``HEAD``, worktree pointers, packed refs — are read from files here;
:data:`git` is the one subprocess helper, for the facts that need a process.
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import TYPE_CHECKING

from .. import project as _project

if TYPE_CHECKING:  # pragma: no cover - typing only
    from .model import Ctx

__all__ = ["worktree_root", "branch", "head_sha", "git"]

GIT_TIMEOUT_SECONDS = 1.5


def worktree_root(path: str | Path) -> Path | None:
    """The nearest directory (upwards) that holds a ``.git``."""
    current = Path(os.path.abspath(path))
    if current.is_file():
        current = current.parent
    while True:
        if (current / ".git").exists():
            return current
        parent = current.parent
        if parent == current:
            return None
        current = parent


def branch(path: str | Path) -> str | None:
    """The checked-out branch of the worktree that holds ``path``; the commit
    SHA when detached (the project reader's own resolution)."""
    root = worktree_root(path)
    if root is None:
        return None
    return _project.read_branch(root)


def head_sha(path: str | Path) -> str | None:
    """The commit ``HEAD`` names, read from files: loose ref, packed ref, or
    the detached SHA itself. Branches live in the common directory when the
    checkout is a linked worktree, so a story worktree on a branch resolves
    too — the case the story runner works in."""
    root = worktree_root(path)
    if root is None:
        return None
    git_dir = _git_dir(root)
    if git_dir is None:
        return None
    try:
        head = (git_dir / "HEAD").read_text(encoding="utf-8").strip()
    except OSError:
        return None
    if not head.startswith("ref: "):
        return head or None
    ref = head[len("ref: ") :]
    for base in _ref_dirs(git_dir):
        loose = base / ref
        try:
            if loose.is_file():
                value = loose.read_text(encoding="utf-8").strip()
                if value:
                    return value
        except OSError:
            return None
    for base in _ref_dirs(git_dir):
        try:
            packed = (base / "packed-refs").read_text(encoding="utf-8")
        except OSError:
            continue
        for line in packed.splitlines():
            if line.startswith("^"):
                continue  # the peeled line of the preceding annotated tag
            if line.endswith(" " + ref):
                return line.split(" ", 1)[0]
    return None


def _ref_dirs(git_dir: Path) -> list[Path]:
    """The directories loose refs and ``packed-refs`` may live in: the git
    directory itself, and — for a linked worktree, whose branches are shared —
    the common directory its ``commondir`` file names."""
    directories = [git_dir]
    common = git_dir / "commondir"
    try:
        relative = common.read_text(encoding="utf-8").strip()
    except OSError:
        return directories
    if not relative:
        return directories
    target = Path(relative)
    resolved = target if target.is_absolute() else git_dir / target
    common_dir = Path(os.path.normpath(resolved))
    if common_dir not in directories:
        directories.append(common_dir)
    return directories


def _git_dir(root: Path) -> Path | None:
    """The git directory behind a checkout or linked worktree."""
    dot_git = root / ".git"
    try:
        if dot_git.is_file():
            content = dot_git.read_text(encoding="utf-8").strip()
            if not content.startswith("gitdir:"):
                return None
            gitdir = Path(content[len("gitdir:") :].strip())
            return gitdir if gitdir.is_absolute() else root / gitdir
        if dot_git.is_dir():
            return dot_git
    except OSError:
        return None
    return None


def git(ctx: "Ctx", args: tuple[str, ...] | list[str], cwd: str | Path) -> subprocess.CompletedProcess:
    """The one ``git`` subprocess a rule may start, at 1.5 s. A second call in
    one rule raises: the budget is the rule's, and the loop resets it."""
    if ctx.git_budget_used:
        raise RuntimeError(
            "a rule may start at most one git subprocess per call (hooks.md 2.4)"
        )
    ctx.git_budget_mark_used()
    return subprocess.run(
        ["git", *[str(argument) for argument in args]],
        cwd=str(cwd),
        capture_output=True,
        text=True,
        timeout=GIT_TIMEOUT_SECONDS,
        check=False,
    )
