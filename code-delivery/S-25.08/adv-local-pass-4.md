---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 4 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-08T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 4
reviewed_head: "feature/S-25.08 @ d1df7d10"
verdict: NOT CLEAN
finding_count: { total_numbered: 7, medium: 1, low: 6, deferred: 1 }
streak_3_clean: "0/3"
novelty: "MEDIUM-LOW"
traces_to: S-25.08
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back in session 2026-10-07/08; reviewed HEAD feature/S-25.08 @ d1df7d10.

# S-25.08 LOCAL Adversary — Pass 4

Verdict: NOT CLEAN. One MEDIUM and six LOW findings. Streak stays 0/3.

Identity tuple: worktree .worktrees/S-25.08, HEAD d1df7d10, story S-25.08, canonical root /Users/zious/Documents/GITHUB/vsdd-factory. Pass-3 review file was absent; the story table was used.

Pass-3 fix verification: L3-001 fixed and load-bearing (Tier 0 in read_txn_files; branch_b_generation_id_is_null tri-state under the Branch B conditions; abort_null_generation_txn raw; Tier 1 black-box rows across three families, three gates and the lock-held twin). L3-002 fixed (lenient_tool_name). L3-003 closed. L3-007 closed. L3-008 fixed. L3-009 fixed on the admission leg (release leg sibling gap = F-L4-001). L3-006 renamed and re-scoped but its remaining assertions are weak (F-L4-006).

Crash-injection suite (cb9e6797) and CI (d1df7d10): the self-heal helper is sound overall (two weaknesses in F-L4-003); the lock-held controls are real; fail_point_scope() held by all 38 in-process tests; the CI failpoints steps work (see F-L4-004).

F-S2508-L4-001 MEDIUM — the release leg turns a non-ENOENT migration-state stat error into a silent no-op. executor.rs bc_index_migration_reservation_release: `if !migration_state_dir.exists() { return; }`. Spec: BC-1.18.013 EC-021 and AC-011 (non-ENOENT release error is a non-fatal tracing::warn!). Failure: .factory mode 000 or a self-referential migration-state symlink → the leg returns with no warn; reservation leaks to TTL GC with no trace; the same guard is in the S-25.09..S-25.12 worktrees. The existing ..._release_leg_no_verdict_reservation_left_blackbox would also pass under the collapse. Routing: implementer + test-writer.

F-S2508-L4-002 LOW (pending intent verification) — E-MAINTENANCE-002 cause-check order: the spec says tool_use_id before any filesystem access; the code stats first (resolve_factory_root, resolve_target_path, then classify_tool_use_id). Unstatable .factory + tool_use_id "../x" gives (io) vs rule-1 order (invalid_tool_use_id). Routing: product-owner/architect, then test-writer.

F-S2508-L4-003 LOW — the crash-suite Branch B helper strips updated_at before comparing (abort_null_generation_txn never touches it), and `pre["generation_id"].is_null()` is also true when absent. Routing: test-writer.

F-S2508-L4-004 LOW — the CI failpoints step comment names s2509_v123_write_error_propagation_failpoints_test (not on this branch) and the step relies only on the exit code (no positive-coverage line). Routing: devops-engineer.

F-S2508-L4-005 LOW — the S-25.08 CHANGELOG entry cites old BC versions and contradicts itself on minimal records (a minimal known STAGING without generation_id with the lock free is state_integrity, EC-045). Routing: implementer.

F-S2508-L4-006 LOW — the re-scoped RULING2 tests contain an unfalsifiable no-roll assertion and a loose message check (contains("retry")||contains("migrat")). Routing: test-writer.

F-S2508-L4-007 LOW [process-gap] — the pass-3 review file was never persisted (repeats the D-1252-OWED pattern). Routing: state-manager.

Deferred D-L4-1 (architectural, phase-5): a protected subtree that is itself a symlink out of the root (e.g. <factory_root>/cycles → /elsewhere/cycles) is not gated when written through its resolved location; the code matches BC-1.18.013 Precondition 6(b)(ii) as written; spec-level gap. D-L3-1 carried over (lower risk now that Tier 0 is minimal).

Scope confirmations: the story claims no S-25.09+ ACs; the newer-schema version gate is S-25.09's; T-6/T-12/T-14 (Kani re-run evidence) are open tasks, not findings.

Novelty: MEDIUM-LOW. No CRITICAL or HIGH in the admission core.
