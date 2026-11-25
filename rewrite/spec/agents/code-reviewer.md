---
kind: agent
id: code-reviewer
package: A6a
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# code-reviewer

Reviewer. Reviews a code diff with no access to specs or factory state, as an outside engineer would.

## Tier and effort

reviewer — `model: opus`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, REPORT, SPAWN, RESULT.

The same lacks as the adversary: no shell, no Edit, no web. A spike runner it dispatches, or one run for its
spike request in a workflow, inherits its walls. The named sets are defined in [[topic:tool-sets]].

## Is given / writes / returns

Is given: the diff patch path; the tree root; the review rules file; the items to account for; claims from a
second pass. For `pattern-sweep`: the tree root and `conventions.md`, and no diff. On demand only: a report
path.

Writes: nothing in the pipeline. On demand only: one review report.

Returns in `data`: review data, as `adversary` ([[schema:review-data]]). Sweep data for `pattern-sweep`
(`findings`, `fixable`) ([[schema:sweep-data]]).

Result schema: the common fields of [[schema:step-result]]. Reviewers return raw findings, never a pass or
fail decision.

## Walls

Read walls: its row in [[table:walls]] denies every class in the state directory except `evidence` and
`guidance` — it sees the diff, the code, and the project's conventions and review rules, which its steps are
given, and nothing that explains the intent. Write scope: `reviews`, creating new files only.

## Methods

`review-diff`, `pattern-sweep` (the maintenance sweep: reads the code against `conventions.md` for
inconsistent patterns; it needs no diff).

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].
