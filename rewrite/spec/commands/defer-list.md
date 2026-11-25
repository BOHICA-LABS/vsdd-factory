---
kind: command
id: defer-list
package: S6
sources: [BLUEPRINT.md#4.9, state.md#2.16, agents-skills.md#2.12]
---

# `vsdd defer list`

Lists the deferred-work register.

## Syntax

`vsdd defer list`

## What it does

The list half of the row "`vsdd defer add`, `list`" — the deferred-work register
([[artifact:deferred-item]]). The `defer` skill: "`list` runs `vsdd defer list`" ([[skill:defer]]).

## Who may call it

Main session; person-invoked through the `defer` skill.

## Output and failure cases

The register's entries. Read-only; an empty register is the negative answer.
