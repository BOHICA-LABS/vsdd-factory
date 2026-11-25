"""The git layer of the worktree library (topics/worktrees.md; plan survey.md
3.3, decisions D5, D17, D20 and D33-13).

One creator of story worktrees, so paths cannot disagree. This module holds
only git: no record access. The base a story branch is cut from is read by
:func:`target_head` and recorded by ``vsdd.state.admission.record_base``, so
the descent checks everywhere use a recorded commit, never a branch name.

Branch and worktree are made by separate commands (D20): a refused
``worktree add -b`` would leave a branch whose retry fails, while a branch
made alone is found again by the next call, which then adds the worktree. The
branch is created at a commit id and never from a remote-tracking name, so
git gives it no upstream by accident (decision D33-25).

With the Bash sandbox on, a git call cannot write under ``.git/worktrees/``
(probe of 2026-11-24, ``topics/platform-proven-2.md`` item 41). Such a
failure raises :class:`WorktreeError` of code ``not_permitted`` with an error
that holds the word ``sandbox`` and git's own line, and undoes nothing: the
next call finishes from what the refused one left.
"""

from __future__ import annotations

from contextlib import contextmanager
from pathlib import Path

from .store import root_of, safe_segment

__all__ = ["unit_tree"]  # the facade lists only state-api names

CODES = ("git", "occupied", "branch_mismatch", "not_descendant", "missing", "not_permitted")


class WorktreeError(Exception):
    """A worktree-library failure with a machine-readable code.

    Codes are :data:`CODES`: ``git`` (a git failure, configuration included),
    ``occupied`` (the story's path exists and is no usable worktree),
    ``branch_mismatch`` (the worktree is not on the story's branch),
    ``not_descendant`` (the branch does not descend from the base),
    ``missing`` (the worktree is gone) and ``not_permitted`` (the Bash
    sandbox refused a write under ``.git/worktrees/``; run outside it).
    """

    def __init__(self, code: str, message: str | None = None):
        super().__init__(message or code)
        self.code = code
        self.message = message or code


def _git(cwd, *args: str, check: bool = True, timeout: int = 60) -> str:
    """S1's git, with every failure classified (D20). Called through the
    module so a test can patch ``vsdd.state.gitrun.git``."""
    from . import gitrun

    try:
        return gitrun.git(cwd, *args, check=check, timeout=timeout)
    except gitrun.GitError as exc:
        raise _classified(str(exc), exc.stderr) from exc


def _classified(message: str, stderr: str) -> WorktreeError:
    # Only git's own refusal is the sandbox: the word may also sit in a path
    # argument of an ordinary failure (ORDER.md rule 11).
    if "operation not permitted" in (stderr or "").lower():
        return WorktreeError(
            "not_permitted",
            f"sandbox: {message} (the Bash sandbox refuses writes under .git/worktrees; "
            f"run the command outside the sandbox) git said: {stderr.strip()}",
        )
    return WorktreeError("git", f"{message} {stderr.strip()}".strip())


def _worktree_add(root, *args: str) -> str:
    """``git worktree add`` after a prune: a directory removed outside git
    (by hand, or by a kill inside ``worktree remove``) leaves its
    registration behind, and every later add refuses until then."""
    _git(root, "worktree", "prune")
    return _git(root, "worktree", "add", *args)


# -------------------------------------------------------------------- paths ---


def _base(root) -> Path:
    return root_of(root) / ".worktrees"


def story_path(root, story: str) -> Path:
    """``<root>/.worktrees/<story>`` — the path ``classify_worktree`` gives
    for the id, so the layout and this library cannot disagree. No git, so
    safe on the hook path. An id outside the story grammar refuses."""
    name = safe_segment(story)
    from ..project import classify_worktree

    classified = classify_worktree(name)
    if classified is None or classified.kind != "story":
        raise WorktreeError("missing", f"{story!r} is not a story id the worktree layout knows")
    return root_of(root) / classified.path


def integration_path(root) -> Path:
    """``<root>/.worktrees/_integration`` — the path ``classify_worktree``
    gives the integration entry."""
    from ..project import classify_worktree

    classified = classify_worktree("_integration")
    return root_of(root) / classified.path


def branch_name(config, story: str) -> str:
    """The story branch of ``story``, from ``[branches].story``
    (``story/{id}``): the pattern's literal prefix and the id."""
    data = getattr(config, "data", config)
    branches = data.get("branches", {}) if isinstance(data, dict) else {}
    pattern = branches.get("story", "story/{id}") if isinstance(branches, dict) else "story/{id}"
    prefix, _, _ = str(pattern).partition("{")
    return prefix + story


def _config(root: Path):
    """The project configuration, or ``None`` when it does not load; the
    branch names then fall back to the design defaults."""
    from ..config import load

    try:
        return load(root / ".factory" / "factory.toml")
    except Exception:
        return None


# ---------------------------------------------------------------- the target ---


def target_head(root, config) -> tuple[str, str]:
    """The branch a story is delivered to, and its commit now.

    ``[branches].integration`` in ``local`` mode; in ``pr`` mode the remote
    head after a fetch, the remote taken from the target branch's upstream
    (decision D33-13) — never a remote name in code.
    """
    root = root_of(root)
    data = getattr(config, "data", config)
    if not isinstance(data, dict):
        data = {}
    branches = data.get("branches", {}) if isinstance(data.get("branches", {}), dict) else {}
    integration = branches.get("integration", "develop")
    delivery = data.get("delivery", {}) if isinstance(data.get("delivery", {}), dict) else {}
    mode = delivery.get("mode", "pr")
    if mode != "pr":
        return integration, _git(root, "rev-parse", integration)
    return integration, _remote_head(root, integration)


def _remote_head(root: Path, branch: str) -> str:
    """Fetch ``refs/heads/<branch>`` from the branch's upstream and return the
    remote-tracking head (decision D33-13)."""
    line = _git(
        root,
        "for-each-ref",
        "--format=%(upstream:remotename) %(upstream:remoteref)",
        f"refs/heads/{branch}",
    ).strip()
    remote, _, ref = line.partition(" ")
    remote, ref = remote.strip(), ref.strip()
    want = f"refs/heads/{branch}"
    if remote:
        if ref != want:
            raise WorktreeError(
                "git",
                f"the upstream of {branch} is {ref or 'unset'}, not {want}; correct it "
                f"with: git branch --set-upstream-to={remote}/{branch} {branch}",
            )
    else:
        remotes = [name for name in _git(root, "remote").splitlines() if name.strip()]
        if len(remotes) != 1:
            raise WorktreeError(
                "git",
                f"the branch {branch} has no upstream and {len(remotes)} remotes; set the "
                f"upstream with: git branch --set-upstream-to=<remote>/{branch} {branch}",
            )
        remote = remotes[0]
    _git(root, "fetch", remote, f"{want}:refs/remotes/{remote}/{branch}")
    return _git(root, "rev-parse", f"refs/remotes/{remote}/{branch}")


# ----------------------------------------------------------------- helpers ---


def _checked_branch(path: Path) -> str | None:
    """The branch checked out at ``path``, or ``None`` when there is none."""
    from ..project import read_branch

    name = read_branch(path)
    if name is None:
        return None
    # A detached head reads back as the commit itself: not a branch.
    if len(name) == 40 and all(ch in "0123456789abcdef" for ch in name):
        return None
    return name


def _ref_exists(repo: Path, branch: str) -> bool:
    out = _git(repo, "rev-parse", "--verify", "--quiet", f"refs/heads/{branch}", check=False)
    return bool(out.strip())


def branch_exists(root, branch: str) -> bool:
    """Whether a local branch ``branch`` exists."""
    return _ref_exists(root_of(root), branch)


def is_ancestor(root, a: str, b: str) -> bool:
    """Whether commit ``a`` is an ancestor of (or equal to) ``b``."""
    from . import gitrun

    try:
        gitrun.git(root_of(root), "merge-base", "--is-ancestor", a, b)
        return True
    except gitrun.GitError:
        return False


def head(path) -> str:
    """The commit checked out at ``path``."""
    return _git(path, "rev-parse", "HEAD")


def is_dirty(path) -> bool:
    """Whether ``path``'s worktree holds uncommitted changes."""
    return bool(_git(path, "status", "--porcelain").strip())


# ----------------------------------------------------------- story branch ---


def _not_descendant(story: str, branch: str, base: str, *, with_worktree: bool) -> WorktreeError:
    """The ``not_descendant`` refusal of ``create_story``, with a remedy that
    works when run (finding S3R-survey-8): git refuses to delete a branch a
    worktree holds, so the worktree-standing case removes the worktree
    before ``git branch -D``."""
    commands = f"git branch -D {branch}"
    discards = "This discards the branch's commits"
    if with_worktree:
        commands = f"git worktree remove --force .worktrees/{story}; {commands}"
        discards += " and the worktree's uncommitted changes"
    return WorktreeError(
        "not_descendant",
        f"the branch {branch} does not descend from the base {base[:12]} it is recorded "
        f"against; correct it with: {commands}; then admit the story again, which cuts it "
        f"from the target head and records that base. {discards}.",
    )


def create_story(root, story: str, branch: str, base: str) -> Path:
    """Create the story's branch at exactly ``base`` and its worktree, each
    when absent and by its own git command (D20). Idempotent.

    Raises ``occupied`` (the path exists and is no worktree),
    ``branch_mismatch`` (the worktree is on another branch), ``not_descendant``
    (the branch does not descend from ``base``), ``git`` and
    ``not_permitted``. Undoes nothing: what a refused call left, the next one
    completes. The branch is made with ``git branch <name> <commit>``, which
    gives it no upstream (decision D33-25).
    """
    root = root_of(root)
    path = story_path(root, story)
    if path.exists():
        if not (path / ".git").exists():
            raise WorktreeError(
                "occupied",
                f".worktrees/{story} exists and is not a worktree; remove it and run again",
            )
        checked = _checked_branch(path)
        if checked != branch:
            raise WorktreeError(
                "branch_mismatch",
                f".worktrees/{story} is on {checked or 'a detached head'}, not {branch}",
            )
        if not is_ancestor(root, base, f"refs/heads/{branch}"):
            raise _not_descendant(story, branch, base, with_worktree=True)
        return path
    if _ref_exists(root, branch) and not is_ancestor(root, base, f"refs/heads/{branch}"):
        raise _not_descendant(story, branch, base, with_worktree=False)
    if not _ref_exists(root, branch):
        _git(root, "branch", branch, base)  # a commit id: no upstream is set
    _worktree_add(root, str(path), branch)
    return path


def verify_story(root, story: str, branch: str, base: str) -> str:
    """The story worktree's head, after the check every gate and delivery
    repeats: the worktree is there, on the story's branch, descending from the
    recorded base. Raises ``missing``, ``branch_mismatch``, ``not_descendant``."""
    path = story_path(root, story)
    if not (path / ".git").exists():
        raise WorktreeError(
            "missing",
            f".worktrees/{story} is gone; admit the story again to recreate it",
        )
    checked = _checked_branch(path)
    if checked != branch:
        raise WorktreeError(
            "branch_mismatch",
            f".worktrees/{story} is on {checked or 'a detached head'}, not {branch}",
        )
    tip = head(path)
    if not is_ancestor(path, base, tip):
        raise WorktreeError(
            "not_descendant",
            f"the head {tip[:12]} of {branch} does not descend from the recorded "
            f"base {base[:12]}; the history was rewritten after the base was recorded. "
            f"Put the rewritten commits back, or record the rewritten base anew "
            "(the delivery does after it rebases)",
        )
    return tip


def remove_story(root, story: str, *, delete_branch: bool = False) -> bool:
    """Remove the story's worktree — and its branch with ``delete_branch`` —
    idempotently; ``True`` when something was removed. The branch is deleted
    by its configured name, so a call killed between the removal and the
    deletion still deletes it on the retry, when no worktree is left to read
    the branch from."""
    root = root_of(root)
    path = story_path(root, story)
    removed = False
    if path.is_dir():
        _git(root, "worktree", "remove", "--force", str(path))
        removed = True
    if delete_branch:
        branch = branch_name(_config(root), story)
        _git(root, "worktree", "prune")  # a stale registration still holds the branch
        if _ref_exists(root, branch):
            _git(root, "branch", "-D", branch)
            removed = True
    return removed


# ------------------------------------------------------------ integration ---


def set_integration(root, commit: str) -> Path:
    """Create the integration worktree at ``commit``, or force it there,
    detached, and clean it (D17). Moves no branch."""
    root = root_of(root)
    path = integration_path(root)
    if not (path / ".git").exists():
        _worktree_add(root, "--detach", str(path), commit)
        return path
    _git(path, "checkout", "--force", "--detach", commit)
    _git(path, "clean", "-fd")
    return path


@contextmanager
def detached(root, commit: str, name: str):
    """A temporary detached worktree at ``.worktrees/_tmp-<name>/``, removed
    on exit and on error (D17)."""
    root = root_of(root)
    path = _base(root) / f"_tmp-{safe_segment(name)}"
    if path.is_dir():
        _git(root, "worktree", "remove", "--force", str(path))
    _worktree_add(root, "--detach", str(path), commit)
    try:
        yield path
    finally:
        try:
            _git(root, "worktree", "remove", "--force", str(path))
        except WorktreeError:
            pass
        _git(root, "worktree", "prune", check=False)


# -------------------------------------------------------------- unit tree ---


def unit_tree(root, workflow: str, subject: str) -> Path:
    """The tree a unit's finding paths are relative to: for ``story`` the
    story's worktree. Any other workflow is ``missing`` until it is built."""
    if workflow == "story" and subject:
        return story_path(root, subject)
    raise WorktreeError(
        "missing",
        f"unit_tree is built for the story workflow only, not {workflow or '(empty)'!r}",
    )
