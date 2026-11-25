---
kind: record
id: lock
package: S2b
sources: [BLUEPRINT.md#4.7, BLUEPRINT.md#4.13, state.md#2.11]
---

# Run lock

`.factory/local/lock.json`. Not tracked. One lock per repository and branch a run can write. Fields in
[[schema:lock]].

```json
{ "locks": [ { "scope": "acme-api:develop", "run": "r-20261126-1402-7q", "workflow": "wave", "subject": "W-02",
               "session": "0c1d…", "taken": "2026-11-26T14:02:00Z", "seen": "2026-11-26T14:31:10Z" } ] }
```

## Taking and releasing

`vsdd survey` takes it once its preconditions pass, scoped to the repository and the branch the run can
write. Every run that can merge into the integration branch takes the same scope, so a standalone `story`
run and its wave exclude each other. A second run on the same scope gets `ok: false` with the holder. The
take is atomic: a file lock on `local/lock.flock` is held around the read, the check and the write.
`vsdd checkpoint` releases it. So does the hook runner when the holder's session ends, which covers a person
closing a session in the middle of a run. The `spike` workflow, `discover` and a report-only `maintain` take
no lock.

## Liveness

The holder counts as alive while `seen` is recent. The hook runner refreshes it through `lock_seen`
([[topic:state-api]]); which calls refresh it, when a lock is stale, what a survey from the holder's own
session does, and how a lock is released by hand are in [[topic:run-lock]].

## Two machines

The lock is local, so it does not stop two machines. They meet at the state push: the second fetches, rebases
and retries, and a real conflict in the same record is reported to a person.

Writes to machine records are atomic (write a temporary file, then rename) under `flush.lock`, so two
sessions that legitimately run at once cannot corrupt a record.

## Team use (decision D33; built when the build widens)

Not on the thin slice path. The lock is unchanged. Two clones are kept apart by the work claim
([[record:work-claim]], [[topic:team-claims]]), not by this record. Where this file says the second
machine fetches, rebases and retries at the state push: with team use off that push now stops and says
so, and with it on the push is the sync ([[table:sync-checks]], T-SOLO).
