---
kind: record
id: evidence
package: S2a
sources: [BLUEPRINT.md#4.7, state.md#2.8, state.md#2.16]
---

# Evidence record

`local/evidence/<subject>/<gate>.<sha12>.json`, with the raw output (`.log`, cut to 256 KB), the JUnit
report, or the diff (`diff.<sha12>.patch`) beside it. Not tracked. One record per gate and commit. Fields in
[[schema:evidence-record]].

Written by `vsdd gate`, `vsdd survey` (diff patches), `vsdd story deliver`.

Each record carries the commit it measured (`sha`), whether the gate passed, and the `git patch-id` of the
subject's diff from its base to that commit (`patch_id`). It also carries a sequence number `seq`, stamped
by the store under `flush.lock` when the record is written: one more than the highest in the subject's
directory. The lookup orders the records of a gate by `seq` and keeps `at` for people, so two records
written within the same second keep their write order and not the accident of their file names
(S3B-records-1; [[schema:evidence-record]]). A re-freeze is a `red` record with
`refreeze: true`. The `red` record carries `frozen`, one entry per frozen file with its SHA-256, `tests`, the
new test identities, and `criteria`, the map from acceptance criterion to test
identities ([[command:gate-red]]). The green record's `flaky` list is copied into the unit's closure record,
because evidence is not kept (see [[record:closure]] and [[topic:evidence-lookup]]). The report of a base
commit, `local/evidence/_base/<sha12>.xml`, is kept beside the records and is not a record.

Cap 8,000 bytes (the raw output beside it is cut to 256 KB). Kept until the subject's unit closes.
The base reports beside the records are not capped records; base reports older than
`state.retain_runs_days` are deleted when a new one is written.

## Team use (decision D33; built when the build widens)

Evidence stays on the clone. One part travels. The checkpoint and `vsdd claim release` write a story's
latest passing red or re-freeze record to `cycles/<cycle>/red/<story>.json`: tracked, written only by
code, with the fields, the schema and the cap of the record above, so a red record that exists can
always travel. The flush checks it as it checks any tracked record. A clone with no local red record
reads that copy ([[table:work-claim-checks]], T-HANDOFF).
