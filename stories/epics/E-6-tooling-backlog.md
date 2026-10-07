---
document_type: epic
epic_id: "E-6"
version: "1.2"
prd_capabilities: [CAP-017]
prd_frs: [FR-041]
status: open
story_count: 2
producer: story-writer
timestamp: 2026-04-25T00:00:00
phase: 2
inputs:
  - .factory/STATE.md
  - .factory/stories/S-6.01-create-adr-skill.md
  - .factory/stories/S-6.02-workspace-toolchain-1-96-and-wasmtime-49-upgrade.md
input-hash: "ac8cece"
level: L3
traces_to: .factory/STATE.md#D-005
last_amended: "2026-10-07 (v1.2) — S-6.02 registered (story-writer): story_count 1→2; Stories table +S-6.02 row (draft, 8 pts, P2; workspace toolchain rustc 1.96 + wasmtime 49 upgrade, carved out of ADR-035 v1.2 §Decision 6); S-6.01 row status ready→merged (confirmed: STORY-INDEX S-6.01 merged; story file status: merged v1.6; develop commit 9dcc52b1 PR #7 'feat: create-adr skill ... (S-6.01)'); inputs +S-6.02. [Prior: 2026-08-30 (v1.1)]"
modified:
  - "v1.0-initial"
  - "v1.1 2026-08-30: frontmatter normalization (level: L3)"
  - "v1.2 2026-10-07: S-6.02 registered (story_count 1→2); S-6.01 row ready→merged; inputs +S-6.02"
---

# Epic E-6: VSDD Self-Improvement / Tooling Backlog

## Description

Skills and tools that improve the factory's own authoring workflow rather than
shipping product features. Catches gaps exposed during use of vsdd-factory by
vsdd-factory itself (dogfooding). E-6 is an open-ended backlog epic: stories are
added as pain points surface during factory operations. The first story, S-6.01,
was created in response to D-005 (see STATE.md), which was identified after the
10-ADR brownfield backfill burst exposed manual-process pain in ADR ID allocation,
ARCH-INDEX drift, and supersession bookkeeping.

Milestone: `v1.0.x` (post-1.1 tooling hardening). Subsystems: SS-06, SS-08, SS-10.

## Source / Origin

- **D-005 in STATE.md:** "Add create-adr skill to v1.0.x roadmap — ADR is the
  only major artifact without a dedicated authoring skill (compare create-prd,
  create-story, create-architecture, create-domain-spec); 10-ADR backfill exposed
  pain points (manual ID allocation, ARCH-INDEX drift, no supersession patcher)."
- **10-ADR brownfield backfill burst:** Commit `c50bb0f` on `factory-artifacts`
  wrote ADR-004 through ADR-013 manually, exposing the gap.
- **FR-041** added to PRD during the same pass to formally capture the capability.

## Goals

- Close the per-artifact create-* skill gap: once S-6.01 ships, every major
  factory artifact type (brief, prd, adr, domain-spec, architecture, story) has a
  dedicated authoring skill with collision-free ID allocation and index sync.
- Reduce manual-process pain in spec authoring: eliminate the category of errors
  (ID conflicts, ARCH-INDEX drift, supersession inconsistency) that caused rework
  during the brownfield backfill burst.
- Dogfood VSDD on its own toolchain: E-6 stories are themselves authored using
  the same VSDD workflow they improve, validating the pipeline end-to-end.

## PRD Capabilities Covered

| Capability ID | Name | Priority |
|--------------|------|----------|
| CAP-017 | Create and manage formal ADR records (covers FR-041 — create-adr authoring skill) | P1 |

## Behavioral Contracts Covered

| BC ID | Title | Story |
|-------|-------|-------|
| BC-6.20.001 | create-adr allocates next sequential ADR-NNN by scanning filesystem and ARCH-INDEX | S-6.01 |
| BC-6.20.002 | create-adr refuses explicit --id override that already exists | S-6.01 |
| BC-6.20.003 | create-adr blocks on filesystem-vs-ARCH-INDEX ID mismatch | S-6.01 |
| BC-6.20.004 | create-adr writes frontmatter with status=proposed (always at creation) | S-6.01 |
| BC-6.20.005 | create-adr validates subsystems_affected against ARCH-INDEX Subsystem Registry | S-6.01 |
| BC-6.20.006 | create-adr validates --supersedes ADR-NNN exists before proceeding | S-6.01 |
| BC-6.20.007 | create-adr bidirectionally patches old ADR's superseded_by on supersession | S-6.01 |
| BC-6.20.008 | create-adr inserts ARCH-INDEX row in numeric order, pipe-aligned | S-6.01 |
| BC-6.20.009 | create-adr scaffolds placeholder section bodies verbatim from template (no ghost-writing) | S-6.01 |
| BC-6.20.010 | create-adr annotates Source/Origin section under --brownfield or implicit-brownfield | S-6.01 |
| BC-6.20.011 | create-adr runs validate-template-compliance.sh as final gate, blocks on non-zero | S-6.01 |
| BC-6.20.012 | create-adr is atomic — any partial-state failure rolls back all side-effects | S-6.01 |

## Acceptance Criteria

| ID | Criterion | Validation Method | Test Scenarios |
|----|-----------|-------------------|---------------|
| EAC-001 | Per-artifact authoring skill exists for all artifact types: brief, prd, adr, domain-spec, architecture, story | Skill catalog audit | `ls plugins/vsdd-factory/skills/create-*/SKILL.md` returns 6 entries |
| EAC-002 | create-adr allocates collision-free IDs and syncs ARCH-INDEX atomically | Integration test (AC-1, AC-4, AC-8 in S-6.01) | All 12 BC-6.20.NNN test suites green |
| EAC-003 | create-adr supports supersession with bidirectional backpatch | Integration test (AC-3 in S-6.01) | BC-6.20.007 test suite green |

Once all stories in E-6 land, the per-artifact authoring story (create-brief,
create-prd, create-adr, create-domain-spec, create-architecture, create-story)
is complete with no gaps.

## Stories

| Story ID | Title | Points | Depends On | Status |
|----------|-------|--------|-----------|--------|
| S-6.01 | Add create-adr skill for ADR authoring | 3 | — | merged |
| S-6.02 | Workspace toolchain upgrade to rustc 1.96 + wasmtime 49 (carved out of ADR-035 v1.2 §Decision 6; predecessor is the wasmtime-48 security fix branch, not a story) | 8 | — | draft |
| *(future)* | Additional tooling backlog stories TBD | — | — | — |

## Dependencies (External)

| System | Capability Needed | Readiness |
|--------|------------------|-----------|
| ARCH-INDEX.md | Authoritative ADR registry and Subsystem Registry | Available |
| `plugins/vsdd-factory/templates/adr-template.md` | ADR scaffold template | Available |
| `plugins/vsdd-factory/bin/validate-template-compliance.sh` | Template compliance validator | Available |
