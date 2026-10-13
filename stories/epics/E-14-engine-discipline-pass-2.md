---
document_type: epic
epic_id: "E-14"
version: "1.5"
title: "Engine Discipline Pass-2 — Process-Gap Remediation"
status: draft
prd_capabilities: [CAP-016, CAP-026]
prd_frs: []
anchor_strategy: greenfield-discipline-gap-codification
priority: P1
target_release: "v1.0-feature-engine-discipline-pass-2"
story_count: 11
subsystems_affected: [SS-04, SS-05, SS-06]
producer: product-owner
timestamp: 2026-05-07T00:00:00Z
phase: 2
level: L3
traces_to: .factory/cycles/v1.0-feature-engine-discipline-pass-1/adv-cycle-pass-1.md
depends_on: ["E-12", "E-13"]
last_amended: "2026-10-13 (v1.5) — S-14.10 SPLIT by human decision (story-writer): NEW S-14.11 (CHANGELOG lint, version-pin sweep, SHA-label truth check, persist-task parity / convergence-prose lint; 7 pts; depends_on S-14.10); S-14.10 reduced to reconciliation, persistence and the dispatch hook (5 pts); story_count 10 -> 11; inputs += S-14.11. | (v1.4) — S-14.10 story file authored and registered (story-writer; D-1257 follow-up opened index-only in this epic): story_count 9 -> 10; Stories Planned row, Process-Gap Source Mapping row, SS-05/SS-06 subsystem-anchor bullets, Stories section range S-14.01..S-14.10, inputs += S-14.10 and the D-1257 lessons file. | (v1.3) — 2026-08-30 frontmatter normalization: add level: L3 (story-writer spec-hygiene sweep)"
inputs:
  - .factory/cycles/v1.0-feature-engine-discipline-pass-1/adv-cycle-pass-1.md
  - .factory/cycles/v1.0-feature-engine-discipline-pass-1/F5-pass-1-fix-plan.md
  - .factory/stories/epics/E-12-engine-governance.md
  - .factory/stories/epics/E-13-artifact-integrity.md
  - .factory/stories/S-14.10-per-story-delivery-process-gap-codification.md
  - .factory/stories/S-14.11-per-story-delivery-lint-gates-changelog-version-pins-sha-labels-convergence-prose.md
  - .factory/cycles/v1.0-brownfield-backfill/lessons.md
input-hash: "eae9e5d"
modified:
  - "v1.2 2026-05-11: F-P8-002 fix burst — forward-ref note corrected"
  - "v1.3 2026-08-30: frontmatter normalization — level: L3 added"
  - "v1.4 2026-10-13: S-14.10 registered (D-1257 follow-up); story_count 9->10"
  - "v1.5 2026-10-13: S-14.10 split by human decision; S-14.11 registered; story_count 10->11"
---
<!-- [process-gap] Frontmatter fields anchor_strategy, depends_on extend the canonical epic-template baseline. Template update tracked separately. -->

# Epic E-14: Engine Discipline Pass-2 — Process-Gap Remediation

## Description

Closes the five process-gap findings surfaced during F5 pass-1 adversarial review
of the v1.0-feature-engine-discipline-pass-1 cycle (adv-cycle-pass-1.md). These gaps
are not implementation defects in the cycle's deliverables (E-12/E-13) — they are
structural weaknesses in the factory pipeline itself: the adversary dispatch workflow,
the convergence state protocol, the pre-F5 lint gate, policy rubric injection, and
the pr-manager step-completion discipline.

E-12 (Engine Governance) and E-13 (Artifact Integrity) codified per-story adversary
convergence and artifact path discipline. E-14 completes the triangle by hardening
the dispatch and validation infrastructure that E-12/E-13 depend on to function
correctly at scale. Without E-14, the F5/F7 workflow has latent friction points that
will recur in every subsequent cycle.

**Critical sequencing note:** S-14.01 (bootstrap convergence-state backfill) MUST
complete before F7 close-out of v1.0-feature-engine-discipline-pass-1 — its absence
causes CONVERGENCE_STATE_MISSING for the 3 bootstrap stories on the next wave-gate
dispatch. All other stories in E-14 (S-14.02 through S-14.05) are forward-looking
improvements for the pass-2 cycle and have no blocking dependency on F7 close.

## PRD Capabilities Covered

| Capability ID | Name | Priority |
|--------------|------|----------|
| CAP-016 | Self-referential process codification via adversarial review | P1 |
| CAP-026 | Enforce per-story adversarial convergence as a pipeline gate | P1 |

## Capability Anchor Justification

**CAP-016 anchor:** All five stories in E-14 are process codification artifacts —
documenting and automating factory workflow behaviors that currently depend on manual
memory or luck. This matches CAP-016's intent ("Self-referential process codification
via adversarial review") as established in E-12.

**CAP-026 anchor:** S-14.01 (state backfill), S-14.02 (dispatch automation), and
S-14.04 (policy rubric injection) directly extend the convergence enforcement
infrastructure established by E-12. S-14.03 (pre-F5 lint) and S-14.05 (pr-manager
step discipline) are guard-rails on the pipeline that ensures convergence evidence
is well-formed before reaching the gate.

## Subsystem Anchors

- **SS-05 (Pipeline Orchestration):** S-14.01 (convergence state file authoring),
  S-14.02 (F5 phase skill / lobster step automation), S-14.04 (adversary dispatch
  pre-injection step), S-14.05 (pr-manager step discipline — per-story-delivery.md
  and orchestrator AGENT.md changes), S-14.10 (Step 4.5 reconcile / persist steps and the dispatch hook
  in both per-story-delivery workflow documents and the story-writer prompt), S-14.11 (the four lint gates' wiring in the same Step 4.5 text). All touch pipeline workflow files under
  `plugins/vsdd-factory/workflows/phases/` or `plugins/vsdd-factory/agents/`.
- **SS-04 (Plugin Ecosystem):** S-14.03 (pre-F5 lint hook or WASM validation).
- **SS-06 (Skill Catalog):** S-14.02 (updates F5 adversarial-refinement skill or
  adds an orchestration wrapper), S-14.04 (updates adversary dispatch skill step),
  S-14.11 (create-story skill CHANGELOG clause).

## Stories Planned

| Story | Title | Size | Subsystem | Priority | Delivery Order |
|-------|-------|------|-----------|----------|----------------|
| S-14.01 | Bootstrap convergence-state backfill (PG-2) | XS | SS-05 | **P0 — BLOCKING for F7** | FIRST (before F7 close-out) |
| S-14.02 | F5 dispatch → state-manager persist automation (PG-1) | S | SS-05, SS-06 | P1 | Second in pass-2 cycle |
| S-14.03 | Pre-F5 artifact lint for placeholder frontmatter (PG-3) | S | SS-04 | P2 | Third |
| S-14.04 | Policy-rubric auto-injection for adversary dispatch (PG-4) | S | SS-05 | P1 | Fourth |
| S-14.05 | pr-manager Step-4/Step-5 early-exit codification (PG-6) | M | SS-05 | P1 | Fifth |
| S-14.06 | [F-P6-002 deferred MEDIUM #1 — filed in F-P6 fix burst] | S | TBD | P2 | Sixth |
| S-14.07 | [F-P6-002 deferred MEDIUM #2 — filed in F-P6 fix burst] | S | TBD | P2 | Seventh |
| S-14.08 | [F-P6-002 deferred MEDIUM #3 — filed in F-P6 fix burst] | S | TBD | P2 | Eighth |
| S-14.09 | Forensic marker cleanup (F-P6-004) | M | SS-04, SS-07 | P2 | Ninth |
| S-14.10 | Per-story-delivery process-gap codification, slice 1 of 2: story-status reconciliation, adversary-review persistence and the dispatch-blocking hook (D-1257; S-25.08 L5-004/L6-002/L7-003, F-S2508-L8-004) | M (5 pts) | SS-05 | P1 | Tenth (independent; blocks S-14.11; blocked from `ready` until two BC clauses are authored) |
| S-14.11 | Per-story-delivery lint gates, slice 2 of 2: CHANGELOG citation discipline, diff-wide version-pin sweep, SHA-label truth check, persist-task parity and convergence-prose lint (D-1257/D-1259/D-1260; F-S2508-L6-001, F-S2508-L9-*, F-S2508-L10-001, F-S2508-L11-001, F-S2508-L8-004/L10-002/L12-001) | M (7 pts) | SS-05, SS-06 | P1 | Eleventh (`depends_on` S-14.10; blocked from `ready` until four BC clauses are authored) |

> **Forward-reference note (F-P7-004, amended F-P8-002):** All E-14 stories EXCEPT S-14.01 carry `cycle: v1.0-feature-engine-discipline-pass-2` in their frontmatter. S-14.01 carries `cycle: v1.0-feature-engine-discipline-pass-1` because it MUST close before F7 close-out of the current cycle (per D-355-AMEND and D-359). The forward-reference pattern applies to S-14.02 through S-14.09; the pass-2 cycle directory will be opened by the state-manager when E-14 work begins. This is accepted planning-tier convention: stories are assigned to their delivery cycle at authoring time, before the cycle directory is opened. The forward reference for S-14.02–S-14.09 is not a broken reference or a data-entry error.

## Dependency Topology

```
S-14.01 (bootstrap backfill) ── F7 gate ──► F7 close-out + next wave-gate unblocked
                                               │
                                               ▼
S-14.02 (dispatch automation) ──────────────► pass-2 dispatch
         │
         ▼
S-14.04 (policy-rubric injection) ──────────► pass-2 dispatch
         │
         ▼
S-14.03 (pre-F5 lint) ──────────────────────► pass-2 dispatch
         │
         ▼
S-14.05 (pr-manager discipline) ─────────────► pass-2 dispatch
```

S-14.01 is the only story with a hard dependency on F7 timing. S-14.02 through
S-14.05 can be delivered in any order within the pass-2 cycle, but the recommended
delivery sequence is S-14.02 → S-14.04 → S-14.03 → S-14.05 (highest → lowest
impact on convergence evidence quality).

S-14.10 and S-14.11 form a two-story chain: `S-14.10 ──► S-14.11` (S-14.11 `depends_on` S-14.10). S-14.10 builds the `story-delivery-gates` crate skeleton, the dispatch hook and the reconcile/persist text; S-14.11 adds four lint gates to that crate and extends the hook. Neither depends on S-14.01..S-14.09. Acyclic.

## Anchored ADRs

| ADR ID | Title |
|--------|-------|
| ADR-017 | Per-story adversary three-perimeter model + phasing |

## Process-Gap Source Mapping

| Story | F5 Finding | Adversary Observation |
|-------|-----------|----------------------|
| S-14.01 | F-LOW-3 (OBS-3) | Bootstrap cohort lacks convergence-state files; F7 must backfill |
| S-14.02 | F-LOW-2 (OBS-2) | F5 dispatch assumes adversary writes; contradicts read-only profile |
| S-14.03 | F-LOW-4 + F-LOW-5 (OBS-5/6) | Pre-F5 placeholder lint check missing; input-hash drift not enforced |
| S-14.04 | F-MED-9 | Policy-rubric injection into adversary dispatch is manual |
| S-14.05 | PG-6 (orchestrator-observed) | pr-manager exits mid-flow after Step 4 or Step 5 |
| S-14.10 | D-1257 [process-gap] lessons (S-25.08 local-adversary cycle) | Story status lags fix commits (3x); adversary reviews not persisted before the next pass |
| S-14.11 | D-1257 / D-1259 / D-1260 [process-gap] lessons (S-25.08 local-adversary cycle) | CHANGELOG EC/AC tokens not owner-qualified; version pins survive a crates-only sweep; SHA labels not verified (wrong "pre-split"); convergence prose restates persist-task status and file counts (3x) |

## Acceptance Criteria

> Acceptance criteria are defined per-story in each story file. See S-14.01 through S-14.11.

## Stories

See Stories Planned table above (S-14.01..S-14.11).

## Dependencies (External)

| Dependency | Type | Notes |
|------------|------|-------|
| E-12 (Engine Governance) | Predecessor | S-14.01 convergence-state backfill depends on E-12 stories merged |
| E-13 (Artifact Integrity) | Predecessor | E-14 completes the governance triangle started by E-12/E-13 |

## Open Questions

| OQ ID | Scope | Description |
|-------|-------|-------------|
| OQ-E14-01 | SS-04 | S-14.03: implement as a WASM hook (PreToolUse on adversary dispatch), a bats lint script, or a skill preamble check? Decision needed before S-14.03 implementation. |
| OQ-E14-02 | SS-05 | S-14.02: does the phase-5 lobster step need a new `capture-adversary-output` step, or should the phase-5 skill itself dispatch state-manager as a sub-step? The orchestrator skill model may constrain this. |
| OQ-E14-03 | SS-05 | S-14.05: pr-manager discipline — invert control (orchestrator drives steps) vs add STEP_COMPLETE sentinel vs add hard validation gate at Step 4/5 exit point. Decision required before S-14.05 implementation. |

## CHANGELOG

| Version | Date | Change |
|---------|------|--------|
| v1.5 | 2026-10-13 | S-14.10 SPLIT by HUMAN DECISION (2026-10-13; story-writer): S-14.10 keeps reconciliation, persistence and the dispatch hook (5 pts, SS-05); NEW S-14.11 takes the CHANGELOG lint, version-pin sweep, SHA-label truth check and convergence-prose parity (7 pts, SS-05/SS-06, `depends_on` S-14.10). story_count 10 -> 11; Stories Planned rows, Process-Gap Source Mapping row, SS-05/SS-06 anchor bullets, dependency note, range S-14.01..S-14.11, inputs += S-14.11. |
| v1.4 | 2026-10-13 | S-14.10 story file authored and registered (story-writer; D-1257 follow-up, opened index-only in this epic by the state-manager): story_count 9 -> 10; Stories Planned row (8 pts, P1, SS-05/SS-06), Process-Gap Source Mapping row, Stories range S-14.01..S-14.10, inputs += S-14.10 and the D-1257 lessons file; modified[] gains the missing v1.3 entry. SS-05 anchor bullet extended below (S-14.10 workflow documents); SS-06 anchor bullet extended (create-story skill). |
| v1.3 | 2026-08-30 | Frontmatter normalization: add level: L3 (story-writer spec-hygiene sweep). |
| v1.2 | 2026-05-11 | F-P8-002 fix burst: forward-ref note corrected — S-14.01 carries `cycle: v1.0-feature-engine-discipline-pass-1` (not pass-2) per D-355-AMEND and D-359. The note now correctly identifies S-14.02–S-14.09 as the pass-2 forward references. |
| v1.1 | 2026-05-11 | F-P7-002 fix burst: story_count 5→9 (S-14.06/07/08 filed in F-P6-002 fix burst; S-14.09 filed in F-P6-004 fix burst). Stories Planned table updated with all 9 stories. Forward-reference note added for `cycle: v1.0-feature-engine-discipline-pass-2` per F-P7-004. |
| v1.0 | 2026-05-07 | Initial authoring (product-owner; B6 process-gap burst from F5 pass-1 adversarial review of v1.0-feature-engine-discipline-pass-1). Five stories authored: S-14.01 (P0 BLOCKING), S-14.02-05 (P1-P2 pass-2). Delivery order: S-14.01 before F7 close-out; S-14.02-05 in next cycle. |
