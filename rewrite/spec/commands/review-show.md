---
kind: command
id: review-show
package: G3
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#2.5]
---

# `vsdd review show`

Renders a unit's stored review findings for a person; a walled finding shows as its walled key and severity
unless the caller may read holdout.

## Syntax

`vsdd review show <unit> [--include-walled]`

## What it does

"Rendered findings" for one unit. The findings "still open when a loop ends are copied into the decision
packet" and the verdict is computed from the stored passes ([[topic:verdicts]]); "a person reads them with
`vsdd review show <unit>`, which renders the [findings]".

Wall behaviour. "The exception is a walled finding: the closure record, the decision packet and
`vsdd review show` keep its walled key and severity only, with the line 'details are inside the holdout
wall', and the full form is in the evaluation file. `vsdd review show <unit> --include-walled` prints it
for a caller whose wall allows `holdout`, and for a person in a terminal. The walled key is `walled:` and
the first 12 hex" characters ([[topic:walled-findings]]).

## Who may call it

"Any caller, subject to its wall" (read-only group).

## Output and failure cases

Rendered findings; "Nothing" is written. Without the flag a walled finding appears only as its walled key
and severity; with the flag from a caller whose wall denies `holdout` it stays summarised.
