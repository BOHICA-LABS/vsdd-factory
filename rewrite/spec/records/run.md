---
kind: record
id: run
package: S2a
sources: [BLUEPRINT.md#4.7, state.md#2.8]
---

# Run record

`local/runs/<run-id>/run.json`. Not tracked. Fields in [[schema:run-record]]: the id itself (the directory
name, `r-<YYYYMMDD>-<HHMM>-<2 of [0-9a-z]>`), workflow, subject, repository,
cycle, session, start, agent count, parent run, the stories admitted to it, the request or question it was
given, whether it holds an amendment, and `ended`, which is empty while the run is live
([[topic:run-lock]]).

Written by `vsdd survey`, which records each story unit it returns in the run record's `stories`
(finding P3BLD-23); the admission commands (`vsdd gate plan --story`, `vsdd story new-fix`) record
admitted stories in it. For `spike` the survey stores the question in the run record and takes no lock;
`vsdd feature route` marks the amendment flag in the run record.

Cap 2,000 bytes. Kept 30 days, or until the cycle closes.

## Team use (decision D33; built when the build widens)

A run record never leaves its clone, and a run does not move between machines
([[topic:team-records]]). The id is unique on one clone only, so tracked state names a run together with
the clone.
