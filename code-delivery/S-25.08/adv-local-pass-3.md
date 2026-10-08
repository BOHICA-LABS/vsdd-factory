---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 3 (RECONSTRUCTED RECORD — source report not saved)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-08T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 3
verdict: NOT CLEAN
finding_count: { total_numbered: 9, deferred: 1, severities: "NOT RECORDED — the source report was not saved" }
streak_3_clean: "0/3"
novelty: "NOT RECORDED"
source_report_saved: false
traces_to: S-25.08
---

# S-25.08 LOCAL Adversary Cascade — Pass 3 (reconstructed record)

> **PROVENANCE NOTE (read first).** The original fresh-context adversary report for pass 3 was
> **not saved to disk** by the session that ran it. This file is NOT the adversary's own text. It
> persists exactly what is recorded in `stories/S-25.08-shared-admission-core-b2-conformance-vp147-rebaseline.md`
> v1.6 §Adversarial Review "Local pass 3" (story-writer, from the orchestrator's brief and the
> review's findings list), registered by the state-manager in the D-1254 burst. Nothing here is
> fabricated or inferred beyond that section: **severities, novelty rating, evidence excerpts and
> reviewer reasoning were not recorded and are therefore absent.** Two subject cells (L3-008 and
> L3-009) were taken by the story-writer from the orchestrator's brief, not from the review file.
> Where the story section and the original report differ, the original report (unrecoverable) would
> have been authoritative; treat this file as a faithful index of the findings and their closure,
> not as primary evidence.

**Review:** S-25.08 LOCAL adversarial review pass 3 (per-story delivery Step 4.5, BC-5.39.001)
**Reviewer:** vsdd-factory:adversary (fresh context)
**Target:** `feature/S-25.08` cut state `5091f88f` (scope AC-001..AC-020 + AC-027)
**Verdict:** NOT CLEAN — nine numbered findings (F-S2508-L3-001..009) plus one deferred integration item (D-L3-1)
**Streak:** 0/3 (any finding resets). Trajectory: pass 1: 9; pass 2: 14 + 2 deferred; pass 3: 9 + 1 deferred.

## Part A — Findings (subjects as recorded in S-25.08 v1.6)

| Finding ID | Subject | Routing | Closure recorded (fix SHA on `feature/S-25.08`) |
|------------|---------|---------|-------------------------------------------------|
| F-S2508-L3-001 | A foreign txn record carrying only `state` + `migration_id` (minimal shape) is classified `E-MAINTENANCE-002 (state_integrity)` instead of a plain foreign refusal (`migration_id` not in K, BC-1.18.013 EC-029) | implementer (+ architect ruling ADR-052 §Error Code Semantics "Txn-record interpretation — tiers") | FIXED `e7ce7b62` (two-tier txn interpretation; AC-029); red `2cd45b01` + `f62d874f` |
| F-S2508-L3-002 | A `PostToolUseFailure` envelope with a non-string `tool_name` (null / number / bool / array / object) does not release the reservation (BC-1.18.013 EC-021(c): release keyed on `tool_use_id` only) | implementer | FIXED `ceddadee` (lenient `tool_name`; AC-011); red `2cd45b01` + `764534a7` |
| F-S2508-L3-003 | Reservation-timestamp "never clamped" retention vector (pre-epoch / non-`u64` `created_at`) lacked an adequate test | test-writer | CLOSED `2cd45b01` (test-only; AC-016) |
| F-S2508-L3-004 | Story AC-020 body, Architecture Mapping `admission.rs` row and Files-to-MODIFY row described a `Path::try_exists` terminal-record stat; the code at `5091f88f` reads the record with `std::fs::read` (ENOENT = absent, other error = `Io`) | story-writer | FIXED in story v1.6 |
| F-S2508-L3-005 | AC-016 / AC-017 `Test:` lines and AC-018's body cited S-25.09 artifacts that do not exist at the cut | story-writer | FIXED in story v1.6 (moved to labeled "Completed in S-25.09" notes) |
| F-S2508-L3-006 | The two surviving in-process tests `..._PC6_RULING1_gate_precedence_{staging,committing}_blocks_shard_cap_precheck_never_runs_no_roll` over-claimed the admission-before-`shard_cap_precheck` ordering (a property of `main::run`, pinned only by the real-binary test) | test-writer | CLOSED `2cd45b01` (re-scoped to `..._PC6_RULING2_admission_call_blocks_live_{staging,committing}_txn_and_performs_no_roll`) |
| F-S2508-L3-007 | The `io` cause for a failed read CALL of a txn record / terminal record lacked a root-safe vector (chmod 000 is bypassed by root; a directory at the record path fails `read` with EISDIR for every uid) | test-writer | CLOSED `2cd45b01` (test-only; AC-018) |
| F-S2508-L3-008 | CHANGELOG overclaims operator-visible diagnostics at the cut (the `[Unreleased] > Fixed` S-25.08 entry described admission diagnostics as emitted, but the dispatcher installs no `tracing` subscriber until S-25.09 delivers the InternalLog channel). Subject from the orchestrator's brief | implementer | FIXED `0bdcf4c6` |
| F-S2508-L3-009 | `.factory` stat errors other than absent collapsed to out-of-scope (`resolve_factory_root` used `is_ok_and(is_dir)`, so EACCES / ELOOP / EIO on the `stat` ADMITTED the write). Subject from the orchestrator's brief | architect (ruling ADR-052 §5a "Factory-root lookup mapping") + implementer | FIXED `4cd72af0` (three-way classification; AC-028); red `f62d874f` |
| D-L3-1 (deferred) | INTEGRATION / wave-gate: `read_txn_files` B2 schema vs the S-25.06 mechanism-A transaction shape — the shared admission core's txn reader must be shown to accept the txn record shape the S-25.06 mechanism-A writer actually produces | deferred to the wave-gate integration review | DEFERRED; verify at the wave gate (S-25.06 delivers the mechanism-A txn writer) |

## Convergence status

Pass 3 had findings, so the streak is 0/3. Every numbered pass-3 finding is recorded CLOSED; D-L3-1 stays
DEFERRED to the wave gate. A fresh pass 4 on the post-fix state is required because the fixes are unreviewed
by an adversary (S-25.08 tip `d1df7d10` at the D-1254 registration).
