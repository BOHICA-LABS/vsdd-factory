---
kind: workflow
id: maintain
package: W11
sources: [pipeline.md#2.3, pipeline.md#2.5, pipeline.md#2.9.10, pipeline.md#2.9.13, BLUEPRINT.md#4.5, BLUEPRINT.md#5.1]
---

# `maintain`

Runs maintenance sweeps and delivers the fixes that are mechanical.

Arguments: `sweeps` (names), `report_only`, `repo`. Preconditions: none beyond the common one.
`maintain` runs report-only while a product cycle is open ([[table:preconditions]]). Sweeps run in
parallel and each returns sweep data: its findings and which of them are mechanically fixable.

## Sweeps in order

### measured sweeps

- Step and agent: one clerk running `vsdd gate sweep` ([[command:gate-sweep]]), a single job that
  runs every measured sweep that is due: the dependency audit, the benchmarks (when configured),
  clone fidelity (when service clones exist), spec coherence (the full `spec` check), the debt report
  (waivers, advisories, flaky inherited tests and the inherited baseline) and the collection of
  configured feedback sources. `--only <check>` selects one.
- Runs when: always.

### dependency triage

- Step and agent: security-reviewer (reviewer), method `security-review`, on the audit output.
- Runs when: the audit reported something.

### docs drift

- Step and agent: technical-writer (builder), method `docs-drift`.
- Runs when: always.

### pattern consistency

- Step and agent: code-reviewer (reviewer), method `pattern-sweep`, which reads the code against
  `conventions.md` and needs no diff.
- Runs when: always.

### scenario freshness

- Step and agent: two steps, split along the wall. holdout-evaluator (reviewer), method
  `holdout-eval`: re-runs the scenarios against the product. scenario-reviewer (reviewer), method
  `review-scenarios`: judges staleness and coverage against the contracts, which the evaluator may
  not read.
- Runs when: hidden scenarios exist.

### accessibility

- Step and agent: ui-reviewer (reviewer), method `ui-review`, lens accessibility.
- Runs when: UI products.

### design drift

- Step and agent: ui-reviewer (reviewer), method `ui-review`, lens design drift.
- Runs when: a design system exists.

### risks and assumptions

- Step and agent: researcher (planner), method `risk-sweep`.
- Runs when: the domain specification has them.

### post-release signals

- Step and agent: researcher (planner), method `discovery`, on what the sweep job collected.
- Runs when: analytics or feedback sources configured.

## Fix stories, cycle, ends

Then `vsdd story new-fix --maintain` ([[command:story-new-fix]]) writes fix stories for the fixable
findings, at most ten per run, admits each, and each is delivered through `story` by `runStories()`,
with the same gates as any story. While a product cycle is open, whether or not a run holds its lock,
the workflow runs in `report_only` mode. The checkpoint writes the report. A critical vulnerability
that cannot be fixed mechanically opens a `finding` decision ([[topic:human-stops]]).

Fixes are delivered inside a cycle. When the run is about to deliver fix stories and no cycle is
open, the tool opens a maintenance cycle (kind `maintenance`, its request written by code from the
sweep findings), so the fixes go through `verify` in delta scope and a `release` decision like any
other change, and the route after the run is `verify`. When a product cycle is open the run is
report-only, so the question does not arise. A run with nothing to deliver opens no cycle.

Ends: a report; no routine stop of its own. When fixes were delivered, the cycle it opened ends at
decision `release`. The return value is a workflow result ([[schema:workflow-result]]).

## Feature flags (decision D31; built when the build widens)

The measured sweeps gain a check, `flags` ([[command:gate-sweep]]). A temporary flag that is on and whose
removal condition is met becomes a cleanup story, after a person has confirmed it where the project's
setting asks for that; flag reads around the interface become fix stories ([[topic:flag-lifecycle]]).

## Team use (decision D33; built when the build widens)

A run that delivers fixes takes the standing `cycle` claim and a claim for each fix story; a report-only
run takes none ([[table:work-claim-scopes]]).
