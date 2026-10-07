---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 1"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-07T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 1
verdict: NOT CLEAN
finding_count: { critical: 0, high: 2, medium: 4, low: 3, nitpick: 0, process_gap: 0 }
streak_3_clean: "0/3"
novelty: HIGH
traces_to: S-25.08
---

# S-25.08 LOCAL Adversary Cascade — Pass 1

**Review:** S-25.08 LOCAL adversarial review pass 1 (per-story delivery Step 4.5, BC-5.39.001)
**Reviewer:** vsdd-factory:adversary (fresh context)
**Target:** `feature/S-25.08` @ `292ffed5` (base `develop` `ce2421be`)
**Verdict:** NOT CLEAN — 9 findings (2 HIGH, 4 MEDIUM, 3 LOW)
**Streak:** 0/3 (BC-5.39.001 3-CLEAN protocol; any finding resets the streak)
**Novelty:** HIGH

> Persisted verbatim-in-facts by state-manager (D-1251) from the in-session adversary delivery; the review
> was previously delivered only in-session. Routing outcomes below reflect the same-day specialist work
> (ADR-052 v1.20, BC-1.18.013 v1.9, BC-1.18.011 v1.17, error-taxonomy v1.38, S-25.08 v1.3).

## Part A — Findings

### F-S2508-L1-001 — HIGH — implementation + spec — failed tool call never releases reservation

Release is keyed on `PostToolUse` only. Claude Code delivers tool failures as `PostToolUseFailure` (registered in
`hooks.json.template`; `invoke.rs` `EventType::from_event_str` maps it to `Other`). A failed tool call therefore
leaks its reservation for the TTL (3600 s) and activation then hits `DRAIN_TIMEOUT_ABORT`. The test modelled
failure as `PostToolUse` + `is_error`, so it could not catch the gap. Violates story AC-003, BC-1.18.013 Pre 6(c),
BC-1.18.011 Pre 6(c).
**Routed:** architect/PO spec (ADR-052 v1.20, BC-1.18.013 EC-021, BC-1.18.011 EC-016), test-writer, implementer.

### F-S2508-L1-002 — HIGH — security — substring classification, fail-open across projects/worktrees

Classification is a substring `contains(".factory/cycles/")` anywhere on disk while migration-state is read from
`CLAUDE_PROJECT_DIR`. Result: fail-open admit mid-COMMITTING from subdirectory/worktree sessions, spurious
migration-state trees, and cross-project blocking.
**Routed:** `factory_root` anchoring (ADR-052 v1.20; EC-023 / EC-018).

### F-S2508-L1-003 — MEDIUM — security — no path canonicalization

`..`, `./`, `//`, case (APFS), and symlinks bypass the gate.
**Routed:** `resolve_target_path` (EC-024 / EC-019).

### F-S2508-L1-004 — MEDIUM — admission/release after `Registry::load`

`Registry::load` failure arms return before admission/release: zero gate evaluations and leaked reservations on a
broken registry.
**Routed:** evaluate before `Registry::load` (O1–O4; EC-022 / EC-017).

### F-S2508-L1-005 — MEDIUM — drain treats `read_dir` error as quiescent

`drain_bc_index_writers` treated a `read_dir` error as quiescent (`unwrap_or(true)`).
**FIXED** `332c14e0` (red test `54a9e782`); sibling fix: terminal-record `try_exists` fails closed (regression pin
in `e76920e7`).

### F-S2508-L1-006 — MEDIUM — test + spec — "foreign" tests could not fail

The "foreign" black-box tests could not fail (both known ids counted as own); the `RefuseForeignMigration` arm was
untested; TC2 asserted only a substring. POLICY 11.
**Routed:** foreign = `migration_id` ∉ K, `txn_migration_known` (EC-027..EC-030 / EC-022..EC-025).

### F-S2508-L1-007 — LOW — process

Stale red-gate STUB docs on implemented functions, garbled TTL error string, stale test header, wrong Kani
decision-row labels.
**Docs/string FIXED** `1438188f`; Kani row labels pending formal-verifier (T-11(a)).

### F-S2508-L1-008 — LOW — timestamp handling

`reservation_is_stale`: a future `created_at` is never stale; pre-1970 values handled inconsistently with
unparseable ones.
**Routed:** timestamp rules (EC-025 / EC-020; 300 s skew).

### F-S2508-L1-009 — LOW — error identity reuse

`BinaryIntegrityFailure` reused for sub-floor TTL and invalid `tool_use_id`.
**Routed:** `ReservationTtlBelowFloor` + `InvalidToolUseId` (EC-031 / EC-026).

## Checked clean

Message format exactness; single evaluation ahead of `shard_cap_precheck`; release-on-block funnel;
reserve-then-verify order; reconciliation under flock with Branch B in-place rewrite; pure cores match tables;
`tool_use_id` charset; TTL 3600/1800; test seam compiled out of release; Kani harnesses call real functions with
covers; no `unwrap`/`expect`/`println` in new code.

## Follow-on finding from spec work (not an adversary pass-1 finding)

`main.rs` registry-error `_ => 0` catch-all would silently exit 0 for future `RegistryError` variants
(BC-7.06.001 v1.13 — catch-all applies to unknown/future variants only; E-REG-001/002/003 are fail-closed
exceptions per BC-1.08.001 v1.4). Red tests `02a9d191`; fix in progress.

## Part B — Disposition summary

| ID | Severity | Status at persist time |
|----|----------|------------------------|
| F-S2508-L1-001 | HIGH | Spec ruled (ADR-052 v1.20); code OPEN (T-8/T-9) |
| F-S2508-L1-002 | HIGH | Spec ruled; code OPEN (T-8/T-9) |
| F-S2508-L1-003 | MEDIUM | Spec ruled; code OPEN (T-8/T-9) |
| F-S2508-L1-004 | MEDIUM | Spec ruled; code OPEN (T-8/T-9) |
| F-S2508-L1-005 | MEDIUM | FIXED `332c14e0` (+ `e76920e7`) |
| F-S2508-L1-006 | MEDIUM | Spec ruled; code OPEN (T-8/T-10) |
| F-S2508-L1-007 | LOW | Docs/string FIXED `1438188f`; Kani labels pending |
| F-S2508-L1-008 | LOW | Spec ruled; code OPEN (T-8/T-10/T-11) |
| F-S2508-L1-009 | LOW | Spec ruled; code OPEN (T-8/T-10) |

Next: implementer green batch (in progress), then LOCAL adversary pass 2 (fresh context; reads only this Part A).
