"""The patch writer (plan gates.md 3.5; PL4-6, VSA-03)."""

import subprocess
from pathlib import Path

import pytest

from tests.unit import gate_project
from vsdd.gates.diffpatch import write_patch
from vsdd.state.gitrun import GitError


def _git(tree: Path, *args: str) -> str:
    done = subprocess.run(["git", *args], cwd=tree, capture_output=True, text=True)
    assert done.returncode == 0, done.stderr
    return done.stdout.strip()


def test_write_patch_names_by_range(tmp_path):
    root, run = gate_project.make_project(tmp_path)
    head = gate_project.red_commit(root)
    tree = gate_project.worktree_of(root)
    base = _git(tree, "merge-base", "develop", "HEAD")

    whole = write_patch(root, gate_project.STORY, base, head, tree=tree)
    assert whole.endswith(f"diff.{head[:12]}.patch")
    assert Path(whole).is_file() and Path(whole).is_absolute()

    ranged = write_patch(root, gate_project.STORY, base, head, tree=tree, since=base)
    assert ranged.endswith(f"diff.{base[:12]}-{head[:12]}.patch")
    # a --since call never replaces the whole patch (PL4-6)
    assert Path(whole).is_file()
    whole_text = Path(whole).read_text()
    assert "sharpen" in whole_text


def test_unknown_commit_raises(tmp_path):
    root, _run = gate_project.make_project(tmp_path)
    tree = gate_project.worktree_of(root)
    with pytest.raises(GitError):
        write_patch(root, gate_project.STORY, "0" * 40, "1" * 40, tree=tree)


def test_parallel_writers_do_not_collide(tmp_path, monkeypatch):
    """Two writers at once never collide and leave no temporary behind: the
    temporary name is unique per writer (the store's write_via_tmp)."""
    import os
    import threading
    from pathlib import Path

    root, run = gate_project.make_project(tmp_path)
    head = gate_project.red_commit(root)
    tree = gate_project.worktree_of(root)
    base = _git(tree, "merge-base", "develop", "HEAD")

    real_replace = os.replace
    at_replace = threading.Barrier(2)

    def slow_replace(src, dst, *args, **kwargs):
        at_replace.wait(timeout=10)  # both writers rename together
        return real_replace(src, dst, *args, **kwargs)

    monkeypatch.setattr(os, "replace", slow_replace)

    errors: list[Exception] = []

    def worker():
        try:
            write_patch(root, gate_project.STORY, base, head, tree=tree)
        except Exception as exc:  # noqa: BLE001 - reported by the assert below
            errors.append(exc)

    threads = [threading.Thread(target=worker) for _ in range(2)]
    for one in threads:
        one.start()
    for one in threads:
        one.join(timeout=60)
    assert errors == [], errors
    evidence = Path(root) / ".factory" / "local" / "evidence" / gate_project.STORY
    assert (evidence / f"diff.{head[:12]}.patch").is_file()
    assert list(evidence.glob(".*.tmp")) == []
