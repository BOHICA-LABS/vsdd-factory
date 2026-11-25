"""Git facts from files, and the one-subprocess helper (plan 7,
test_gitfs.py)."""

from __future__ import annotations

import subprocess

import pytest

from tests.hooks import runner_helpers as helpers
from vsdd.hooks import gitfs
from vsdd.hooks.model import Ctx


def git(command: str, cwd) -> None:
    subprocess.run(command, shell=True, cwd=str(cwd), check=True, capture_output=True)


@pytest.fixture()
def repo(tmp_path):
    root = tmp_path / "repo"
    root.mkdir()
    git("git init -q -b main .", root)
    (root / "file.txt").write_text("one\n")
    git("git add .", root)
    git("git -c user.email=t@t -c user.name=T commit -qm one", root)
    return root


def test_branch_and_head_in_a_main_checkout(repo):
    assert gitfs.branch(repo) == "main"
    sha = gitfs.head_sha(repo)
    assert sha is not None and len(sha) == 40

    git("git checkout -qb story/S-1.02", repo)
    assert gitfs.branch(repo) == "story/S-1.02"
    assert gitfs.head_sha(repo) == sha  # the same commit, read from files


def test_branch_in_a_linked_worktree(repo, tmp_path):
    git("git worktree add -q -b story/S-9.01 wt", repo)
    worktree = repo / "wt"
    assert gitfs.branch(worktree) == "story/S-9.01"
    assert gitfs.branch(worktree / "sub" / "deep" if (worktree / "sub").exists() else worktree) == "story/S-9.01"
    assert gitfs.worktree_root(worktree) == worktree

    # Detached head reads as the commit:
    git("git checkout -q --detach", worktree)
    sha = gitfs.head_sha(worktree)
    assert gitfs.branch(worktree) == sha
    assert gitfs.head_sha(worktree) == sha


def test_head_sha_on_an_attached_branch_in_a_worktree(repo):
    """The story-worktree case: a linked worktree with a branch checked out
    resolves its head through the common directory, loose and packed."""
    import subprocess

    git("git worktree add -q -b story/S-9.01 wt", repo)
    worktree = repo / "wt"
    rev = lambda: subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=str(worktree), capture_output=True, text=True, check=True
    ).stdout.strip()
    expected = rev()
    assert gitfs.head_sha(worktree) == expected  # the branch is resolved, not None

    (worktree / "two.txt").write_text("two\n")
    git("git add .", worktree)
    git("git -c user.email=t@t -c user.name=T commit -qm two", worktree)
    assert gitfs.head_sha(worktree) == rev()  # the worktree's own newer commit

    # The branch is packed into the shared packed-refs; the common directory
    # answers, and a peeled annotated-tag line never masquerades as the head.
    git("git tag -a -m heavy heavy-topic", worktree)
    git("git pack-refs --all", repo)
    assert gitfs.head_sha(worktree) == rev()


def test_nearest_worktree_root(repo, tmp_path):
    git("git worktree add -q -b story/S-9.01 wt", repo)
    nested = repo / "wt" / "deep"
    nested.mkdir()
    assert gitfs.worktree_root(nested) == repo / "wt"
    assert gitfs.worktree_root(tmp_path / "nowhere") is None
    assert gitfs.branch(tmp_path / "nowhere") is None
    assert gitfs.head_sha(tmp_path / "nowhere") is None


def test_second_git_call_in_one_rule_raises(tmp_path):
    project = helpers.make_project(tmp_path)
    ctx = Ctx(helpers.payload("PreToolUse/bash-main.json") | {"cwd": str(project)},
              {"VSDD_FACTORY_ROOT": str(project), "HOME": os.environ["HOME"]},
              "gate", str(project))
    ctx.git_budget_reset()
    gitfs.git(ctx, ("status",), project)
    with pytest.raises(RuntimeError):
        gitfs.git(ctx, ("log",), project)


import os  # noqa: E402  (used by the budget test above)
