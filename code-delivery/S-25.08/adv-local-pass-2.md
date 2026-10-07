---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 2"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-07T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 2
verdict: NOT CLEAN
finding_count: { critical: 0, high: 2, medium: 7, low: 5, nitpick: 0, process_gap: 0, deferred: 2 }
streak_3_clean: "0/3"
novelty: MEDIUM
traces_to: S-25.08
---

# S-25.08 LOCAL Adversary Cascade — Pass 2

**Review:** S-25.08 LOCAL adversarial review pass 2 (per-story delivery Step 4.5, BC-5.39.001)
**Reviewer:** vsdd-factory:adversary (fresh context)
**Target:** `feature/S-25.08` @ `25e5464e` (base `develop` `ce2421be`)
**Verdict:** NOT CLEAN — 14 in-scope findings (2 HIGH, 7 MEDIUM, 5 LOW) plus 2 deferred (D-1, D-2)
**Streak:** 0/3 (BC-5.39.001 3-CLEAN protocol; any finding resets the streak)
**Novelty:** MEDIUM
**Admission core:** no CRITICAL defect found in the admission core.

> Persisted verbatim-in-facts by state-manager (D-1252) from the in-session adversary delivery; the review
> was previously delivered only in-session. Routing outcomes below reflect the same-day specialist work
> (ADR-052 v1.21, BC-1.18.013 v1.10, BC-1.18.011 v1.18, BC-3.08.001 v1.35, error-taxonomy v1.39, S-25.08 v1.4).

## Part A — Findings

### F-S2508-L2-001 — HIGH — spec-flaw — Forbidden Dependencies rule reversed

The story's Forbidden Dependencies rule said `factory-dispatcher` must not depend on `last-amended-migrate`.
`factory-dispatcher` legitimately depends on `last-amended-migrate` per ADR-051 §8; the forbidden edge is the reverse.
**Fixed:** S-25.08 v1.4.

### F-S2508-L2-002 — HIGH — POLICY 19 — load-bearing ADR version cites in the story

About 32 load-bearing `ADR-052 vX.Y` cites appeared in the story (ACs, tasks, rules).
**Fixed:** S-25.08 v1.4 (§-anchor form).

### F-S2508-L2-003 — MEDIUM — test-flaw — over-cap blocked-write tests lacked a gate-OPEN positive control

The over-cap blocked-write tests asserted only the blocked outcome; without a gate-OPEN positive control they could
not distinguish a working gate from a test that blocks for an unrelated reason.
**Fixed:** test `5023296c` (positive control green).

### F-S2508-L2-004 — MEDIUM — stale CHANGELOG

The CHANGELOG entry did not match shipped behavior and current spec cites.
**Fixed:** `5091f88f`, refreshed `e7a490cd`.

### F-S2508-L2-005 — MEDIUM — traceability — AC `Test:` names did not exist (8 ACs)

Eight ACs named `Test:` functions that do not exist in the tree.
**Fixed:** S-25.08 v1.4 (every `Test:` line re-pointed to verified names; ACs with no real test flagged inline).

### F-S2508-L2-006 — MEDIUM — verification-gap — EC-031 "nothing mutated" unreachable at production entry

The BC-1.18.013 EC-031 "nothing mutated" guarantee was unreachable through the production entry point.
**Fixed:** ADR-052 v1.21 ruling — `pub(crate)` `run_bc_index_migration_with_ttl` seam (`07fccafd`).

### F-S2508-L2-007 — MEDIUM — foreign `migration_id` logged raw

An untrusted foreign `migration_id` was written to diagnostics without sanitization.
**Fixed:** `sanitize_diagnostic_id` (`80618991`), red test `21398fed`.

### F-S2508-L2-008 — MEDIUM — verification-gap — AC-016 warn reason tokens untested

The AC-016 warn reason tokens had no test.
**Pinned green:** `21398fed`; later re-ruled as coordinator stderr tokens (ADR-052 v1.21 item 33(c)).

### F-S2508-L2-009 — MEDIUM — traceability — `registry_error_exit_code` work unanchored

The `registry_error_exit_code` work had no story/BC anchor.
**Fixed:** S-25.08 v1.4 AC-027 plus BC-7.06.001 / BC-1.08.001 anchors.

### F-S2508-L2-010 — LOW — relative `CLAUDE_PROJECT_DIR` alias not filtered

A relative `CLAUDE_PROJECT_DIR` spelling was accepted as a lexical alias.
**Fixed:** `as_given_factory_root_spelling` (`80618991`).

### F-S2508-L2-011 — LOW — stale doc comments

Stale comments ("foreign", scope guard, `main.rs`).
**Fixed:** `a75e94de`.

### F-S2508-L2-012 — LOW — `BinaryIntegrityFailure` reused; wildcard cause match

`BinaryIntegrityFailure` was reused for state-integrity failures and the cause mapping used a wildcard arm.
**Fixed:** `AdmissionStateIntegrity { kind }` plus exhaustive match (`0e90e101`).

### F-S2508-L2-013 — LOW — admission blocks labelled `shard-cap-gate`

Admission blocks carried the `shard-cap-gate` label.
**Fixed:** `NativeGate` / `migration-admission` (`67fc6437`).

### F-S2508-L2-014 — LOW — `append_log_backfill_*` delegates absent

The per-migration `append_log_backfill_*` delegate entry points the spec named did not exist.
**Resolved:** ADR-052 v1.21 F-014 neutral renames.

## Deferred

### D-1 — dispatcher installs no tracing subscriber

Admission diagnostics emitted through `tracing` are discarded because the dispatcher installs no subscriber.
**Re-routed:** admission diagnostics via `InternalLog` (ADR-052 v1.21, `41acb16a`). The remaining ~65 crate-wide
sites go to NEW story S-26.06 (human decision 2026-10-07).

### D-2 — coordinator anchored on process cwd vs admission on `CLAUDE_PROJECT_DIR`

**Fixed:** `resolve_session_project_root` (`07fccafd`, `f8726c34`).

## Checked clean

Reserve-then-verify; release-on-block funnel; evaluation before `Registry::load`; terminal record actually read;
io / state_integrity partition; `resolve_target_path`; `tool_use_id` grammar; timestamp math;
`is_tool_completion_event`; `registry_error_exit_code` exhaustive; `resolve_shard_gate_precedence` removed; Kani
10 harnesses call real functions + S10; test seam compiled out of release; no `unwrap` / `expect` / `println`.

## Part B — Disposition summary

| ID | Severity | Class | Status at persist time |
|----|----------|-------|------------------------|
| F-S2508-L2-001 | HIGH | spec-flaw | FIXED (S-25.08 v1.4) |
| F-S2508-L2-002 | HIGH | POLICY 19 | FIXED (S-25.08 v1.4) |
| F-S2508-L2-003 | MEDIUM | test-flaw | FIXED (`5023296c`) |
| F-S2508-L2-004 | MEDIUM | docs | FIXED (`5091f88f`, `e7a490cd`) |
| F-S2508-L2-005 | MEDIUM | traceability | FIXED (S-25.08 v1.4) |
| F-S2508-L2-006 | MEDIUM | verification-gap | FIXED (ADR-052 v1.21; `07fccafd`) |
| F-S2508-L2-007 | MEDIUM | security | FIXED (`80618991`; red `21398fed`) |
| F-S2508-L2-008 | MEDIUM | verification-gap | PINNED green (`21398fed`); re-ruled ADR-052 v1.21 item 33(c) |
| F-S2508-L2-009 | MEDIUM | traceability | FIXED (S-25.08 v1.4 AC-027) |
| F-S2508-L2-010 | LOW | hardening | FIXED (`80618991`) |
| F-S2508-L2-011 | LOW | docs | FIXED (`a75e94de`) |
| F-S2508-L2-012 | LOW | error identity | FIXED (`0e90e101`) |
| F-S2508-L2-013 | LOW | labelling | FIXED (`67fc6437`) |
| F-S2508-L2-014 | LOW | spec/impl naming | RESOLVED (ADR-052 v1.21 F-014) |
| D-1 | deferred | observability | admission diagnostics via InternalLog (`41acb16a`); remainder to S-26.06 |
| D-2 | deferred | anchoring | FIXED (`07fccafd`, `f8726c34`) |

Next: execute the S-25.08 / S-25.09 split (D-1252(f)), then LOCAL adversary pass 3 per story (fresh context; reads only
this Part A).
