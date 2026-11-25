---
kind: command
id: checkpoint
package: S3
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.5]
---

# `vsdd checkpoint`

Last step of every workflow: closes what the evidence supports, opens decisions, commits and pushes state,
releases the lock and returns next.

## Syntax

`vsdd checkpoint <workflow> --run <id> [--outcome failed]`; `vsdd checkpoint brief` is
[[command:checkpoint-brief]].

## What it does

"Turns the run's step records and evidence into closures and decisions, writes holdout evaluations (with any
walled findings), retires research reports a step superseded, stops products the run started, flushes,
pushes, releases the lock, and returns `next`. The `release` checkpoint closes the cycle. The `discover`
assess checkpoint opens the cycle with the `brief` decision."

"The command reads the step records and evidence of the run, closes the units the
evidence supports, opens a decision packet for each blocked unit, saves state, releases the lock, removes
expired spike worktrees and returns `status`, `progress`, the decisions it opened and `next`. That object is
the workflow's return value, so the main session receives a few hundred bytes per run." "Every return after
a successful survey goes through the checkpoint, which releases it, including on failure."

Digest computation for the decisions it opens: "`vsdd checkpoint` computes the digest of what is being
decided (the specification file set for `spec`, the commit for `release`), over the files exactly as they
are" ([[topic:digests]], [[record:decision]]).

## Who may call it

`clerk`; the main session for the `brief` variant.

## Writes

Closure records ([[record:closure]]), decision records and packets ([[record:decision]]), holdout
evaluations ([[record:holdout-evaluation]]), `cycle.json` close fields or its opening, retirement of
superseded research, the state push, releasing the lock ([[record:lock]]).

## Output and failure cases

`status`, `progress`, the decisions opened and `next` ([[schema:checkpoint-result]], and
`next` is the workflow result's next object). With `--outcome failed` the run closes as failed, which the
next run retries.

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. The checkpoint keeps one deferred item for each live temporary flag, opens
the `merge` decision that a flag's removal waits for, and prints the flag lines of the `spec` packet and
the flag section of the `release` packet ([[topic:flag-lifecycle]], [[topic:flag-system-choice]]).

## Team use (decision D33; built when the build widens)

Not on the thin slice path. The checkpoint writes the tracked copy of the red record and pushes the
branch of each story in flight under the branch's own name (T-HANDOFF), removes the claims its run no
longer needs (T-RELEASE), and publishes through the sync. Its result gains the sync's outcome, the story
branches the remote did not accept and the story worktrees that hold uncommitted changes. When the sync
does not end `synced` the run is ended all the same, and the next sync publishes
([[table:work-claim-checks]], [[command:sync]]).
