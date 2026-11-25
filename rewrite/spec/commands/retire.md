---
kind: command
id: retire
package: S6
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#2.4, agents-skills.md#2.12]
---

# `vsdd retire`

Deletes one file in a class that allows deletion (deferred, research) and refuses anywhere else.

## Syntax

`vsdd retire <path>`

## What it does

"Identified specification artifacts, stories and scenarios are never deleted; they are retired. Deferred
items and research may be deleted when they are resolved or superseded, because their names are slugs and
git keeps the history. The hook runner blocks every shell deletion inside the state directory, so deletion
goes through one command, `vsdd retire <path>`, which deletes only in the classes `deferred` and `research`
and refuses anywhere else. The main session runs it in the `defer` and `research` skills."

Inside a workflow nobody needs it: "a research step returns the paths it superseded in `data.superseded`,
and the checkpoint retires them." The `defer` skill's `resolve` "runs `vsdd retire <path>`, the one command
that deletes a file in a class that allows deletion" — `defer resolve`, `lesson retire` and
`research retire` are one `retire`.

## Who may call it

Main session (the row with `note`, `defer`, `driver`, `lock release`).

## Writes

Deletion of the named file in the `deferred` or `research` class; nothing else.

## Output and failure cases

Refusal outside the two deletable classes — "refused anywhere else" — is the designed failure; the hook
runner blocks shell deletions, so the command is the only route.
