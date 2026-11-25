---
kind: command
id: explain
package: H9
sources: [BLUEPRINT.md#4.9, hooks.md#2.12, hooks.md#2.13]
---

# `vsdd explain`

Explains one block by its id, or the most recent block in the project.

## Syntax

`vsdd explain <id>` and `vsdd explain --last`

## What it does

"Explain a block." "Every block has an id that appears in the message the model sees and in the log":

```text
$ vsdd explain b-7f3a2c
Blocked: Edit of .worktrees/S-1.02/src/lib.rs by vsdd-factory:implementer at 21:40:03
Rule:    tests-first (gate, can be relaxed in [hooks.rules.tests-first])
Why:     no failing test run is recorded for story S-1.02
Fix:     write the failing test, then run the red gate: vsdd gate red --story S-1.02
Looked at: evidence store, gate "red", subject S-1.02 (no passing record); paths.source = ["src/**"]
```

"`vsdd explain --last` shows the most recent block of the current project." The record it reads is an event
of type `rule.block` in the event log ([[topic:event-log]], [[schema:event]]).

## Who may call it

"People, the status skill" use it; the read-only row of the command table puts it with the views "any
caller, subject to its wall". With the terminal shim it "work[s] in a terminal, which is where the designs
send a person when a session cannot help".

## Output, exit behaviour and failure cases

Human-readable block explanation; nothing is written. An unknown id or no recorded block is the negative
answer (exit 1).
