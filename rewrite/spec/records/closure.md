---
kind: record
id: closure
package: S2a
sources: [BLUEPRINT.md#4.7, state.md#2.8, state.md#2.5]
---

# Closure record

`cycles/<cycle>/units/<workflow>[.<subject>].json`, for example `spec.json`, `plan.json`, `wave.W-02.json`,
`story.S-7.03.json`, `verify.json`; and in `cycles/_standing/`, `ingest.json` or `import.json`, which hold
the baseline (see [[record:baseline]]). Tracked. One schema serves every unit; the fields are in
[[schema:closure-record]].

## Written by

`vsdd checkpoint`, `vsdd story deliver`, `vsdd plan schedule`, `vsdd gate plan --story` and
`vsdd story new-fix` (admission), `vsdd import` — and `vsdd survey`, which writes `delivery.branch`
and `delivery.base` through the worktree library (finding VSA-16). Every writer goes through one
write path, `closure_update`, which owns the read-modify-write under the file lock and the
validation; no command writes the record file itself.

## What a closure record holds

- The plan's closure record also holds `waves`, the map from wave to stories that `vsdd plan schedule`
  computed.
- A wave's record holds its stories and its gate rounds. In a hub there is one wave closure record per
  repository with stories in the wave (`wave.<W-NN>.<repo>.json`), and the wave is closed when every one of
  them is.
- `verify.json` holds the checks per round and the properties proven.
- A story's record, a wave's record and `verify.json` also hold `flaky`, the identities of inherited tests a
  gate recorded as flaky; the command that writes the record copies them from the gate's evidence, which is
  not kept. A story's `admitted_by` says how it became ready: `plan`, or the run that admitted it through
  `vsdd gate plan --story` or `vsdd story new-fix`.
- The stage entries are in the order the stages finished, which is what lets `revise` clear "from the review
  stage onward" without a list of stages.
- A story's record carries `closed` when it is closed: the ISO stamp of the checkpoint that closed it, like
  every stamp in the record family — not a boolean. (The cycle record's `closed` is the cycle's close
  date, written by the release checkpoint.)
- The `verdict` in the closure record is the convergence verdict as `vsdd.state.verdicts.summary` computes
  it: perimeter, head, patch_id, passes, rounds_used, outcome, and the counts of advisory, dismissed and
  waived findings. See [[topic:verdicts]].

## Cap and lifetime

Cap 16,000 bytes. Kept permanently.

## Team use (decision D33; built when the build widens)

Not on the thin slice path. A story's record gains `delivery.pushed` (the head the holder last pushed,
or last brought in and checked), `delivery.merged_by`, `delivery.landed` and `delivery.hand_commits`;
where a run id is stored, the clone's id is stored beside it ([[topic:team-records]],
[[table:team-delivery-checks]]). The red record a hand-off needs is not kept in this record, whose cap
it could pass: it has a tracked copy of its own ([[record:evidence]]). The schema stays open.
