---
kind: command
id: spike-record
package: S6
sources: [BLUEPRINT.md#4.9, state.md#2.16, BLUEPRINT.md#6, pipeline.md#2.8]
---

# `vsdd spike record`

Validates a spike finding and completes the ledger entry; `--increment` records one session of a named
spike.

## Syntax

`vsdd spike record --id <ledger id> --json-file <path> [--increment]`

## What it does

"Validates the finding against the schema and completes the entry" — the finding schema
([[schema:spike-finding]]). It is the runner's last command: "The runner's last command is `vsdd spike
record`. A hook refuses to let it finish without one: for a workflow step it blocks the result call, and
for an agent dispatched in a session it refuses the stop" ([[rule:spike-bounds]]).

Increment. "`--increment`, for a named spike, records one session's progress without a verdict; the stop
check accepts it." A named spike "spans many sessions, so each session's work ends with
`vsdd spike record --increment`" ([[topic:named-spikes]]). "Its first command is `vsdd spike open --name
<name> --run <id>` and whose last is `vsdd spike record --increment`. No clerk opens the entry."

## Who may call it

`spike-runner`. Like every spike command it takes "a file the runner writes in the temporary directory",
never standard input.

## Writes

The ledger entry: completed with the finding, or advanced by one session's increment
([[artifact:spike-ledger-entry]]).

## Output and failure cases

The completed entry, or the ledger's current state and next action when called from the named-spike flow.
Failure: the finding does not validate against the schema, or the ledger id is unknown.
