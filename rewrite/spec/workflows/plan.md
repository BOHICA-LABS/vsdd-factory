---
kind: workflow
id: plan
package: W5
sources: [pipeline.md#2.3, pipeline.md#2.5, pipeline.md#2.9.5, pipeline.md#2.9.13, BLUEPRINT.md#4.5, BLUEPRINT.md#5.1]
---

# `plan`

Decomposes the approved specification into stories, schedules waves, assigns hidden scenarios and
proves the toolchain and CI work.

Arguments: `guidance`, `repo`. The scope (full or delta) comes from the survey. Preconditions: a
cycle is open; the specification is approved ([[table:preconditions]]).

## Stages in order

### survey

- Step and agent: clerk.
- Produces: survey; preconditions: a cycle is open and the specification is approved. The scope is
  `delta` after an amendment or on a project with a baseline.

### outline

- Step and agent: story-writer (planner), method `decompose`.
- Produces: epics and the story list with contract references and dependencies (outline data).
- Gate: `r.status == "done"`.

### write

- Step and agent: story-writer (planner), method `story`, one per epic, in parallel.
- Produces: story files — acceptance criteria traced to contract clauses, size estimate, criticality,
  kind, flags. Service clones are stories of kind `facade`.
- Gate: `r.status == "done"` for every epic.

### schedule and readiness

- Step and agent: one clerk running `vsdd plan schedule` ([[command:plan-schedule]]), as one job.
- Produces: the wave schedule by topological layering with priority and the stories-per-wave limit,
  the assignment of each hidden scenario to the earliest wave that covers its contracts, the plan
  gate result, the baseline check, and an estimate of stories, waves and agents. The baseline check
  records the suite on the integration branch; for an empty repository it records "no suite yet" and
  requires the scaffold story.
- Gate: `gate.passed`; fixes by story-writer, at most 2 rounds, each ending with
  `vsdd gate plan --story <id>` for the stories it changed.

### review

- Step and agent: adversary (reviewer), method `review-stories`; fixes by story-writer.
- Produces: review data.
- Gate: `converge()` at the `plan` perimeter, cap 2 ([[topic:convergence]]).

### close

- Step and agent: clerk.
- Produces: `next` is `wave`. Decision `plan` only when configured or when the estimate is over
  `stops.plan_ceiling_agents` ([[topic:human-stops]]).

## Readiness

No agent writes product files in this workflow. The plan gate requires `chore` stories in `E-0` for
what the project lacks, and devops builds them through the story playbook: the project scaffold for
an empty repository; a test harness with a JUnit report when the project has no suite; the CI
pipeline when `delivery.ci = "none"` and there is a remote. `vsdd plan schedule` puts the scaffold
story alone in the first wave, so every other story starts after the build and test commands exist.
The scaffold's green gate is the first run of the commands that the approved toolchain file wrote
into `factory.toml`, and a wrong command shows there as a `blocked` decision with the reason
`environment`, not later in every story. A missing tool on the machine is the same decision.

The plan gate checks coverage and shape in code: every contract the cycle added or changed is covered
by a story criterion (on a first release that is every contract; on a project with a baseline it is
the cycle's delta, so recovered or already-built contracts need no new story), every criterion traces
to a clause, the graph has no cycle, no story is over the size limit, no placeholder text, every wave
has scenarios or a recorded reason.

## Ends

Normally nothing to decide; `next` is `wave`. The return value is a workflow result
([[schema:workflow-result]]).

## Feature flags (decision D31; built when the build widens)

In the write stage the story-writer applies the setting `flags.require` and writes the flag records
([[config:flags]], [[artifact:flag]]); the plan gate holds it to them (F-REQUIRE, F-RECORD and F-READY
in [[table:flag-checks]]).
