---
kind: topic
id: long-commands
package: G1
sources: [BLUEPRINT.md#4.9, state.md#2.16]
---

# Long commands run detached and re-attach

The Bash tool waits for a foreground command two minutes by default and ten at most. Every gate that
builds or tests, `story deliver` and `release` therefore run their work as a detached job, keyed by the
command, its subject and the commit, and the command itself re-attaches: the first call starts the job, and
every call waits for it and returns either the result or a pending answer. The caller runs the same command
again until the result arrives. There is no job id to carry. By default a call waits 100 seconds, which is
inside the Bash default. `--wait <seconds>` is a global option: every command accepts it, and a command
that is not a job ignores it, so the closing command in a step prompt adds `--wait 540` whatever the
closing command is — the architect's `feature route` is not a job, and the route must still be written —
and tells the agent to set a Bash timeout of 600000. One clerk covers a whole CI wait.
`limits.parallel_gates` caps how many jobs run at once ([[config:limits]]).

A call that outlasts its Bash timeout is not killed there. The platform moves it to the background, and
kills it when the step's agent gives its final response ([[topic:platform-proven-2]], item 43). Nothing is
lost either way, and only because the job is detached: its worker is double-forked into a session of its
own, which is what survives that kill. This is proven on macOS, with the Bash sandbox off and on; Linux is
open ([[doc:platform]]). An agent whose call is moved to the background runs the same command again, which
re-attaches, and does not wait for or read the background task. The platform's notice at that moment tells
it the opposite, to wait for the result and not to start a second copy; a job command starts none. So the
agent is given the rule together with that reason, where it will have read it before the notice arrives: in
every method that ends with a gate command and in the clerk's rules. A lint test checks the sentence is
there.

The pending answer, from the state design, which owns the tool's behaviour: "returns either the command's
ordinary result or `{ok: true, state: "pending", retry: "<the same command>"}`. The caller runs the same
command again. There is no `--start`, no job id to carry from one tool call to the next, and no second
output shape to parse." A new session re-attaches to a job an earlier one left running.

"A command marked 'job' runs its work detached, and the command itself re-attaches. The job is keyed by the
command, its subject and the commit (`story deliver` by the story)." The job itself is a job record,
`local/jobs/<key>.json` and `<key>.log`, where the key is a hash of the command, its subject and the commit
([[record:job]]).

Commands that run this way: every gate that builds or tests ([[command:gate-red]], [[command:gate-green]],
[[command:gate-wave]], [[command:gate-release]], [[command:gate-ingest]], [[command:gate-contracts]],
[[command:gate-integration]], [[command:gate-sweep]]), plus [[command:story-deliver]], [[command:release]]
and [[command:plan-schedule]]. It applies by default with `--wait <seconds>`; the other gates and commands
return directly, accepting `--wait` and ignoring it.

## Team use (decision D33; built when the build widens)

`vsdd sync` is a job command as well ([[command:sync]]). Its key is the command and the name of the
state branch, with no commit, as `story deliver` is keyed by its story alone. The function it runs is
also called in process by the survey and the checkpoint, where it has a budget that keeps those commands
inside the default limit above ([[table:sync-checks]], T-FRESH).

A sync's result differs from a gate's in one way that the job layer must be told. A gate's result
belongs to a commit and stays true, so a finished result that nobody collected is rightly served to the
next call with the same key. A sync's result is the state of the remote at one moment. Served to a call
that comes after the job has ended, it would be an old outcome, and no sync would run. The job
specification of a sync therefore says that its worker marks its own result as collected when it writes
it. A call that started the job, or that attached while it ran, is given the result; any later call
starts a new sync, which costs one listing when nothing has changed. What an earlier sync did stays
readable in the clone record ([[record:clone]]). Package M1 adds that one field to the job layer, with
a test in which a commit pushed to the remote after a detached sync has ended is in the mount after the
next `vsdd sync`.

The sync that the `decisions` rule starts after a settlement ([[rule:decisions]]) is a job of its own.
Its key is the command and `settle:<decision id>`, and that id names the one record its flush may
commit. It does not attach to a `vsdd sync` that is already running, because that sync may have flushed
before the settlement was written; `local/sync.lock` puts the two in a row ([[command:sync]]).
