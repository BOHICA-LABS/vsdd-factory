---
kind: command
id: feature-route
package: G5
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.4]
---

# `vsdd feature route`

Derives a change request's route from the stored change analysis and writes it into `cycle.json`.

## Syntax

`vsdd feature route --run <id>`

## What it does

The delta-and-route stage's closing command: "the tool derives the route from the delta record the step just
wrote, writes it into `cycle.json`, and for the standard route marks the run as an amendment. No clerk
step." The route: "`trivial` when the intent is not a bug fix, no contract changes or additions, the
estimate is within the story size limit and no critical module is touched. `bugfix` when the intent is a
bug fix and no contract changes; `hotfix` when its severity is critical. Otherwise `standard`. With
`cross_repo` a fast route keeps its name and yields one story per repository, joined by `depends_on`"
([[table:routes]]).

"The route result also lists `readiness`: the chore stories the project still needs (a test harness and
JUnit report when the baseline has no suite; CI when `delivery.ci = "none"` and there is a remote)."

Writing the route into `cycle.json` is what makes it readable anywhere: "The `route` field of the cycle's
`cycle.json`, which is tracked, so the route table and the `verify` survey can read it in any clone; the
amendment flag in the run record" ([[record:cycle]]).

## Who may call it

"`architect`, `clerk`" — the architect, "as the last act of its delta step".

## Output and failure cases

`{ok, route, amendment, stories, readiness}` ([[schema:route-data]]). Refuses when the run has no stored
delta record to derive from.

## Team use (decision D33; built when the build widens)

On the standard route the command takes the `spec` claim, and answers `ok: false` with the holder when
another clone has it ([[table:work-claim-scopes]]).
