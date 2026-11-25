"""The five conditions by file reads only, the mount and the git helper
(state-core.md 3.3; topics/state-directory.md)."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

import pytest

from vsdd.state import mount
from vsdd.state.gitrun import GitError, git

from ._repo import make_project


def _write_state_toml(root: Path) -> None:
    state = root / ".factory"
    state.mkdir(exist_ok=True)
    (state / "factory.toml").write_text("schema = 1\n", encoding="utf-8")


def test_detect_five_conditions(tmp_path):
    root = tmp_path / "p"
    root.mkdir()
    assert mount.detect(root) == "none"
    state = root / ".factory"
    state.mkdir()
    (state / "STATE.md").write_text("# old\n", encoding="utf-8")
    assert mount.detect(root) == "legacy"
    _write_state_toml(root)
    assert mount.detect(root) == "partial"  # factory.toml is there, the mount test fails
    # An invalid factory.toml is still ready once the mount test passes: the
    # mount test reads the worktree's HEAD file, not the configuration.
    git(root, "init", "-q", "-b", "develop")
    git(root, "config", "user.email", "t@e.st")
    git(root, "config", "user.name", "T")
    (root / "README.md").write_text("# p\n", encoding="utf-8")
    git(root, "add", "-A")
    git(root, "commit", "-qm", "x")
    state.rename(root / "aside")  # the mount replaces the plain directory
    git(root, "worktree", "add", "-q", "--detach", str(state), "HEAD")
    (state / "factory.toml").write_text("schema = not-one\n", encoding="utf-8")
    # Point HEAD at the state branch name so the mount test sees it.
    pointer = (state / ".git").read_text(encoding="utf-8").strip()
    gitdir = Path(pointer.split("gitdir: ", 1)[1])
    if not gitdir.is_absolute():
        gitdir = state / gitdir
    (gitdir / "HEAD").write_text("ref: refs/heads/factory-artifacts\n", encoding="utf-8")
    assert (state / "factory.toml").is_file()  # the invalid file is on disk
    assert mount.detect(root) == "ready"
    assert mount.mounted(root) is True
    # State exists but is not mounted.
    plain = tmp_path / "q"
    plain.mkdir()
    git(plain, "init", "-q", "-b", "develop")
    (plain / ".git" / "refs" / "heads").mkdir(parents=True, exist_ok=True)
    (plain / ".git" / "refs" / "heads" / "factory-artifacts").write_text(
        "0123456789abcdef0123456789abcdef01234567\n", encoding="utf-8"
    )
    assert mount.detect(plain) == "unmounted"


def test_detect_reads_files_only(tmp_path, monkeypatch):
    """No git on PATH: detect runs on loose files and packed-refs alone."""
    root = tmp_path / "r"
    root.mkdir()
    git(root, "init", "-q", "-b", "develop")
    (root / ".git" / "packed-refs").write_text(
        "# pack-refs with: peeled fully-peeled sorted \n"
        "0123456789abcdef0123456789abcdef01234567 refs/heads/factory-artifacts\n",
        encoding="utf-8",
    )
    monkeypatch.setenv("PATH", str(tmp_path / "no-bin"))
    monkeypatch.delenv("GITORIAL", raising=False)
    assert mount.detect(root) == "unmounted"
    assert mount.detect(tmp_path / "nothing") == "none"


def test_git_raises_with_stderr(tmp_path):
    root = tmp_path / "g"
    root.mkdir()
    git(root, "init", "-q")
    with pytest.raises(GitError) as raised:
        git(root, "rev-parse", "--verify", "definitely-not-a-ref^{commit}")
    assert raised.value.returncode != 0
    assert raised.value.stderr.strip()
    # Without check, a non-zero exit is not an error.
    out = git(root, "rev-parse", "--verify", "definitely-not-a-ref^{commit}", check=False)
    assert out == ""


def test_git_timeout(tmp_path):
    root = tmp_path / "h"
    root.mkdir()
    git(root, "init", "-q")
    with pytest.raises(GitError) as raised:
        git(root, "fsck", timeout=0)
    assert raised.value.returncode == 124


def test_ensure_excludes_is_idempotent(tmp_path):
    root = tmp_path / "e"
    root.mkdir()
    git(root, "init", "-q", "-b", "develop")
    mount.ensure_excludes(root)
    exclude = root / ".git" / "info" / "exclude"
    once = exclude.read_text(encoding="utf-8")
    assert ".factory/" in once and ".worktrees/" in once
    mount.ensure_excludes(root)
    assert exclude.read_text(encoding="utf-8") == once


def test_ensure_excludes_repairs_a_missing_trailing_newline(tmp_path):
    """An exclude file whose last line has no newline must not glue the
    appended pattern onto it ('*.log' becoming '*.log.factory/')."""
    root = tmp_path / "n"
    root.mkdir()
    git(root, "init", "-q", "-b", "develop")
    exclude = root / ".git" / "info" / "exclude"
    exclude.write_text("# one\n*.log", encoding="utf-8")
    mount.ensure_excludes(root)
    text = exclude.read_text(encoding="utf-8")
    assert "*.log\n" in text
    assert "*.log.factory/" not in text
    assert ".factory/\n" in text and ".worktrees/\n" in text
    # The write went through a temporary file: no leftover sits beside it.
    assert sorted(p.name for p in exclude.parent.iterdir()) == ["exclude"]


def test_mount_state_raises_outside_a_repository(tmp_path):
    root = tmp_path / "m"
    root.mkdir()
    with pytest.raises(GitError):
        mount.mount_state(root)


def test_git_ignores_inherited_repository_variables(tmp_path):
    """GIT_DIR and friends from the environment must not redirect cwd
    (finding S3R-state-core-24)."""
    elsewhere = tmp_path / "elsewhere"
    elsewhere.mkdir()
    git(elsewhere, "init", "-q", "-b", "main")
    root = tmp_path / "r"
    root.mkdir()
    git(root, "init", "-q", "-b", "main")
    old_environ = dict(os.environ)
    try:
        os.environ["GIT_DIR"] = str(elsewhere / ".git")
        os.environ["GIT_WORK_TREE"] = str(elsewhere)
        located = git(root, "rev-parse", "--absolute-git-dir")
    finally:
        os.environ.clear()
        os.environ.update(old_environ)
    assert Path(located) == (root / ".git").resolve()


def test_git_maps_missing_binary_to_giterror(tmp_path, monkeypatch):
    """A missing git raises GitError, not FileNotFoundError."""
    root = tmp_path / "g"
    root.mkdir()
    monkeypatch.setenv("PATH", str(tmp_path / "empty-bin"))
    with pytest.raises(GitError) as raised:
        git(root, "status")
    assert raised.value.returncode == 127


def test_git_timeout_decodes_stderr(tmp_path):
    root = tmp_path / "t"
    root.mkdir()
    git(root, "init", "-q")
    with pytest.raises(GitError) as raised:
        git(root, "fsck", timeout=0)
    message = str(raised.value)
    assert "b'" not in message and 'b"' not in message
    assert raised.value.returncode == 124


def test_mount_state_survives_a_registered_but_missing_worktree(tmp_path):
    """A worktree add killed halfway leaves a registered worktree whose
    directory moved aside; the next mount prunes and retries."""
    root = make_project(tmp_path / "p")
    # The person moved the half-built .factory aside: registration dangles.
    subprocess.run(["rm", "-rf", str(root / ".factory")], check=True)
    mount.mount_state(root)  # would raise 'missing but already registered'
    assert mount.mounted(root) is True


def test_mount_state_finishes_an_interrupted_pointer(tmp_path):
    """A .factory holding only its own .git pointer is an interrupted mount:
    it is cleared and the worktree is added again."""
    root = make_project(tmp_path / "q")
    pointer = (root / ".factory" / ".git").read_text(encoding="utf-8")
    subprocess.run(["rm", "-rf", str(root / ".factory")], check=True)
    (root / ".factory").mkdir()
    (root / ".factory" / ".git").write_text(pointer, encoding="utf-8")
    mount.mount_state(root)
    assert mount.mounted(root) is True
    assert (root / ".factory" / "factory.toml").is_file()


def test_mount_state_never_removes_a_nested_repository(tmp_path):
    """A .factory whose only entry is a .git DIRECTORY is a repository of its
    own: not an interrupted mount, never removed (finding
    S3R-state-core-C1-1 — the rmtree ate a nested repository's history)."""
    root = make_project(tmp_path / "n")
    nested = root / ".factory"
    pointer = (nested / ".git").read_text(encoding="utf-8")
    subprocess.run(["rm", "-rf", str(nested)], check=True)
    nested.mkdir()
    subprocess.run(["git", "init", "-q", str(nested)], check=True, capture_output=True)
    subprocess.run(["git", "config", "user.email", "factory@example.test"], cwd=nested, check=True)
    subprocess.run(["git", "config", "user.name", "Factory Test"], cwd=nested, check=True)
    (nested / "precious.md").write_text("# history\n", encoding="utf-8")
    subprocess.run(["git", "add", "-A"], cwd=nested, check=True)
    subprocess.run(["git", "commit", "-qm", "precious"], cwd=nested, check=True)
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=nested, check=True, capture_output=True, text=True
    ).stdout.strip()
    (nested / "precious.md").unlink()  # only .git remains
    # mount_state refuses (GitError from worktree add) but never deletes.
    with pytest.raises(GitError):
        mount.mount_state(root)
    assert (nested / ".git").is_dir()
    assert head == subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=nested, check=True, capture_output=True, text=True
    ).stdout.strip()
    assert pointer  # sanity: the real pointer text is a file's content
