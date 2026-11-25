---
kind: command
id: plan-schedule
package: G5
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.5]
---

# `vsdd plan schedule`

Computes waves, assigns scenarios, runs the plan gate, records the baseline check and estimates.

## Syntax

`vsdd plan schedule --run <id>`. A job command ([[topic:long-commands]]).

## What it does

"One job: computes waves from dependencies and limits and stores them in the plan's closure record, assigns
scenarios, runs the plan gate, records the baseline check, and estimates." The `schedule and readiness`
stage produces: "The wave schedule by topological layering with priority and the stories-per-wave limit, the
assignment of each hidden scenario to the earliest wave that covers its contracts, the plan gate result, the
baseline check, and an estimate of stories, waves and agents. The baseline check records the suite on the
integration branch; for an empty repository it records 'no suite yet' and requires the scaffold story."

Readiness: "The plan gate requires `chore` stories in `E-0` for what the project lacks ... `vsdd plan
schedule` puts the scaffold story alone in the first wave, so every other story starts after the build and
test commands exist." The wave map is stored once, in the plan's closure record
([[topic:stored-and-derived]], [[record:closure]]).

## Who may call it

`clerk` ("One clerk running `vsdd plan schedule`, as one job").

## Writes

The plan's closure record with `waves`, the map from wave to stories; the plan gate result and the baseline
check into evidence; the scenario assignment.

## Output and failure cases

The gate passes or the fixes route back to the story-writer, "at most 2 rounds, each ending with
`vsdd gate plan --story <id>` for the stories it changed" ([[schema:gate-result]]).
