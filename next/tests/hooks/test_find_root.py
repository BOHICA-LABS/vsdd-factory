"""Project-discovery tests for py/find-root.sh (B2; hooks.md 2.8).

Cases run find-root.sh under sh, bash and dash in real temporary git repositories:
the upward walk, the stop at $HOME and at /, linked worktrees (absolute and
relative gitdir paths), and legacy pre-1.0 detection.
"""

from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

from ._helpers import FIND_ROOT_SH, INTERPRETERS, parsed_root, source_find_root


def git(repo: Path, *args: str) -> None:
    subprocess.run(
        ["git", "-C", str(repo), *args],
        check=True,
        capture_output=True,
        timeout=60,
        env={"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@example.com",
             "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@example.com",
             "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_SYSTEM": "/dev/null",
             "HOME": str(repo.parent), "PATH": "/usr/bin:/bin:/usr/local/bin",
             "LC_ALL": "C"},
    )


def init_factory_project(root: Path, name: str = "proj", legacy: bool = False) -> Path:
    """A real git repository holding factory state, with one commit."""
    project = root / name
    (project / ".factory").mkdir(parents=True)
    marker = project / "file.txt"
    marker.write_text("x\n", encoding="utf-8")
    if legacy:
        (project / ".factory" / "STATE.md").write_text("# old\n", encoding="utf-8")
    else:
        (project / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
    git(root, "init", "-q", str(project))
    git(project, "add", "file.txt")
    git(project, "commit", "-qm", "init")
    return project


def add_worktree(project: Path, worktree: Path, relative: bool) -> None:
    args = ["worktree", "add", "-q", str(worktree), "-b", worktree.name]
    if relative:
        git(project, "-c", "worktree.useRelativePaths=true", *args)
    else:
        git(project, *args)


class FindRootCase(unittest.TestCase):
    def setUp(self) -> None:
        tmp = tempfile.TemporaryDirectory(prefix="vsdd-findroot-")
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)

    def assert_found(self, where: Path, want: Path, home: Path | None = None) -> None:
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                root, legacy = parsed_root(source_find_root(interpreter, where, home=home))
                self.assertEqual(Path(root).resolve(), want.resolve(), f"shell {interpreter}")
                self.assertTrue(root.startswith("/"), f"root must be absolute: {root!r}")

    def assert_legacy(self, where: Path, home: Path | None = None) -> None:
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                root, legacy = parsed_root(source_find_root(interpreter, where, home=home))
                self.assertEqual((root, legacy), ("", "1"), f"shell {interpreter}")

    def assert_nothing(self, where: Path, home: Path | None = None) -> None:
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                root, legacy = parsed_root(source_find_root(interpreter, where, home=home))
                self.assertEqual((root, legacy), ("", ""), f"shell {interpreter}")


class TestWalk(FindRootCase):
    def test_subdirectory_walks_up_to_project_root(self) -> None:
        project = init_factory_project(self.root)
        deep = project / "src" / "deep" / "deeper"
        deep.mkdir(parents=True)
        self.assert_found(deep, project)

    def test_stop_at_home_even_when_home_holds_factory_state(self) -> None:
        home = self.root / "home"
        (home / ".factory").mkdir(parents=True)
        (home / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
        child = home / "child"
        child.mkdir()
        # No project anywhere below home: nothing may be adopted.
        self.assert_nothing(child, home=home)
        # ... and a trailing-slash HOME must stop identically (R13).
        home_with_slash = Path(str(home) + "/")
        self.assert_nothing(child, home=home_with_slash)

    def test_project_below_home_is_found(self) -> None:
        home = self.root / "home2"
        home.mkdir()
        project = init_factory_project(home, "mysite")
        child = project / "docs"
        child.mkdir()
        self.assert_found(child, project, home=home)

    def test_plain_directory_finds_nothing(self) -> None:
        plain = self.root / "plain"
        plain.mkdir()
        self.assert_nothing(plain)

    def test_legacy_project_detected(self) -> None:
        project = init_factory_project(self.root, "oldproj", legacy=True)
        self.assert_legacy(project)

    def test_relative_start_is_inert(self) -> None:
        # A relative CLAUDE_PROJECT_DIR means "not a factory project" by design.
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                result = subprocess.run(
                    [INTERPRETERS[interpreter], "-c",
                     f'CLAUDE_PROJECT_DIR=relative/path . "{FIND_ROOT_SH}"; '
                     'printf \'ROOT=%s\\n\' "$FACTORY_ROOT"'],
                    capture_output=True, text=True, timeout=60,
                    cwd=str(self.root),
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout.strip(), "ROOT=")


class TestWorktrees(FindRootCase):
    def test_absolute_gitdir_detection_exact(self) -> None:
        project = init_factory_project(self.root, "main-exact")
        wt = self.root / "wt-exact"
        add_worktree(project, wt, relative=False)
        # Inside the worktree: the main checkout is the project.
        self.assert_found(wt, project)
        # A CLAUDE_PROJECT_DIR outside the worktree and outside the project finds
        # nothing: the worktree only answers for its own directory.
        outside = self.root / "outside-place"
        outside.mkdir()
        self.assert_nothing(outside)

    def _relative_worktree(self, name: str, legacy: bool = False) -> tuple[Path, Path]:
        project = init_factory_project(self.root, name, legacy=legacy)
        wt = self.root / f"wt-{name}"
        add_worktree(project, wt, relative=True)
        gitdir_line = (wt / ".git").read_text().strip()
        if not gitdir_line.startswith(("gitdir: ../", "gitdir: ..\\")):
            self.skipTest(f"this git writes absolute gitdir lines: {gitdir_line!r}")
        return project, wt

    def test_relative_gitdir_worktree_git_2_48_style(self) -> None:
        """R7: worktree.useRelativePaths writes `gitdir: ../main/.git/worktrees/x`;
        the root must still resolve, from any cwd."""
        project, wt = self._relative_worktree("main-rel")
        # From inside the worktree...
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter, cwd=str(wt)):
                root, _ = parsed_root(source_find_root(interpreter, wt, home=self.root, cwd=str(wt)))
                self.assertEqual(Path(root).resolve(), project.resolve())
                self.assertTrue(root.startswith("/"), f"root is relative: {root!r}")
        # ... and from a cwd unrelated to the worktree, exercising the
        # absolute-isation of the relative path.
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter, cwd="/"):
                root, _ = parsed_root(source_find_root(interpreter, wt, home=self.root, cwd="/"))
                self.assertEqual(Path(root).resolve(), project.resolve(), f"shell {interpreter}")

    def test_worktree_of_legacy_project(self) -> None:
        project, wt = self._relative_worktree("main-legacy", legacy=True)
        _ = project
        self.assert_legacy(wt)


class TestSharedFixtureTable(unittest.TestCase):
    """The same tree the Python twin (vsdd.project.find_project_root) walks:
    one table through both implementations, so they cannot disagree."""

    def test_each_case_through_every_interpreter(self) -> None:
        import tempfile

        from ..root_cases import build_table

        with tempfile.TemporaryDirectory(prefix="vsdd-roottable-") as td:
            for case in build_table(Path(td)):
                for interpreter in INTERPRETERS:
                    with self.subTest(case=case.name, shell=interpreter):
                        root, _ = parsed_root(
                            source_find_root(interpreter, case.start, home=case.home)
                        )
                        want = case.expected.resolve() if case.expected else None
                        got = Path(root).resolve() if root else None
                        self.assertEqual(got, want, f"shell {interpreter}")


if __name__ == "__main__":
    unittest.main()
