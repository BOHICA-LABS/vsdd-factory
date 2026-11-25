---
kind: command
id: gate-release
package: G6b
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.6]
---

# `vsdd gate release`

Runs the release checks as one job: baseline, formal, perf and ui.

## Syntax

`vsdd gate release [--only <check>] [--repo <name>] --run <id>`. A job command
([[topic:long-commands]]).

## What it does

The `verify` workflow's measured checks as one job. The baseline check ("`vsdd gate release --only
baseline`") "runs every round, first, alone". It blocks when: "The suite, lint or build fails; a
traceability link is missing; a module's mutation score is under its criticality threshold; a scan reports
critical or high; documentation links or references are stale. On a project with a baseline record these
are compared against it: only what is new blocks, and the thresholds apply to modules the cycle touched"
([[record:baseline]]).

The measured job "runs every measured check that is due as one job: `formal` (for every verification
property in scope whose module changed), `perf` (when budgets are configured), `ui` (UI products). The
checks run in parallel inside the tool; `--only <check>` selects one." It blocks when: "A proof fails, a
fuzz target crashes, or a property has no harness (fixed by a `harness` story that the verifier builds); a
regression beyond the budget; a UI check fails."

Required checks by the cycle's route: "standard and first release, all that apply; `trivial`, baseline
only; `bugfix`, baseline, one adversarial pass over the delta, scenarios for the regression subset,
security when the module is critical or high, formal for affected properties; `hotfix`, baseline and one
adversarial pass." "Evidence is tied to the commit, so after a fix round the tool reports which checks are
out of date and only those run again."

## Who may call it

`clerk` ("baseline-r<k>", "measured-r<k>").

## Writes

Evidence ([[record:evidence]]). The results feed the convergence report the checkpoint writes.

## Output and failure cases

"Gate result" ([[schema:gate-result]]); failing checks are reported with the others, and the fixes route
through [[command:story-new-fix]] (`--verify`).

## Feature flags (decision D31; built when the build widens)

The job gains one check, `flags`, and the baseline check gains F-RUNBOOK: the rows of
[[table:flag-release-checks]] and of [[table:flag-story-checks]] whose `form` names this job.

## Team use (decision D33; built when the build widens)

Wherever the target branch has an upstream, with team use on or off, the gate lists the hand commits
since the last wave as `hand_commits`, each with its patch, and fails when it cannot measure that range,
as the wave gate does ([[table:team-delivery-checks]], T-NOTICE).
