---
kind: topic
id: risks-pipeline
sources: [pipeline.md#7]
---

# Pipeline risks

The risks the pipeline design names and what mitigates each.

## R1. The hook on StructuredOutput receives the whole result

- Effect: settled. The platform probe of 2026-11-23 showed that the hook on `StructuredOutput`
  receives the whole result and that a block makes the agent correct it and return again.
- Mitigation: none now. A later platform release could change it. The weekly conformance tier
  re-checks it. There is no `vsdd step record` command: nine of the twenty agents have no shell, so
  that fallback could not have worked. If the behaviour ever changes, the hook stores the result at
  agent stop from the subagent's transcript, which needs no tool in the agent, and grounding moves
  into the verdict computation.

## R2. The convergence rule exists twice, in JavaScript and in the tool

- Effect: they drift, and a script thinks a unit converged when the tool does not.
- Mitigation: conformance fixtures. The tool's answer always wins, so drift costs a wasted run, not a
  wrong merge. The route exists once, in the tool.

## R3. One clean pass may let more defects through than three

- Effect: defects reach the wave or release perimeter, or users.
- Mitigation: three perimeters, coverage accounting, two lenses on critical modules. Measure the rate
  of findings first caught at a later perimeter during the end-to-end proof and raise the cap or add
  a lens if it is high.

## R4. The driver depends on the main session following `next`

- Effect: "a skill can start a workflow" is unconfirmed; more typing.
- Mitigation: the briefing prints the exact command, so a person can drive by hand; enforcement does
  not depend on the driver.

## R5. An agent could fabricate a gate result or write an evidence file

- Effect: a script takes a wrong branch.
- Mitigation: guards read the evidence store, hooks deny agent writes to it, and the tool re-measures
  at delivery. A determined agent with a shell is the hook designer's residual risk.

## R6. The 1,000-agent ceiling, 16 concurrent agents and same-session resume

- Effect: a large wave cannot finish in one run, and a long run dies with its session.
- Mitigation: story and agent budgets with `partial` results; the survey resumes from disk in any
  session.

## R7. The `vsdd` tool is now the critical component

- Effect: about sixteen commands; a bug there is a pipeline bug; a schema mismatch breaks a script.
- Mitigation: build and test it first; schemas from one source; the script harness runs against
  recorded tool outputs.

## R8. Parallel stories merging into one branch

- Effect: a change that passed alone breaks the branch after another merge.
- Mitigation: delivery does not rebase because the base moved: it rebases only when the pull request
  conflicts, and then a clean rebase (same `git patch-id`) keeps the verdict and the green evidence
  and needs a build check only, while a changed patch gets a scoped review. The merge passes the
  current head, whose patch id must match the verdict's. The wave gate runs the full suite on the
  integration head, and breakage found there is fixed forward as stories.

## R9. The hidden-scenario wall for the evaluator

- Effect: an evaluator with a shell can read source it should not see.
- Mitigation: the evaluator's tools are restricted to starting the product and exercising it; the
  leak check on its text before fix stories are written. Residual for interpreted products.

## R10. Spike worktrees start from the main checkout's head

- Effect: the experiment misses the asking story's work.
- Mitigation: the request carries `ref` and the runner checks it out first. Needs confirming on the
  platform.

## R11. Auto-deferral of blocked stories changes the wave plan without a person

- Effect: scope appears to slip.
- Mitigation: nothing is dropped: the release gate requires every planned story to be delivered or
  descoped by a person, and deferrals are listed in the wave result.

## R12. Reviewer cost

- Effect: Opus at extra-high effort on every pass, plus a fixed cost per agent start.
- Mitigation: a realistic story is 8 to 12 agent starts, and a six-story wave 70 to 90, against well
  over twenty per story before; the lens count follows criticality; clerk steps run at low effort.
  The end-to-end proof measures the real figures.

## R13. Named spikes overlap with the user's own `resume-spike` skill

- Effect: two ways to resume.
- Mitigation: the workflow uses the same branch, worktree and ledger file names, so either can resume
  the other's work.
