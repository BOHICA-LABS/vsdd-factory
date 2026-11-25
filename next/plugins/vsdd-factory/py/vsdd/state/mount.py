"""The state branch and the five project conditions (topics/state-directory.md).

State lives on the branch ``factory-artifacts``, mounted as a git worktree at
``<project root>/.factory``. Everything here reads files: no subprocess, so
:func:`detect` works wherever ``git`` is not on ``PATH`` and never raises.
The five conditions — ``none``, ``legacy``, ``ready``, ``partial``,
``unmounted`` — are the ones every design uses; ``partial`` has one cause, a
broken mount.
"""

from __future__ import annotations

import os
import shutil
from pathlib import Path

from .gitrun import GitError, git

__all__ = ["detect"]  # the facade publishes the state-api names only

STATE_BRANCH = "factory-artifacts"

_HEAD_LINE = f"ref: refs/heads/{STATE_BRANCH}"


def _read(path: Path) -> str | None:
    try:
        return path.read_text(encoding="utf-8")
    except OSError:
        return None


def _resolve_gitdir(git_path: Path) -> Path | None:
    """The git directory a ``.git`` file points at (a linked worktree)."""
    content = _read(git_path)
    if content is None or not content.startswith("gitdir:"):
        return None
    target = Path(content[len("gitdir:") :].strip())
    if not target.is_absolute():
        target = git_path.parent / target
    return target


def _git_dirs(root: Path) -> list[Path]:
    """The repositories whose refs belong to ``root``: the ``.git`` directory
    itself, or a linked worktree's gitdir plus its common directory."""
    git_path = root / ".git"
    dirs: list[Path] = []
    if git_path.is_dir():
        dirs.append(git_path)
    elif git_path.is_file():
        gitdir = _resolve_gitdir(git_path)
        if gitdir is not None:
            dirs.append(gitdir)
            common = _read(gitdir / "commondir")
            if common is not None and common.strip():
                candidate = Path(common.strip())
                if not candidate.is_absolute():
                    candidate = gitdir / candidate
                dirs.append(candidate)
    return dirs


def _ref_exists(root: Path, ref: str) -> bool:
    """A ref is its loose file or a line of ``packed-refs``: file reads only."""
    for gitdir in _git_dirs(root):
        loose = gitdir / ref
        if loose.is_file():
            return True
        packed = _read(gitdir / "packed-refs")
        if packed is not None:
            for line in packed.splitlines():
                line = line.strip()
                if not line or line.startswith("#") or line.startswith("^"):
                    continue
                parts = line.split()
                if len(parts) == 2 and parts[1] == ref:
                    return True
    return False


def _mount_head_ok(root: Path) -> bool:
    """The mount test: ``.factory/.git`` points at a worktree whose ``HEAD``
    is ``refs/heads/factory-artifacts``."""
    pointer = _read(root / ".factory" / ".git")
    if pointer is None or not pointer.startswith("gitdir:"):
        return False
    gitdir = Path(pointer[len("gitdir:") :].strip())
    if not gitdir.is_absolute():
        gitdir = root / ".factory" / gitdir
    head = _read(gitdir / "HEAD")
    return head is not None and head.strip() == _HEAD_LINE


def detect(root) -> str:
    """One of the five conditions of topics/state-directory.md, by file reads
    only; never raises."""
    try:
        root = Path(root)
        state = root / ".factory"
        config_file = state / "factory.toml"
        legacy_file = state / "STATE.md"
        if state.is_dir():
            if config_file.is_file():
                return "ready" if _mount_head_ok(root) else "partial"
            if legacy_file.is_file():
                return "legacy"
        branch_present = _ref_exists(root, f"refs/heads/{STATE_BRANCH}") or _ref_exists(
            root, f"refs/remotes/origin/{STATE_BRANCH}"
        )
        return "unmounted" if branch_present else "none"
    except Exception:  # a broken installation never makes detect raise
        return "none"


def mounted(root) -> bool:
    """True when the state worktree is mounted and on the state branch."""
    return _mount_head_ok(Path(root))


def ensure_excludes(root) -> None:
    """Put ``.factory/`` and ``.worktrees/`` into ``.git/info/exclude``.

    The file is rewritten atomically through a temporary file, and a missing
    trailing newline on the person's own lines is repaired first, so an
    appended pattern can never glue onto the last one.
    """
    root = Path(root)
    info = root / ".git" / "info"
    info.mkdir(parents=True, exist_ok=True)
    path = info / "exclude"
    current = _read(path) or ""
    lines = current.splitlines()
    wanted = []
    for directory in (".factory", ".worktrees"):
        line = f"{directory}/"
        if line not in lines:
            wanted.append(line)
    if wanted:
        text = current
        if text and not text.endswith("\n"):
            text += "\n"  # '*.log' must never become '*.log.factory/'
        text += "\n".join(wanted) + "\n"
        temporary = info / "exclude.vsdd-tmp"
        temporary.write_text(text, encoding="utf-8")
        os.replace(temporary, path)


def _is_interrupted_mount(state: Path) -> bool:
    """A ``.factory`` holding only the worktree's own ``.git`` pointer: an
    interrupted ``worktree add``, safe to build again from scratch.

    The pointer must be a regular file — a ``.git`` directory is a repository
    of its own and is never removed — whose gitdir is a worktree registration
    of this repository (under the common directory's ``worktrees/``) and
    points back at this ``.factory``. Anything else is somebody's data, and
    the caller must refuse rather than clear it (finding
    S3R-state-core-C1-1: a nested repository lost its history to the rmtree).
    """
    try:
        if sorted(entry.name for entry in state.iterdir()) != [".git"]:
            return False
    except OSError:
        return False
    pointer = state / ".git"
    if not pointer.is_file():
        return False
    gitdir = _resolve_gitdir(pointer)
    if gitdir is None or not gitdir.is_dir():
        return False
    # The registration's own gitdir file must point back at this .factory.
    back = _read(gitdir / "gitdir")
    if back is None:
        return False
    target = Path(back.strip())
    if not target.is_absolute():
        target = gitdir / target
    try:
        if target.resolve() != pointer.resolve():
            return False
        # And the registration must belong to this repository: the gitdir
        # sits under the common directory's worktrees/.
        common = _read(gitdir / "commondir")
        common_dir = Path(common.strip()) if common and common.strip() else gitdir.parent
        if not common_dir.is_absolute():
            common_dir = gitdir / common_dir
        if common_dir.resolve() != (state.parent / ".git").resolve():
            return False
        return gitdir.resolve().parent == (common_dir / "worktrees").resolve()
    except OSError:
        return False


def mount_state(root) -> None:
    """Mount the existing state branch (local, else ``origin``'s) at
    ``<root>/.factory``; raises ``GitError``.

    A ``worktree add`` killed halfway leaves a registered worktree whose
    directory is missing or holds only the ``.git`` pointer; ``prune`` and the
    interrupted-mount check let the next run finish the job instead of failing
    with a missing-but-already-registered worktree.
    """
    root = Path(root)
    target = root / ".factory"
    # An interrupted mount holds only its own .git pointer; clear it first so
    # prune sees a missing worktree, then let prune drop the registration.
    if target.is_dir() and _is_interrupted_mount(target):
        shutil.rmtree(target)
    git(root, "worktree", "prune")
    if _ref_exists(root, f"refs/heads/{STATE_BRANCH}"):
        git(root, "worktree", "add", target, STATE_BRANCH)
        return
    if _ref_exists(root, f"refs/remotes/origin/{STATE_BRANCH}"):
        git(
            root,
            "worktree",
            "add",
            "--track",
            "-b",
            STATE_BRANCH,
            target,
            f"origin/{STATE_BRANCH}",
        )
        return
    raise GitError(f"no {STATE_BRANCH} branch exists locally or on origin", 1, "")
