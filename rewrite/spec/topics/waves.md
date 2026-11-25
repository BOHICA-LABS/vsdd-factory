---
kind: topic
id: waves
sources: [state.md#2.5, state.md#2.14, pipeline.md#2.9.5]
---

# Where a wave lives

A wave is a group of stories built in parallel. The wave is the one plan fact not in a story file: `vsdd
plan schedule` computes it and stores it in the plan's closure record.

- **Membership** is the `waves` map in `cycles/<cycle>/units/plan.json` ([[record:closure]]), computed by
  `vsdd plan schedule` ([[command:plan-schedule]]) from dependencies and limits.
- **Progress, gate rounds and gate status** are in each wave's own closure record,
  `cycles/<cycle>/units/wave.<W-NN>.json`.
- In a hub ([[topic:multi-repo-state]]), a wave has one closure record per repository with stories in the
  wave, `wave.<W-NN>.<repo>.json`; the wave is closed when every one of them is.

The wave identifier is `W-NN`, allocated per cycle by `vsdd plan schedule` ([[table:identifiers]]).

## What the schedule step computes

The schedule-and-readiness step of the `plan` workflow runs `vsdd plan schedule` as one job. It produces the
wave schedule by topological layering with priority and the stories-per-wave limit
([[config:limits]]); the assignment of each hidden scenario to the earliest wave that covers its contracts
([[artifact:holdout-scenario]]); the plan gate result; the baseline check; and an estimate of stories, waves
and agents.

A wave's closure record holds its stories and its gate rounds. A story's closure record holds `admitted_by`
and its gate evidence identities ([[record:closure]], [[topic:status-report]]). Whether a story is in
progress is derived: its wave is open and it is not done ([[topic:stored-and-derived]]).

Scenarios for a wave: there is no separate wave scenario kind; the scenarios for a wave are those whose
contracts the wave's stories implement, and the rotation sample is drawn by `vsdd` from a recorded seed
([[artifact:holdout-scenario]]).
