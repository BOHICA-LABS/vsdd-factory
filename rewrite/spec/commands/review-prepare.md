---
kind: command
id: review-prepare
package: G3
sources: [BLUEPRINT.md#4.9, state.md#2.16, agents-skills.md#2.12]
---

# `vsdd review prepare`

For an on-demand review: writes the diff patch and returns the next free report path.

## Syntax

`vsdd review prepare <lens> [base]`

## What it does

"For an on-demand review: writes the diff patch and returns its path and the next free report path." The
`review` skill "first runs `vsdd review prepare <lens> [base]`, one command that writes the" patch, so the
reviewer's inputs and the report destination are chosen by code, not by the reviewer. `review diff` and
`review path` are one `review prepare`.

## Who may call it

Main session (the command table's review row).

## Writes

The diff patch and the report path it names.

## Output and failure cases

The patch path and the next free report path. No other behaviour is specified.

## Team use (decision D33; built when the build widens)

The report path is taken inside a sync ([[table:work-claim-checks]], T-MINT).
