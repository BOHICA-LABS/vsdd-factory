---
document_type: pipeline-state
level: ops
version: "10.81"
status: in_progress
producer: state-manager
timestamp: 2026-10-07T04:39:48Z
phase: "S-25.08 (E-25 add-on; shared admission core + B2 conformance + VP-147 re-baseline, BC-1.18.011/BC-1.18.013) TDD IMPLEMENTATION IN PROGRESS (Step 4 of per-story delivery; red gate done, implementer T-3/T-4 in progress) in .worktrees/S-25.08 on feature/S-25.08 (local-only, not pushed, no PR). S-25.06 (blocked on S-25.08) in-progress on feature/S-25.06 (local-only). develop 117dafdf. S-25.02 cluster-5 DELIVERED/MERGED (PR #842, ddd99212). Cluster-6 (migrations) F1 delta analysis remains queued per D-1170's sequencing."
last_amended: "2026-10-06 (v10.80→v10.81) — state-manager: S2508-REDGATE-AMBIGUITY-REGISTRATION (single-commit TD-VSDD-053; D-1246). Registered the four normative S-25.08 Red-Gate spec decisions: BC-1.18.013 v1.6, BC-1.18.011 v1.13, error-taxonomy v1.35, ADR-052 v1.18 alignment, S-25.08 v1.1 in-progress, S-25.06 v1.7, S-25.02 v4.9, BC-INDEX v5.103, STORY-INDEX v4.481."

inputs: []
input-hash: "[live-state]"
traces_to: prd.md
project: vsdd-factory
mode: brownfield
pipeline: in_progress
current_step: "S2508-REDGATE-AMBIGUITY-REGISTRATION — S-25.08 Red-Gate ambiguity resolutions + delivery progress registration (state-manager, single-commit TD-VSDD-053; D-1246, within D-1244/D-1245). Four normative spec decisions (orchestrator-routed, product-owner-authored): (1) E-MAINTENANCE-001 scope keyed on written path family + exact format string; (2) Branch B in-place abort_reason null_generation marker; (3) decide_terminal_record_reconciliation five-row table, record-absent NoOp; (4) uniform completion-record-mismatch suffix. Registered: BC-1.18.013 v1.6, BC-1.18.011 v1.13, BC-INDEX v5.103 (2,007 BCs), error-taxonomy v1.35, ADR-052 v1.18 same-version alignment, S-25.08 v1.1 (ready to in-progress; red gate done on local feature/S-25.08, implementer T-3/T-4 in progress), S-25.06 v1.7, S-25.02 v4.9, STORY-INDEX v4.481, E-25 epic S-25.08 status cell. [D-1244-REVERIFY] still OPEN before merge. pipeline: stays in_progress. trajectory-tail →1→1→2→1 LENGTH=4 (unchanged). NEXT = S-25.08 implementer T-3/T-4 to green, Step 4.5 LOCAL adversary, then S-25.06. v10.80→v10.81."
current_cycle: v1.0-brownfield-backfill
dtu_required: false
dtu_assessment: 2026-04-25
dtu_clones_built: "n/a"
dtu_services: []
---

<!--
  STATE.md SIZE BUDGET (per D-421(c) + D-422(c) reconciliation):
  Soft target: <=415 lines (compact-state target <200); hard cap: 500 lines (validate-state-md-size). Hard cap margin from soft-target = 500 - 415 = 85; margin from actual = 500 - 200 = 300 (D-446(c) dual-margin form). 200 lines (wc-l; v10.81, literal `wc -l .factory/STATE.md` run last before commit per the banner-wc-l lesson; this banner edit is single-line and does not change the count).
  Historical content belongs in cycle files, NOT here.
  Full pre-compaction STATE.md (v10.78, 489 lines): cycles/v1.0-brownfield-backfill/state-md-archive-through-v10.78.md
  Older history: git -C .factory log -p -- STATE.md + burst-log.md + decision-log.md.
-->

# Pipeline State: vsdd-factory

> **Self-referential note:** vsdd-factory IS the project being onboarded. Engine and product are the same repository.

## Project Metadata

| Field | Value |
|-------|-------|
| **Product** | vsdd-factory |
| **Repository** | /Users/jmagady/Dev/vsdd-factory |
| **Mode** | brownfield-onboarding |
| **Language** | Rust + Bash + Markdown |
| **Started** | 2026-04-25 |
| **Last Updated** | 2026-10-06 — **S2508-REDGATE-AMBIGUITY-REGISTRATION (v10.80→v10.81)** (state-manager; single-commit TD-VSDD-053; D-1246). Four normative S-25.08 Red-Gate spec decisions registered (BC-1.18.013 v1.6, BC-1.18.011 v1.13, error-taxonomy v1.35, ADR-052 v1.18 alignment; BC-INDEX v5.103, STORY-INDEX v4.481); S-25.08 v1.1 ready→in-progress (red gate done on local `feature/S-25.08`; implementer T-3/T-4 in progress); S-25.06 v1.7; S-25.02 v4.9. trajectory-tail →1→1→2→1 LENGTH=4 (unchanged). Prior burst: **S2508-SPLIT-ADR052-V118-REGISTRATION (v10.79→v10.80)** (state-manager; single-commit TD-VSDD-053; D-1244/D-1245). ADR-052 v1.18 formal-finding exception registered (human-approved; re-verification of B2 obl1 Kani + VP-146 + VP-143 required before merge); S-25.06 split into NEW S-25.08 (8 pts, ready) + S-25.06 v1.6 (13 pts); VP-147 NEW (VP-INDEX v3.26, 147 VPs); STORY-INDEX v4.480 (19 uncatalogued stories registered); BC-INDEX v5.102; lesson L-BB-D1245 + follow-up S-12.16. trajectory-tail →1→1→2→1 LENGTH=4 (unchanged). NEXT: S-25.08 delivery (worktree from develop; test-writer failing tests T-1/T-2). Prior burst: STATE-MD-COMPACTION-2026-10-06 (v10.78→v10.79; STATE.md 489→190 lines, history archived verbatim to `cycles/v1.0-brownfield-backfill/state-md-archive-through-v10.78.md`). Earlier: D-1243 S2506-VP146-KANI-SPEC-REGISTRATION (v10.77→v10.78): VP-146 (kani-proof) registered, VP-INDEX v3.25, BC-1.18.013 v1.2 / BC-INDEX v5.101, S-25.06 v1.4 in-progress / STORY-INDEX v4.479. |
| **Current Phase** | **[UPDATED 2026-10-06, D-1246: S-25.08 TDD IN PROGRESS on local `feature/S-25.08` (`.worktrees/S-25.08`; red gate done T-1/T-2; implementer T-3/T-4 in progress) — then S-25.06; T-10 live activation AFTER PR merge.]** (D-1244/D-1245 S-25.06 SPLIT context:) S-25.06 TDD IN PROGRESS on `feature/S-25.06` (local-only; Step 4 of per-story delivery); PR #843 merged (`117dafdf`); S-25.02 cluster-5 DELIVERED/MERGED (PR #842, `ddd99212`, D-1240). Clusters 6 (migrations) + 7 (Cohort-B flip CAPSTONE) queued per D-1170. ADR-052 ACCEPTED (D-1233). See Session Resume Checkpoint. |
| **Current Cycle** | v1.0-brownfield-backfill |

## Phase Progress

| Phase | Status | Artifact |
|-------|--------|----------|
> Rows for "Phases 0-B..D-647" through "SESSION-WRAP-PAUSE-2026-09-06" archived to `cycles/v1.0-brownfield-backfill/phase-progress-archive.md` (earlier compactions). Rows for "S2502-F4-GATE-RESOLVED-INCREMENTAL-BY-BC-CLUSTER (D-1170)" through "S2506-SPEC-AUTHORING-READY-BURST (D-1241)" archived verbatim to `cycles/v1.0-brownfield-backfill/state-md-archive-through-v10.78.md` (section `## Phase Progress`) during the STATE-MD-COMPACTION-2026-10-06 burst.
| **S2506-VP146-KANI-SPEC-REGISTRATION 2026-10-06 (D-1243)** | **S-25.06 VP-146 REGISTERED — KANI RUN NEXT** | VP-146 (kani-proof) registered; VP-INDEX v3.25 (146 VPs); BC-1.18.013 v1.2 / BC-INDEX v5.101 (2,007 BCs); S-25.06 v1.4 in-progress / STORY-INDEX v4.479. NEXT: formal-verifier T-8a. v10.77→v10.78. |
| **S2506-TDD-IN-PROGRESS-PR843-MERGED-REGISTRATION 2026-10-06 (D-1242)** | **S-25.06 TDD IN PROGRESS** | PR #843 merged (`117dafdf`); S-25.06 TDD on `feature/S-25.06` (local-only); story v1.3. v10.76→v10.77. |
| **S2508-SPLIT-ADR052-V118-REGISTRATION 2026-10-06 (D-1244/D-1245)** | **S-25.06 SPLIT + ADR-052 v1.18 REGISTERED — S-25.08 DELIVERY NEXT** | Human decisions: T-10 after merge; ADR-052 v1.18 formal-finding exception approved (re-verify B2 obl1 Kani + VP-146 + VP-143 before merge); B2-1..B2-4 kept in E-25; S-25.06 → S-25.08 (8) + S-25.06 (13). VP-INDEX v3.26 (147), BC-INDEX v5.102, STORY-INDEX v4.480. v10.79→v10.80. |
| **S2508-REDGATE-AMBIGUITY-REGISTRATION 2026-10-06 (D-1246)** | **S-25.08 TDD IN PROGRESS — RED GATE DONE** | Four normative spec decisions (path-family-keyed E-MAINTENANCE-001 + format string; Branch B `null_generation` marker; 5-row `decide_terminal_record_reconciliation` table, record-absent NoOp; uniform mismatch suffix): BC-1.18.013 v1.6, BC-1.18.011 v1.13, BC-INDEX v5.103, error-taxonomy v1.35, ADR-052 v1.18 alignment; S-25.08 v1.1 in-progress, S-25.06 v1.7, S-25.02 v4.9, STORY-INDEX v4.481. v10.80→v10.81. |
| **STATE-MD-COMPACTION-2026-10-06** | **COMPACTED** | STATE.md 489→under 200 lines; pre-compaction file archived verbatim. v10.78→v10.79. |

## Current Phase Steps

> Rows through v10.77 archived verbatim to `cycles/v1.0-brownfield-backfill/state-md-archive-through-v10.78.md` (section `## Current Phase Steps`) and `burst-log.md`. This table keeps the last 5 steps only per state-manager content-routing discipline.

| Step | Agent | Status | Output |
|------|-------|--------|--------|
| S2508-REDGATE-AMBIGUITY-REGISTRATION (v10.80→v10.81) | state-manager | COMPLETE | S-25.08 Red-Gate ambiguity resolutions registered (D-1246): BC-1.18.013 v1.6, BC-1.18.011 v1.13, BC-INDEX v5.103, error-taxonomy v1.35, S-25.08 v1.1 in-progress, S-25.06 v1.7, S-25.02 v4.9, STORY-INDEX v4.481. |
| S2508-SPLIT-ADR052-V118-REGISTRATION (v10.79→v10.80) | state-manager | COMPLETE | S-25.06/S-25.08 spec + split registered (D-1244/D-1245): ADR-052 v1.18, S-25.08 NEW, S-25.06 v1.6, VP-147, STORY-INDEX v4.480, BC-INDEX v5.102, VP-INDEX v3.26; lesson L-BB-D1245 + S-12.16. |
| STATE-MD-COMPACTION-2026-10-06 (v10.78→v10.79) | state-manager | COMPLETE | STATE.md compaction; pre-compaction file archived verbatim; no content lost. |
| S2506-VP146-KANI-SPEC-REGISTRATION (v10.77→v10.78) | state-manager | COMPLETE | VP-146 spec burst registered (D-1243): VP-INDEX v3.25, BC-INDEX v5.101, STORY-INDEX v4.479. |
| S2506-TDD-IN-PROGRESS-PR843-MERGED-REGISTRATION (v10.76→v10.77) | state-manager | COMPLETE | PR #843 merged to develop `117dafdf`; S-25.06 TDD in progress (D-1242). |

## Identifier Conventions

| Type | Format | Authoritative Source | Count |
|------|--------|----------------------|-------|
| Subsystem | SS-NN | `specs/architecture/ARCH-INDEX.md` | 10 |
| Behavioral Contract | BC-S.SS.NNN | `specs/behavioral-contracts/ss-NN/` | **2,007** (BC-INDEX v5.103; BC-1.18.013 registered D-1241; v1.6 + BC-1.18.011 v1.13 at D-1246) |
| Verification Property | VP-NNN | `specs/verification-properties/VP-INDEX.md` | **147** per VP-INDEX frontmatter total_vps (VP-INDEX v3.26 at D-1245; NEW VP-147 kani-proof anchoring B2 obl1; kani-proof 8) |
| Story | S-N.MM | `stories/S-N.MM-<short>.md` | 246 registered in STORY-INDEX v4.481 = 202 story files + 44 index-only stubs (19 retired + 24 draft + 1 merged); the 19 previously uncatalogued files (S-23.01..S-23.14 STALE strip-model, S-26.01..S-26.05) registered D-1244/D-1245; Total (active) 225 |
| Epic | E-N | `stories/epics/E-N-<short>.md` | 25 (E-25 Validation Integrity, epic file v1.4, story_count 8; E-23 STALE re-scope OWED; E-24 HOLDING; E-22 dissolved-retained) |
| ADR | ADR-NNN | `specs/architecture/decisions/ADR-NNN.md` | 52 (ARCH-INDEX v4.48; ADR-052 v1.18 ACCEPTED, formal-finding exception approved 2026-10-06, D-1244) |
| **Merged Count** | merged_count | `stories/sprint-state.yaml` | **123** (S-25.02 cluster-5 PR #842 `ddd99212`, D-1240; PR #843 is not a story delivery) |

## Story Status

- 177 registered-file-resident stories; E-18 EPIC COMPLETE (D-744); E-22 DISSOLVED (D-961); E-23 STALE (S-23.01..S-23.14 re-scope OWED, ADR-045 v1.3); E-24 W1 COMPLETE; E-25: S-25.01/S-25.04 MERGED, S-25.02 clusters 1-5 MERGED (cluster-6/7 queued), S-25.06 IN PROGRESS (v1.7, 13 pts, depends_on S-25.08), S-25.08 IN PROGRESS (v1.1, 8 pts, W3a, blocks S-25.06; red gate done, implementer T-3/T-4; D-1244/D-1246), S-25.07 draft. Full per-story ledger: archive file section `## Story Status`.
- **Merged (123 per merged_count).** **In-flight (2):** S-25.08 (feature/S-25.08, local-only; red gate done; implementer T-3/T-4 in progress), S-25.06 (feature/S-25.06, local-only; blocked on S-25.08). **Next to dispatch:** S-25.08 Step 4.5 LOCAL adversary after implementer green; re-verify B2 obl1 Kani (VP-147) before merge.

## Active Branches

| Branch / Tag | SHA | Notes |
|--------------|-----|-------|
| main | **51023185** | v1.0.0-rc.25 bundle+retag commit 2026-09-04; merge commit `101ebb64` (release PR #808) is main's immediate parent. |
| develop | **`117dafdf`** | PR #843 (`chore(config): register shard-config artifact path pattern`) merged 2026-10-06 (prior base `ddd99212`, PR #842 cluster-5). Not a story delivery; merged_count 123. |
| feature/S-25.06 | **IN PROGRESS (local-only; not pushed; no PR)** | `.worktrees/S-25.06`; head `08211795` + uncommitted Kani proof work. |
| feature/S-25.08 | **IN PROGRESS (local-only; not pushed; no PR)** | `.worktrees/S-25.08`; base `117dafdf`; red gate DONE (commits `1b28017e` + `c5e24633`, T-1/T-2); implementer T-3/T-4 in progress (D-1246). S-25.08 blocks S-25.06 (S-25.06 branch rebases onto it after merge). |
| factory-artifacts | **(see `git -C .factory log -1`)** | HEAD is owned by git; no artifact cites its own SHA (TD-VSDD-053). |
| v1.0.0-rc.25 (tag) | **101ebb64** | SHIPPED 2026-09-04. |

> Older branch rows (merged+deleted feature/fix branches, rc.23/rc.24 tags) archived verbatim to `cycles/v1.0-brownfield-backfill/state-md-archive-through-v10.78.md` (section `## Active Branches`).

## Concurrent Cycles

| Cycle | Type | Status | Notes |
|-------|------|--------|-------|
| v1.0-brownfield-backfill | brownfield | ACTIVE — S-25.06 TDD IN PROGRESS | S-25.02 clusters 1-5 DELIVERED/MERGED; S-25.06 in flight; clusters 6-7 queued. Full historical Notes cell (~56KB) archived verbatim to the archive file (section `## Concurrent Cycles`) — closes [D-1240-DRIFT-001] unbounded-growth. |
| v1.0-feature-engine-discipline-pass-1 | feature | PAUSED | F5 pass-75 D-510. META-LEVEL-30 CANDIDATE-CONFIRMED. trajectory-tail →7→9→7→9 LENGTH=4. |
| F-block-ai-attribution-message-file-arm | feature | F3 COMPLETE — F4 READY | E-16 under SS-07/SS-04; milestone v1.0.0-rc.17 |
| v1.0-feature-plugin-async-semantics-pass-1 | feature | CLOSED | All PRs merged; rc.14 shipped |

## Decisions Log

> D-001..D-606 (exhaustive): decision-log.md + decisions-log-archive.md. D-379..D-454 (F5): cycles/v1.0-feature-engine-discipline-pass-1/decision-log.md. D-1192..D-1246 (exhaustive appendix sections): cycles/v1.0-brownfield-backfill/decision-log.md SoT (each authored there the same burst it is codified). The pre-compaction summary-row table (D-1211..D-1243 (exhaustive) rows, backfill-owed range notes) is preserved verbatim in `state-md-archive-through-v10.78.md` (section `## Decisions Log`). D-999 SKIPPED.

| ID | Decision | Summary | Phase | Date |
|----|----------|---------|-------|------|
| D-1246 | D-1246-S2508-REDGATE-AMBIGUITY-RESOLUTIONS-FOUR-NORMATIVE-DECISIONS | S-25.08 Red-Gate ambiguity resolutions (orchestrator-routed, product-owner-authored): (1) E-MAINTENANCE-001 `<scope>` keyed on the written path family (`BC-INDEX` / `.factory/cycles/`) with exact format string, migration_id to the tracing diagnostic only; (2) Branch B rewrites txn in place to ABORTED + `abort_reason: "null_generation"` (file retained, marker informational); (3) `decide_terminal_record_reconciliation` five-row table, own-live-txn + lock + record-absent => NoOp; (4) uniform ` (completion-record mismatch — operator investigation required)` suffix on Branch C failures (plain message for foreign-migration refusal / EWOULDBLOCK). BC-1.18.013 v1.6, BC-1.18.011 v1.13, error-taxonomy v1.35, ADR-052 v1.18 alignment; S-25.08 v1.1 in-progress (worktree `.worktrees/S-25.08`, local-only, red gate done, implementer T-3/T-4); S-25.06 v1.7; S-25.02 v4.9. Full text: decision-log.md. | S-25.08 F4 TDD | 2026-10-06 |
| D-1245 | D-1245-ORCHESTRATOR-DECISIONS-VP147-DECYCLE-LESSON-S12-16 | Orchestrator decisions 2026-10-06 (state-manager registration): VP-146 v1.2 and NEW VP-147 (kani-proof; B2 obl1; anchor S-25.08) allocated; input-hash graph de-cycled (ADR→BC→taxonomy→story); canonical name `decide_terminal_record_reconciliation`. `[process-gap]` L-BB-D1245 — architect bypassed Edit/Write with Python heredocs twice (VP-146.md, VP-133.md; remediated via Write); follow-up S-12.16 (E-12, draft stub). Hook false positives recorded (validate-factory-path-staging on `cd .factory && git add` / absolute -C path; destructive-command-guard on read-only `git show \| grep 'rm -rf'`). Full text: decision-log.md. | S-25.06/S-25.08 spec + split | 2026-10-06 |
| D-1244 | D-1244-S2506-HUMAN-DECISIONS-ADR052-V118-S2508-SPLIT | Human decisions 2026-10-06: (a) T-10 live activation runs AFTER PR merge from merged code; (b) ADR-052 v1.18 formal-finding exception APPROVED (frozen §4e/§5a/§5c changed; re-verification of B2 obl1 Kani + VP-146 + VP-143 REQUIRED before merge); (c) B2 conformance fixes B2-1..B2-4 kept in E-25 scope; (d) S-25.06 split into NEW S-25.08 (8 pts) + S-25.06 v1.6 (13 pts). Full text: decision-log.md. | S-25.06/S-25.08 spec + split | 2026-10-06 |
| D-1243 | D-1243-S2506-VP146-KANI-SPEC-REGISTRATION | S-25.06 VP-146 spec-burst registration (state-manager). NEW VP-146 (architect; kani-proof; BC-1.18.013 mechanism-A crash-recovery decision core, a1..a6 / 7 proof fns) — VP-INDEX v3.25 (total_vps 146), verification-architecture v1.38, verification-coverage-matrix v1.36; BC-1.18.013 v1.2 / BC-INDEX v5.101; S-25.06 v1.4 in-progress / STORY-INDEX v4.479. Full text: decision-log.md. | S-25.06 F4 TDD | 2026-10-06 |
| D-1242 | D-1242-S2506-TDD-IN-PROGRESS-PR843-MERGED-REGISTRATION | Pipeline-position registration. PR #843 merged to develop `117dafdf`; S-25.06 TDD in progress on `feature/S-25.06` (local-only); story v1.2→v1.3. Full text: decision-log.md. | S-25.06 F4 TDD | 2026-10-06 |

## Skip Log

| Step | Skipped? | Justification |
|------|----------|----------------|
| UX Spec | yes | CLI-only product with no UI surfaces |
| Gene Transfection Assessment | yes | Not applicable — engine and product are same repo |
| D-413..D-1088 (exhaustive) | ARCHIVED | Full detail: decision-log.md SoT.; ARCHIVED; 2026-06-14..2026-08-26 |

## Blocking Issues

| Blocker | Status | Risk Statement |
|---------|--------|----------------|
| **[D-1244-REVERIFY] ADR-052 v1.18 re-verification: B2 obligation-1 Kani suite (VP-147 re-baseline, S-25.08 AC-010) + VP-146 (S-25.06 T-8a) + VP-143 (fault-injection/integration)** | **OPEN 2026-10-06 — REQUIRED BEFORE MERGE of S-25.08 / S-25.06 (human decision, D-1244)** | Frozen §4e/§5a/§5c changed by the formal-finding exception; existing proofs were 7/7 PROVED against the PRE-v1.18 model. T-10 live activation runs AFTER PR merge from merged code (D-1244). |
| **[D-1232-OBL-2] APFS hybrid durability: mandated fsync sequence + disk-image VM-kill test** | **PENDING 2026-09-20 — blocks cluster-5 F4 activation on macOS** | Activation-boundary scope; anchored cluster-5/S-25.02. Does NOT gate merges (migration shipped DORMANT). |
| **[D-1232-OBL-3] Apply CLAUDE.md ADR-052 EXCEPTION amendment** | **OPEN 2026-09-20 — apply at activation** | Pre-approved amendment; anchored cluster-5/S-25.02. |
| **[D-1232-OBL-4] Deploy 4 dispatcher-guard amendments** | **OPEN 2026-09-20 — deploy at activation boundary (rc.26 release)** | Anchored cluster-5/S-25.02. |
| **[D-1163] Branch protection on `develop` not enabled** | **OPEN 2026-09-04 — HUMAN/ADMIN ACTION REQUIRED** | Token lacks admin; see archive file for full text. |
| **[C-1/C-2/C-4/C-5] exec_subprocess CWE-706/362/284 HIGH/MED SECURITY findings (D-972)** | **OPEN 2026-08-11** | Preserved verbatim in archive file (section `## Drift Items / Tech Debt`). |

> Resolved/discharged blockers ([D-1232-OBL-1] DISCHARGED D-1239; [PR-842-CI] RESOLVED D-1240) and all earlier rows: verbatim in the archive file (section `## Blocking Issues`) and `cycles/v1.0-brownfield-backfill/blocking-issues-resolved.md`.

## Drift Items / Tech Debt

| Item | Status | Notes |
|------|--------|-------|
| **[S-25.06-DRIFT-001] Cycle-file compaction** | **PARTIALLY ADDRESSED 2026-10-06** | STATE.md compacted this burst (v10.79); cycle-file (decision-log/burst-log/lessons) size-budget compaction remains deferred, anchored S-25.06 (E-25) per its original row (verbatim in archive). |
| **[D-1240-DRIFT-001] Concurrent Cycles Notes-cell unbounded growth** | **RESOLVED 2026-10-06 (compaction)** | Cell reduced to a short note; history in archive file. |
| **[D-1245-PG-001] Architect Edit/Write bypass (Python heredocs; VP-146.md, VP-133.md)** | **OPEN 2026-10-06 — codification follow-up S-12.16 (E-12; STORY-INDEX v4.480 index-only draft stub)** | `[process-gap]` L-BB-D1245 in lessons.md; related [D-1238-HYG-002]; remediated via Write. |
| **[D-1245-HOOK-FP-001/002] Hook false positives** | **OPEN 2026-10-06 — anchored E-26 hook-hardening backlog (S-26.02 covers 001)** | 001: `validate-factory-path-staging` blocks `cd .factory && git add` and absolute `git -C /abs/.factory add` (use `git -C .factory add` from repo root). 002: `destructive-command-guard` blocks read-only `git show \| grep 'rm -rf'`. |
| **Other open Drift Items (~130 rows)** | **OPEN — anchored per row** | IDs in open state at v10.78 (full rows, owners, and anchors verbatim in archive file section `## Drift Items / Tech Debt`): D-1239-DRIFT-001/002, D-1238-HYG-001/002, PR842-FOLLOWUP-001/002, D-1237-DRIFT-001, CV-DIR-F1a/F1b, D-1212-DRIFT-002, D-1211, D-1207, D-1206, D-1204/1203/1202/1200 (S-12.12), D-1201/D-1192 (S-12.09), D-1191 (T-12), D-1188, D-1186 (x3), D-1184 (x2), D-1181, D-1177 (x2), D-1175, D-1173 (x2), D-1172, D-1164 (x3), D-1163 (x2), D-1156 (x3), D-1157, D-1152 (x4), D-1151 (x3), D-1150, D-1149, D-1144, D-1140, D-1138 (x3), D-1130 (x3), D-1129, D-1081, D-1082, D-1070..D-1077 (exhaustive), D-1062/1064/1057, D-953/954, D-945, TD-VSDD-061..063/101, D-1222-DRIFT-001, D-1221-PG-001, VP-for-BC-1.18.005 owed to Phase F6 rows (D-1171, PASS3..PASS6, D-1174). |

## Historical Content

- `cycles/v1.0-brownfield-backfill/state-md-archive-through-v10.78.md` (full pre-compaction STATE.md v10.78, verbatim — Phase Progress rows, Last Updated priors, Current Phase, Current Phase Steps, Story Status, Active Branches, Concurrent Cycles, Decisions Log rows D-1211..D-1243 (exhaustive), Blocking/Drift rows, prior Session Resume Checkpoint)
- `cycles/v1.0-brownfield-backfill/burst-log.md` | `session-checkpoints.md` | `lessons.md` | `decision-log.md` | `blocking-issues-resolved.md` | `phase-progress-archive.md`
- `cycles/v1.0-brownfield-backfill/decision-log-archive-through-D1056.md` | `decision-log-archive-through-D1191.md` | `burst-log-archive-through-D1056.md` | `lessons-archive-pre-D1057.md`
- `cycles/v1.0-feature-plugin-async-semantics-pass-1/burst-log.md` | `session-checkpoints.md` | `lessons.md`
- `cycles/v1.0-feature-engine-discipline-pass-1/burst-log.md`

## Session Resume Checkpoint (2026-10-06 — S2508-REDGATE-AMBIGUITY-REGISTRATION v10.80→v10.81; develop 117dafdf; main 51023185; merged_count 123; v1.0.0-rc.25 SHIPPED; PIPELINE IN_PROGRESS — S-25.08 TDD IN PROGRESS (red gate done), S-25.06 blocked on it)

> Prior checkpoints archived: v10.80 (condensed; verbatim via `git -C .factory show ee19219d:STATE.md`) and v10.79 (condensed) in `cycles/v1.0-brownfield-backfill/session-checkpoints.md` (v10.79 verbatim via `git -C .factory show 4043e128:STATE.md`); v10.78 (D-1243) verbatim in `cycles/v1.0-brownfield-backfill/state-md-archive-through-v10.78.md`.

**THIS BURST (D-1246):** registered the four normative S-25.08 Red-Gate spec decisions (orchestrator-routed, product-owner-authored): (1) E-MAINTENANCE-001 `<scope>` keyed on written path family + exact format string; (2) Branch B in-place `abort_reason: "null_generation"` marker; (3) `decide_terminal_record_reconciliation` five-row table, record-absent NoOp; (4) uniform completion-record-mismatch suffix. Versions: BC-1.18.013 v1.6, BC-1.18.011 v1.13, BC-INDEX v5.103, error-taxonomy v1.35, ADR-052 v1.18 same-version alignment, S-25.08 v1.1 (in-progress), S-25.06 v1.7, S-25.02 v4.9, STORY-INDEX v4.481, E-25 epic S-25.08 status cell. S-25.08: worktree `.worktrees/S-25.08`, branch `feature/S-25.08` (local-only; base `117dafdf`), red gate done (`1b28017e` + `c5e24633`), implementer T-3/T-4 in progress. [D-1244-REVERIFY] OPEN before merge.

**PRIOR BURST (D-1244/D-1245):** registered the S-25.06/S-25.08 spec + split work. Human decisions 2026-10-06: T-10 live activation runs AFTER PR merge from merged code; ADR-052 v1.18 formal-finding exception APPROVED (frozen §4e/§5a/§5c changed — re-verification of B2 obl1 Kani + VP-146 + VP-143 REQUIRED before merge); B2 conformance fixes B2-1..B2-4 kept in E-25 scope; S-25.06 split into NEW S-25.08 (8 pts) + S-25.06 v1.6 (13 pts). Registered: ADR-052 v1.18, ARCH-INDEX v4.48, BC-1.18.013 v1.5, BC-1.18.011 v1.12, BC-INDEX v5.102, error-taxonomy v1.34, VP-133 v1.2, VP-134 v1.1, VP-143 v1.2, VP-146 v1.2, NEW VP-147 v1.1, VP-INDEX v3.26 (147 VPs), verification-architecture v1.39, verification-coverage-matrix v1.37, S-25.08 v1.0, S-25.06 v1.6, S-25.02 v4.8, E-25 v1.4, STORY-INDEX v4.480 (+19 uncatalogued stories, +S-12.16 stub). Lesson L-BB-D1245 recorded. NEXT: S-25.08 delivery — create worktree from develop; test-writer failing tests T-1/T-2.

### §1. Position (a)
S-25.06 split (D-1244): S-25.08 (shared admission core + B2 conformance + VP-147 re-baseline; v1.1 IN PROGRESS on local `feature/S-25.08` — red gate done, implementer T-3/T-4 in progress; blocks S-25.06) is being delivered; S-25.06 (v1.7; 13 pts; in-progress on local `feature/S-25.06`, `.worktrees/S-25.06`) depends_on S-25.08. develop @ `117dafdf`. S-25.02 clusters 1-5 DELIVERED/MERGED (cluster-5 PR #842 `ddd99212`, D-1240). Order: S-25.08 → S-25.06 → S-25.02 cluster 7.

### §2. Convergence (b)
Cluster-5 F4-code LOCAL cascade converged pass-4 CLEAN (accept-at-floor, D-386 Option C, D-1240); crash-recovery proven (Kani 7/7 + fault-injection 30/30, [D-1232-OBL-1] DISCHARGED D-1239). Cycle-level streak 3/3 CONVERGED, unchanged. trajectory-tail →1→1→2→1 LENGTH=4 (ADR-052 concurrency-core axis, CLOSED).

### §3. In-flight / Abandoned (c)
- In flight: S-25.08 TDD on `feature/S-25.08` (`.worktrees/S-25.08`, local-only, base `117dafdf`; red gate commits `1b28017e` + `c5e24633`; implementer T-3/T-4 in progress). S-25.06 TDD on `feature/S-25.06` (local-only; commits `31890c0d`, `a00fabf3`, `36424701`, `f9db5857`, `08211795`; uncommitted Kani work).
- factory-artifacts: the S-25.06/S-25.08 spec + split work is now REGISTERED (this burst); working tree clean after commit (sidecar-learning.md / regression-state.json auto-telemetry committed with it).
- Abandoned: cluster-5 "Item 3" fail-closed tightening (reverted per BC-1.18.002 v1.8 INV2/EC-030; S-25.07 filed).

### §4. Pending human decisions / open blockers (d)
- Activation-boundary obligations OPEN/PENDING: [D-1232-OBL-2] APFS VM-kill test; [D-1232-OBL-3] CLAUDE.md ADR-052 amendment; [D-1232-OBL-4] deploy 4 dispatcher guards (rc.26).
- T-10 (live migration) timing RESOLVED (D-1244): runs AFTER PR merge from merged code. OPEN before merge: [D-1244-REVERIFY] re-verification of B2 obl1 Kani + VP-146 + VP-143 against ADR-052 v1.18.
- S-25.07 joint review (product-owner + architect + security-reviewer) on BC-1.18.002 INV2/EC-030 — to be scheduled, not blocking.
- Remaining work, priority order: (1) S-25.06 TDD to merge; (2) cluster-6 (migrations) F1 delta analysis; (3) E-26 registration DEFERRED behind E-25; (4) hook-hardening batch #837-841 -> rc.26. 4 PRs open: #769, #768, #729, #632. input-hash currency refresh (907 files) OWED (OWED #2).

### §5. WIP branches (e)
`feature/S-25.08` and `feature/S-25.06` (above; both local-only). Inert: `fix/d999-sentinel-code-migration` @ `bf642fd9`; `feature/S-21.04-story-worktree-write-path-discipline` @ `323f440f`.

### §6. Resume command (f)
`/vsdd-factory:rehydrate-wave` then `/vsdd-factory:next-step`. First action (D-1246): continue S-25.08 delivery in `.worktrees/S-25.08` (`feature/S-25.08`, local-only; red gate done) — implementer T-3/T-4 to green, then Step 4.5 LOCAL adversary 3-CLEAN, demo, push, pr-manager; re-verify B2 obl1 Kani (VP-147) before merge. Then S-25.06 (rebase onto merged S-25.08; formal-verifier T-8a Kani VP-146 re-baseline, VP-143 prongs); T-10 live activation after S-25.06 PR merge. Spec versions at v10.81: BC-INDEX v5.103 (2,007 BCs; BC-1.18.013 v1.6, BC-1.18.011 v1.13), VP-INDEX v3.26 (147 VPs), STORY-INDEX v4.481, ARCH-INDEX v4.48 (52 ADRs; ADR-052 v1.18), error-taxonomy v1.35, S-25.08 v1.1, S-25.06 v1.7, S-25.02 v4.9, E-25 epic v1.4, verification-architecture v1.39, verification-coverage-matrix v1.37.

### §7. HEADs
- `develop`: **`117dafdf`** (run `git rev-parse origin/develop` for live). merged_count **123**.
- `main`: **`51023185`** (tag `v1.0.0-rc.25` → `101ebb64`). UNCHANGED.
- `factory-artifacts`: run `git -C .factory log -1` (no self-cited SHA per TD-VSDD-053).

### §8. BC-5.39.001 streak
Cluster-5 F4-code cascade CONVERGED + MERGED (pass-4 CLEAN, D-1240). Cycle-level streak 3/3 CONVERGED. All cluster cascades 1-5 CLOSED and MERGED. PIPELINE IN_PROGRESS — NEXT = S-25.06 TDD continuation, then cluster-6 F1.
