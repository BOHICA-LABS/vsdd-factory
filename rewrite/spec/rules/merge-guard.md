---
kind: rule
id: merge-guard
package: H5
sources: [hooks.md#2.10, hooks.md#2.11.4, BLUEPRINT.md#5.4]
---

# merge-guard

Implements `pipeline.md` guard G2 and `packaging.md` need H5: no agent merges into, or pushes to, the
integration or release branch except through the tool.

| Registration | |
|---|---|
| Kind | gate |
| Runs on | `PreToolUse`, tool `Bash` |
| Result | `Block` |
| Fails | closed — "the actions are irreversible" |
| Relax | yes |

## What it checks

The rule reads the tool call's text and decides in two steps; it does not enumerate spellings.

A segment ([[topic:rule-model]]) is *git-bearing* when one of its words, after quote removal, is `git`
or ends in `/git` — for an opaque segment, when its text, which carries what the segment stands for,
holds `git` as a word; a parsed segment whose argument is a command string of its own (a `find -exec`
or `sh -c` payload carrying a shell joiner) is read the same way, its words running as typed. A
segment that is not git-bearing is not judged: `grep -rn "git push" .` is not a git call.

A git-bearing segment is *plain* when its program is git itself, reached directly or through a wrapper
whose option grammar the parser's wrapper table knows; every global option before the verb is in the
one table of git's separate-value globals (`-C`, `-c`, `--git-dir`, `--work-tree`, `--namespace`,
`--super-prefix`, `--config-env`, `--attr-source` — `--exec-path` takes no separate value); the verb
is a literal word of git's own command list, so an aliased verb (`ci` for `commit`) is not plain; and
no word of the command holds a dollar sign or a backtick, except inside the value of a free-text
option (`-m`/`--message`, `-F`/`--file`). A governed verb's options must each be one its own option
table knows, spelled out in full: an abbreviated spelling (`--forc`), a bundle holding a governed
letter (`branch -qf`), an unknown option (`update-ref --stdin`), or a value attached to a
branch-resetting letter (`-Cmain`, `-Bmain`, `--force-create=main`) makes the call not plain. A `-c`,
`--config-env` or `GIT_CONFIG_*` key that redirects what a verb means — `alias.*`, `push.*`,
`remote.<name>.push` — and a `git config` write of such a key are not plain either. A plain segment
is judged as below, at the place git would act: the git directory when `--git-dir` or `GIT_DIR` names
one — the last `--git-dir` given, the project found from that directory (symlinks resolved), the main
checkout's root for `<root>/.git` and for `<root>/.git/worktrees/<name>`, and HEAD read from that
directory itself — else the segment's own directory, which every `-C` threads in order (the last
standing) and a wrapper's `--chdir` sets; a `--chroot` leaves the directory unknowable. A
substitution inside a free-text value runs as its own command and is judged under the same rules. A
segment whose own directory is in no factory project is still judged when one of those names a place
inside one.

A git-bearing segment that is not plain is not read further: it blocks with `opaque_git`, unless
position alone shows it harmless — its directory is known and inside a factory project, the branch
checked out there is not guarded, and its text names no guarded branch (by letter case too on macOS,
whose file system answers `Main` for `main`), no `-C`, no `--git-dir`, no `--work-tree`, no `GIT_DIR`
or `GIT_WORK_TREE`, no `cd`, `pushd` or `popd` word, and none of the verbs that can reach another
branch from anywhere (`push`, `fetch`, `pull`, `update-ref`, `symbolic-ref`, `branch`, `switch`,
`checkout`, `rebase`, `worktree`, `send-pack`, `fast-import`, `filter-branch`). A segment in no
factory project blocks when its text itself carries a reach into one: a `-C`, a `--git-dir` or
`--work-tree`, a `GIT_DIR`/`GIT_WORK_TREE` spelling, a `cd`, or a far verb. So a builder's
`timeout 60 git commit -m x` passes in a story worktree and blocks where the integration branch is
checked out; `timeout 60 git push origin develop` blocks everywhere. A here-document a shell
interpreter (`sh`, `bash`, `dash`, `zsh`, `ksh`) reads is the commands themselves and rides the
segment's text; the body `cat`, `tee` or `git commit -F -` reads is data and does not. An export, a
declaration or a bare assignment of `GIT_DIR` or `GIT_WORK_TREE` before a program that is not that
segment's own git call — and an opaque segment whose text names either — makes every later
git-bearing segment `where_unknown`, which blocks. A git call in a `{ ... }` body — a function
definition — runs where its function is called, a place the definition's directory does not give, and
is not plain.

The plain judgements, at the place git would act:

- a tool call that runs `gh pr merge` (`direct_merge`);
- `git merge`, `git rebase`, `git cherry-pick`, `git revert` or `git am` while the integration or
  release branch is checked out (each adds a commit) (`integrate_on_guarded`);
- `git commit` (including `--amend`) and a `git reset` that moves the branch while one of those
  branches is checked out, which matters most in local delivery mode, where the integration branch is
  the product branch (`commit_on_guarded`);
- `git push` and `git send-pack` whose target is one of those branches, including `HEAD:<branch>`
  forms, `@` for the checked-out branch, and a bare push from one of them (`push_to_guarded`);
- a command that moves, replaces or deletes one of those branches without checking it out —
  `git update-ref` or `git symbolic-ref` naming it, a `git branch` mutator naming it, a
  `git switch -C` or `git checkout -B` creating or resetting it, and a `git fetch`, `git pull` or
  `git push` refspec whose destination is it (`git fetch . story/S-1.02:develop`)
  (`moves_guarded_branch`);
- `git filter-branch` while one of those branches is checked out, or naming one
  (`integrate_on_guarded`, `moves_guarded_branch`); `git fast-import` writes refs from the standard
  input, where the parse cannot follow, and is never plain.

A segment in no factory project is not judged, while a plain git call whose directory is unknowable
blocks `where_unknown`. Every block's fix line says to run the git command by itself, plainly, from the
story worktree, and names the governed subcommands (the constant `GOVERNED`: `story deliver`,
`release`); for the main session it names `/vsdd-factory:feature`.

The rule applies to every caller, the main session included, whatever command families the caller's row
grants.

There is no `vsdd pr-merge` subcommand: merging is a library function inside `story deliver`, so no
caller can merge with only a head-commit check.

The checks that make a merge legitimate are the tool's, in code: the list under "What it refuses" in
[[command:story-deliver]]. A `gh` or `git` process the tool starts is not a tool call and is not seen by hooks.

## Limit

The rule reads the text of a tool call and cannot see git run by a script, make or an interpreter; what
is merged is guaranteed by delivery's own checks ([[command:story-deliver]]). A guard at the ref itself
is an open decision (ORCH-7).

## Boundary with bash-git

The command-family grants live in [[table:command-families]] and [[rule:bash-git]]; whatever commits
to, merges into, pushes to or moves the integration and release branches is this rule's subject. A person
may still commit to their own integration branch in a terminal: hooks see tool calls only.
