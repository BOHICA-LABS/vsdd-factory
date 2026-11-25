---
kind: command
id: defer-add
package: S6
sources: [BLUEPRINT.md#4.9, state.md#2.16, agents-skills.md#2.12]
---

# `vsdd defer add`

Adds an entry to the deferred-work register.

## Syntax

`vsdd defer add ...`

## What it does

"The deferred-work register" — the add half of the row "`vsdd defer add`, `list`". The `defer` skill
"instructs the main session: `add` runs `vsdd defer add` and writes the entry" ([[artifact:deferred-item]]).
Register limits are one of the things [[command:validate]] reports, "always as warnings with exit 0, so a
41st deferred item never turns a project's CI red". An entry is closed by deletion through
[[command:retire]] or by the resolve form of the skill.

## Who may call it

Main session (the command table's row "`new <kind>`, `note`, `defer add/list`, `retire <path>`, `driver
on/pause/off`, `lock release` | Main session"); the person reaches it through the `defer` skill, which is
person-invoked.

## Writes

One entry in the deferred-work register ([[artifact:deferred-item]]).

## Output and failure cases

Not specified beyond the write.
