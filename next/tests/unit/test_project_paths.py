"""Path classes, glob semantics and the find-root home default (state-core.md 3.4)."""

from __future__ import annotations

import subprocess
from pathlib import Path

from vsdd.globs import glob_regex
from vsdd.project import find_project_root, path_class

from ._repo import make_project


def test_path_class_precedence(tmp_path):
    root = make_project(tmp_path / "a")
    assert path_class(root / "src" / "deep" / "module.py") == "source"
    assert path_class(root / "tests" / "test_module.py") == "tests"
    # Precedence, with a real overlap: a path that matches both the tests and
    # the source globs is tests, because precedence names it first.
    from ._repo import set_keys

    set_keys(root, "paths", source=["**", "src/**"])
    overlapping = root / "tests" / "double_class.py"
    overlapping.write_text("def test_double():\n    assert True\n", encoding="utf-8")
    assert path_class(overlapping) == "tests"


def test_path_class_in_a_story_worktree(tmp_path):
    root = make_project(tmp_path / "b")
    worktree = root / ".worktrees" / "S-9.01"
    subprocess.run(
        ["git", "worktree", "add", "-q", "-b", "story/9.01", str(worktree), "develop"],
        cwd=root, check=True, capture_output=True,
    )
    (worktree / "src").mkdir(exist_ok=True)
    nested = worktree / "src" / "grown.py"
    nested.write_text("value = 2\n", encoding="utf-8")
    assert path_class(nested) == "source"


def test_path_class_none_outside_and_under_state(tmp_path):
    root = make_project(tmp_path / "c")
    assert path_class(root / ".factory" / "factory.toml") is None
    assert path_class(root / ".factory" / "local" / "runs" / "x.json") is None
    outside = tmp_path / "outside"
    outside.mkdir()
    (outside / "code.py").write_text("x = 1\n", encoding="utf-8")
    assert path_class(outside / "code.py") is None


def test_glob_semantics():
    star = glob_regex("src/*.py")
    assert star.fullmatch("src/a.py")
    assert not star.fullmatch("src/deep/a.py")  # * does not cross /
    assert glob_regex("src/**").fullmatch("src/deep/a.py")  # ** does
    assert glob_regex("src/**").fullmatch("src/a")
    question = glob_regex("src/?.py")
    assert question.fullmatch("src/a.py")
    assert not question.fullmatch("src/ab.py")
    literal = glob_regex("cli.py")
    assert literal.fullmatch("cli.py")
    assert not literal.fullmatch("xcli.pyx")
    # **/ also matches zero directories, in both shapes (S3R-state-core-25).
    leading = glob_regex("**/tests.rs")
    assert leading.fullmatch("tests.rs")  # a root-level file, no directory
    assert leading.fullmatch("a/b/tests.rs")
    middle = glob_regex("a/**/b")
    assert middle.fullmatch("a/b")  # zero directories between
    assert middle.fullmatch("a/x/y/b")


def test_find_root_stops_below_home_by_default(tmp_path, monkeypatch):
    home = tmp_path / "home"
    project = home / "work"
    (project / ".factory").mkdir(parents=True)
    (project / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
    (project / "sub").mkdir()
    monkeypatch.setenv("HOME", str(home))
    # The default home is $HOME: the walk reaches the project below it.
    assert find_project_root(str(project / "sub")) == project
    # A stray ~/.factory adopts nothing, and $HOME itself is never a root.
    (home / ".factory").mkdir()
    (home / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
    assert find_project_root(str(project / "sub")) == project
    assert find_project_root(str(home / ".factory" / "factory.toml")) is None
    assert find_project_root(str(home)) is None
    stranger = tmp_path / "unrelated"
    (stranger / ".factory").mkdir(parents=True)
    (stranger / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
    assert find_project_root(str(stranger)) == stranger
    plain = tmp_path / "plain"
    (plain / "deep").mkdir(parents=True)
    assert find_project_root(str(plain / "deep")) is None
