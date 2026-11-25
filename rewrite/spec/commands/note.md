---
kind: command
id: note
package: S2a
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#2.7, hooks.md#2.11.10]
---

# `vsdd note`

Adds a note a person asked to keep to `notes.json`.

## Syntax

`vsdd note "<text>"`

## What it does

"Adds a note to `notes.json`." The notes field of the status report: "Free text a person asked to keep,
through `vsdd note`" — 5 entries of 200 characters; the oldest is dropped ([[record:notes]],
[[schema:status]]). The `status` skill tells the main session to save a standing instruction with it, which
is what survives compaction: "Something a person said that must survive [is saved] only if it was saved
with `vsdd note`."

## Who may call it

The main session, on request ("A person leaves a note | `vsdd note "<text>"` | `notes.json` | The main
session, on request"). The `auto-allow` rule grants the main session's own call of it: "including for the
main session `vsdd driver`, `vsdd checkpoint brief` and `vsdd note`, which the driver and the brief skill
run."

## Writes

Tracked `.factory/notes.json` ([[record:notes]]); the report that reads it is computed, never stored
([[command:status]]).

## Failure cases

The 5-entry, 200-character limits are enforced at display; nothing else is specified.
