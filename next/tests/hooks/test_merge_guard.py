"""The merge-guard rule in process (plan rules.md section 7): one test per
row of plan 3.3's table, the push and ref-move forms, the callers, and the
opaque segments."""

from __future__ import annotations

import subprocess

from tests.hooks import rule_projects
from tests.hooks.runner_helpers import run_hook
from tests.hooks.test_runner import base_env
from vsdd.hooks.model import RuleEntry

ROW = RuleEntry(
    id="merge-guard",
    on=(("PreToolUse", ("Bash",)),),
    kind="gate",
    fail="closed",
    summary="fixture row for the merge-guard tests",
)

IMPLEMENTER = "vsdd-factory:implementer"


def call(root, command: str, *, agent_type: str = IMPLEMENTER, cwd=None, home=None):
    payload = {
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "session_id": "sess-rules",
        "agent_type": agent_type,
        "cwd": str(cwd if cwd is not None else root),
        "tool_input": {"command": command},
    }
    env = base_env(root)
    if home:
        env["HOME"] = str(home)
    code, _out, err = run_hook(payload, "gate", entries=(ROW,), env=env)
    return code, err


def test_case_table(tmp_path):
    p = rule_projects.story_project(tmp_path)
    # One row per line of plan 3.3's table, judged in the main checkout,
    # which sits on the integration branch.
    blocked = [
        "gh pr merge 12 --squash",                       # direct_merge
        "git merge origin/develop",                      # integrate_on_guarded
        "git rebase -i HEAD~3",                          # integrate_on_guarded
        "git cherry-pick abc123",                        # integrate_on_guarded
        "git revert HEAD",                               # integrate_on_guarded
        "git am patches.mbox",                           # integrate_on_guarded
        "git commit --allow-empty -m x",                 # commit_on_guarded
        "git commit --amend --no-edit",                  # commit_on_guarded
        "git reset --hard",                              # commit_on_guarded
        "git push origin develop",                       # push_to_guarded
        "git update-ref refs/heads/develop abc",         # moves_guarded_branch
        "git branch -f develop abc",                     # moves_guarded_branch
        "git switch -C develop",                         # moves_guarded_branch
        "git checkout -B develop",                       # moves_guarded_branch
        "git fetch . story/x:develop",                   # moves_guarded_branch
    ]
    for command in blocked:
        code, err = call(p.root, command)
        assert code == 10, (command, err)
    # And the passing controls.
    for command in ("git status", "git log --oneline -3"):
        code, err = call(p.root, command)
        assert code == 0, (command, err)

    # The spec's pair: a commit inside the story worktree passes while the
    # main checkout sits on develop, where a bare commit is blocked.
    code, err = call(p.root, "git commit --allow-empty -m x", cwd=p.worktree)
    assert code == 0, err
    code, err = call(p.root, "git commit --allow-empty -m x")
    assert code == 10, err


def test_push_forms(tmp_path):
    p = rule_projects.story_project(tmp_path)
    for command in (
        "git push origin develop",
        "git push origin HEAD:develop",
        "git push origin HEAD",
        "git push origin +topic:develop",
        "git push origin :develop",
        "git push origin refs/heads/develop",
        "git push",
        "git push origin",
        "git push --all",
        "git push --mirror",
    ):
        code, err = call(p.root, command)
        assert code == 10, (command, err)
    for command in (
        "git push origin topic",
        "git push origin topic:refs/heads/topic",
        "git push origin HEAD:story/S-9.01",
    ):
        code, err = call(p.root, command)
        assert code == 0, (command, err)


def test_ref_moves(tmp_path):
    p = rule_projects.story_project(tmp_path)
    blocked = (
        "git update-ref refs/heads/develop abc",
        "git symbolic-ref refs/heads/develop refs/heads/other",
        "git branch -f develop abc",
        "git branch --force develop abc",
        "git branch -d develop",
        "git branch -m elsewhere",       # renames the current branch
        "git switch -C develop",
        "git checkout -B develop",
        "git fetch . topic:develop",
        "git pull . topic:develop",
    )
    for command in blocked:
        code, err = call(p.root, command)
        assert code == 10, (command, err)
    passing = (
        "git update-ref refs/heads/topic abc",
        "git branch -d topic",
        "git branch -m topic other",
        "git switch -c fresh",
        "git switch develop",
        "git checkout -B topic",
        "git checkout develop",           # plain checkout moves worktrees, not branches
        "git fetch origin",
        "git pull",                        # a bare pull moves nothing (D6)
        "git fetch . topic:story/S-9.01",
    )
    for command in passing:
        code, err = call(p.root, command)
        assert code == 0, (command, err)


def test_reset_forms(tmp_path):
    p = rule_projects.story_project(tmp_path)
    for command in (
        "git reset --hard",
        "git reset HEAD --hard",       # the mode flag after the revision (S4R-rules-22)
        "git reset --soft HEAD~1",
        "git reset 0f11e2a",
        "git reset --mixed 0f11e2a -- src/x",
    ):
        code, err = call(p.root, command)
        assert code == 10, (command, err)
    for command in (
        "git reset",
        "git reset HEAD",
        "git reset HEAD -- src/x",
        "git reset -- src/x",
    ):
        code, err = call(p.root, command)
        assert code == 0, (command, err)


def test_rebase_rewrites_the_branch_argument(tmp_path):
    """``git rebase <upstream>`` in a story worktree rewites the story branch,
    never the upstream (S4R-rules-5); the second positional is what gets
    rewritten, and naming a guarded one there blocks."""
    p = rule_projects.story_project(tmp_path)
    for command in (
        "git rebase develop",
        "git rebase origin/develop",
        "git rebase --onto main develop",
        "git rebase develop story/S-9.01",
        "git rebase --onto main develop story/S-9.01",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 0, (command, err)
    for command in (
        "git rebase origin/main develop",
        "git rebase --onto story/S-9.01 origin/main develop",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 10, (command, err)
        assert "integrate_on_guarded" in err, err


def test_pull_with_refspec_integrates(tmp_path):
    """On a guarded branch a pull that names a refspec merges it there
    (S4R-rules-6); a bare pull passes (decision D6)."""
    p = rule_projects.story_project(tmp_path)
    for command in (
        "git pull origin story/S-9.01",
        "git pull . story/S-9.01",
    ):
        code, err = call(p.root, command)
        assert code == 10, (command, err)
        assert "integrate_on_guarded" in err, (command, err)
    for command in ("git pull", "git pull origin"):
        code, err = call(p.root, command)
        assert code == 0, (command, err)


def test_wrappers_do_not_hide_git(tmp_path):
    """A wrapped git call is judged like the bare one (S4R-rules-7); the
    wrapper's own options are skipped, and the wrapper is matched by its
    basename wherever it is mounted."""
    p = rule_projects.story_project(tmp_path)
    for command in (
        "/usr/bin/git commit -m x",
        "command git commit -m x",
        "command -p git commit -m x",
        "env git commit -m x",
        "env -i git commit -m x",
        "env -u FOO git commit -m x",
        "env -i /usr/bin/env git commit -m x",
        "env GIT_AUTHOR_DATE=now git commit -m x",
        "exec git commit -m x",
        "time git commit -m x",
        "time -p git commit -m x",
        "nice git commit -m x",
        "nice -n 5 git commit -m x",
        "nohup git push origin develop",
        "nohup time git commit -m x",
        "nohup nice -n 1 git commit -m x",
    ):
        code, err = call(p.root, command)
        assert code == 10, (command, err)
    # And the wrapped sed is a sed for writes: the implementer before red is
    # judged... that is tests-first's rule; here, a wrapped merge is seen.
    code, err = call(p.root, "nohup git merge origin/develop")
    assert code == 10, err
    assert "integrate_on_guarded" in err, err


def test_opaque_git_segments(tmp_path):
    """A heredoc or a subshell never turns a plain git segment opaque
    (S4R-rules-3), and the subshell is read by its own text alone."""
    p = rule_projects.story_project(tmp_path)
    for command in (
        'git commit -m "$(cat <<\'MSG\'\nsubject\nMSG\n)"',
        "git commit -F - <<MSG\nsubject\nMSG",
        "(cd elsewhere && make) && git commit -m x",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 0, (command, err)


def test_opaque_global_values_and_trees(tmp_path):
    """Global options take their values with them (S4R-rules-21), and a
    --git-dir/--work-tree redirect is judged at the branch that tree has
    checked out."""
    p = rule_projects.story_project(tmp_path)
    # -C's value is skipped, so the verb is found and the opaque call blocked.
    code, err = call(p.root, 'sh -c "git -C /srv/main commit"')
    assert code == 10, err
    assert "opaque_git" in err, err
    # A transparent redirect to the main checkout, where develop sits.
    redirected = f"git --git-dir={p.root}/.git --work-tree={p.root} commit -m x"
    code, err = call(p.root, redirected, cwd=p.worktree)
    assert code == 10, (redirected, err)
    assert "commit_on_guarded" in err, err
    # The same redirect seen through an opaque segment.
    code, err = call(p.root, f'sh -c "git --work-tree={p.root} commit -m x"')
    assert code == 10, err
    assert "opaque_git" in err, err
    # A redirect with no resolvable tree fails closed; one outside every
    # factory project is not judged (D7).
    code, err = call(p.root, 'sh -c "git --work-tree=$NOWHERE commit -m x"')
    assert code == 10, err
    other = tmp_path / "other-repo"
    other.mkdir()
    code, err = call(p.root, f"git --work-tree={other} commit -m x", cwd=p.worktree)
    assert code == 0, err


def test_judged_at_the_git_directory(tmp_path):
    """git reads HEAD from its git directory, never from ``--work-tree``
    (S4R-rules-C1-2): the branch is judged at ``--git-dir`` or a ``GIT_DIR``
    assignment, else at the segment's own directory, and an unresolvable git
    directory blocks ``where_unknown``."""
    p = rule_projects.story_project(tmp_path)
    # A --work-tree anywhere does not move the checkout the verb speaks to:
    # from the main checkout, on develop, the commit is still held.
    for command in (
        "git --work-tree=/tmp commit -m x",
        "git --work-tree=/tmp merge origin/develop",
        "git --work-tree=/tmp reset --hard",
        "git --work-tree=/tmp push origin develop",
    ):
        code, err = call(p.root, command, cwd=p.root)
        assert code == 10, (command, err)
        assert "merge-guard/" in err, (command, err)
    # From the story worktree the same call is judged at the story branch.
    code, err = call(p.root, "git --work-tree=/tmp commit -m x", cwd=p.worktree)
    assert code == 0, err
    # A GIT_DIR assignment — bare, or riding a wrapper — names the
    # repository that is judged: the main checkout's, where develop sits.
    for command in (
        f"GIT_DIR={p.root}/.git git commit -m x",
        f"env GIT_DIR={p.root}/.git git commit -m x",
        f"git --git-dir={p.root}/.git commit -m x",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 10, (command, err)
        assert "commit_on_guarded" in err, (command, err)
    # A git directory that cannot be resolved blocks fail closed.
    for command in (
        "git --git-dir=$NOWHERE commit -m x",
        "GIT_DIR=$NOWHERE git commit -m x",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 10, (command, err)
        assert "where_unknown" in err, (command, err)


def test_unrelated_repository_on_any_branch(tmp_path):
    """An unrelated repository is judged by its own project — that is, by
    none, whatever branch it sits on (decision D7, S4R-rules-4)."""
    p = rule_projects.story_project(tmp_path)
    for branch in ("develop", "main"):
        other = tmp_path / f"other-{branch}"
        other.mkdir()
        env = {"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t",
               "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@t"}
        subprocess.run(["git", "init", "-b", branch, str(other)], check=True,
                       capture_output=True, env=env)
        (other / "f.txt").write_text("x\n")
        subprocess.run(["git", "-C", str(other), "add", "-A"], check=True,
                       capture_output=True)
        subprocess.run(["git", "-C", str(other), "commit", "-qm", "one"], check=True,
                       capture_output=True, env=env)
        code, err = call(p.root, "git commit --allow-empty -m x", cwd=other)
        assert code == 0, (branch, err)
        code, err = call(p.root, f"git push origin {branch}", cwd=other)
        assert code == 0, (branch, err)


def test_every_caller(tmp_path):
    p = rule_projects.story_project(tmp_path)
    for agent in ("", IMPLEMENTER, "vsdd-factory:clerk"):
        code, err = call(p.root, "git commit --allow-empty -m x", agent_type=agent)
        assert code == 10, (agent, err)
    # The main session's fix names the story command.
    _code, err = call(p.root, "git commit --allow-empty -m x", agent_type="")
    assert "/vsdd-factory:feature" in err, err


def test_unrelated_repository_passes(tmp_path):
    p = rule_projects.story_project(tmp_path)
    other = tmp_path / "other-repo"
    other.mkdir()
    env = {"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t", "GIT_COMMITTER_NAME": "t",
           "GIT_COMMITTER_EMAIL": "t@t"}
    subprocess.run(["git", "init", "-b", "main", str(other)], check=True, capture_output=True, env=env)
    (other / "f.txt").write_text("x\n")
    subprocess.run(["git", "-C", str(other), "add", "-A"], check=True, capture_output=True)
    subprocess.run(["git", "-C", str(other), "commit", "-qm", "one"], check=True,
                   capture_output=True, env=env)
    # A commit on main of an unrelated repository is not judged (decision D7).
    code, err = call(p.root, "git commit --allow-empty -m x", cwd=other)
    assert code == 0, err
    code, err = call(p.root, "git push origin main", cwd=other)
    assert code == 0, err


def test_unknown_directory_blocks(tmp_path):
    p = rule_projects.story_project(tmp_path)
    # The directory is unknowable, so the guarded verb is not judged safely.
    for command in (
        "cd ~ && git commit -m x",
        "cd $SOMEWHERE && git merge origin/develop",
    ):
        code, err = call(p.root, command)
        assert code == 10, (command, err)
        assert "where_unknown" in err, err


def test_opaque(tmp_path):
    p = rule_projects.story_project(tmp_path)
    for command in (
        "(cd somewhere && git merge origin/develop)",
        'sh -c "git push origin develop"',
        "echo $(git push origin develop)",
    ):
        code, err = call(p.root, command)
        assert code == 10, (command, err)
        assert "opaque_git" in err, err
    # A git call the rule cannot read further is decided by position alone
    # (S4R-rules-C2-2): on the guarded checkout it blocks, read-only or not,
    # while from the story worktree the read-only forms are harmless.
    for command in (
        "echo $(git status)",
        "echo $(git branch --show-current)",
    ):
        code, err = call(p.root, command)
        assert code == 10, (command, err)
        assert "opaque_git" in err, err
    for command in (
        "echo $(git status)",
        "echo $(git diff --stat)",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 0, (command, err)
    for command in (
        "cat <<NOTE\nsome text\ngit nothing here\nNOTE",
        "git log --oneline -5",
    ):
        code, err = call(p.root, command)
        assert code == 0, (command, err)


def test_substitutions_in_gits_own_arguments(tmp_path):
    """A substitution in an argument the verdict depends on reads as any
    guarded branch: it blocks (S4R-rules-C1-1), the plain ``$B`` expansion
    included."""
    p = rule_projects.story_project(tmp_path)
    for command, cwd in (
        ("git push origin $(git branch --show-current)", p.root),
        ("git push origin $(echo develop)", p.root),
        ("git push origin HEAD:$(echo develop)", p.root),
        ("git push origin $B", p.root),
        ("git branch -f $(echo develop) HEAD", p.root),
        ("git update-ref refs/heads/$(echo develop) abc", p.root),
        ("git switch -C $(echo develop)", p.root),
        ("git checkout -B $(echo develop)", p.root),
        ("git rebase origin/develop $(echo develop)", p.worktree),
        ("git fetch origin topic:$(echo develop)", p.worktree),
    ):
        code, err = call(p.root, command, cwd=cwd)
        assert code == 10, (command, err)
        assert "opaque_git" in err, (command, err)
    # Arguments the verdict does not read stay free.
    code, err = call(p.root, 'git commit -m "$(echo message)"', cwd=p.worktree)
    assert code == 0, err
    code, err = call(p.root, "git reset $(echo HEAD~1)", cwd=p.worktree)
    assert code == 0, err


def test_colon_less_fetch_and_pull_pass_in_a_story(tmp_path):
    """A colon-less fetch or pull refspec writes no local branch (FETCH_HEAD
    and the remote-tracking refs only): the usual prelude of
    ``git rebase origin/develop`` is not blocked (S4R-rules-C1-7)."""
    p = rule_projects.story_project(tmp_path)
    for command in ("git fetch origin develop", "git pull origin develop"):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 0, (command, err)
    # The guarded-branch block for a pull with a refspec stands.
    for command in ("git pull origin story/S-9.01", "git fetch . story/x:develop"):
        code, err = call(p.root, command)
        assert code == 10, (command, err)


def test_builder_forms_pass_in_a_story_worktree(tmp_path):
    """Every plain form a builder uses in a story worktree passes the rule
    (the merge-guard judgement): the whole of the must-not-block list."""
    p = rule_projects.story_project(tmp_path)
    for command in (
        "git add -A",
        "git add src/x.py",
        "git commit -m 'goes the story'",
        "git commit --amend --no-edit",
        "git status --short",
        "git diff HEAD~1",
        "git log --oneline -3",
        "git stash",
        "git stash list",
        "git restore src/x.py",
        "git rebase origin/develop",
        'git commit -F - <<MSG\nthe message\nMSG',
        f"cd {p.worktree} && git commit --allow-empty -m x",
        "git commit -m 'costs $5'",
        # A heredoc message is message text: far verbs and guarded names in
        # it are words of the message, not of the command
        # (S4R-rules-C1-1, step4-rules-fix-3-C1-1).
        'git commit -m "$(cat <<\'MSG\'\nfeat: add the branch selector\nMSG)"',
        'git commit -m "$(cat <<\'MSG\'\nfix: retry the fetch on timeout\nMSG)"',
        'git commit -m "$(cat <<\'MSG\'\ndocs: describe the checkout step\nMSG)"',
        'git commit -m "$(cat <<\'MSG\'\nfix: keep the main loop alive\nMSG)"',
        "git commit -m 'fix: retry the fetch on timeout'",
        # A here-document read by cat marks only its own segment
        # (S4R-rules-C1-1).
        "cat > notes.txt <<'EOF'\nsome notes\nEOF\ngit commit -m 'feat: add the branch selector'",
        # A quoted argument that names git is not a git call
        # (step4-rules-fix-3-C1-8).
        'grep -rn "git push" .',
        "grep -n 'git checkout' README.md",
        "grep -rn 'git branch -f' .",
        "echo 'next: git rebase onto develop'",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 0, (command, err)


def test_unplaced_commands_that_reach_into_a_project(tmp_path):
    """A not-plain git call from a directory in no factory project blocks
    when its text itself carries the reach into one — a ``-C``, a ``cd``, a
    far verb — because real git would run it there
    (S4R-rules-C1-2)."""
    p = rule_projects.story_project(tmp_path)
    neutral = tmp_path / "neutral"
    neutral.mkdir()
    root = str(p.root)
    for command in (
        f"sh -c 'git -C {root} commit -m x'",
        f"(cd {root} && git commit -m x)",
        f"env -S 'git -C {root} commit -m x'",
        f"timeout 10 git -C {root} commit -m x",
        f"git $(echo -C) {root} commit -m x",
        f"flock lock.file git -C {root} commit -m x",
        f"caffeinate -i git -C {root} commit -m x",
        f"echo $(git -C {root} push origin develop)",
        f"sh -c 'git -C {root} push origin develop'",
    ):
        code, err = call(p.root, command, cwd=neutral)
        assert code == 10, (command, err)


def test_a_cd_in_an_unreadable_segment_moves_it(tmp_path):
    """A ``cd`` word in a not-plain segment's text means the segment runs
    somewhere its directory does not name: it blocks
    (step4-rules-fix-3-C1-1)."""
    p = rule_projects.story_project(tmp_path)
    root = str(p.root)
    for command in (
        f"sh -c 'cd {root} && git commit -m x'",
        f"bash -c 'cd {root} && git commit -m x'",
        f'eval "cd {root}; git commit"',
        'git commit -m "$(cd ' + root + ' && git commit -m y)"',
        f"nohup sh -c 'cd {root} && git commit -m x'",
        f"find {root} -maxdepth 1 -exec sh -c 'cd \"$1\" && git commit -m x' _ {{}} \\;",
        f"xargs -I% sh -c 'cd % && git commit -m x'",
        f"caffeinate -i sh -c 'cd {root} && git commit -m x'",
        f"script -q /dev/null sh -c 'cd {root} && git commit -m x'",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 10, (command, err)


def test_a_heredoc_that_feeds_a_shell_is_the_commands(tmp_path):
    """A here-document a shell interpreter reads holds the commands
    themselves: its text is part of the segment, so the pushes and cds in it
    are judged (step4-rules-fix-3-C1-2)."""
    p = rule_projects.story_project(tmp_path)
    root = str(p.root)
    for command in (
        "sh <<'EOF'\ngit push origin develop\nEOF",
        f"sh <<'EOF'\ncd {root} && git commit -m x\nEOF",
        "bash <<'EOF'\ngit push origin develop\nEOF",
        "dash <<'EOF'\ngit push origin develop\nEOF",
        "ksh <<'EOF'\ngit push origin develop\nEOF",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 10, (command, err)
        assert "opaque_git" in err, (command, err)
    # A cat reading the same body reads data: the body stays out of the text.
    code, err = call(p.root, "cat <<'EOF'\ngit push origin develop\nEOF", cwd=p.worktree)
    assert code == 0, err


def test_abbreviated_and_bundled_options_are_not_plain(tmp_path):
    """An abbreviated, bundled or value-attached option of a governed verb
    makes the call not plain (step4-rules-fix-3-C1-3): on the integration
    branch it blocks."""
    p = rule_projects.story_project(tmp_path)
    for command in (
        "git switch -Cdevelop",
        "git switch --force-create=develop",
        "git switch --force-c develop",
        "git checkout -Bdevelop",
        "git checkout -fB develop",
        "git branch -qf develop HEAD",
        "git branch --forc develop HEAD",
        "git branch -f -- develop HEAD",
        "printf 'update refs/heads/main HEAD\\n' | git update-ref --stdin",
        "git push --mirr origin",
        "git push --al origin",
        "git push -- origin HEAD:develop",
        "git push origin @",
        "git update-ref --stdin",
    ):
        code, err = call(p.root, command)
        assert code == 10, (command, err)
    # From the story worktree, the same spellings cannot touch a guarded
    # branch unless they name one: push --mirr and the -- refspec do.
    for command in (
        "git switch -Cdevelop",
        "git checkout -Bdevelop",
        "git branch -qf develop HEAD",
        "printf 'update refs/heads/develop HEAD\\n' | git update-ref --stdin",
        "git push --mirr origin",
        "git push -- origin HEAD:develop",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 10, (command, err)
    # Ordinary full spellings stay plain and are judged as themselves.
    for command in ("git switch -c fresh", "git checkout -B topic"):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 0, (command, err)


def test_aliases_and_config_cannot_speak_for_git(tmp_path):
    """An aliased verb is not a verb the parse can read, a ``-c`` key that
    redirects a verb is not plain, and ``git config`` writing one of those
    keys is judged the same way (step4-rules-fix-3-C1-4)."""
    p = rule_projects.story_project(tmp_path)
    for command in (
        "git -c alias.ci=commit ci -m x",
        "git config alias.ci commit && git ci",
        "git -c 'alias.p=push origin HEAD:develop' p",
        "git -c remote.origin.push=HEAD:refs/heads/develop push origin",
        "git config remote.origin.push HEAD:refs/heads/develop && git push origin",
        "GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=alias.ci GIT_CONFIG_VALUE_0=commit git ci -m x",
        "git send-pack origin HEAD:refs/heads/develop",
        "git fast-import",
        "git filter-branch --tree-filter 'echo x' develop",
    ):
        code, err = call(p.root, command)
        assert code == 10, (command, err)
    # fast-import reads its ref writes from the standard input: never plain.
    code, err = call(p.root, "git fast-import", cwd=p.worktree)
    assert code == 10, err
    code, err = call(
        p.root, "git send-pack origin HEAD:refs/heads/develop", cwd=p.worktree
    )
    assert code == 10, err
    # An ordinary configuration write of no redirecting key passes.
    code, err = call(p.root, "git config user.email a@b.c", cwd=p.worktree)
    assert code == 0, err


def test_multi_minus_c_last_gitdir_and_function_bodies(tmp_path):
    """git threads every ``-C`` and takes the last ``--git-dir``; a cd under
    CDPATH or into a directory that does not yet exist is unknowable; a git
    call in a function body runs where the body is called
    (step4-rules-fix-3-C1-5)."""
    p = rule_projects.story_project(tmp_path)
    out = tmp_path / "out"
    out.mkdir()
    for command, cwd in (
        ("git -C . -C ../.. commit", p.worktree),
        (f"git -C . -C {p.root} commit", p.worktree),
        (f"git --git-dir={p.worktree}/.git --git-dir={p.root}/.git commit", p.worktree),
        (f"export CDPATH={tmp_path}; cd {p.story} && git commit -m x", p.root),
        (f"ln -s {p.root}/.git {out}/g && git --git-dir={out}/g commit -m x", p.worktree),
        (f"git worktree add -f {out}/x develop && cd {out}/x && git commit -m x", p.worktree),
        (f"f() {{ git commit -m x; }}; cd {p.root} && f", p.worktree),
    ):
        code, err = call(p.root, command, cwd=cwd)
        assert code == 10, (command, err)


def test_more_spellings_of_a_git_environment_export(tmp_path):
    """Every assignment of GIT_DIR or GIT_WORK TREE before a program poisons
    the later git-bearing segments: a declaration, a special builtin, an
    eval (step4-rules-fix-3-C1-6)."""
    p = rule_projects.story_project(tmp_path)
    for command in (
        f"eval export GIT_DIR={p.root}/.git; git commit -m x",
        f"GIT_DIR={p.root}/.git : ; git commit -m x",
        f"declare -x GIT_DIR={p.root}/.git; git commit -m x",
        f"readonly GIT_WORK_TREE={p.root}; git commit -m x",
        f"typeset GIT_DIR={p.root}/.git; git commit -m x",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 10, (command, err)
        assert "where_unknown" in err, (command, err)


def test_guarded_names_answer_to_letter_case_on_macos(tmp_path):
    """On macOS a guarded branch is answered to in any letter case
    (step4-rules-fix-3-C1-7)."""
    import sys

    if sys.platform != "darwin":
        import pytest

        pytest.skip("macOS case-insensitive file system")
    p = rule_projects.story_project(tmp_path)
    for command in ("git branch -f Develop HEAD", "git update-ref refs/heads/Develop HEAD"):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 10, (command, err)
        assert "moves_guarded_branch" in err, (command, err)


def test_unreadable_calls_name_their_reach_in_the_worktree(tmp_path):
    """The position test's own clauses decide worktree cases no other clause
    decides (step4-rules-fix-3-C1-10): a redirect in the text, a guarded
    name in the text, and the fix line on every block."""
    p = rule_projects.story_project(tmp_path)
    root = str(p.root)
    for command in (
        f"sh -c 'git -C {root} commit -m x'",
        f"env -S 'git --git-dir={root}/.git commit -m x'",
        f"sh -c 'git log develop'",
        f"echo $(git log develop)",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 10, (command, err)
        assert "opaque_git" in err, (command, err)
        # The fix line of every block (step4-rules-fix-3-C1-10).
        assert "by itself, plainly, from the story worktree" in err, (command, err)
        assert "vsdd story deliver" in err, (command, err)
        assert "vsdd release" in err, (command, err)


def test_env_split_string_is_opaque(tmp_path):
    """env's -S/--split-string value is the command env runs: every spelling
    makes the segment opaque, so the call is judged where it lands
    (S4R-rules-7). On the guarded checkout the commit blocks; the plain
    commit in the worktree passes."""
    p = rule_projects.story_project(tmp_path)
    for command in (
        'env -S "git commit -m x"',
        "env -S'git commit -m x'",
        'env --split-string "git commit -m x"',
        'env --split-string="git commit -m x"',
        'env -S "git push origin develop"',
        'env -S "git commit -m x" extra args',
    ):
        code, err = call(p.root, command)
        assert code == 10, (command, err)
        assert "opaque_git" in err, (command, err)
    for command in (
        'env -S "git commit -m x"',
        "env -S'git commit -m x'",
        'env --split-string "git commit -m x"',
        'env --split-string="git commit -m x"',
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 0, (command, err)
    # The push form names the guarded branch: it blocks everywhere.
    code, err = call(p.root, 'env -S "git push origin develop"', cwd=p.worktree)
    assert code == 10, err
    # The same spellings hide a sed from write_targets no longer: the
    # unreadable write is judged where it lands (S4R-rules-7).


def test_substitutions_anywhere_are_not_plain(tmp_path):
    """A dollar sign or a backtick in any word of the call but a free-text
    option's value makes it not plain (S4R-rules-C1-1): it blocks on the
    guarded checkout, and position decides in the story worktree."""
    p = rule_projects.story_project(tmp_path)
    blocked_worktree = (
        "git push origin $(echo HEAD:develop)",
        "git fetch origin $(echo topic:develop)",
        "git push $(echo origin develop)",
        "git push $ARGS",
        "git push $(echo --all)",
        "git branch $(echo -f develop) HEAD",
        "git branch $FLAGS develop",
        "git switch $(echo -C develop)",
        "git checkout $(echo -B) develop",
        "git fetch $(echo origin topic:develop)",
        "git rebase $(echo origin/develop develop)",
    )
    for command in blocked_worktree:
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 10, (command, err)
    # On the guarded checkout every one of these blocks, an expanded verb
    # included; from the story worktree the harmless ones pass, position
    # alone deciding (S4R-rules-C2-2). The message substitutions read opaque
    # (the parser does not enter them), the rest only not plain: the verdict
    # is the same from the worktree — but a substitution inside a free-text
    # value leaves the call plain: the commit is judged as itself, and the
    # substitution runs as its own harmless command (S4R-rules-C1-1).
    not_plain = (
        "git $(echo commit) -m x",
        "git $V -m x",
        "FOO=$x git commit -m x",
    )
    for command in not_plain:
        code, err = call(p.root, command)
        assert code == 10, (command, err)
        assert "opaque_git" in err, (command, err)
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 0, (command, err)
    code, err = call(p.root, "git commit --message=$(echo note)")
    assert code == 10, err
    assert "commit_on_guarded" in err, err
    code, err = call(p.root, "git commit --message=$(echo note)", cwd=p.worktree)
    assert code == 0, err
    # A dollar in a free-text option's value is plain: the commit is judged
    # as itself.
    for command in (
        "git commit -m 'costs $5'",
        'git commit --message="costs $5"',
        "git commit -F $NOTEFILE",
    ):
        code, err = call(p.root, command)
        assert code == 10, (command, err)
        assert "commit_on_guarded" in err, (command, err)
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 0, (command, err)


def test_global_option_table_is_one(tmp_path):
    """The separate-value globals live in one table, --namespace,
    --config-env and --attr-source in it, --exec-path out of it
    (S4R-rules-C2-1): the value is consumed and the call judged."""
    p = rule_projects.story_project(tmp_path)
    for command in (
        "git --namespace foo commit -m x",
        "git --config-env foo=bar commit -m x",
        "git --attr-source HEAD commit -m x",
        "git --namespace foo -C %s commit -m x" % p.root,
        "git --attr-source HEAD -C %s commit -m x" % p.root,
    ):
        code, err = call(p.root, command)
        assert code == 10, (command, err)
        assert "commit_on_guarded" in err, (command, err)


def test_wrappers_are_read_or_covered(tmp_path):
    """The wrapper table knows the common runners with their option grammar
    (S4R-rules-C2-2), and a wrapper outside it is covered by the not-plain
    ruling: both block on the guarded checkout."""
    p = rule_projects.story_project(tmp_path)
    for command in (
        "timeout 10 git commit -m x",
        "timeout -s TERM 10 git commit -m x",
        "timeout --preserve-status 10 git commit -m x",
        "stdbuf -o 0 git commit -m x",
        "setsid git commit -m x",
        "setsid --wait git commit -m x",
        "ionice -c 2 git commit -m x",
        "sudo git commit -m x",
        "sudo -u root git commit -m x",
        "timeout 60 git push origin develop",
        # wrappers the table does not know: the class is closed
        "flock lock.file git commit -m x",
        "xargs -n1 git commit -m x",
    ):
        code, err = call(p.root, command)
        assert code == 10, (command, err)
    # In the story worktree the plain commit through a known wrapper passes,
    # judged at the wrapper's own place.
    for command in (
        "timeout 10 git commit -m x",
        "stdbuf -o 0 git commit -m x",
        "setsid git commit -m x",
        "ionice -c 2 git commit -m x",
        "sudo git commit -m x",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 0, (command, err)
    # An unknown wrapper's commit is harmless in the worktree — the words run
    # there, and the text names no guarded branch.
    code, err = call(p.root, "flock lock.file git commit -m x", cwd=p.worktree)
    assert code == 0, err


def test_a_read_wrapper_decides_the_verdict(tmp_path):
    """Only a wrapper the table reads decides these: a push of the story
    branch through timeout passes in the worktree, while the same wrapper
    carrying a commit into the main checkout blocks, through the ``cd`` and
    through env's own ``--chdir`` (S4R-rules-C2-2)."""
    p = rule_projects.story_project(tmp_path)
    code, err = call(p.root, f"timeout 10 git push origin story/{p.story}", cwd=p.worktree)
    assert code == 0, err
    for command in (
        f"cd {p.root} && timeout 10 git commit -m x",
        f"env --chdir={p.root} git commit -m x",
        f"sudo -D {p.root} git commit -m x",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 10, (command, err)


def test_exported_git_dir_poisons_the_rest(tmp_path):
    """An export or a bare assignment of GIT_DIR or GIT_WORK_TREE in an
    earlier segment makes every later git-bearing segment where_unknown
    (S4R-rules-C1-2)."""
    p = rule_projects.story_project(tmp_path)
    for command in (
        f"export GIT_DIR={p.root}/.git && git commit -m x",
        f"export GIT_WORK_TREE={p.root} && git commit -m x",
        f"GIT_DIR={p.root}/.git && git commit -m x",
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == 10, (command, err)
        assert "where_unknown" in err, (command, err)
    # A redirect in one segment poisons only the later ones.
    code, err = call(
        p.root, f"git --git-dir={p.root}/.git commit -m x && git status",
        cwd=p.worktree,
    )
    assert code == 10, err


def test_judged_at_a_linked_worktrees_git_dir(tmp_path):
    """A git directory is read for the branch it itself has checked out: the
    main checkout's for ``<root>/.git``, the story's for
    ``<root>/.git/worktrees/<name>`` (S4R-rules-C1-2)."""
    p = rule_projects.story_project(tmp_path)
    worktree_git_dir = f"{p.root}/.git/worktrees/{p.story}"
    for command, want in (
        (f"git --git-dir={p.root}/.git commit -m x", 10),
        (f"git --git-dir={worktree_git_dir} commit -m x", 0),
        (f"git --git-dir={worktree_git_dir} push origin story/{p.story}", 0),
    ):
        code, err = call(p.root, command, cwd=p.worktree)
        assert code == want, (command, err)
    # A directory in no factory project is judged when its git directory
    # names a place inside one (S4R-rules-C1-2): the project and the branch
    # come from the git directory.
    outside = tmp_path / "elsewhere"
    outside.mkdir()
    for command, want, code_name in (
        (f"cd {outside} && git --git-dir={p.root}/.git commit -m x", 10, None),
        (f"cd {outside} && git --git-dir={p.root}/.git push origin HEAD", 10,
         "push_to_guarded"),
        (f"cd {outside} && git --git-dir={p.root}/.git update-ref refs/heads/develop HEAD",
         10, "moves_guarded_branch"),
        (f"git -C {outside} --git-dir={p.root}/.git --work-tree={outside} commit -m x",
         10, None),
        (f"cd {outside} && git --git-dir={p.root}/.git commit -m x", 10, None),
    ):
        code, err = call(p.root, command, cwd=outside)
        assert code == want, (command, err)
        if code_name:
            assert code_name in err, (command, err)
    # A git directory in no factory project is judged by none.
    other = tmp_path / "other-repo"
    other.mkdir()
    code, err = call(p.root, f"git --git-dir={other} commit -m x", cwd=p.worktree)
    assert code == 0, err
