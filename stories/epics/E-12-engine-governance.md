---
document_type: epic
epic_id: "E-12"
version: "1.5"
title: "Engine Governance — Per-Story Adversarial Convergence Discipline + WASM-Plugin Context Resolver Platform"
status: draft
prd_capabilities: [CAP-008, CAP-016, CAP-026]
prd_frs: []
anchor_strategy: greenfield-discipline-gap-codification
priority: P1
target_release: "v1.0-feature-engine-discipline-pass-1"
story_count: 10
subsystems_affected: [SS-01, SS-04, SS-05]
producer: product-owner
timestamp: 2026-05-06T00:00:00Z
phase: 2
level: L3
traces_to: .factory/specs/architecture/decisions/ADR-017-per-story-adversary-phasing.md
depends_on: ["E-7"]
last_amended: "2026-09-11 (v1.5) — story_count 9→10: S-12.13 registered (E-SHD Display-parity lint gate + stale-narrative gate, 5 pts, P1, backlog; supersedes S-12.11 stub; cluster-4 re-cascade fix-wave-2, D-1210)"
inputs:
  - .factory/specs/architecture/decisions/ADR-017-per-story-adversary-phasing.md
  - .factory/specs/architecture/ARCH-INDEX.md
  - .factory/specs/behavioral-contracts/BC-INDEX.md
  - .factory/stories/epics/E-7-process-codification.md
input-hash: "11a7203"
modified:
  - "v1.3 2026-05-08: F-P22-003/004/005 subsystem swap fixed, frontmatter synced"
---
<!-- [process-gap] Frontmatter fields anchor_strategy, depends_on extend the canonical epic-template baseline (same as E-9 v1.0 / E-11 v1.0). Template update tracked as follow-up. -->

# Epic E-12: Engine Governance — Per-Story Adversarial Convergence Discipline + WASM-Plugin Context Resolver Platform

## Description

E-12 is the engine governance platform epic for cycle v1.0-feature-engine-discipline-pass-1.
It spans two delivery phases:

**Phase 1 (original, completed):** Closes the documented-vs-implemented gap on per-story
adversarial convergence. ADR-017 defines a three-perimeter scope contract (in-story body,
story boundary, wave close) for the adversary agent and mandates per-story convergence as a
gate condition before PR merge. Delivered via: (1) workflow + agent documentation updates
(Step 4.5 in `per-story-delivery.md`, adversary scope contract, orchestrator MANDATORY STEPS
reconciliation, wave-gate Gate 3 narrowing — S-12.01), and (2) a native WASM enforcement hook
(`validate-per-story-adversary-convergence` — S-12.02). Both stories merged 2026-05-07.

**Phase 2 (F3-amendment, D-361/D-366):** Closes F-P2-001 (convergence hook inert in production
due to missing wave-state→plugin_config wiring) via a generic, factory-agnostic WASM-plugin
Context Resolver platform. ADR-018 codifies the platform architecture. 6 new stories:
S-12.06 (HOST_ABI docs), S-12.03 (ContextResolver trait + ResolverRegistry), S-12.05
(hook-sdk extensions), S-12.04 (WASM loading + lifecycle), S-12.07 (vsdd-context-resolvers
crate + WaveContextResolver), S-12.08 (convergence hook migration — closes F-P2-001).
Platform is factory-agnostic: dispatcher core knows nothing about wave/story/cycle vocabulary;
per-factory resolvers ship in separate crates (WaveContextResolver is the first concrete resolver).

This is the spiritual successor to E-7 (Process Codification): E-7 codified
spec-first/TDD-Iron-Law discipline gaps surfaced in the S-6.01 sub-cycle; E-12
codifies the per-story adversarial convergence gap surfaced in the
v1.0-feature-engine-discipline-pass-1 F2 spec evolution. Pattern is identical —
process gap identified, documented in ADR, codified into enforcement mechanism.

## PRD Capabilities Covered

| Capability ID | Name | Priority |
|--------------|------|----------|
| CAP-008 | Gate tool calls with pre-execution behavioral checks (PreToolUse hooks) | P0 |
| CAP-016 | Self-referential process codification via adversarial review | P1 |
| CAP-026 | Enforce per-story adversarial convergence as a pipeline gate | P1 |

## Capability Anchor Justification

**Primary anchor:** CAP-008 ("Gate tool calls with pre-execution behavioral checks
(PreToolUse hooks)") per `domain-spec/capabilities.md` §CAP-008. The
`validate-per-story-adversary-convergence` WASM hook (S-12.02) is a PreToolUse gate
that blocks the `git push` / PR-merge tool call when the per-story convergence
condition is unmet. This is the same CAP-008 surface as the existing protection hooks
in SS-04.

**CAP-016 anchor:** ADR-017's three-perimeter model and Step 4.5 workflow update
(S-12.01) are process codification artifacts — the same capability class as E-7's
lessons-codification work.

**SS-05 (Pipeline Orchestration) anchor:** S-12.01 touches `per-story-delivery.md`,
`adversary.md` scope contract, and orchestrator AGENT.md MANDATORY STEPS — all
SS-05 artifacts (`plugins/vsdd-factory/agents/`, `plugins/vsdd-factory/workflows/phases/`).

**SS-04 (Plugin Ecosystem) anchor:** S-12.02 delivers a new native WASM crate at
`crates/hook-plugins/validate-per-story-adversary-convergence/` — an SS-04 artifact.

## Subsystem Anchors

- **SS-05 (Pipeline Orchestration):** owns `plugins/vsdd-factory/workflows/phases/per-story-delivery.md`,
  `plugins/vsdd-factory/agents/adversary.md`, `plugins/vsdd-factory/agents/orchestrator/AGENT.md`,
  `plugins/vsdd-factory/workflows/wave-gate.md`. S-12.01 modifies these files to
  insert Step 4.5, add the three-perimeter scope contract to adversary.md, reconcile
  MANDATORY STEPS, and narrow Gate 3 language.
- **SS-04 (Plugin Ecosystem):** owns `crates/hook-plugins/`. S-12.02 adds the
  `validate-per-story-adversary-convergence` native WASM crate, registers it in
  `plugins/vsdd-factory/hooks-registry.toml`, and adds bats tests + cargo unit tests.

## Stories Planned

| Story | Description | Size | Subsystem | Depends On | Status |
|-------|-------------|------|-----------|------------|--------|
| S-12.01 | Workflow + agent doc updates (per-story-delivery.md Step 4.5, adversary.md scope contract, wave-gate Gate 3 narrowing, orchestrator MANDATORY STEPS reconciliation) | M | SS-05 | — | completed |
| S-12.02 | `validate-per-story-adversary-convergence` WASM hook (Rust crate, hooks-registry.toml registration, bats tests + cargo tests) | M | SS-04 | S-12.01 | completed |
| S-12.06 | HOST_ABI context-injection contract docs (factory-agnostic; ships first in Phase 2) | S | SS-04 | — | draft |
| S-12.03 | ContextResolver trait + ResolverRegistry generic dispatcher core | M | SS-01 | S-12.06 | draft |
| S-12.05 | hook-sdk resolver-authoring extensions | M | SS-04 | S-12.06 | draft |
| S-12.04 | WASM resolver loading + lifecycle + error isolation | M | SS-04 | S-12.03 | draft |
| S-12.07 | vsdd-context-resolvers crate + WaveContextResolver (first concrete resolver) | M | SS-04 | S-12.04, S-12.05 | draft |
| S-12.08 | Migrate validate-per-story-adversary-convergence to consume plugin_config.wave_context.stories (closes F-P2-001) | M | SS-04 | S-12.07 | draft |
| S-15.03 | ARCH-INDEX Cite-Refresh Hook + Lessons Retroactive-Sweep Verification (validate-index-cite-refresh hook; validate-lesson-retroactive-sweep hook) | M | SS-01, SS-04 | — | draft |

## Dependency Topology (Intra-epic)

**Phase 1 (completed):**
```
S-12.01 (Workflow + agent docs) ──→ S-12.02 (WASM hook implementation)
```

S-12.01 must ship first because the WASM hook (S-12.02) enforces the contract
defined by the Step 4.5 workflow update. Implementing the hook before the workflow
is defined would produce a gate with no documented recovery path.

**Phase 2 — F3-amendment (WASM-plugin Context Resolver platform):**
```
S-12.06 (HOST_ABI docs) ──→ S-12.03 (ContextResolver trait + ResolverRegistry)
                        ──→ S-12.05 (hook-sdk extensions)
                                ↓                    ↓
                            S-12.04 (WASM loading + lifecycle)
                                ↓
                            S-12.07 (vsdd-context-resolvers + WaveContextResolver)
                                ↓
                            S-12.08 (convergence hook migration — closes F-P2-001)
```

S-12.06 ships first (HOST_ABI docs establish the factory-agnostic context-injection
contract; foundational, doc-only). S-12.03 and S-12.05 can proceed in parallel once
S-12.06 is merged. S-12.04 (WASM loading) requires S-12.03 (trait definition).
S-12.07 (concrete resolver crate) requires both S-12.04 and S-12.05.
S-12.08 (migration) requires S-12.07 (WaveContextResolver available).

## Anchored BCs

| BC ID | Description | Subsystem | Priority |
|-------|-------------|-----------|----------|
| BC-5.39.001 | Per-story adversary step 4.5 is present and sequenced correctly in per-story-delivery.md | SS-05 | P0 |
| BC-5.39.002 | Adversary.md scope contract covers all three perimeters (in-story body, story boundary, wave close) | SS-05 | P0 |
| BC-4.10.001 | validate-per-story-adversary-convergence WASM hook blocks PR merge when convergence gate unsatisfied | SS-04 | P0 |
| BC-4.10.002 | validate-per-story-adversary-convergence degrades gracefully when convergence state file absent | SS-04 | P1 |
| BC-1.13.001 | Dispatcher loads resolver registry + injects context before hook dispatch (SS-01) | SS-01 | P0 |
| BC-4.12.001 | Resolver lifecycle invariant (mtime-cache load-at-startup) | SS-04 | P0 |
| BC-4.12.002 | Resolver ABI/payload schema (ResolverInput/ResolverOutput; RESOLVER_ABI_VERSION=1) | SS-04 | P0 |
| BC-4.12.003 | Resolver capability model (path_allow per resolver in resolvers-registry.toml) | SS-04 | P0 |
| BC-4.12.004 | Resolver error/crash isolation | SS-04 | P0 |
| BC-4.12.005 | Context-injection merging contract | SS-04 | P0 |

## Anchored ADRs

| ADR ID | Title |
|--------|-------|
| ADR-017 | Per-story adversary three-perimeter model + phasing |
| ADR-018 | WASM-plugin Context Resolver platform architecture |

## Anchored VPs

| VP ID | Type | Description |
|-------|------|-------------|
| VP-071 | kani | Convergence block invariant: hook always returns deny when convergence flag absent |
| VP-073 | kani | Resolver load purity (F2-amendment) |
| VP-074 | kani | Resolver error isolation (F2-amendment) |
| VP-075 | kani | Context-injection determinism (F2-amendment) |
| VP-076 | kani | Resolver capability confinement (F2-amendment) |

## Relationship to E-7

E-7 (Process Codification, status: open) codified five S-6.01 sub-cycle lessons into
agent prompts, lint hooks, and rule files. E-12 is NOT a reopening of E-7 — E-7 is
converged and its stories are complete. E-12 is the next process codification wave,
targeting a different gap (per-story adversarial convergence) surfaced after E-7 closed.
The pattern is identical; the discipline gap is distinct.

## Open Questions

| OQ ID | Scope | Description |
|-------|-------|-------------|
| OQ-E12-01 | SS-04 | Convergence state file format: TOML vs JSON vs plain flag file. Decision needed before S-12.02 implementation. |
| OQ-E12-02 | SS-05 | Gate 3 narrowing scope: does narrowing apply to wave-gate only or also to per-story-delivery phased gate? |

## CHANGELOG

| Version | Date | Change |
|---------|------|--------|
| v1.0 | 2026-05-06 | Initial authoring for cycle v1.0-feature-engine-discipline-pass-1 as F3 prerequisite. Two stories: S-12.01 (workflow + agent docs, SS-05) + S-12.02 (WASM hook, SS-04). Anchored BCs: BC-5.39.001/002 (SS-05), BC-4.10.001/002 (SS-04). ADR: ADR-017. VP: VP-071. |
| v1.1 | 2026-05-07 | F3-amendment (D-366): scope widened from 'per-story adversary workflow' to 'engine governance platform'. 6 new stories added (S-12.03..S-12.08; WASM-plugin Context Resolver platform). story_count 2→8. New BCs: BC-1.13.001 (SS-01) + BC-4.12.001-005 (SS-04). New ADR: ADR-018. New VPs: VP-073-076. subsystems_affected expanded to include SS-01. Dependency graph established: S-12.06 → {S-12.03, S-12.05} → S-12.04 → S-12.07 → S-12.08. Bootstrap pattern flipping right-side-up: S-12.03..S-12.08 are first stories in cycle history subject to Step 4.5 per-story adversary convergence. |
| v1.2 | 2026-05-08 | F-P21-003 (fix-burst-20): S-15.03 re-anchored from E-15 (Plugin Async Semantics — incorrect) to E-12 (Engine Governance — correct per governance/discipline scope alignment). S-15.03 subsystems [SS-04] → [SS-01, SS-04]. story_count 8→9. No change to subsystems_affected (SS-01 already present from v1.1). Refs: F-P21-003, L-P21-002. |
| v1.3 | 2026-05-08 | F-P22-003/004/005 + L-P21-002 retroactive sweep: Stories Planned table subsystem swap corrected for S-12.03 (SS-04→SS-01) and S-12.06 (SS-01→SS-04). Frontmatter `version:` bumped 1.0→1.3 (was stale; body already at 1.2 in fix-burst-20). Frontmatter `title:` synced to match H1 (added "+ WASM-Plugin Context Resolver Platform" suffix from v1.1 F3-amendment). L-P21-002 retroactive sweep on all 9 stories: S-12.01/02/04/05/06/07/08/S-15.03 verified clean; S-12.03 SS drift was the sole finding (covered by F-P22-003 fix above). Refs: F-P22-003, F-P22-004, F-P22-005, L-P21-002, L-P19-001. |

## Amendment 2026-05-08 (v1.2 → v1.3)

**Driver:** F-P22-003/004/005 + L-P21-002 retroactive sweep.

**Changes:**
1. Stories Planned table: swap subsystems for S-12.03 (SS-04→SS-01) and S-12.06 (SS-01→SS-04) per F-P22-003 and L-P21-002 retroactive sweep.
2. Frontmatter `version:` synced 1.0 → 1.3 (was stale; body already at 1.2 in fix-burst-20).
3. Frontmatter `title:` synced to match H1 (added "+ WASM-Plugin Context Resolver Platform" suffix from v1.1 F3-amendment).
4. L-P21-002 retroactive sweep on all 9 stories under E-12 — verified epic anchor + subsystems coherence.

**Refs:** F-P22-003, F-P22-004, F-P22-005, L-P21-002, L-P19-001 (same-burst retroactive sweep).
