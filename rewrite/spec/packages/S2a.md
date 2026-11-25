---
kind: package
id: S2a
depends_on: [S1]
sources: [BLUEPRINT.md#9.2]
---

# S2a: Record stores
<!-- generated: start -->
## Delivers

Run, step, evidence, closure, decision, cycle, request and note stores, with `vsdd note`; the digest

## Depends on

- [[package:S1]]

As written in the build plan: S1

## Implements

- [[artifact:change-request]]: A change request (CR-NNN), queued or running; the only copy of a request.
- [[command:note]]: Adds a note a person asked to keep to notes.json.
- [[record:closure]]: One per unit: how far the unit has got, its stages, verdict, delivery and what closed it.
- [[record:cycle]]: cycle.json: what a cycle is (kind, title, route, baseline, request) and, once closed, its close fields.
- [[record:decision]]: A question only a person can answer, stored as a record and a packet a person can read.
- [[record:evidence]]: The file a gate writes for one gate and commit, with the raw output, JUnit report or diff patch beside it.
- [[record:notes]]: Up to five notes a person asked to keep, in the tracked notes.json.
- [[record:run]]: run.json: one workflow run, its session, agent count, admitted stories and whether it holds an amendment.
- [[record:step]]: The stored copy of a step result, with the calling agent's type and the worktree head added by the hook.
- [[topic:digests]]: The digest an approval is bound to: how it is computed, which files a spec decision covers, and when a decision reads as stale.
- [[topic:evidence-lookup]]: Evidence as records: what a gate writes and which record each gate's lookup returns, including after a clean rebase.
- [[topic:machine-records]]: The kinds of record only code writes, which are tracked, how long each is kept, and why step records and evidence are not tracked.

## Read first

- [[doc:conventions]]
- every file listed under Implements, and the files those link to
<!-- generated: end -->
## Notes
