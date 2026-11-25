---
kind: command
id: env-start
package: G1
sources: [BLUEPRINT.md#4.9, state.md#2.16]
---

# `vsdd env start`

Starts the product on its own port for a demo, a UI review or scenario evaluation, recorded with the caller.

## Syntax

`vsdd env start --subject <id>`

## What it does

"Start the product on its own port for a demo, a UI review or scenario evaluation. The process is recorded
with the caller, and is stopped when that agent stops or at the checkpoint." The port comes from the
product's own configuration: "A product process on its own port (`{port}` in `commands.run`, or
`$VSDD_PORT`), recorded as a job record with the caller's agent id from the caller marker"
([[topic:caller-identity]]). The caller's identity is what lets the hook runner clean up: "On
`SubagentStop` it stops any product process that agent started with `vsdd env start` and left running; the
checkpoint stops whatever is left."

Where the product runs, and how the port is chosen (G1): a story id runs in its story worktree, anything
else in the integration worktree (`.worktrees/_integration/`), both paths under the root as
[[topic:worktrees]] lays them out, and a worktree directory that does not exist is the negative answer.
The port is the lowest in 28100 to 28199 that no live product record holds, chosen without a socket under
the store's `ports.lock`. A product that exits within two seconds of the start is retried on the next
port, three in all; `ok: true` therefore means started, not yet listening, and the caller's first request
retries. A start at the same head commit answers the running product's record; a start at a new commit
stops the old product and starts again. Each start decides on a record read after its lock probe, and a
lock held over a record whose product's process group is gone is a start in flight: the call waits while
the lock stays held for the running record that holder will write, takes the lock when it comes free, and
answers or replaces only from a record whose group lives (the recorded pid is only the group's leader, so
the group is what liveness is judged on, as `stop_products` signals it). A lock held over no usable
record - a start call that has taken the lock but not yet written, or a holder that will exit without
writing - is waited on the same way, whether the record is absent or still `starting`: the start that
waits answers the holder's product at the same commit, replaces it at another, takes a freed lock, and
fails only after the five seconds; a `starting` record whose settle lock has come free is a start call
that died mid-settle, and the product it left holding the lock is stopped by the pid the locked body
wrote.

## Who may call it

"`demo-recorder`, `ui-reviewer`, `holdout-evaluator`, `clerk`."

## Writes

A job record with the port, process id and the caller's agent id ([[record:job]]); the product process
itself.

## Output and failure cases

`{ok, url, port, pid}`. A refused start is the negative answer, `{ok: false, error, log}`, exit 1: the
worktree for the subject does not exist, `commands.run` is empty, no port is free in 28100 to 28199, the run
command exits on three ports, the product lock stays held, `ports.lock` stays held while another start is
choosing a port, the product record would exceed the job record's 4,000-byte cap less the 128 bytes kept for
later writes, or another start of the subject is still settling after the five-second wait; `log` names the
product's log file. A subject outside the id grammar (`^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$`) is a usage error,
exit 2. The product is stopped by [[command:env-stop]], by the hook runner when that agent stops, and by
[[command:checkpoint]] for anything left.

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. In a project that uses feature flags the command gives the product its flag
values itself: how is F-KEY in [[table:flag-story-checks]], and which flags it turns on is in
[[topic:flag-lifecycle]].
