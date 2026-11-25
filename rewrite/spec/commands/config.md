---
kind: command
id: config
package: S1
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#2.12]
---

# `vsdd config`

Prints the project configuration as data.

## Syntax

`vsdd config --json`

## What it does

"The configuration as data." "A workflow script cannot read this file. The survey returns the values a run
needs in its `config` and `limits` fields. `vsdd config --json` prints the whole file as data"
([[topic:project-configuration]]). The key list it prints is the one list in state design section 2.12;
"an unknown key is an error and two lists would reject each other's keys".

## Who may call it

Read-only: "Any caller, subject to its wall."

## Output and failure cases

The whole `factory.toml` as one JSON object; writes nothing. Invalid configuration is an error the briefing
reports ("Unknown keys and wrong types are errors"), and "an invalid value never relaxes a gate".
