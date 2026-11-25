---
kind: workflow
id: release
package: W7
sources: [pipeline.md#2.3, pipeline.md#2.5, pipeline.md#2.9.7, pipeline.md#2.9.13, BLUEPRINT.md#4.5, BLUEPRINT.md#5.1]
---

# `release`

Ships the authorised commit, records the new baseline and writes the retrospective.

Arguments: `retro` (`false` to skip), `cycle`, `repo`. Preconditions: decision `release` approved for
the verified commit (for the cycle named by `cycle`, when two are open)
([[table:preconditions]]).

## Stages in order

### survey

- Step and agent: clerk.
- Does: precondition — decision `release` approved for the verified commit.

### release

- Step and agent: one clerk running `vsdd release --run <id>` ([[command:release]]), a single
  idempotent command that does the next missing part each time it is run.
- Does: prepare — the release steps from `[release]` in project configuration, which are the version
  (computed from the route) written to `version_files`, the change log (rendered from the delivered
  stories), and the release branch or pull request; it ends with the check that the only changes
  since the approved commit are release files. Publish: tag, publish, and for a critical bug fix the
  merge of `hotfix/<version>` back into the integration branch, which waits for the integration lock
  and opens a `merge` decision if it conflicts. It records the new baseline that later delta runs
  compare against.
- Gate: `ok`.

### retro

- Step and agent: researcher (planner), method `retrospective`, skipped with `retro: false`.
- Does: a retrospective report under `research/` (kind `retrospective`) from the cycle's step records
  — pass counts, findings by perimeter, spikes, failures, and proposals, each naming the line it
  would add to `conventions.md` or `review-rules.md`. A person adopts one with
  `/vsdd-factory:author conventions` or `author review-rules` ([[skill:author]]). There is no lessons
  register. The step returns `data.superseded`, the paths of earlier reports this one replaces, and
  the checkpoint retires them. It never blocks and needs no approval.
- Gate: none.

### close

- Step and agent: clerk.
- Does: the checkpoint closes the cycle, recording the close date, the version and the state commit
  in `cycle.json`; `next` is done, or `feature` when a change request is queued.

## Notes

In a multi-repository project the same workflow, run at the hub after one `release` decision, releases
the repositories in the order their stories' dependencies give ([[topic:multi-repo]]).

## Where it stops for a person, and what it returns

The release decision itself is the stop: it is opened by `verify` and settled by the person
([[topic:human-stops]]). A conflicted hotfix merge back into the integration branch opens a `merge`
decision. Ends: done. The return value is a workflow result ([[schema:workflow-result]]).

## Team use (decision D33; built when the build widens)

The run holds the cycle's claim, so one clone releases. The `release` packet lists the cycle's hand
commits and names who approved. In `pr` mode `vsdd release` puts nothing on a shared branch except
through a pull request, and records what it put there, so that it is not taken for a hand commit
([[command:release]], [[topic:team-delivery]]).
