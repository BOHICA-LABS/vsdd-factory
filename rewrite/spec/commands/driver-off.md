---
kind: command
id: driver-off
package: S2b
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.4]
---

# `vsdd driver off`

Clears the driver flag.

## Syntax

`vsdd driver off`

## What it does

"Writes `local/driver.json` with the session id, for the `drive` skill" — the cleared position: the flag is
absent ([[record:driver-flag]]). The loop's end: "At done, or after two runs without progress, it runs
`vsdd driver off`" ([[skill:drive]]).

## Who may call it

Main session; granted to the main session's own calls by `auto-allow`.

## Writes

Removes the driver state from `local/driver.json` ([[record:driver-flag]]).

## Output and failure cases

Acknowledgement; the flag is local and untracked.
