---
name: orchestrator-per-story-delivery
description: Orchestrator workflow reference for the per-story TDD delivery cycle (red-green-refactor, PR, review, merge). Loaded by the orchestrator agent during the corresponding phase. Not directly invokable.
---

> **Global Operating Rules:** Read `../../docs/FACTORY.md` and `../../docs/VSDD.md` for factory-wide constraints.


# Per-Story Code Delivery Cycle

**Canonical source** for per-story TDD delivery. The `deliver-story` skill (`skills/deliver-story/SKILL.md`) is the entry point that dispatches through this workflow; this file is the playbook. If the two disagree, this file wins — update the skill to match.

Reference file for the orchestrator. Load during Phase 3 implementation.

## Per-Story Delivery (within each wave)

1. Spawn devops-engineer: "Create worktrees for Wave N stories"
2. For each story (parallelized within wave groups):
   a. Spawn test-writer: "Create stubs in .worktrees/STORY-NNN/"
   b. Spawn test-writer: "Write failing tests in .worktrees/STORY-NNN/"
   c. Spawn implementer: "Implement via TDD in .worktrees/STORY-NNN/"
   c2. Run Step 4.5 adversary convergence loop (BC-5.39.001, ADR-017):
      - Dispatch adversary against the story diff + spec + anchored BCs only (BC-5.39.002 scope)
      - Loop until passes_clean >= 3 AND last_classification == "NITPICK_ONLY"
      - State file: .factory/cycles/<cycle-id>/<story-id>/adversary-convergence-state.json
      - Step 5 MUST NOT begin while convergence criterion is not met
      - Context WALL: adversary MUST NOT see .factory/holdout-scenarios/story-scenarios/**
   c3. Run story-level holdout gate (BC-5.39.003):
      - Spawn holdout-evaluator with ONLY: .factory/holdout-scenarios/story-scenarios/STORY-NNN/
        + .factory/specs/product-brief.md + running binary (built in .worktrees/STORY-NNN/)
      - Evaluator exercises the story's public surface via wire-level output assertions;
        scope is bounded to the story's TOUCHED surface — minutes, not hours
      - Scenarios: 2–4 per story; SINGLE-USE; authored by product-owner at story-materialization
        time (Phase 2); test-writer and implementer have NEVER read them (strict asymmetry wall)
      - Gate threshold: every scenario score >= 0.80 (mean >= 0.80 across 2–4 scenarios)
      - BLOCKING: any unsatisfied scenario → holdout-evaluator reports OBSERVED_BEHAVIOR_ONLY
        to orchestrator (never scenario text — contamination control prevents scenario leakage
        to implementer) → orchestrator dispatches implementer with observed-behavior description
        → implementer fixes → adversary convergence streak RESETS to 0/3 → re-gate from c2
      - PASS: all scenarios satisfied → mark each scenario lifecycle_status: consumed and
        move to .factory/holdout-scenarios/evaluations/story-STORY-NNN/ (scenarios are
        single-use: consumed at first evaluation, never re-run for this story)
      - Three-tier separation: story scenarios (here), wave scenarios (Step 3f wave gate),
        and end-of-pipeline scenarios (Phase 4) are independent pools — consuming story
        scenarios does NOT affect the wave or final evaluation pools
      - Step 5 (demo recording) MUST NOT begin until holdout gate passes
   d. Spawn demo-recorder: "Record per-AC demos in .worktrees/STORY-NNN/.
      Output to docs/demo-evidence/<STORY-ID>/ (committed to feature branch).
      The <STORY-ID> subfolder prevents evidence-report.md and AC-*.md collisions across stories.
      Use VHS for CLI or Playwright for web. Both success and error paths.
      Generate docs/demo-evidence/<STORY-ID>/evidence-report.md."
   e. Spawn implementer: "Push feature/STORY-NNN to remote"
   f. Spawn pr-manager: "Run the full PR process for STORY-NNN.
      Feature branch: feature/STORY-NNN. Target: develop.
      Project path: <resolved-project-path>.
      AUTHORIZE_MERGE=yes.
      Follow your 9-step process: populate PR description from template,
      verify demo evidence, create PR via github-ops, security review,
      pr-reviewer convergence loop, wait for CI, dependency check, merge,
      post-merge cleanup. Execute ALL 9 steps — emit STEP_COMPLETE for each.
      Do NOT skip any step. Do NOT exit when a sub-agent returns APPROVE —
      that is step 5's input, not your completion signal.
      Read ../../templates/pr-description-template.md for the PR body format."
      NOTE: Do NOT compose the PR body or gh commands in this task —
      pr-manager owns the full PR lifecycle and uses its own templates.
   g. Spawn devops-engineer: "Remove worktree .worktrees/STORY-NNN"
3. After all stories in wave merged: run wave integration gate
4. Wave gate passes -> start next wave

## Wave Integration Gate

After all stories in a wave have merged to develop, run this loop (max 10 cycles):

a. Spawn implementer: "Run FULL test suite on merged develop"
b. Spawn adversary: "Review combined wave diff with fresh context"
c. Spawn code-reviewer: "Constructive code review of wave diff (post-adversary)"
d. Spawn security-reviewer: "Wave security review"
e. Spawn consistency-validator: "Cross-story consistency check — spec traceability,
   BC coverage, story dependency integrity across wave"
f. Spawn holdout-evaluator: "Re-evaluate affected holdout scenarios"
g. Spawn demo-recorder: "Record wave-level integration demos" (if multi-story wave)
h. If ALL pass → wave gate passes, proceed to next wave
i. If issues found:
   - Route findings per VSDD.md Feedback Integration Loop
   - Spawn pr-manager to triage findings into fix stories (STORY-NNN-FIX-001)
   - Run each fix story through per-story delivery (worktree → fix → demo → PR → review → merge)
   - Go back to (a) to re-validate
After 10 cycles: escalate to human.

## Dependency-Ordered Merge Coordination

Before allowing pr-manager to merge a story PR:
1. Read the story's `depends_on` from STORY-INDEX.md
2. For each dependency: verify its PR is already merged
3. If any dependency PR is NOT merged: hold this merge until it is
4. After all deps merged: proceed with merge

## Task Sizing Rules

When spawning implementers or test-writers:
- Check story points: S/M (1-5) -> max 2 stories per agent
- L/XL (8-13) -> exactly 1 story per agent
- NEVER combine "write code" and "run full test suite" in one agent task
- If an agent times out, spawn a NEW agent with narrower scope
- Before each implementation wave, verify the workspace compiles:
  spawn a test-writer with task "run cargo check and report status"

## Story Split Recovery

When pr-manager returns with "diff too large, recommend split":

1. **Close the PR** — Spawn github-ops: "Close PR without merging, add label `split-needed`"
2. **Keep the worktree** — Do NOT remove `.worktrees/STORY-NNN/` — the work is preserved
3. **Human approval** — Present the split recommendation to the human:
   "STORY-NNN PR is too large (>500 lines). Recommend splitting. Approve split?"
4. **Split the story** — Spawn story-writer: "Split STORY-NNN into smaller stories.
   Preserve the existing implementation in the worktree. Update STORY-INDEX.md,
   dependency-graph.md, and wave schedule with the new stories."
5. **Human reviews split** — Human approves the new story breakdown
6. **Create sub-worktrees** — Spawn devops-engineer: "Create worktrees for split stories,
   cherry-pick relevant commits from the original worktree"
7. **Resume per-story delivery** — Each split story goes through the normal cycle
   (tests → implementation → demo → PR → review → merge)
8. **Clean up original** — After all split stories merge, remove the original worktree

If human rejects the split, the orchestrator can override the size finding
(add a note to review-findings.md) and tell pr-manager to continue the review loop.

## Token Budget Check

Before spawning test-writer or implementer for a story, read the story's
**Token Budget Estimate** section:
- If estimated budget exceeds 60% of the target agent's context window,
  flag the story for splitting before proceeding
- If the Token Budget Estimate section is missing, spawn the story-writer
  to add it before proceeding

## Red Gate Delegation

For each story in the current wave:
1. Spawn test-writer: "Create compilable stubs for STORY-NNN" (Stub Architect)
2. Wait for completion. Verify cargo check passes.
3. If cargo check fails: spawn new test-writer to fix stubs
4. Spawn test-writer: "Write failing tests for STORY-NNN" (Test Writer)
5. Wait for completion. Verify cargo test compiles + all tests fail.
6. If Red Gate fails: spawn new test-writer to fix tests
7. Record both steps in `.factory/cycles/<cycle-id>/<story-id>/implementation/red-gate-log.md`

Only after Red Gate verified -> spawn implementer for STORY-NNN

## Verification Discipline

After every specialist dispatch, independently verify the exit condition before proceeding. Agent reports are claims — verification output is evidence. Run the verification command yourself and read the full output.

## Model Selection

Use the least powerful model that can handle each specialist dispatch. Mechanical tasks (worktree ops, stubs, demo recording) use fast/cheap models. Tasks requiring BC understanding or multi-file coordination use standard models. Review triage and architectural judgment use the most capable model.

If an agent reports BLOCKED or DONE_WITH_CONCERNS, consider re-dispatching with a stronger model before splitting the task.

## Adversarial Pass Context Injection

When spawning the adversary for Pass 2+:
1. Include FINDINGS.md (cross-pass tracker) in the context
2. Include the phase-specific prompt template
3. Explicitly state: "These findings were resolved since your last pass:
   [list]. Verify the fixes in Part A, then find NEW issues in Part B."

## State-Manager Ordering (MANDATORY)

State-manager runs LAST in every dispatch burst. Never dispatch state-manager concurrently with story-writer or product-owner — version-race pattern causes the same regression repeatedly. Wait for all artifact-producing agents to complete, then dispatch state-manager to update indexes and STATE.md.

## Main-Checkout Sync Protocol

**BC-5.43.001 v1.3 — ADR-031 v1.3 §Decision 2 — S-21.01 Layer-2 deliverable**

This protocol is the **safety-net layer** for INV-E21-001 (Nested Worktree Path Exclusivity).
It guards against the server-side origination threat vector: a contributor PR merged via
`gh pr merge` may carry `.factory/`-pathed files that then arrive on the main product
checkout via a subsequent `git pull origin develop`, where `.factory/` is physically mounted
as a nested worktree. Without this gate, the subsequent merge or working-tree update would
silently destroy factory artifact content.

### Mandatory Pre-Check: Before Any git pull / git merge on the Main Checkout

Before executing **any** `git pull`, `git merge`, or `git checkout <target>` command that
advances the HEAD of the product branch on the **main product checkout** (the checkout where
`.factory/` is physically mounted as a nested worktree), the orchestrator MUST run the
following path-intersection pre-check:

```
git diff --name-only HEAD..<target-ref>
```

Where `<target-ref>` is the ref being merged, pulled, or checked out (e.g.,
`origin/develop`, `feature/S-21.01`, a specific SHA). For `git checkout <branch>`
where `<branch>` is a local branch name, use `HEAD..origin/<branch>` (the remote
tracking ref) as `<target-ref>` — this is the conservative form that reflects the
server-side state rather than a possibly-stale local ref that may not include all
merged commits.

**This requirement covers:**
- Documented orchestrator dispatch steps (e.g., post-merge sync via `git pull origin develop`)
- Ad-hoc operator Bash issued on the main checkout
- Any `git pull --ff-only`, `git pull --rebase`, or `git checkout <target>`
  that results in a working-tree update on the product branch

### If .factory/ Paths Appear in the Diff: HALT with FactoryPathDeletionInMergeDiff

If the output of `git diff --name-only HEAD..<target-ref>` contains any line matching
`^\.factory/`, the operation MUST be **halted before the merge/pull/checkout command
is executed**. Emit the following actionable error:

```
HALTED: FactoryPathDeletionInMergeDiff

Product-branch merge/checkout would modify .factory/ path(s) owned by the
factory-artifacts worktree.

Paths at risk:
  <path1>
  <path2>
  ...

This indicates a dual-tracking condition (issue #342 class): these paths are tracked
on both the product branch and the factory-artifacts worktree. Proceeding would
silently destroy factory artifact content.

Required actions before proceeding:
  1. On the source branch: ensure these paths are NOT tracked on the product branch
     (git rm --cached <path> per companion issue #341).
  2. Verify the factory-artifacts worktree is not serving uncommitted edits to these paths.
  3. Once the dual-tracking is resolved, the merge/checkout is safe to retry.
```

The merge, pull, or checkout command MUST NOT be executed after this error. Escalate to
the orchestrator for human review.

**Any `.factory/` path in the product-branch diff is anomalous** — halt even for additions,
not just deletions (BC-5.43.001 Invariant 2).

### If No .factory/ Paths: Proceed Transparently

When `git diff --name-only HEAD..<target-ref>` returns no line matching `^\.factory/`,
the merge, pull, or checkout operation MUST proceed normally. No additional gate fires.
The pre-check is transparent and safe to proceed on clean diffs.

### Fail-Open When git diff Fails

If the `git diff --name-only HEAD..<target-ref>` command itself fails — including a
non-zero exit code (e.g., network error, unresolvable ref) or git unavailable — the
agent MUST:
1. Log a warning describing the failure and the non-zero exit code or error
2. Proceed with the merge/pull/checkout (fail-open)

An unresolvable pre-check is NOT a blocking condition — it means the pre-check could not
run, not that a hazard was detected. This ensures no session is wedged by a transient
git error (BC-5.43.001 Invariant 4).

### Protocol Exclusions (Not In Scope for This Gate)

The following surfaces are explicitly excluded and do NOT require the pre-check:

- **`pr-manager.md` `gh pr merge`:** Server-side merge — no local Bash command advances
  the product-branch HEAD; excluded by BC-5.43.001 PC3.
- **`devops-engineer.md §Inter-Wave Rebase`:** Operates on the story worktree
  (`.worktrees/STORY-NNN/`) where `.factory/` is NOT physically mounted.
- **State-manager `git -C .factory` operations:** Always scoped to the factory-artifacts
  worktree; never touches the product-branch HEAD.
- **`git fetch` and other non-working-tree operations:** Do not modify the working tree;
  pre-check is not required (BC-5.43.001 PC3).

---

## Context Discipline for Sub-Agent Spawning (DF-021)

When spawning sub-agents, pass only the specific detail files relevant to
the agent's task, not entire directories:
- **Implementer:** module-decomposition.md + dependency-graph.md + api-surface.md +
  relevant BC files
- **Formal-verifier:** verification-architecture.md + purity-boundary-map.md +
  tooling-selection.md + relevant VP files
- **Story-writer:** module-decomposition.md + dependency-graph.md
- **Test-writer:** api-surface.md + test-vectors.md + relevant BC files
- **Consistency-validator:** ARCH-INDEX.md + verification-coverage-matrix.md +
  BC-INDEX.md + VP-INDEX.md
