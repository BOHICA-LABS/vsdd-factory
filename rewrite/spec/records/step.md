---
kind: record
id: step
package: S2a
sources: [BLUEPRINT.md#4.7, state.md#2.8, hooks.md#2.11.7]
---

# Step record

`local/runs/<run-id>/steps/<workflow>/<subject>/<name>.<attempt>.json`, keyed by run, workflow, subject,
name and attempt. Not tracked. The stored copy of a step result ([[schema:step-result]]) plus what the hook
adds; the fields are in [[schema:step-record]].

A script gives every step of one run and unit a different name, so no record is written over: the steps of
a review loop carry the loop and the pass in their names (`review-adversary-l2-p1`). The name is only the
key. What a record is, the tool reads from the identity inside it (`method`, `loop`, `pass`), as
[[topic:verdicts]] sets out.

## Written by

The hook runner, when the step returns: the `step-results` rule ([[rule:step-results]]) stores the payload's
`agent_type` and the worktree head (`worktree_head`) with every record, keyed by run, workflow, subject,
name and attempt. It stores a record only for a live run. Step records are written only through `store_step`
([[topic:state-api]]), so the path and the cap have one owner.

## Grounding and walled findings

The rule grounds each finding before it stores the record, and marks one that stays ungrounded
`grounded: false` ([[rule:step-results]], [[topic:verdicts]]). A walled finding's full form is also written
to the evaluation file in the holdout class; see [[topic:walled-findings]].

## Cap and lifetime

Cap 64,000 bytes. Kept as long as the run record.
