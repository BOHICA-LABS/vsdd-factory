---
kind: topic
id: cycles
sources: [state.md#2.5, state.md#2.8, BLUEPRINT.md#5.2, BLUEPRINT.md#4.7]
---

# Cycles

A cycle is one pass of the pipeline over a body of work, from a brief or a change request to a release: the
first build of a product, one feature, one bug fix.

## The cycle record

The cycle record is `cycles/<cycle>/cycle.json` ([[record:cycle]]). The cycle identifier is a slug of 2 to 40
lower-case characters, chosen when the cycle opens. It holds:

- `id`, `kind` (`product`, `feature`, `hotfix`, `maintenance`), `title`;
- `route` (`standard` from the opening when a brief opens the cycle; written by `vsdd feature route`
  ([[command:feature-route]]) when a change request opened it; empty for a maintenance cycle);
- `opened`;
- `baseline` (a release tag, an ingest or an import; empty for a first release
  — [[record:baseline]]);
- `request` (`brief`, or the id of the one request file, `CR-<NNN>` — [[artifact:change-request]]);
- and once closed: `closed`, `version`, `archive_commit`.

There is no separate request copy and no summary file: totals and pass counts are derived from the closure
records ([[topic:status-report]]).

## Who opens and closes a cycle

Cycle openers: `vsdd checkpoint brief` ([[command:checkpoint-brief]]), the `discover` assess checkpoint (the
same code), the `feature` survey ([[command:survey]]), and `vsdd story new-fix --maintain`
([[command:story-new-fix]]) when it delivers fixes. The `release` checkpoint closes the cycle (a person can
also run `vsdd cycle close` [[command:cycle-close]] in a terminal to repair one). An amendment request
belongs to the open cycle and runs inside it ([[topic:amendments]]). A second cycle beside the main one is a
critical fix only ([[topic:hotfix-cycle]]).

## The standing cycle

`cycles/_standing/` has the same shape, for work outside a product cycle: ingest, discover, maintain and
spikes, and the ingest or import baseline. It never closes; it keeps the latest closure per workflow and
subject, the baseline, the latest `spec` decision, and other settled decisions for 90 days.

## What closing a cycle does

The `release` checkpoint closes the cycle. It adds to `cycles/<cycle>/cycle.json` the close date, the
release version and the commit of the state branch just before pruning (totals and pass counts are derived
from the closure records, so there is no separate summary file), then removes `demo/<cycle>/` and
`holdout/evaluations/<cycle>/` from the tree and the cycle's runs and evidence from the local store. The
removed files stay in the state branch's history at the recorded commit. The living tree therefore holds
full detail for the open cycle only, and its size is bounded by the size of the specification, not by how
long the project has run. `state.prune_closed_cycles = false` keeps the detail ([[config:state]]).

When the cycle closes the route starts `feature` with the oldest queued change request
([[artifact:change-request]]).
