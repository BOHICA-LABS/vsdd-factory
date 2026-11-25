"""The worktree library's git layer (plan survey.md 3.3, section 7)."""

from __future__ import annotations

import shlex
import shutil

import pytest

from ._repo import make_project, set_keys
from .story_ready import make_project as make_story_project

from vsdd.state import admission, gitrun
from vsdd.state.cycles import closure_read, closure_update
from vsdd.state.store import git as run_git
from vsdd.state.worktrees import (
    WorktreeError,
    branch_name,
    create_story,
    detached,
    integration_path,
    remove_story,
    set_integration,
    story_path,
    target_head,
    unit_tree,
    verify_story,
)

BRANCH = "story/S-9.01"
STORY = "S-9.01"  # the id BRANCH is built from, in the sample project


def _git(root, *args: str) -> str:
    return run_git(root, *args)


def test_create_story_at_exact_base(tmp_path):
    root = make_project(tmp_path)
    base = _git(root, "rev-parse", "develop")
    path = create_story(root, "S-9.01", BRANCH, base)
    assert path == root / ".worktrees" / "S-9.01"
    assert _git(root, "rev-parse", BRANCH) == base
    assert (path / ".git").exists()


def test_create_story_idempotent(tmp_path):
    root = make_project(tmp_path)
    base = _git(root, "rev-parse", "develop")
    first = create_story(root, "S-9.01", BRANCH, base)
    second = create_story(root, "S-9.01", BRANCH, base)
    assert first == second
    # Decision D33-25: a story branch is cut from a commit id and never
    # carries an upstream by accident.
    assert _git(root, "for-each-ref", "--format=%(upstream)", f"refs/heads/{BRANCH}") == ""


def test_create_story_refuses_occupied_and_foreign_branch(tmp_path):
    root = make_project(tmp_path)
    base = _git(root, "rev-parse", "develop")
    occupied = root / ".worktrees" / "S-9.01"
    occupied.mkdir(parents=True)
    (occupied / "scratch.txt").write_text("not a worktree", encoding="utf-8")
    with pytest.raises(WorktreeError) as occupied_error:
        create_story(root, "S-9.01", BRANCH, base)
    assert occupied_error.value.code == "occupied"

    shutil.rmtree(occupied)
    _git(root, "worktree", "add", "-b", "other/branch", str(occupied), base)
    with pytest.raises(WorktreeError) as mismatch:
        create_story(root, "S-9.01", BRANCH, base)
    assert mismatch.value.code == "branch_mismatch"


def test_target_head_local_and_pr(tmp_path):
    root = make_project(tmp_path)
    set_keys(root, "delivery", mode="local")
    config = _config(root)
    branch, head = target_head(root, config)
    assert branch == "develop"
    assert head == _git(root, "rev-parse", "develop")

    # Decision D33-13: the remote comes from the branch's upstream and is
    # never named in code; this remote is not called origin.
    bare = tmp_path / "gitea.git"
    run_git(tmp_path, "init", "-q", "--bare", "-b", "develop", str(bare))
    _git(root, "remote", "add", "gitea", str(bare))
    _git(root, "push", "-q", "-u", "gitea", "develop")
    set_keys(root, "delivery", mode="pr")
    config = _config(root)
    branch, head = target_head(root, config)
    assert branch == "develop"
    assert head == _git(root, "rev-parse", "refs/remotes/gitea/develop")

    # What arrives on the remote after a fetch is what the target reads: the
    # remote-tracking ref is wound back, and the fetch brings the truth back.
    (root / "later.txt").write_text("later", encoding="utf-8")
    _git(root, "add", "-A")
    _git(root, "commit", "-qm", "advance")
    _git(root, "push", "-q", "gitea", "develop")
    _git(root, "update-ref", "refs/remotes/gitea/develop", head)  # pretend no fetch happened
    assert target_head(root, config)[1] == _git(root, "rev-parse", "develop")

    # An upstream whose ref is not the branch: refused, naming the command,
    # which corrects the upstream when it is run (decision D33-13).
    _git(root, "branch", "aside", head)
    _git(root, "push", "-q", "gitea", "aside")
    _git(root, "branch", "--set-upstream-to=gitea/aside", "develop")
    with pytest.raises(WorktreeError) as wrong_ref:
        target_head(root, config)
    assert "develop" in wrong_ref.value.message
    suggested = wrong_ref.value.message.split("correct it with: ", 1)[1].strip()
    correction = shlex.split(suggested)
    assert correction[:2] == ["git", "branch"]
    run_git(root, *correction[1:])  # the printed command must work as printed
    assert target_head(root, config)[1]

    # No upstream and exactly one remote: that remote serves.
    _git(root, "config", "--unset", "branch.develop.remote")
    _git(root, "config", "--unset", "branch.develop.merge")
    assert target_head(root, config)[1] == _git(root, "rev-parse", "refs/remotes/gitea/develop")

    # No upstream and two remotes: refused, naming the command that sets one.
    _git(root, "remote", "add", "forge", str(tmp_path / "gitea.git"))
    with pytest.raises(WorktreeError) as no_upstream:
        target_head(root, config)
    assert "develop" in no_upstream.value.message
    assert "--set-upstream-to" in no_upstream.value.message


def test_verify_story_refusals(tmp_path):
    root = make_project(tmp_path)
    base = _git(root, "rev-parse", "develop")
    create_story(root, "S-9.01", BRANCH, base)
    assert verify_story(root, "S-9.01", BRANCH, base) == _git(root, "rev-parse", "HEAD")

    remove_story(root, "S-9.01")
    with pytest.raises(WorktreeError) as missing:
        verify_story(root, "S-9.01", BRANCH, base)
    assert missing.value.code == "missing"

    create_story(root, "S-9.01", BRANCH, base)
    worktree = story_path(root, "S-9.01")
    _git(worktree, "checkout", "-q", "--detach", base)
    with pytest.raises(WorktreeError) as detached_head:
        verify_story(root, "S-9.01", BRANCH, base)
    assert detached_head.value.code == "branch_mismatch"
    _git(worktree, "checkout", "-q", BRANCH)

    # A base the history does not contain: the head does not descend from it.
    with pytest.raises(WorktreeError) as rewritten:
        verify_story(root, "S-9.01", BRANCH, "0" * 40)
    assert rewritten.value.code == "not_descendant"

    # The same refusal on creation names a remedy that works: delete the
    # branch, then admit again (admit cuts it from the target head and
    # records that base). With the worktree standing, the worktree is
    # removed first — git refuses to delete a branch it holds
    # (finding S3R-survey-8); the remedy is exercised for real in
    # test_not_descendant_remedy_works_in_both_cases.
    with pytest.raises(WorktreeError) as branch_base:
        create_story(root, "S-9.01", BRANCH, "1" * 40)
    assert branch_base.value.code == "not_descendant"
    assert f"git branch -D {BRANCH}" in branch_base.value.message
    assert "git worktree remove --force .worktrees/S-9.01" in branch_base.value.message


def test_not_descendant_remedy_works_in_both_cases(tmp_path):
    """Both ``not_descendant`` refusals of ``create_story`` print commands
    that work when run, and admitting the story again recovers it (finding
    S3R-survey-8): with the worktree standing (the common case) and with the
    branch alone."""
    root = make_story_project(tmp_path)
    from .story_ready import SESSION

    begun = _begin(root, session=SESSION)
    run_id = begun["run"]["id"]
    admit = lambda: admission.admit(
        root, cycle="c-1", story=STORY, run=run_id, admitted_by="plan"
    )

    # A first, sound admission: the closure, branch and worktree stand.
    create_story(root, STORY, BRANCH, run_git(root, "rev-parse", "develop"))
    admit()
    assert (story_path(root, STORY) / ".git").exists()

    # The recorded base is not an ancestor (history rewritten): refused,
    # with the worktree standing. The printed remedy — worktree remove,
    # then branch -D — runs as printed, and admission recovers.
    closure_update(root, "c-1", "story", STORY, _wrong_base)
    with pytest.raises(WorktreeError) as with_tree:
        create_story(root, STORY, BRANCH, "0" * 40)
    assert with_tree.value.code == "not_descendant"
    assert "git worktree remove --force .worktrees/S-9.01" in with_tree.value.message
    _run_printed_remedy(root, with_tree.value.message)
    assert not story_path(root, STORY).exists()  # the remedy took the worktree
    assert not _branch_exists(root, BRANCH)
    admit()  # the branch is missing: the target head is read and recorded again
    fresh = closure_read(root, "c-1", "story", STORY)["delivery"]["base"]
    assert fresh == run_git(root, "rev-parse", "develop")
    assert verify_story(root, STORY, BRANCH, fresh)

    # The same refusal with no worktree: the remedy deletes the branch alone.
    remove_story(root, STORY)  # the worktree goes, the branch stands
    closure_update(root, "c-1", "story", STORY, _wrong_base)
    with pytest.raises(WorktreeError) as without_tree:
        create_story(root, STORY, BRANCH, "0" * 40)
    assert without_tree.value.code == "not_descendant"
    assert "worktree remove" not in without_tree.value.message
    _run_printed_remedy(root, without_tree.value.message)
    assert not _branch_exists(root, BRANCH)
    admit()
    fresh = closure_read(root, "c-1", "story", STORY)["delivery"]["base"]
    assert fresh == run_git(root, "rev-parse", "develop")
    assert (story_path(root, STORY) / ".git").exists()


def _begin(root, *, session: str) -> dict:
    from vsdd.state.locks import lock_scope, run_begin

    return run_begin(
        root,
        scope=lock_scope(root),
        workflow="story",
        subject=STORY,
        session=session,
        cycle="c-1",
    )


def _wrong_base(record: dict) -> None:
    record.setdefault("delivery", {})["base"] = "0" * 40


def _branch_exists(root, branch: str) -> bool:
    return bool(run_git(root, "for-each-ref", f"refs/heads/{branch}").strip())


def _run_printed_remedy(root, message: str) -> None:
    """Run the commands after ``correct it with: `` (up to ``; then admit``),
    as the person the refusal addresses would."""
    commands = message.split("correct it with: ", 1)[1].partition("; then admit")[0]
    for command in commands.split(";"):
        parts = shlex.split(command.strip())
        assert parts[0] == "git"
        run_git(root, *parts[1:])


def test_remove_story_keeps_branch(tmp_path):
    root = make_project(tmp_path)
    base = _git(root, "rev-parse", "develop")
    create_story(root, "S-9.01", BRANCH, base)
    assert remove_story(root, "S-9.01") is True
    assert not story_path(root, "S-9.01").exists()
    assert remove_story(root, "S-9.01") is False  # idempotent
    # The worktree is gone and the branch stands: delete_branch removes it.
    assert remove_story(root, "S-9.01", delete_branch=True) is True
    assert _git(root, "for-each-ref", f"refs/heads/{BRANCH}").strip() == ""
    assert remove_story(root, "S-9.01", delete_branch=True) is False  # nothing left
    create_story(root, "S-9.01", BRANCH, base)
    assert remove_story(root, "S-9.01", delete_branch=True) is True
    assert _git(root, "for-each-ref", f"refs/heads/{BRANCH}").strip() == ""


def test_set_integration_detached_moves_no_branch(tmp_path):
    root = make_story_project(tmp_path)
    first = _git(root, "rev-parse", "develop")
    (root / "grown.md").write_text("grown", encoding="utf-8")
    _git(root, "add", "-A")
    _git(root, "commit", "-qm", "grow")
    later = _git(root, "rev-parse", "develop")

    path = set_integration(root, later)
    assert path == integration_path(root)
    from vsdd.project import read_branch

    assert read_branch(path) == later  # detached: the commit itself, no branch
    assert _git(root, "rev-parse", "develop") == later  # moves no branch
    set_integration(root, first)
    assert read_branch(path) == first


def test_detached_removed_on_exit_and_error(tmp_path):
    root = make_project(tmp_path)
    commit = _git(root, "rev-parse", "develop")
    with detached(root, commit, "probe") as path:
        assert path.is_dir()
        assert path.name == "_tmp-probe"
    assert not path.exists()
    with pytest.raises(RuntimeError):
        with detached(root, commit, "oops") as path:
            assert path.is_dir()
            raise RuntimeError("boom")
    assert not path.exists()


def test_unit_tree_story_and_unbuilt(tmp_path):
    root = make_project(tmp_path)
    assert unit_tree(root, "story", "S-9.01") == story_path(root, "S-9.01")
    with pytest.raises(WorktreeError) as unbuilt:
        unit_tree(root, "feature", "")
    assert unbuilt.value.code == "missing"


def test_story_path_refuses_ids_outside_the_grammar(tmp_path):
    root = make_project(tmp_path)
    assert story_path(root, "S-9.01") == root / ".worktrees" / "S-9.01"
    with pytest.raises(WorktreeError) as not_story:
        story_path(root, "not-a-story")
    assert not_story.value.code == "missing"


def test_removed_worktree_directory_does_not_block(tmp_path):
    """A worktree directory removed outside git leaves its registration
    behind; the library prunes before it adds, so a by-hand removal (or a
    kill inside worktree remove) does not wedge the story for good."""
    root = make_project(tmp_path)
    base = _git(root, "rev-parse", "develop")
    create_story(root, "S-9.01", BRANCH, base)
    shutil.rmtree(root / ".worktrees" / "S-9.01")
    create_story(root, "S-9.01", BRANCH, base)
    assert (story_path(root, "S-9.01") / ".git").exists()

    # The same for the integration worktree (set_integration) and the
    # temporary detached worktree, which share the add path.
    commit = _git(root, "rev-parse", "develop")
    set_integration(root, commit)
    shutil.rmtree(integration_path(root))
    set_integration(root, commit)
    assert (integration_path(root) / ".git").exists()


def test_remove_story_deletes_branch_after_worktree_gone(tmp_path):
    """A call killed between the removal and the branch deletion deletes the
    branch on the retry: it is deleted by its configured name (D33-13)."""
    root = make_project(tmp_path)
    base = _git(root, "rev-parse", "develop")
    create_story(root, "S-9.01", BRANCH, base)
    shutil.rmtree(root / ".worktrees" / "S-9.01")  # the kill, before branch -D
    assert remove_story(root, "S-9.01", delete_branch=True) is True
    assert _git(root, "for-each-ref", f"refs/heads/{BRANCH}").strip() == ""
    assert remove_story(root, "S-9.01", delete_branch=True) is False  # nothing left


def test_git_failure_with_sandbox_in_path_is_code_git(tmp_path, monkeypatch):
    """Only git's own 'Operation not permitted' is the sandbox: the word may
    sit in a path of an ordinary failure (ORDER.md rule 11)."""
    root = make_project(tmp_path / "sandbox")
    base = _git(root, "rev-parse", "develop")

    def failing(cwd, *args, **kwargs):
        raise gitrun.GitError(
            f"git -C {root} worktree add exited 128: fatal: not a working tree",
            128,
            "fatal: not a working tree",
        )

    monkeypatch.setattr(gitrun, "git", failing)
    with pytest.raises(WorktreeError) as error:
        create_story(root, "S-9.01", BRANCH, base)
    assert error.value.code == "git"
    monkeypatch.undo()


def test_refused_add_names_the_sandbox(tmp_path, monkeypatch):
    root = make_project(tmp_path)
    base = _git(root, "rev-parse", "develop")
    real = gitrun.git
    state = {"refused": False}

    def flaky(cwd, *args, **kwargs):
        if not state["refused"] and "worktree" in args:
            state["refused"] = True
            raise gitrun.GitError(
                "git worktree add exited 1: Operation not permitted",
                1,
                "Operation not permitted while trying to write .git/worktrees",
            )
        return real(cwd, *args, **kwargs)

    monkeypatch.setattr(gitrun, "git", flaky)
    with pytest.raises(WorktreeError) as refused:
        create_story(root, "S-9.01", BRANCH, base)
    assert refused.value.code == "not_permitted"
    assert "sandbox" in refused.value.message
    assert "Operation not permitted" in refused.value.message  # git's own line
    monkeypatch.undo()
    # The branch was made and nothing was undone: the next call finishes.
    assert _git(root, "for-each-ref", f"refs/heads/{BRANCH}").strip()
    path = create_story(root, "S-9.01", BRANCH, base)
    assert (path / ".git").exists()


def test_branch_name_from_config(tmp_path):
    root = make_project(tmp_path)
    from vsdd.config import load

    assert branch_name(load(root / ".factory" / "factory.toml"), "S-9.01") == "story/S-9.01"
    set_keys(root, "branches", story="epic/{id}")
    assert branch_name(load(root / ".factory" / "factory.toml"), "S-9.01") == "epic/S-9.01"


def _config(root):
    from vsdd.config import load

    return load(root / ".factory" / "factory.toml")
