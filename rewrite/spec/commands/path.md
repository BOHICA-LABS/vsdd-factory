---
kind: command
id: path
package: S1
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#2.6]
---

# `vsdd path`

Prints the path of an identified artifact.

## Syntax

`vsdd path <id>`

## What it does

Derived view ([[topic:stored-and-derived]]): resolves one identifier to its file path under the state
layout, where every artifact path is derived from the identifier and the kind's directory rule
([[table:identifiers]]). Wall behaviour as for [[command:list]].

## Who may call it

"Any caller, subject to its wall" (read-only group).

## Output and failure cases

The path; writes nothing. An unknown identifier is the negative answer (exit 1).
