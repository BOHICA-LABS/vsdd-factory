---
kind: command
id: coverage
package: S1
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#2.6]
---

# `vsdd coverage`

Derived coverage and traceability view.

## Syntax

`vsdd coverage`

## What it does

Derived view ([[topic:stored-and-derived]]): "Reverse links, traceability matrix, coverage matrix, UI
traceability", formerly "stored in several files and kept in step by policy", are computed by
`vsdd refs <id>` and `vsdd coverage` from the artifacts' references and the closure records.

Wall behaviour as for [[command:list]]: walled content left out by default; `--include-walled` only for a
caller whose wall permits the class.

## Who may call it

"Any caller, subject to its wall" (read-only group).

## Output and failure cases

The derived coverage view; writes nothing.
