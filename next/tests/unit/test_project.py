"""Project detection: roots, state directories, worktrees, story and spike
detection (hooks.md 2.4, blueprint 4.7)."""

import os
from pathlib import Path


from vsdd.project import (
    classify_worktree,
    containing_worktree,
    detect,
    find_project_root,
    find_state_dir,
    has_legacy_state,
    is_initialised,
    main_checkout,
    read_branch,
)


def _point_branch(worktree: Path, branch: str) -> None:
    """Fake a linked worktree: a .git pointer to a gitdir holding one HEAD."""
    gitdir = worktree / ".explicit-gitdir"
    gitdir.mkdir(parents=True, exist_ok=True)
    (gitdir / "HEAD").write_text(f"ref: refs/heads/{branch}\n", encoding="utf-8")
    (worktree / ".git").write_text(f"gitdir: {gitdir}\n", encoding="utf-8")


def _fake_platform_worktree(repo: Path, worktree: Path, branch: str) -> None:
    """Fake a platform worktree: a .git pointer into the repo's gitdir tree."""
    gitdir = repo / ".git" / "worktrees" / worktree.name
    gitdir.mkdir(parents=True, exist_ok=True)
    (gitdir / "HEAD").write_text(f"ref: refs/heads/{branch}\n", encoding="utf-8")
    worktree.mkdir(parents=True, exist_ok=True)
    (worktree / ".git").write_text(f"gitdir: {gitdir}\n", encoding="utf-8")


_GIT_ENV = {
    "GIT_AUTHOR_NAME": "t",
    "GIT_AUTHOR_EMAIL": "t@example.com",
    "GIT_COMMITTER_NAME": "t",
    "GIT_COMMITTER_EMAIL": "t@example.com",
    "GIT_CONFIG_GLOBAL": "/dev/null",
    "GIT_CONFIG_SYSTEM": "/dev/null",
    "PATH": "/usr/bin:/bin:/usr/local/bin",
    "LC_ALL": "C",
}


def _git(repo: Path, *args: str) -> None:
    import subprocess

    subprocess.run(
        ["git", "-C", str(repo), *args],
        check=True,
        capture_output=True,
        timeout=60,
        env={**_GIT_ENV, "HOME": str(repo)},
    )


def make_project(tmp_path: Path, name: str = "proj", config: bool = True) -> Path:
    root = tmp_path / name
    state = root / ".factory"
    state.mkdir(parents=True)
    if config:
        (state / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
    return root


class TestRootDetection:
    def test_walks_up_to_the_project_root(self, tmp_path):
        root = make_project(tmp_path)
        deep = root / "src" / "nested"
        deep.mkdir(parents=True)
        assert find_project_root(deep) == root
        assert find_state_dir(deep) == root / ".factory"

    def test_nearest_state_directory_wins(self, tmp_path):
        outer = make_project(tmp_path, "outer")
        # A factory project nested inside another one (a vendored sample, say):
        # paths inside it resolve to the nearer project.
        inner = make_project(outer / "samples", "inner")
        deep = inner / "deep" / "deeper"
        deep.mkdir(parents=True)
        assert find_project_root(deep) == inner
        assert find_state_dir(deep) == inner / ".factory"
        # ...while the outer project's own files still resolve to outer.
        (outer / "src").mkdir()
        assert find_project_root(outer / "src") == outer
        assert outer.is_dir() and inner.is_dir()

    def test_none_outside_a_factory_project(self, tmp_path):
        plain = tmp_path / "plain"
        plain.mkdir()
        assert find_project_root(plain) is None
        assert find_state_dir(plain) is None
        assert detect(plain) is None

    def test_uninitialised_state_directory_is_not_a_project(self, tmp_path):
        root = make_project(tmp_path, "halfway", config=False)
        assert find_state_dir(root) == root / ".factory"
        assert find_project_root(root) is None
        assert not is_initialised(root / ".factory")

    def test_legacy_state_detected(self, tmp_path):
        root = tmp_path / "old"
        (root / ".factory").mkdir(parents=True)
        (root / ".factory" / "STATE.md").write_text("# pre-1.0\n", encoding="utf-8")
        assert has_legacy_state(root) is True
        assert find_project_root(root) is None
        (root / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
        assert has_legacy_state(root) is False


class TestWorktrees:
    def test_classification(self):
        integration = classify_worktree("_integration")
        assert (integration.kind, integration.id) == ("integration", "")
        spike = classify_worktree("spike-uv-cold")
        assert (spike.kind, spike.id) == ("spike", "uv-cold")
        story = classify_worktree("S-9.01")
        assert (story.kind, story.id) == ("story", "S-9.01")

    def test_dot_names_classify_as_none(self):
        assert classify_worktree(".hidden") is None

    def test_containing_worktree_for_a_deep_path(self, tmp_path):
        root = make_project(tmp_path)
        wt = root / ".worktrees" / "S-9.01" / "src"
        wt.mkdir(parents=True)
        found = containing_worktree(wt / "lib.py")
        assert (found.kind, found.id) == ("story", "S-9.01")
        assert found.path == root / ".worktrees" / "S-9.01"

    def test_worktrees_directory_itself_is_none(self, tmp_path):
        root = make_project(tmp_path)
        (root / ".worktrees").mkdir()
        assert containing_worktree(root / ".worktrees") is None

    def test_detect_combines_root_and_worktree(self, tmp_path):
        root = make_project(tmp_path)
        (root / ".worktrees" / "spike-probe" / "lab").mkdir(parents=True)
        project = detect(root / ".worktrees" / "spike-probe" / "lab" / "try.py")
        assert project.root == root
        assert project.in_spike is True
        assert project.worktree.id == "probe"


class TestSharedRootTable:
    """One fixture table through both implementations: the shell decides the
    session project, Python the governing project per path (hooks.md 2.5), so
    find_project_root must mirror find-root.sh case for case."""

    def test_each_case(self):
        import tempfile

        from ..root_cases import build_table

        with tempfile.TemporaryDirectory(prefix="vsdd-roottable-") as td:
            for case in build_table(Path(td)):
                got = find_project_root(case.start, home=case.home)
                # Compare resolved: git writes the resolved spelling of the main
                # checkout into a worktree's .git file, so one case can only
                # promise the same directory, not the same bytes.
                assert (got is None) == (case.expected is None), case.name
                if got is not None:
                    assert got.resolve() == case.expected.resolve(), (
                        f"{case.name}: {got} != {case.expected}"
                    )

    def test_symlinked_start_keeps_the_literal_path(self, tmp_path):
        """No symlink resolution: the root is the lexically walked ancestor, so
        it compares equal to the string path the launcher hands over (macOS
        /tmp versus /private/tmp)."""
        real = make_project(tmp_path)
        link = tmp_path / "link-to-project"
        link.symlink_to(real, target_is_directory=True)
        deep = link / "sub"
        deep.mkdir()
        root = find_project_root(deep)
        assert root == Path(os.path.abspath(link)), root


class TestDetectThroughTheBranch:
    """Spike and story detection reads the branch of the worktree containing
    the path (hooks.md in_spike), not only the directory name."""

    def _git_project(self, tmp_path) -> Path:
        root = make_project(tmp_path)
        (root / "f.txt").write_text("x\n", encoding="utf-8")
        _git(root, "init", "-q", ".")
        _git(root, "add", "f.txt")
        _git(root, "commit", "-qm", "init")
        return root

    def test_spike_branch_in_a_platform_worktree(self, tmp_path):
        root = self._git_project(tmp_path)
        _fake_platform_worktree(root, root / ".claude" / "worktrees" / "quick", "spike/quick")
        project = detect(root / ".claude" / "worktrees" / "quick")
        assert project.root == root
        assert project.in_spike is True
        assert project.worktree.kind == "spike"
        assert project.worktree.id == "quick"

    def test_non_story_directory_is_not_a_story(self, tmp_path):
        root = make_project(tmp_path)
        (root / ".worktrees" / "tmp").mkdir(parents=True)
        project = detect(root / ".worktrees" / "tmp")
        assert project.worktree is None

    def test_bare_spike_dash_name_is_not_a_story(self, tmp_path):
        root = make_project(tmp_path)
        (root / ".worktrees" / "spike-").mkdir(parents=True)
        project = detect(root / ".worktrees" / "spike-")
        assert project.worktree is None

    def test_non_story_directory_with_story_branch_is_the_story(self, tmp_path):
        root = make_project(tmp_path)
        (root / ".worktrees" / "platform-17").mkdir(parents=True)
        _point_branch(root / ".worktrees" / "platform-17", "story/S-25.01")
        project = detect(root / ".worktrees" / "platform-17")
        assert project.worktree.kind == "story"
        assert project.worktree.id == "S-25.01"


class TestContainingWorktreeAnchoring:
    def test_deeper_vendored_worktrees_do_not_win(self, tmp_path):
        root = make_project(tmp_path)
        vendored = root / ".worktrees" / "S-1.02" / "vendor" / ".worktrees" / "x"
        vendored.mkdir(parents=True)
        found = containing_worktree(vendored, root=root)
        assert (found.kind, found.id) == ("story", "S-1.02")

    def test_integration_carries_the_repo_name(self, tmp_path):
        root = make_project(tmp_path)
        wt = root / ".worktrees" / "_integration" / "api-server" / "src"
        wt.mkdir(parents=True)
        found = containing_worktree(wt, root=root)
        assert found.kind == "integration"
        assert found.repo == "api-server"


class TestMainCheckoutNormalisation:
    def test_relative_gitdir_is_normalised(self, tmp_path):
        main = write_checkout(tmp_path / "main", "ref: refs/heads/develop\n")
        worktrees = main / ".git" / "worktrees" / "wt1"
        worktrees.mkdir(parents=True)
        (worktrees / "HEAD").write_text("ref: refs/heads/spike/x\n", encoding="utf-8")
        worktree = tmp_path / "linked"
        write_checkout(worktree, "", gitdir_sub=str(worktrees))
        relative = write_checkout(tmp_path / "linked-rel", "", gitdir_sub="../main/.git/worktrees/wt1")
        assert main_checkout(relative) == main
        assert main_checkout(relative) == main_checkout(worktree)


def write_checkout(base: Path, head_content: str, gitdir_sub: str | None = None) -> Path:
    base.mkdir(parents=True, exist_ok=True)
    if gitdir_sub is None:
        git = base / ".git"
        git.mkdir()
        (git / "HEAD").write_text(head_content, encoding="utf-8")
    else:
        (base / ".git").write_text(f"gitdir: {gitdir_sub}\n", encoding="utf-8")
    return base


class TestBranchReading:
    def test_main_checkout_head(self, tmp_path):
        repo = write_checkout(tmp_path / "repo", "ref: refs/heads/story/S-9.01\n")
        assert read_branch(repo) == "story/S-9.01"

    def test_linked_worktree_head_through_gitdir_pointer(self, tmp_path):
        main = write_checkout(tmp_path / "main", "ref: refs/heads/develop\n")
        worktrees = main / ".git" / "worktrees" / "wt1"
        worktrees.mkdir(parents=True)
        (worktrees / "HEAD").write_text("ref: refs/heads/story/S-9.01\n", encoding="utf-8")
        worktree = tmp_path / "linked"
        write_checkout(worktree, "", gitdir_sub=str(worktrees))
        assert read_branch(worktree) == "story/S-9.01"

    def test_relative_gitdir_pointer(self, tmp_path):
        main = write_checkout(tmp_path / "main", "ref: refs/heads/main\n")
        worktrees = main / ".git" / "worktrees" / "wt1"
        worktrees.mkdir(parents=True)
        (worktrees / "HEAD").write_text("ref: refs/heads/spike/x\n", encoding="utf-8")
        worktree = main / ".." / "linked"  # linked checkouts live anywhere
        worktree = tmp_path / "linked2"
        write_checkout(worktree, "", gitdir_sub="../main/.git/worktrees/wt1")
        assert read_branch(worktree) == "spike/x"
        assert worktree.is_dir()
        assert main.is_dir()

    def test_detached_head_returns_the_sha(self, tmp_path):
        repo = write_checkout(tmp_path / "detached", "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b\n")
        assert read_branch(repo) == "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b"

    def test_no_git_returns_none(self, tmp_path):
        plain = tmp_path / "plain"
        plain.mkdir()
        assert read_branch(plain) is None


class TestMainCheckout:
    def test_resolves_through_the_pointer(self, tmp_path):
        main = write_checkout(tmp_path / "main", "ref: refs/heads/develop\n")
        worktrees = main / ".git" / "worktrees" / "wt1"
        worktrees.mkdir(parents=True)
        (worktrees / "HEAD").write_text("ref: refs/heads/story/S-9.01\n", encoding="utf-8")
        worktree = tmp_path / "linked"
        write_checkout(worktree, "", gitdir_sub=str(worktrees))
        assert main_checkout(worktree) == main
        assert read_branch(worktree) == "story/S-9.01"

    def test_a_main_checkout_is_not_a_linked_worktree(self, tmp_path):
        repo = write_checkout(tmp_path / "repo", "ref: refs/heads/main\n")
        assert main_checkout(repo) is None
