---
kind: workflow
id: discover
package: W10
sources: [pipeline.md#2.3, pipeline.md#2.5, pipeline.md#2.9.9, pipeline.md#2.9.13, BLUEPRINT.md#4.5, BLUEPRINT.md#5.1]
---

# `discover`

`scan`: research opportunities and rank ideas. `assess`: market check for one idea, brief or request.

Arguments: `mode` (`scan` or `assess`), `subject` (idea id, brief path or text), `repo`.
Preconditions: none beyond the common one, and it takes no lock ([[table:preconditions]]).

## `scan` mode, stages in order

### survey

- Step and agent: clerk.
- Produces: discovery configuration, and the idea registry with rejected and deferred ideas.

### collect

- Step and agent: clerk running `vsdd discover collect` ([[command:discover-collect]]): the tool runs
  each command configured under `[[discover.source]]` ([[config:discover]]) and stores the output.
- Produces: files under `.factory/research/inputs/<source>/<date>.*`, which the researcher reads. The
  researcher has no shell and never holds a credential.

### research

- Step and agent: researcher (planner), up to four in parallel, each only if configured: market,
  customer feedback, competitors, usage analytics; methods `market-intel` and `discovery`.
- Produces: signal files.

### synthesize

- Step and agent: researcher (planner), method `discovery`.
- Produces: candidate ideas.

### score

- Step and agent: in parallel, each scoring every idea in one step with method `score-ideas`:
  product-owner (value, alignment, urgency), architect (feasibility, effort), adversary at the
  reviewer tier (novelty and challenge). The script takes the geometric mean.
- Produces: scores.

### register

- Step and agent: clerk running `vsdd idea register` ([[command:idea-register]]).
- Produces: duplicates and ideas in cool-down removed; the rest registered.

### briefs

- Step and agent: researcher (planner), one per idea above the threshold.
- Produces: idea briefs, stored as `.factory/research/<date>-idea-<ID>.md`.

### close

- Step and agent: clerk.
- Produces: decision `ideas` with the ranked report.

A schedule is a headless command (`claude -p "/vsdd-factory:discover"` with the Workflow tool allowed)
run by cron or launchd; the recipe is documented and tested by [[packaging:documentation]].

## `assess` mode

Survey, one research step, one synthesis step by researcher giving a recommendation of go, caution or
stop with its evidence (and a draft brief when the subject was raw text), then a checkpoint that
attaches the assessment to the brief, opens the cycle and opens decision `brief`. It opens the cycle
through the same code as `vsdd checkpoint brief` ([[command:checkpoint-brief]]), so that when the
brief is approved, `spec`, `plan`, `wave`, `verify` and `release` have a cycle directory for their
closures, decisions and evidence. A fixture test holds it: assess, approve the brief, and the `spec`
survey sees an open cycle.

## Ends

Decision `ideas` (scan) or `brief` (assess) — human stops ([[topic:human-stops]]). The return value is
a workflow result ([[schema:workflow-result]]).
