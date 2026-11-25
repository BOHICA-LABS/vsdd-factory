---
kind: command
id: events-schema
package: T3
sources: [hooks.md#2.12, hooks.md#2.13]
---

# `vsdd events schema`

Prints the JSON Schema the event writer validates against.

## Syntax

`vsdd events schema` — spelled as two words in the command line; the registry id joins them
(`events-schema`).

## What it does

"`vsdd events schema` prints the JSON Schema that the writer validates against, so the documented schema
cannot drift from the code." The writer is `vsdd/events.py`, "the one writer, used by the runner and by the
tool, in process"; the schema is the event schema ([[schema:event]]). The common fields every line carries:
`v`, `ts` (ISO 8601, UTC), `type` (dotted), `severity` (`info`, `warn`, `block`, `error`) and
`session_id` ([[topic:event-log]]).

## Who may call it

"Observability stack, reference generation" (the hook design's contribution to the command line). It is not
among the commands that work outside an initialised project ([[topic:command-conventions]]).

## Output and failure cases

The schema document; writes nothing.
