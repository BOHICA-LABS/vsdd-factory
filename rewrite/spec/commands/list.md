---
kind: command
id: list
package: S1
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#2.6]
---

# `vsdd list`

Derived catalogue of one artifact kind, showing computed statuses and leaving walled content out by
default.

## Syntax

`vsdd list <kind>`

## What it does

Derived view ([[topic:stored-and-derived]]): the catalogue of one artifact kind, "from a cache keyed by
path, size and modification time". "`<kind>` is an artifact kind from `artifacts.toml` (`contract`, `story`,
`scenario`, `property` and the rest)" ([[table:artifacts]]). "Whether a contract is implemented, an epic
done, a decision record superseded" and a story in progress are computed, not stored: "`list` shows
computed statuses."

Wall behaviour. "These and `review show` leave walled content out by default; `--include-walled` prints it,
and the hook allows that flag only for a caller whose wall permits the class." An unknown kind is refused
for any caller with a wall, "as if it revealed every walled class".

## Who may call it

"Any caller, subject to its wall" (read-only group).

## Output and failure cases

Prints the derived catalogue; write-free. Failure: an unknown `<kind>` refused for a caller with a wall,
per the wall rule above.
