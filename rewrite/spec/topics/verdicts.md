---
kind: topic
id: verdicts
package: G3
sources: [pipeline.md#2.10.3, state.md#2.8, BLUEPRINT.md#4.10, BLUEPRINT.md#4.13, hooks.md#2.11.7]
---

# Verdicts

How the tool computes a verdict from the stored passes.

The script computes the verdict with `converge()`; the tool computes the same object from the stored passes,
as a function that `vsdd gate demo`, `vsdd story deliver`, the survey and the checkpoint call. It is not a
command and it is not stored as a record of its own: after the checkpoint the verdict is in the unit's
closure record, which is what a fresh clone reads. The tool's result is the one guards read. There is no
stored verdict record: a stored verdict would be a second copy of a verdict that could disagree with the
first. The schema is [[schema:verdict]].

## Which records are passes

Every step record carries the identity its step was given ([[schema:step-result]]): the `method`, and for a
review, fix or arbitrate step the `loop` and the `pass`. The tool reads those fields and never parses a step
name.

- A **review record** is a `done` record that carries a `pass`, whose `method` belongs in
  [[table:methods]] to an agent of the reviewer tier ([[table:roster]]), and whose stored `agent_type` is
  that owner. A record that fails any of the three is not a review record, so a builder that returns a
  clean review, under its own method or a reviewer's, changes nothing. The hook runner stores the calling
  agent's type with every record ([[record:step]]); the stored type is also what `vsdd spike open` reads as
  the asker of a `needs_spike` result.
- A **fix record** is a `done` record that carries a `pass`, is not a review record and holds fix data
  ([[schema:fix-data]]). An **arbitration record** is a `done` record with the method `arbitrate` from the
  agent that owns it.
- The tool validates each record against its schema and honours the stored grounding mark before it
  counts the record: a finding stored with ``grounded: false`` is counted as the mark says, and no
  grounding check runs at count time (see Grounding below).

## What the tool rebuilds

Loops are numbered within a run: loop 1 reviews the unit's whole diff, and each later loop reviews only the
commits added since the loop before it converged (the story's review after a delivery repair). The tool
takes the unit's records from every run, ordered by the start of the run, then `loop`, then `pass`, and
replays `converge()` ([[topic:prelude-converge]]) over each loop with these inputs:

- **A pass** is the review records that share a run, a loop and a pass number. Its findings, claim checks
  and coverage are theirs joined; its head is their `reviewed_sha`, which must be one commit.
- **Items to account for.** In the first loop, the story's acceptance criteria (for another perimeter, the
  items its gate names). In a later loop, the files changed between the commit the loop before it converged
  on and the head it reviewed.
- **Disputes and claims** come from the `resolutions` of the fix records of the same loop and pass.
- **Rulings** come from the arbitration record of the loop.
- **Waivers** are the keys and coverage items of settled `finding` decisions for the unit
  ([[record:decision]]).
- **The cap** is `convergence.passes` (rounds, for a wave or a release) for a loop 1 and 2 for a later
  loop; `used` is the number of loop-1 passes recorded for the unit in earlier runs, and 0 for a later
  loop.

The replay does not stop at the first pass with nothing open: the verdict of a loop is the one its last
recorded pass gives. A unit's verdict is the verdict of the last loop in that order. It covers a head when,
counting back from that loop to the nearest loop 1, every loop converged in the recorded order, and the last
one's head is that head or a commit with the same `git patch-id` ([[topic:evidence-lookup]]). The loops are
tied to one another by their recorded order and subject alone: no review pass stores the commit the loop
before it converged on, so no covering condition reads one. The conformance fixtures hold one case for each input above, and one in which
a builder echoes a reviewer's method ([[packaging:tests-conformance]]).

## Grounding

The grounding check is applied when the step result is stored: the path must exist in the unit's worktree,
the line range must lie inside the file, and the quote must occur within three lines of that range. A
finding of category `holdout` is exempt: it is built by the tool from scenario scores and has no location.
See [[topic:holdout-findings]] and [[topic:walled-findings]].

A finding that still fails the check on a step's second return is stored with `grounded: false`
([[rule:step-results]]). `judge()` and the tool's copy treat such a finding as advisory whatever its
severity. The script judges the copy its step returned, so the two verdicts agree only when that copy
carries the mark; how it gets there is in [[rule:step-results]]. Where they disagree the tool's verdict
stands: the checkpoint records the stage from it, and the next run continues from `reviewed`.

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. For a story that names a feature flag, or that builds the product's flag
system, the tool adds items to those a review must account for, and for those items only `checked`
counts: F-ITEMS in [[table:flag-story-checks]].

## Team use (decision D33; built when the build widens)

Not on the thin slice path. Each hand commit in a perimeter's range is one more item a review must
account for, `hand:<12 hex>`, given with the path of its patch, and only `checked` counts for it:
T-ITEMS in [[table:team-delivery-checks]].
