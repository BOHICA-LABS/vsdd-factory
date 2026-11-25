---
kind: command
id: refs
package: S1
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#2.4, state.md#2.15]
---

# `vsdd refs`

Derived reverse links for one identifier.

## Syntax

`vsdd refs <id>`

## What it does

Derived view ([[topic:stored-and-derived]]). Nothing stores reverse links: "'Which stories implement this
contract' is a query (`vsdd refs BC-2.03.007`)." Forward references live in the artifacts; this command
computes the reverse. Aliases resolve too: an imported identifier that did not fit goes into `aliases`,
"so references in code and commit messages still resolve through `vsdd refs`".

## Who may call it

"Any caller, subject to its wall" (read-only group). Wall behaviour as for [[command:list]]: walled
content left out by default, `--include-walled` only for a caller whose wall permits the class.

## Output and failure cases

The reverse links of the identifier; writes nothing. An unknown identifier returns the negative answer
(exit 1), a wall-refusal is the read of the classes the read reveals.
