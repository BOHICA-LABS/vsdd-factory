---
kind: topic
id: worktrees
package: S3
sources: [packaging.md#2.10, state.md#2.16, state.md#2.17, BLUEPRINT.md#4.1, BLUEPRINT.md#4.6]
---

# The worktree library

"(library, no subcommand) worktrees. One library, used by `vsdd survey` (create the story's worktree and
branch, or return the existing one), by the two commands that admit a story written after the survey
(`vsdd gate plan --story`, `vsdd story new-fix`), and by `vsdd story deliver` (verify the head, remove the
worktree). One creator, so paths cannot disagree. It is built in package S3."

"`vsdd pr-merge`, `vsdd worktree`, `vsdd cache` and `vsdd record-test` do not exist. Merging is a library
function inside `story deliver`. Worktree creation is one library used by `survey` and by the two admission
commands, so there is one merge route and one creator of worktrees."

Paths, from the blueprint's name table:

- `.worktrees/<story-id>/` for a story
- `.worktrees/_integration/` for wave and release gates (`.worktrees/_integration/<repo>/` in a hub)
- `.worktrees/spike-<name>/` for a named spike
- quick spikes use a platform worktree

Branches. `story/<id>`, `spike/<name>`, and `hotfix/<version>` for a critical fix cut from a release tag;
the integration and release branch names come from `[branches]` in the project configuration.

Removal. Story worktrees are removed by [[command:story-deliver]] at the merge. `status --fix` "removes
spike worktrees past retention and worktrees of delivered stories" ([[command:status]]). The retention
rule: "the ledger entry lists the worktree and branch; `vsdd checkpoint` and `vsdd status --fix` remove
spike worktrees older than `spikes.retain_days` (14) and those whose asking unit is delivered; the branch
`spike/<name>` of a named spike is kept until the spike is closed" ([[config:spikes]]).

Admission creates the branch and worktree together, with this library and under the run's lock
([[command:gate-plan]], [[command:story-new-fix]], [[topic:story-admission]]).

## The base of a story branch

A story branch is cut from the head of the branch the story will be delivered to, read at the moment the
branch is created. That target is the integration branch (in `pr` mode its remote head after a fetch), or in
a hotfix cycle `hotfix/<version>`, which the library creates at the release tag the cycle's `baseline` names
when the cycle's first story is admitted. In `pr` mode the fetch takes its remote from the target branch's
upstream and never names a remote in code; an upstream whose ref is not the target branch, and a missing
upstream with zero or several remotes, are refused with the branch and the git command that corrects them.
Admission records the target branch and that commit as `delivery.branch` and `delivery.base` in the story's
closure record ([[record:closure]]) when it creates the branch; the commit is the `base` of the story unit
([[schema:story-unit]]). The descent check every gate and
delivery make, the red gate's comparison report and the green gate's diff all use the recorded commit, never
a branch name, so stories delivered meanwhile do not change what a story is measured against.

## The integration worktree

`.worktrees/_integration/` is always on a detached head, because the integration branch is normally checked
out in the person's own checkout and git checks a branch out in one worktree only. Each command that uses it
(the wave and release gates, and local delivery in [[command:story-deliver]]) first sets it to the commit it
needs. Nothing but local delivery ever moves a branch from it.

## Team use (decision D33; built when the build widens)

A story branch is created at a commit id, so git gives it no upstream. It is pushed under its own name
to the remote of the branch it will be delivered to, and no remote's name is in code. A fetch of a
target branch or of a story branch goes into its remote-tracking ref, never into a local branch
([[table:sync-checks]], T-REMOTE). A story branch that holds a hand commit is never rebased, and the
one push that may replace commits on the remote names the head it expects
([[table:team-delivery-checks]], T-NEVER).
