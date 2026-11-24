"""One shared fixture table for the two implementations of the find-root rules.

``py/find-root.sh`` (shell) and ``vsdd.project.find_project_root`` (Python)
must agree, because the shell decides the session project and Python decides
the governing project per path (hooks.md 2.5). :func:`build_table` lays out one
filesystem tree with real git repositories and returns the cases both test
files consume:

- ``tests/hooks/test_find_root.py`` sources the shell script under sh, bash
  and dash with ``CLAUDE_PROJECT_DIR`` and ``HOME`` set from the case;
- ``tests/unit/test_project.py`` calls ``find_project_root(start, home=home)``.
"""

from __future__ import annotations

import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class RootCase:
    name: str
    start: Path  # CLAUDE_PROJECT_DIR / the path detection starts at
    home: Path  # the home directory the walk must stop below
    expected: Path | None  # the project root, or None when nothing may be adopted


def _git(env_home: Path, repo: Path, *args: str) -> None:
    subprocess.run(
        ["git", "-C", str(repo), *args],
        check=True,
        capture_output=True,
        timeout=60,
        env={
            "GIT_AUTHOR_NAME": "t",
            "GIT_AUTHOR_EMAIL": "t@example.com",
            "GIT_COMMITTER_NAME": "t",
            "GIT_COMMITTER_EMAIL": "t@example.com",
            "GIT_CONFIG_GLOBAL": "/dev/null",
            "GIT_CONFIG_SYSTEM": "/dev/null",
            "HOME": str(env_home),
            "PATH": "/usr/bin:/bin:/usr/local/bin",
            "LC_ALL": "C",
        },
    )


def build_table(sandbox: Path) -> list[RootCase]:
    """Lay the shared tree out inside ``sandbox`` and return the cases."""
    sandbox.mkdir(parents=True, exist_ok=True)
    home = sandbox / "home"
    home.mkdir(exist_ok=True)

    # A project under the home directory; the walk adopts it from below only.
    under_home = home / "site"
    (under_home / ".factory").mkdir(parents=True, exist_ok=True)
    (under_home / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
    (under_home / "docs").mkdir(exist_ok=True)

    # A stray factory state in the home directory itself: it must adopt nothing.
    stray = sandbox / "stray-home"
    (stray / ".factory").mkdir(parents=True, exist_ok=True)
    (stray / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
    (stray / "child").mkdir(exist_ok=True)

    # An ordinary project, reached from below.
    project = sandbox / "project"
    (project / ".factory").mkdir(parents=True, exist_ok=True)
    (project / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
    marker = project / "file.txt"
    marker.write_text("x\n", encoding="utf-8")
    (project / "src" / "nested" / "deeper").mkdir(parents=True, exist_ok=True)
    _git(home, sandbox, "init", "-q", str(project))
    _git(home, project, "add", "file.txt")
    _git(home, project, "commit", "-qm", "init")

    # A linked worktree outside the project: the .git pointer pulls the main
    # checkout into the walk, from any directory inside the worktree.
    linked = sandbox / "linked-wt"
    _git(home, project, "worktree", "add", "-q", str(linked), "-b", "linked-wt")
    (linked / "deep").mkdir(exist_ok=True)

    # A linked worktree of an unrelated, non-factory repository.
    unrelated_repo = sandbox / "unrelated"
    unrelated_repo.mkdir(exist_ok=True)
    filler = unrelated_repo / "file.txt"
    filler.write_text("x\n", encoding="utf-8")
    _git(home, sandbox, "init", "-q", str(unrelated_repo))
    _git(home, unrelated_repo, "add", "file.txt")
    _git(home, unrelated_repo, "commit", "-qm", "init")
    unrelated_wt = sandbox / "unrelated-wt"
    _git(home, unrelated_repo, "worktree", "add", "-q", str(unrelated_wt), "-b", "unrelated-wt")

    # A plain directory with nothing anywhere above it but the sandbox.
    (sandbox / "plain").mkdir(exist_ok=True)

    return [
        RootCase("walks_up_from_a_subdirectory", project / "src" / "nested" / "deeper", home, project),
        RootCase("project_below_home_is_found", under_home / "docs", home, under_home),
        RootCase("stray_home_factory_adopts_nothing", stray / "child", stray, None),
        RootCase("linked_worktree_yields_the_main_checkout", linked / "deep", home, project),
        RootCase("worktree_of_an_unrelated_repository_is_inert", unrelated_wt, home, None),
        RootCase("plain_directory_finds_nothing", sandbox / "plain", home, None),
    ]
