---
kind: workflow
id: verify
package: W7
sources: [pipeline.md#2.3, pipeline.md#2.5, pipeline.md#2.9.6, pipeline.md#2.9.13, BLUEPRINT.md#4.5, BLUEPRINT.md#5.1]
---

# `verify`

Checks the release candidate: full suite and scans, system review, security, formal proofs, hidden
scenarios, performance, UI, docs.

Arguments: `only` (check names), `cycle`, `repo`. The scope and the list of required checks come from
the survey. The scope is `full` for a first release and `delta` otherwise. A first release is a cycle
whose baseline is empty: there was no earlier release, no ingest and no import. So the first cycle on
an ingested or imported codebase runs in delta scope, and the whole legacy system is not put through
full verification because one feature was added. Preconditions: a cycle is open; every wave is
closed, or the cycle has no plan (its route is `trivial`, `bugfix` or `hotfix`, or its kind is
`maintenance`) and every story admitted to it is delivered ([[table:preconditions]]).

## Checks in each round

The loop is `converge()` at the `release` perimeter with a cap of 3 rounds
([[topic:convergence]]). In each round the baseline runs first; if it passes, the measured job and
every due review run in parallel.

### baseline

- Step and agent: `baseline-r<k>`: clerk running `vsdd gate release --only baseline`
  ([[command:gate-release]]).
- Runs: every round, first, alone.
- Blocking when: the suite, lint or build fails; a traceability link is missing; a module's mutation
  score is under its criticality threshold; a scan reports critical or high; documentation links or
  references are stale. On a project with a baseline record these are compared against it: only what
  is new blocks, and the thresholds apply to modules the cycle touched.

### measured

- Step and agent: `measured-r<k>`: one clerk running `vsdd gate release`, which runs every measured
  check that is due as one job: `formal` (for every verification property in scope whose module
  changed), `perf` (when budgets are configured), `ui` (UI products). The checks run in parallel
  inside the tool; `--only <check>` selects one.
- Runs: beside the reviewers, after the baseline passes.
- Blocking when: a proof fails, a fuzz target crashes, or a property has no harness (fixed by a
  `harness` story that the verifier builds); a regression beyond the budget; a UI check fails.

### adversarial

- Step and agent: `system-review-<unit>-r<k>`, adversary (reviewer): one per subsystem in full scope,
  one for the delta otherwise. Method `review-system`.
- Runs: when the unit changed since its last clean pass.
- Blocking when: the convergence rule, with `system` findings in reach.

### security

- Step and agent: `security-r<k>`, security-reviewer (reviewer): triage of the scan output and review
  of the security boundaries. Method `security-review`.
- Runs: as above.
- Blocking when: the convergence rule.

### scenarios

- Step and agent: `holdout-r<k>-b<i>`, holdout-evaluator (reviewer). Method `holdout-eval`.
- Runs: when product source changed since the last scenario evidence.
- Blocking when: the thresholds in [[topic:convergence]] and [[topic:holdout-findings]].

### ui review

- Step and agent: `ux-review-r<k>`, ui-reviewer (reviewer), including the final demo when configured;
  a finding about the final demo has the owner `demo-recorder`. Method `ui-review`.
- Runs: UI products.
- Blocking when: the convergence rule.

### docs

- No step of its own. The baseline check reports stale documentation as findings.
- Runs: every round, with the baseline.
- Blocking when: the findings become a fix story of kind `docs`, written by technical-writer.

## Fix rounds and required checks

Blocking findings become fix stories (`vsdd story new-fix --verify`, which admits each) delivered
through `story` by `runStories()`, whose reserve check applies here as it does in `wave`: a fix round
that does not fit beside the reserve for the close returns `partial`. Evidence is tied to the commit,
so after a fix round the tool reports which checks are out of date and only those run again.

Required checks by the cycle's route ([[table:routes]]): standard and first release, all that apply;
`trivial`, baseline only; `bugfix`, baseline, one adversarial pass over the delta, scenarios for the
regression subset, security when the module is critical or high, formal for affected properties;
`hotfix`, baseline and one adversarial pass.

## Ends

The checkpoint writes the convergence report from the evidence and opens decision `release` — a
routine human stop ([[topic:human-stops]]). If the loop did not converge the packet says so and lists
what is open, and the person can hold or accept the risk. The return value is a workflow result
([[schema:workflow-result]]).

## Feature flags (decision D31; built when the build widens)

`vsdd gate release` runs one more check, `flags`, and its baseline check holds the flag runbook
([[command:gate-release]]). The `release` packet gains a section that lists the flags the release ships,
turns on and removes, and how to roll each back ([[topic:flag-lifecycle]]).
