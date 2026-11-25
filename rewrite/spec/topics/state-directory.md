---
kind: topic
id: state-directory
sources: [state.md#1, state.md#2.2, state.md#decision-keep-the-separate-branch-and-take-every, state.md#mounting, BLUEPRINT.md#5.2]
---

# The state directory and its storage

In 1.0 a project's factory state is still one directory, `.factory/`, kept on its own git branch and
mounted as a worktree. What changes is who does the work. Code mounts it, validates every write before it
lands, commits it and pushes it. No agent keeps state. The lock lease and the state-manager agent are gone,
and the person's own checkout is never switched, pulled or committed to. The one thing the tool does there is
the fast-forward that hands over a local delivery ([[command:story-deliver]]).

The rules of storage:

1. One state directory, independent of every product branch and worktree.
2. Each fact is stored in one place. A link is stored on the artifact that depends on the other, and the
   reverse view is derived ([[topic:stored-and-derived]]).
3. There are two kinds of file. Agents write prose artifacts, and code validates them before the write.
   Code writes records, and no agent can write them. See [[topic:artifact-rules]] and
   [[topic:machine-records]].
4. Every file has a size cap that is checked before the write. Nothing is compacted afterwards, because
   nothing is allowed to grow.
5. Nothing derived is stored, and nothing stored is updated after the fact. No commit id, count or version
   is ever patched to match something else.
6. Everything needed to resume is on disk at each step boundary. Tracked state is enough to resume on
   another machine; the local store only saves repeating work on this one.
7. The subsystem costs an agent nothing until it is used: one model-visible skill, no index to load, no
   state file an agent must read before working.

Rules 3 and 4 are carried out by the artifact rules ([[topic:artifact-rules]]) and the machine records
([[topic:machine-records]]); rule 5 is the stored-versus-derived division ([[topic:stored-and-derived]]).

## Decision: keep the separate branch, and take every chore away from agents and people

The old design kept state on an orphan branch (`factory-artifacts`) mounted as a git worktree at
`.factory/`. That storage is kept and everything around it is replaced. Three alternatives were weighed.

| | A. Separate branch mounted as a worktree (chosen) | B. Tracked directory on the integration branch | C. Tracked directory on every work branch, so state travels with code |
|---|---|---|---|
| The person's checkout | Never switched, pulled or committed to (local delivery fast-forwards it) | Must stay on the integration branch, and state commits land in it | Free, but every worktree holds its own copy of the state |
| One state directory | Yes, whatever branch any checkout is on | Yes, if story worktrees leave the directory out | No. The right copy depends on which unit the work belongs to |
| Product history and pull requests | No state in either | One state commit per sync on the integration branch | State files in every story pull request |
| Protected branches | Unaffected | The integration branch must accept direct pushes | Works |
| Hidden scenarios | Outside the product tree, and skipped by search tools because the directory is excluded | Inside the product tree and inside packages built from it | In every worktree |
| A fresh clone | One command mounts the state | Works at once | Works at once |
| Specification and code change in one commit | No | No | Yes |

B and C save the mount. In exchange B constrains the person's checkout for as long as the project lives,
and C gives every hook rule and every prompt the job of working out which copy is the right one. The
pipeline design leaves the person's checkout alone, apart from the fast-forward of local delivery, and the
hooks and agents-skills designs both assume a single state directory. A is the only
option that satisfies all three without a standing constraint.

The justification against low touch is that the branch was never what cost the effort. The chores around it
were, and each one moves into code: creating, mounting and repairing the branch is [[command:init]] and
[[command:status]] ([[topic:state-commits]]); the lock lease is the local run lock
([[topic:run-lock]]); committing is `flush` ([[topic:state-commits]]); pushing is part of
[[command:checkpoint]]; guarding the branch is one precondition checked by file reads
([[topic:hooks-inert]]); the `STATE.md` commit-id citations and the `.gitignore` edit become a trailer on
each state commit and two lines in `.git/info/exclude`, written at mount. What is left for a person: run
`/vsdd-factory:setup` once per clone.

## Mounting

- Location: `<project root>/.factory`. Branch: `factory-artifacts`. The name is kept because the eight
  existing projects, the hook design's precondition and every document already use it.
- `vsdd init` (run by the `setup` skill) does one of three things. With no state branch anywhere it creates
  the orphan branch with a first commit holding `factory.toml`, `notes.json` and `.gitignore`, and mounts
  it. With a state branch on the remote or locally it mounts that. With pre-1.0 state it converts nothing:
  it reports `legacy` and prints the dry-run command and then the `vsdd import` command, which only a person
  may run ([[topic:importer]]). There is no status record to build: the briefing is computed
  ([[topic:status-report]]).
- It writes `.factory/` and `.worktrees/` into `.git/info/exclude`. Search tools that honour git exclusions
  then skip the state directory in a search from the project root, which answers `agents-skills.md`
  N-STATE-8 for every search tool that honours them. The hook design's read walls still cover the rest.
- The state branch's own `.gitignore` contains `local/`.
- Story and spike worktrees are worktrees of product branches, so they contain no `.factory/` at all. A
  `.factory` path inside a linked worktree is always a mistake, and the hook design's `write-scopes` check 5
  blocks it and prints the right path.

## The five project conditions

How code tells the conditions apart, with file reads only:

| Condition | Test |
|---|---|
| Not a factory project | Neither `.factory/factory.toml` nor `.factory/STATE.md` exists. Hooks are inert and silent. |
| Pre-1.0 state | `.factory/STATE.md` exists and `.factory/factory.toml` does not. Hooks are inert except for one briefing line: "Pre-1.0 factory state found. Run /vsdd-factory:setup to import it." |
| 1.0, ready | `.factory/factory.toml` exists, and `.factory/.git` is a file pointing at a worktree whose `HEAD` is `refs/heads/factory-artifacts`. An invalid file is reported by the loader; the mount test itself reads no configuration. |
| 1.0, partial | The configuration file is there but the mount test fails. All gates run, state writes are blocked, and the briefing names `/vsdd-factory:status --fix`, which remounts. |
| State exists but is not mounted | `.factory/` is absent and `refs/heads/factory-artifacts` or `refs/remotes/origin/factory-artifacts` exists. `vsdd doctor` and `setup` report it. Hooks are silent there (decision D24). |

The function is `vsdd.state.detect(root)` and returns `none`, `legacy`, `ready`, `partial` or `unmounted`.
These are the five conditions every design uses. `partial` has one cause, a broken mount.

## What this design provides

`vsdd.state.detect` and the five conditions are one of the interfaces the state design provides to the
others; the full list is in [[topic:seams-state]]. The layout of the directory is [[topic:state-layout]].

## Team use (decision D33; built when the build widens)

Not on the thin slice path. Three things in this file change when it is built. The state branch on a
remote is found through the remotes git lists, never by the name `origin`. On a clone that did not
create the state, `vsdd init` joins the team when team use is on, and otherwise mounts as it does today
and says that another clone wrote the state; `detect` keeps its five conditions
([[table:team-checks]], T-JOIN). And rule 6 holds as written: what another machine resumes from is
tracked state and the branches on the remote ([[topic:team-records]]).
