---
kind: topic
id: machine-records
package: S2a
sources: [BLUEPRINT.md#4.7, state.md#2.8]
---

# Machine records

Machine records are the records written only by code. There are 14 kinds. An agent cannot write any of them:
each lives in a path class with `write = "code"`, so no tool call can create or change one. Whether an
artifact is approved, a story ready or done, or a property proven is read from the decision or closure
record that made it so, and code never rewrites tracked files to say it.

## Tracked records

Tracked records are committed to the state branch:

- [[record:cycle]] — `cycles/<cycle>/cycle.json`
- [[record:closure]] — one per unit, `cycles/<cycle>/units/...`; a [[record:baseline]] is the closure record
  of an ingest or an import
- [[record:decision]] — and its packet, `cycles/<cycle>/decisions/...`
- [[record:notes]] — `notes.json`
- [[record:holdout-evaluation]] — `holdout/evaluations/...`

## Untracked records

Untracked records live under `.factory/local/`:

- [[record:run]] and [[record:step]] — the run and its step records
- [[record:evidence]] — what the gates measured
- [[record:job]] — detached jobs and started products
- [[record:lock]] — the run lock
- [[record:driver-flag]] — the driver switch
- [[record:caller-marker]], [[record:spike-admission]], [[record:config-fingerprint]] — the hook runner's
  own records, under `local/hooks/`

## Why step records and evidence are not tracked

They are per run and per commit, they are large in total (hundreds of step records in a cycle), and they can
be regenerated: a gate is a command, and a step can be run again. The closure record is the tracked
statement of what they proved.

There is no stored verdict record: a verdict is computed from the step records, and after the checkpoint it
is in the closure record (see [[topic:verdicts]]). There is no status record: the briefing and `vsdd status`
are one function that computes its answer from the records each time, so no copy can be missing or stale.

## Team use (decision D33; built when the build widens)

Decision D33 adds two kinds: the work claim, tracked ([[record:work-claim]]), and the clone record,
untracked ([[record:clone]]). It also adds a tracked copy of one existing kind, a story's red record
([[record:evidence]]). The count above is unchanged until they are built.
