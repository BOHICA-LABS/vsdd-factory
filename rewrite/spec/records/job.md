---
kind: record
id: job
package: G1
sources: [BLUEPRINT.md#4.7, state.md#2.8]
---

# Job record

`local/jobs/<key>.json` and `<key>.log`, where the key is a hash of the command, its subject and the commit.
Not tracked. Fields in [[schema:job-record]].

Two further per-key locks hang beside the record. `<key>.cmd.lock` is held by the worker and inherited by
its command. The starting `attach` unlinks it before each new attempt, so a descendant that escaped an
earlier command (one that left its group with `setsid`) holds only an unlinked inode and can neither delay
the new worker's lock-taking nor authorise a kill for it. While the lock is held, a process group the
record names was born of that attempt's command; before any kill, `attach` compares the group leader's `ps`
start time with the record's time, so a number reused after the old group exited or a reboot is never
signalled. `<key>.settle.lock` is held by an `env start` call and is
never passed to the product: held, a `starting` record belongs to a live start call; free, it is stale.
A record also carries `command_pgid` and `command_started` while a command of a job runs; a worker that
could not take the cmd lock records neither.

A detached gate or delivery job, or a started product, keyed by command, subject and commit. Every long
command (`vsdd gate ...`, `vsdd story deliver`, `vsdd release`) runs its work detached, and the command
itself re-attaches: the first call starts the job, and every call waits for it and returns either the result
or `{ok: true, state: "pending", retry: "<the same command>"}`. There is no job id to carry. A product
started by `vsdd env start` is a job record with its port, process id and the caller's agent id, and is
stopped by `env stop`, by the hook runner when that agent stops, and by the checkpoint for anything left.

Written by any long command, the first time it is run for that key; and by `vsdd env start`.

Cap 4,000 bytes plus the log. Kept 7 days.
