---
kind: command
id: driver-on
package: S2b
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.4]
---

# `vsdd driver on`

Marks the driver active for this session in `local/driver.json`.

## Syntax

`vsdd driver on`

## What it does

"Writes `local/driver.json` with the session id, for the `drive` skill" — the `on` position of the driver
flag: `on`, `paused` at a decision id, or absent; and the session ([[record:driver-flag]]). The driver is
the person-invoked `drive` skill: "it runs `vsdd driver on`; reads `next`; if it is a workflow, starts it
with the Workflow tool; when it returns, reads `next` from its result; and repeats" ([[skill:drive]]).
"After a compaction the briefing tells the main session that the driver was active or paused and how to
continue." The status report carries it as `driver: {state: "on", at: null}`.

"The driver only steers. If the main session starts the wrong workflow, that workflow's survey refuses, so
enforcement does not depend on the driver behaving."

## Who may call it

Main session (the command table's driver row); the `auto-allow` rule grants the main session's own
`vsdd driver` call.

## Writes

`local/driver.json`: the session id and state `on` ([[record:driver-flag]]).

## Output and failure cases

Acknowledgement; the flag is local and untracked.
