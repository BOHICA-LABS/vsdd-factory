---
kind: command
id: story-deliver
package: G4a
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.1, BLUEPRINT.md#4.13, packaging.md#2.10]
---

# `vsdd story deliver`

The only merge onto the integration branch: verdict check, push, pull request, CI wait, merge and worktree
removal.

## Syntax

`vsdd story deliver --story <id> --run <id>`. A job command "keyed ... by the story"
([[topic:long-commands]]), run by a clerk who "waits on [it] by running the same command again".

## What it does

"Does everything mechanical, as one detached job ... push, open the pull request with a body rendered from
the story and its evidence and a trailer naming the story (`Factory-Story: S-7.03`) (or adopt one that is
already open), wait for CI, merge under a file lock, remove the worktree. It rebases only when the pull
request conflicts with the integration branch." The governed merge passes `--match-head-commit`, "and only
when that head's patch id equals the one the verdict covers" ([[topic:governed-delivery]]).

Agents cannot merge or push to the integration branch any other way ([[rule:merge-guard]]). "Delivery never
force-pushes the integration or release branch. After a rebase it pushes the story's own branch with
`--force-with-lease`. It never skips git hooks, and its commits carry no AI attribution."

Conflict repair: before the repair step runs ([[topic:story-script]]), the tool brings the integration
branch's head into the story branch itself (a merge on the story branch, pushed with `--force-with-lease`
where the branch has one). This is why the builder wall rows hold only `git-commit` and `git-read`
([[table:walls]]): the repair needs no `git-integrate` grant and no new command family, and the least
privilege of the builder rows is unchanged.

The target branch is the branch the story's base was read from ([[topic:worktrees]]): the integration
branch, or `hotfix/<version>` in a hotfix cycle.

## What it refuses

The tool merges only when every one of these holds, and returns `refused` naming the first that does not:

- the story is recorded in the run it is given, its worktree exists at the recorded path, and its branch
  descends from the recorded base ([[topic:worktrees]]);
- for a story whose kind has a red gate ([[table:story-kinds]]), a passing red or re-freeze record exists on
  a commit between the base and the head ([[topic:evidence-lookup]]);
- green evidence covers the head;
- a converged verdict covers the head ([[topic:verdicts]]);
- the demo gate holds ([[command:gate-demo]]);
- in `pr` mode with `delivery.ci = "required"`, CI is green. With `delivery.ci = "none"` (the repository has
  no CI, recorded by `vsdd init`) it does not wait for checks; only then does "no checks" count as a pass.

Verdict binding: "The verdict and the green evidence are bound to the head commit and to the `git patch-id`
of the story's diff. After a clean rebase the patch id is unchanged, so both still hold and the tool runs
the build command only. After a repair, or when a rebase changed the patch, the review loop runs once more
with a cap of 2 over only the new commits" ([[topic:verdicts]]).

## Local mode

With `delivery.mode = "local"` (no remote) there is no push, pull request or CI wait, and `pr` is empty. The
merge is made away from every checkout a person works in and then handed to the target branch:

1. Under the delivery file lock the tool reads the head of the target branch, sets the integration worktree,
   which is on a detached head ([[topic:worktrees]]), to that commit, and merges the story branch there with
   the configured merge strategy. Git hooks run and the commit carries no AI attribution. A conflict leaves
   that worktree reset and returns `conflict`.
2. The tool then moves the target branch from the head it read to the new commit, and only from that head.
   When no worktree has the branch checked out it runs `git update-ref refs/heads/<branch> <new> <old>`.
   When a worktree has it checked out, which with no remote is normally the person's own checkout, it runs
   `git -C <that worktree> merge --ff-only <new>`: git moves the branch, the index and the files together,
   keeps uncommitted changes the merge does not touch, and refuses, changing nothing, when it would
   overwrite one or when another git operation is in progress there.
3. If the branch moved after step 1 (a person committed to it), the tool starts again from step 1.
4. If git refuses the fast-forward, delivery returns `checkout_dirty` with `ok: false` and an `error` that
   names the checkout and the fix: commit or stash the changes there, then run the story again. Nothing has
   moved, the story stays undelivered, and the next delivery repeats the merge.

That fast-forward is the one thing the tool ever does in a checkout it does not own. `vsdd release` moves
the release branch the same way in local mode ([[command:release]]).

The delivery tests cover, in local mode: the target branch checked out in the person's checkout and clean;
with an uncommitted change the merge would overwrite; with an uncommitted change it does not touch; checked
out nowhere; and moved by a person between step 1 and step 2.

## Who may call it

`clerk` ("deliver-<n>: clerk running `vsdd story deliver`").

## Writes

"The pull request (with the story trailer), the merge with the head-commit match, the `pr.created` and
`pr.merged` events, the story's delivery closure, which is what makes the story done"
([[record:closure]], [[topic:event-log]]); worktree removal via the worktree library
([[topic:worktrees]]).

## Output, exit behaviour and messages

"Delivery data with `diff_patch`" for the range from the reviewed commit to the head
([[schema:delivery-data]]). Its `state` is one of:

- `merged`;
- `awaiting_merge`: "a person must approve; the checkpoint opens a `merge` decision with the link, and once
  it is approved the next `vsdd story deliver` for the story merges" ([[topic:governed-delivery]]);
- `ci_failed`, `conflict`;
- `review_stale`: "the story's patch changed since the converged review";
- `refused`: something under "What it refuses" does not hold, named in `error`;
- `checkout_dirty`: local mode only, as above. The workflow ends `blocked` with the `error` line as its
  summary, and the checkpoint treats it as a blocked step of kind `other` ([[topic:human-stops]]): no
  decision opens the first time, and running the story again once the checkout is clean delivers it.

`ok` says whether the command ran and reached a state the script can act on: `merged`, `awaiting_merge`,
`ci_failed`, `conflict` and `review_stale` are `ok: true` (each starts the delivery repair loop's next
move in [[topic:story-script]]), while `refused`, `checkout_dirty` and an empty `state` — the command
itself could not run — are `ok: false`. The schema's one-line rule:
[[schema:delivery-data]].

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. Package F3 adds two sections to the pull request body: "Rollback", in every
pull request of every project, and "Feature flags", for a story that names a flag
([[topic:flag-lifecycle]]). A flag adds no refusal and no wait to this command.

## Team use (decision D33; built when the build widens)

Not on the thin slice path, and local mode is untouched. In `pr` mode the host is reached through one
module, which opens, reads, merges and closes a pull request and answers whether it can serve a remote.
Nothing else in the tool names the host's tool, and no configuration key names the module.

With team use on or off: the branch is pushed under its own name to the remote of its target, never
to an upstream ref (T-REMOTE); commits that arrived on it are brought in, where T-BRANCH's `runs`
cell has the remote asked, and a push of it is judged by a fetch, not by its exit status (T-BRANCH); a branch that holds a hand commit is never rebased or forced; and where this
file says `--force-with-lease`, the lease names the head the tool expects (T-NEVER). The closure gains
`delivery.pushed`, `delivery.merged_by`, `delivery.landed` and `delivery.hand_commits`, and the result
gains the state `queued` (T-DELIVERY).

On a team the command also syncs first and refuses a story whose claim this clone does not hold. The
checks are [[table:team-delivery-checks]] and [[table:sync-checks]].
