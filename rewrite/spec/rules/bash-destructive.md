---
kind: rule
id: bash-destructive
package: H2
sources: [hooks.md#2.10, hooks.md#2.11.1]
---

# bash-destructive

Carries over the old `destructive-command-guard`, with the same reason codes so the old test cases can
be harvested.

| Registration | |
|---|---|
| Kind | gate |
| Runs on | `PreToolUse`, tool `Bash` |
| Result | `Block` |
| Fails | closed — "the actions are irreversible" |
| Relax | yes; `allow_pr_close = true` is the shown key (hooks.md 2.6) |
| Spike exemption | no (the safety rules still apply inside a spike) |

## What it checks

It works on a parsed command (segments, tokens, redirect targets), not the whole string — this removes
the old false positives (a commit message that mentions `git reset --hard` was blocked) and the old
misses (`rm -r -f`, `rm --recursive`).

Blocked:

- Recursive `rm` of `/`, `~`, `$HOME`, `*`, `.*`.
- Recursive delete (`rm -r`, `find -delete`, `find -exec rm`) under the state directory or the
  project's `source` and `tests` classes, except build output and worktree directories.
- Delete, truncate or clobbering redirect of a state file.
- `git reset --hard`, `git clean -f` (not a dry run), `git checkout -- .`, `git restore .`,
  `git stash drop` and `clear`.
- `git branch -D` of a protected branch, `git filter-branch`, `git filter-repo`,
  `git reflog expire --expire=now`, `git gc --prune=now`, `git worktree remove --force` outside the
  worktree roots.
- `gh repo delete`, `gh release delete`, `gh issue delete`, `gh pr close`.
- `curl`, `wget` or `fetch` piped into a shell or interpreter.
- Recursive `chmod` or `chown` on protected paths.

Inside a spike worktree the "discard work" commands (`git reset --hard`, `git clean -f`,
`git checkout -- .`, `git restore .`, `git stash drop`) are allowed when the command has no `cd`, `-C`,
`--git-dir` or `--work-tree` that points elsewhere, because a spike worktree is disposable.

## Siblings

Command families are in [[table:command-families]]; the git-side rule is [[rule:bash-git]]; shell
writes into the state directory are [[rule:write-scopes]] check 4.
