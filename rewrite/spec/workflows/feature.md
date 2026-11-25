---
kind: workflow
id: feature
package: W6
sources: [pipeline.md#2.3, pipeline.md#2.5, pipeline.md#2.9.4, pipeline.md#2.9.13, BLUEPRINT.md#4.5, BLUEPRINT.md#5.1]
---

# `feature`

Takes a change request for an existing product: analyses the delta, chooses the route, then either
delivers one story or evolves the specification.

Arguments: `request` (required on the first run, unless `amend`), `intent` (`feature`,
`enhancement`, `bug-fix`, `deprecation`, `port`), `severity`, `guidance`, `amend` (run the amendment
the open cycle is waiting for), `cycle`, `repo`. Preconditions: a baseline exists (a release, an
ingest whose `spec` decision is approved, or an import). With `amend`: a cycle is open and a
`spec-gap` decision was settled `amend` ([[table:preconditions]]).

## Stages in order

### survey

- Step and agent: clerk; the request travels as `--request-uri`.
- Produces: survey — the change request in flight if any, the baseline, and the branch stories are
  delivered to (`hotfix/<version>` for a critical bug, [[topic:worktrees]]).
- Gate: a baseline exists: a release, an ingest whose `spec` decision is approved, or an import.
  Every request is recorded once, as `.factory/requests/CR-NNN.md` ([[artifact:change-request]]). If
  another cycle is active and the request is not critical, it stays queued and the run returns.
  Otherwise the survey opens the cycle, whose `cycle.json` names the request. With `amend: true` the
  survey opens nothing: it runs inside the open cycle, takes every `spec-gap` decision settled
  `amend` as its request, and marks the run as holding the amendment.

### delta and route

- Step and agent: architect (planner), method `delta-analysis`. Its closing command is
  `vsdd feature route --run <id>` ([[command:feature-route]]): the tool derives the route from the
  delta record the step just wrote, writes it into `cycle.json`, and for the standard route marks the
  run as an amendment. No clerk step.
- Produces: delta record and delta data — intent, severity, contracts changed and added, modules,
  whether any is critical, UI, cross-repository, estimated files. The route: `trivial` when the
  intent is not a bug fix, no contract changes or additions, the estimate is within the story size
  limit and no critical module is touched. `bugfix` when the intent is a bug fix and no contract
  changes; `hotfix` when its severity is critical. Otherwise `standard`. With `cross_repo` a fast
  route keeps its name and yields one story per repository, joined by `depends_on`. The route result
  also lists `readiness`: the chore stories the project still needs (a test harness and JUnit report
  when the baseline has no suite; CI when `delivery.ci = "none"` and there is a remote).
- Gate: the script uses the route in the gate result the step returned. On a fast route the
  specification stays locked, so no contract can change; the plan gate refuses a story whose criteria
  cite a contract outside the approved set, and the green gate fails a `trivial`-route story whose
  diff touches a critical module.

### story-draft (fast routes)

- Step and agent: story-writer (planner), method `story`. Its closing command is
  `vsdd gate plan --story <id> --run <id>` for each story it wrote; at most 2 attempts. No clerk
  step.
- Produces: one story per repository touched, and first the readiness chore stories the route
  listed. For a bug the story must include a regression test criterion. When a story passes, the same
  command admits it and returns its unit in the gate result's `unit`, and run again for a story
  already admitted it returns the same unit; the step returns the units as `data.units`.
- Gate: through `gated()` with `each`: every story's `plan` entry passed, and the unit whose `id` is
  that entry's `subject` is in `data.units`. A story that fails is named in the retry note; if one
  still fails after the last attempt the step is blocked with kind `gate`, and no story is delivered
  while one is missing.

### deliver (fast routes)

- Step and agent: `story` as a child through `runStories()`, readiness stories first.
- Produces: story result.
- Gate: `status == "done"`.

### evolve (standard route)

- Step and agent: product-owner, architect, and ux-designer for UI (planner), contracts first and the
  others in parallel.
- Produces: contract, architecture, verification-property and UX deltas; new hidden scenarios;
  assessments again if a new external service or reference implementation appears.
- Gate: artifacts valid.

### checks, review (standard route)

- Step and agent: as `spec`, in delta scope.
- Gate: as `spec`.

### close

- Step and agent: clerk.
- Produces: fast routes — `next` is `verify`. Standard route: decision `spec` with the delta packet.

A deprecation is a standard-route feature whose delta removes contracts. A port from a reference
implementation is a standard-route feature whose stories have kind `port`. A market assessment is not
part of this workflow by default; `discover` in `assess` mode can be run on the request first.

## Where it stops for a person, and what it returns

Ends: fast routes end with nothing to decide; the standard route ends at decision `spec`
([[topic:human-stops]], [[topic:amendments]]). The return value is a workflow result
([[schema:workflow-result]]).

## Feature flags (decision D31; built when the build widens)

A request for the product's flag system adds contracts, so it takes the standard route and ends at
decision `spec` ([[topic:flag-system-choice]]). A request to turn a flag on, or to remove one, is a story
with `flag_change` ([[topic:flag-lifecycle]]). The plan gate of the story-draft stage applies
F-REQUIRE, F-RECORD and F-READY ([[table:flag-checks]]).

## Team use (decision D33; built when the build widens)

The survey opens a cycle under the `open` claim, `vsdd feature route` takes the `spec` claim on the
standard route, and a change request gets its id inside a sync ([[table:work-claim-scopes]];
[[table:work-claim-checks]], T-MINT).
