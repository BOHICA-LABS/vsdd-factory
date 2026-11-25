---
kind: topic
id: workflow-shape
sources: [pipeline.md#1, pipeline.md#2.3, pipeline.md#2.5, pipeline.md#2.9, BLUEPRINT.md#5.1]
---

# Workflow shape

The common shape of every workflow: survey, stages, checkpoint; what a step is given; what is on disk
at each boundary; failure handling; nesting and locking. This topic links; the [[workflow:story]] and
[[workflow:wave]] files show the shape twice over.

Twelve JavaScript workflows live in `<plugin>/workflows/`: `ingest`, `discover`, `spec`, `feature`,
`plan`, `wave`, `story`, `verify`, `release`, `maintain`, `multi` and `spike`. Each starts with a
survey step that asks a deterministic command what is already done, runs only the remaining stages,
and ends with a checkpoint step that records state and names the next command. Nesting is one level:
only `story` is ever called by another workflow. Scripts steer, tools measure and hooks enforce, so
every gate is a measured fact with an evidence record, and merging, closing a wave and publishing are
refused without it ([[topic:trust-layers]]).

Every workflow is the same three parts.

1. Survey. One clerk step runs `vsdd -C <dir> survey <workflow> [--subject <id>] [--repo <name>]
   --json` ([[command:survey]]). The command first checks that the hook runner is alive for this very
   call (the caller marker the hook wrote for it) and that every precondition is met. Only then does
   it commit any pending state, open a run record, take the run lock, and make the worktrees and
   diff patches the units need. It returns the survey object: project directory, plugin root, the
   run id, preconditions with whether each is met, the units and their positions, each stage name
   recorded as `done` (a stage not listed is `todo`), the limits and thresholds from project
   configuration, open and settled decisions, and `next`. If a precondition is unmet no lock is taken
   and the script returns at once with the reason and `next`. The preconditions are in
   [[table:preconditions]]; the limits in [[table:limits]].
2. Stages. The script runs each stage that is not `done`, in order, in parallel where the stage table
   says so. Every step goes through the `step()` wrapper ([[topic:prelude-steps]]), which sets model
   and effort from the tier table, retries an unusable result, serves spike requests and counts
   agents.
3. Checkpoint. One clerk step runs `vsdd checkpoint <workflow> --run <id> --json`
   ([[command:checkpoint]]). The command reads the step records and evidence of the run, closes the
   units the evidence supports, opens a decision packet for each blocked unit, saves state, releases
   the lock, removes expired spike worktrees and returns `status`, `progress`, the decisions it
   opened and `next`. That object is the workflow's return value, so the main session receives a few
   hundred bytes per run. It decides from disk and takes no status from the script.

## How a workflow knows what is already done

The script never reads state. The survey command does, and it is code. A stage is `done` when three
things hold: the unit's closure record says so, or, inside a run that has not reached its checkpoint,
a step record with status `done` exists for the stage and unit; the artifacts that result listed still
exist and pass their structural check; and any gate the stage must pass has a passing evidence record
for the current commit (the survey measures the gate again when only the evidence record is missing).
The tool keeps no list of stages. It records the stage names the scripts report, and each script skips
the names the survey returns as done; the stage tables of the twelve workflow files are the
specification the scripts are written from. Nothing reopens by itself: a stage is marked not done only
when a person settles `revise` or `hold` (from the stage named by `stage=`, or from the review stage
by default) or when an amendment is opened. A person who edits a draft artifact by hand between runs
names the stages to run again with the workflow's `only` argument. Running the same command in a new
session therefore continues the work. In the same session the platform's own resume replays finished
steps from its cache, which is faster but is not relied on. See [[topic:resume]].

## What every step is given

In this order: the method to read, as an absolute path; its task in one or two sentences; the unit's
identifiers and file paths, never file contents (for a reviewer: the tree root, the diff patch, the
evidence paths and the review-rules path); the command it must finish with, if any; the findings or
structured claims it must act on; spike findings already obtained for the step; and, last, the step
identity it must echo back and the closing command again, so both survive a compaction of the step.
Agents read the files themselves, and the information walls decide what each agent type can open
([[topic:information-walls]]).

## What is on disk at each step boundary

- A producing step returns: its artifacts (files, and commits on the unit's branch), written by the
  agent; the evidence record for the commit it left, written by `vsdd gate`, run by the agent as its
  last act; the step record, written by the hook runner.
- A review step returns: the step record holding its findings, claim checks and coverage, written by
  the hook runner.
- A clerk step returns: whatever the command writes — run record, worktrees, evidence, fix stories,
  the pull request, the merge — written by the tool.
- The workflow returns: decision packets, closed units, saved state, released lock, `next`, written
  by `vsdd checkpoint`.

Nothing that matters lives only in a script variable. Where the script computes something that later
code must trust (a convergence verdict), the tool recomputes it from the stored records with the same
rule, and a fixture test keeps the two implementations in step. The route exists once, in the tool
([[table:routes]]).

## Failure handling

`agent()` returns `null` when a step dies or is skipped; the wrapper retries once and then marks the
unit `failed`. A failed unit does not stop its siblings. A `partial` or `failed` run is repaired by
running the same command again, because the survey starts from disk. After a successful survey every
script runs its body inside `try`/`finally`, so the checkpoint runs on every path, including an
exception (`--outcome failed`), and the lock is always released.

## Child stories and the shared budget

`wave`, `verify`, `feature`, `maintain` and `multi` all start child stories through one stamped
prelude function, `runStories()` ([[topic:prelude-run-stories]]), which enforces the agent budget and
reserve. The per-run limits are in [[table:limits]].

## Concurrency

Once preconditions pass, the survey takes a lock scoped to the repository and branch the run writes
([[topic:run-lock]]). A second run that would write the same branch is refused. `discover`,
`maintain` with `report_only` and `spike` take no lock: a spike writes only its own worktree and the
ledger, so one can start while a wave runs. A critical bug fix takes a separate lock because it works
from the release tag, on the branch `hotfix/<version>` ([[topic:hotfix-cycle]]).
