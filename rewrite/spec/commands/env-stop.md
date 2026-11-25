---
kind: command
id: env-stop
package: G1
sources: [BLUEPRINT.md#4.9, state.md#2.16]
---

# `vsdd env stop`

Stops a product process that env start started.

## Syntax

`vsdd env stop`

## What it does

Stops the product process [[command:env-start]] recorded for the caller. It is one of the three stops: "It
is stopped by `env stop`, by the hook runner when that agent stops, and by the checkpoint for anything
left." The command takes no argument: the caller selects what stops. With a caller marker it stops the
products that marker's agent and session recorded; without one, a person's terminal, it stops every
product, and from a tool call it stops nothing and answers the negative, naming the one-segment form
(`vsdd -C <dir> env stop`) that stops from where a marker can be read. A call that carries `VSDD_CALLER` is
a tool call wherever its marker can be read or not — pruned, looked for under another `-C` root, already
read ([[topic:caller-identity]]) — so it never takes the stop-everything branch, terminal or not.

## Who may call it

"`demo-recorder`, `ui-reviewer`, `holdout-evaluator`, `clerk`."

## Writes

Removes the job record of the running product ([[record:job]]) and terminates the process.

## Output and failure cases

`{ok, stopped: [{subject, port, pid}]}`. Stopping a process no job record names — a stop with nothing
running, or one refused because a tool call carried no caller marker — is the negative answer,
`{ok: false, stopped: [], error}`, exit 1.
