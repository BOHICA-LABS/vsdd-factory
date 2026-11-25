---
kind: command
id: gate-sweep
package: G7
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.10]
---

# `vsdd gate sweep`

Runs every measured maintenance sweep that is due as one job.

## Syntax

`vsdd gate sweep [--only <check>] --run <id>`. A job command ([[topic:long-commands]]).

## What it does

The `maintain` workflow's measured sweeps: "One clerk running `vsdd gate sweep`, a single job that runs
every measured sweep that is due: the dependency audit, the benchmarks (when configured), clone fidelity
(when service clones exist), spec coherence (the full `spec` check), the debt report (waivers, advisories,
flaky inherited tests and the inherited baseline; this replaces the technical-debt register sweep) and the
collection of configured feedback sources. `--only <check>` selects one." Each sweep "returns sweep data:
its findings and which of them are mechanically fixable" ([[schema:sweep-data]]); the debt-report figure it
produces is the one [[command:report-debt]] also exposes.

Fixable findings become stories: "Then `vsdd story new-fix --maintain` writes fix stories for the fixable
findings, at most ten per run, admits each" ([[command:story-new-fix]]). "While a product cycle is open,
whether or not a run holds its lock, the workflow runs in `report_only` mode."

## Who may call it

`clerk` (the `measured sweeps` step).

## Writes

Evidence ([[record:evidence]]).

## Output and failure cases

"Gate result" ([[schema:gate-result]]); "A failing check is reported with the others"
([[topic:gates]]). "A critical vulnerability that cannot be fixed mechanically opens a `finding` decision."

## Feature flags (decision D31; built when the build widens)

The job gains one check, `flags`: the rows of [[table:flag-release-checks]] and of
[[table:flag-story-checks]] whose `form` names the sweep. F-CHOICE of [[table:flag-checks]] runs inside
the spec coherence check, where its findings are advisory.
