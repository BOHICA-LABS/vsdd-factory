---
kind: command
id: cycle-close
package: S3
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#2.5]
---

# `vsdd cycle close`

Repairs a cycle that did not close; person only.

## Syntax

`vsdd cycle close`

## What it does

"Repairs a cycle that did not close; person only. Opening and closing otherwise happen inside the checkpoint
and the survey" — the `release` checkpoint closes a cycle normally, "recording the close date, the version
and the state commit in `cycle.json`" ([[record:cycle]]). This command is the repair path for a cycle left
open when that did not happen.

When the cycle closes, evaluations and demo files leave the working tree and stay in the state branch's
history, unless `state.prune_closed_cycles = false` keeps them.

## Who may call it

The person — "Acts only a person may perform ... otherwise in their own terminal"
([[topic:command-conventions]]).

## Writes

The close fields of `cycle.json` for the stuck cycle ([[record:cycle]]).

## Output and failure cases

Not specified beyond the repair effect.
