---
kind: command
id: release
package: G7
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.7]
---

# `vsdd release`

One idempotent command that prepares and then publishes a release; the only route onto the release branch.

## Syntax

`vsdd release --run <id>`. A job command ([[topic:long-commands]]).

## What it does

"One idempotent command that does the next missing part" each time it is run.

- Prepare: "the release steps from `[release]` in project configuration, which are the version (computed
  from the route) written to `version_files`, the change log (rendered from the delivered stories), and the
  release branch or pull request; it ends with the check that the only changes since the approved commit are
  release files" ([[config:release]]).
- Publish: "the tag, the publish commands, and for a critical bug fix the merge of `hotfix/<version>` back
  into the integration branch, which waits for the integration lock and opens a `merge` decision if it
  conflicts. It records the new baseline that later delta runs compare against"
  ([[topic:governed-delivery]], [[record:baseline]]).

Survey precondition: "decision `release` approved for the verified commit (for the cycle named by `cycle`,
when two are open)". In a multi-repository project "the same workflow, run at the hub after one `release`
decision, releases the repositories in the order their stories' dependencies give".

In local delivery mode the release branch, and the merge of a hotfix back into the integration branch,
are moved the way local delivery moves the integration branch: the merge is made in the integration
worktree and handed to the branch, with a fast-forward where the branch is checked out
([[command:story-deliver]]).

## Who may call it

`clerk` ("One clerk running `vsdd release --run <id>`").

## Writes

Version files, the change log, the tag, the publish commands, the hotfix merge-back, the new baseline
record, the release closure. One of the two governed merge routes ([[topic:governed-delivery]]).

## Output and failure cases

`{ok, state, version, tag}` ([[schema:release-result]]). The checkpoint of `release` closes the cycle,
"recording the close date, the version and the state commit in `cycle.json`; `next` is done, or `feature`
when a change request is queued".

## Feature flags (decision D31; built when the build widens)

The change log gets one line for each flag the release turns on or removes ([[topic:flag-lifecycle]]).

## Team use (decision D33; built when the build widens)

Not on the thin slice path. In `pr` mode the command moves no shared branch by a push of its own. The
merge of `hotfix/<version>` back into the integration branch, and a release commit that `[release]`
puts on the integration or release branch, each go through a pull request that the delivery's host
module opens and merges ([[command:story-deliver]]): a repository set up as decision D33 expects
refuses a direct push to those branches. The integration lock named above is a file lock on one clone;
what orders two clones is the host's merge. The command records in the `release` closure each commit
and pull request it puts on a target branch, so the hand-commit check does not count them
([[table:team-delivery-checks]], T-NEVER, T-NOTICE).
