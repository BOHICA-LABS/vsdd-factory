---
kind: command
id: driver-pause
package: S2b
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.4]
---

# `vsdd driver pause`

Marks the driver paused at a decision.

## Syntax

`vsdd driver pause <decision>`

## What it does

"Writes `local/driver.json` with the session id, for the `drive` skill" — the paused position of the driver
flag, at the decision id that stopped the loop ([[record:driver-flag]]). The loop pauses itself at a human
decision: "At a human decision it runs `vsdd driver pause` with the decision's id and stops; when the person
settles that decision in the same session, the context the `decide` hook returns tells the main session to
resume the loop, so the person does not type `/vsdd-factory:drive` again after each approval."
Or "after two runs without progress" the loop turns off instead ([[skill:drive]], [[command:driver-off]]).

## Who may call it

Main session; granted to the main session's own calls by `auto-allow`.

## Writes

`local/driver.json`: state `paused` at the named decision ([[record:driver-flag]]).

## Output and failure cases

Acknowledgement; the briefing reports the paused state and how to continue.
