---
kind: command
id: checkpoint-brief
package: S3
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.9]
---

# `vsdd checkpoint brief`

Opens the cycle and the brief decision when the person says the draft brief is ready; takes no run.

## Syntax

`vsdd checkpoint brief`

## What it does

"Takes no run. Opens the cycle and the `brief` decision when the person says the draft is ready." The
command table: "`checkpoint brief` takes no run and opens the cycle and the `brief` decision; the
checkpoint of `discover` in assess mode does the same through the same code." Assess
mode "opens the cycle through the same code as `vsdd checkpoint brief`, so that when the brief is
approved, `spec`, `plan`, `wave`, `verify` and `release` have a cycle directory for their closures,
decisions and evidence."

## Who may call it

"`clerk`; the main session for `brief`" — the brief checkpoint is the main session's. The `auto-allow`
rule grants the main session's own `vsdd checkpoint brief` call, "which the driver and the brief skill
run".

## Writes

`cycle.json` for the new cycle ([[record:cycle]]) and the open `brief` decision
([[record:decision]], [[table:decision-kinds]]).

## Output and failure cases

Returns through the checkpoint result shape; "Opening and closing otherwise happen inside the checkpoint
and the survey" — there is no other opener of a cycle.
