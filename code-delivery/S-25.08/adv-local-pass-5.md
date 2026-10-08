---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 5 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-08T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 5
reviewed_head: "feature/S-25.08 @ a7fc5930"
verdict: NOT CLEAN
finding_count: { total_numbered: 4, high: 0, medium: 1, low: 3, deferred: 0 }
streak_3_clean: "0/3"
novelty: "LOW-MEDIUM"
traces_to: S-25.08
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back, session 2026-10-07/08; reviewed HEAD feature/S-25.08 @ a7fc5930.

# S-25.08 LOCAL Adversary — Pass 5

Identity tuple: worktree .worktrees/S-25.08 (basename matches S-25.08), HEAD a7fc5930, story S-25.08, canonical root /Users/zious/Documents/GITHUB/vsdd-factory. Specs read from canonical .factory/: BC-1.18.013 v1.14, BC-1.18.011 v1.22, ADR-052 v1.25, error-taxonomy v1.42. Git not runnable; code read at the worktree tip.

Verdict: NOT CLEAN. One MEDIUM and three LOW findings. Streak stays 0/3. Novelty: LOW-MEDIUM. Nothing CRITICAL or HIGH; the production code in the admission core and release leg is correct. All four findings concern test strength, documentation or CI diagnostics.

Pass-4 fix verification: L4-001 release leg — code fixed (probe_release_migration_state_dir maps ENOENT alone to Ok(false); any other stat error → Err → tracing::warn! naming path and error_kind, then return; never a verdict; matches EC-021 and AC-031), but no test pins it (F-S2508-L5-001). L4-001 sibling exclusive.lock — correct, cosmetic only (try_exists()? → Io; old and new forms give the same outcome). L4-002 evaluation order — ruled and pinned (the ADR-052 §Error Code Semantics ruling and BC-1.18.013 Pre 6(c) rule 1 v1.14 match the code order; vectors (g)(h)(i) catch the two mutants). L4-003 self-heal helper — load-bearing. L4-004 CI floor — correct (MIN_TESTS=39 equals the 39 #[test] fns; no CARGO_TERM_COLOR; captured libtest output is not a TTY). L4-005 CHANGELOG — only partly fixed (F-S2508-L5-002). L4-006 RULING2 tests — load-bearing (EXACT_BC_INDEX_BLOCK_REASON matches BC-1.18.013 Pre 6(b); the no-roll claim gone; the COMMITTING twin is deterministic). L4-007 missing review file — closed.

F-S2508-L5-001 — MEDIUM — the L4-001 fix has no test that fails if it is removed (TD-VSDD-059 paper-fix risk). Anchor: executor::bc_index_migration_reservation_release (the match probe_release_migration_state_dir arm). The test-file docs in tests/bc_1_18_013_release_stat_error_test.rs and AC-031 claim the warn-versus-silent difference "is NOT observable … in S-25.08". Evidence: the 4 helper tests call probe_release_migration_state_dir directly; the black-box ..._release_with_eloop_migration_state_exits_0_deletes_nothing_blackbox passes on the pre-fix code. The "not observable" claim is wrong: tests/s2508_diagnostics_test.rs defines `impl tracing::Subscriber for Capture` and `captured(f)` via tracing::subscriber::with_default, and bc_index_migration_reservation_release(payload, cwd) is pub. Failure: either mutant passes every test — reverting to `if !migration_state_dir.exists() { return; }` with the helper left in place, or changing `Err(e) => { tracing::warn!(..); return; }` to `Err(_) => return`. Fix: an in-process test using Capture with a self-referential migration-state symlink, calling for PostToolUse and PostToolUseFailure, asserting exactly one WARN on target bc_1_18_011_migration with error_kind ≠ NotFound and a path field; an ENOENT control with zero warns; correct the "not observable" wording. Routing: test-writer, then story-writer.

F-S2508-L5-002 — LOW — L4-005 only partly fixed: the CHANGELOG cites stale versions and S-25.09-only tokens. Anchor: CHANGELOG.md [Unreleased] > Fixed, the S-25.08 entry. Line 21 "current versions BC-1.18.011 v1.21, BC-1.18.013 v1.13, ADR-052 v1.24" (live: v1.22, v1.14, v1.25). The two-tier bullet (line 30) says "(kind=txn_record_malformed, EC-045 …)" and "(branch=live_coordinator)"; grep finds neither in the worktree src (they belong to S-25.09), and line 24 says S-25.08 has no operator-visible channel. Line 29 presents the exclusive.lock change as a behavior change, but io was already the outcome. Fix: drop the version pin or cite by §-anchor (POLICY 19); remove the S-25.09 tokens; reword the exclusive.lock sentence. Routing: implementer.

F-S2508-L5-003 — LOW — the CI positive-coverage re-run hides its own output on failure. Anchor: .github/workflows/ci.yml, cargo-host step "cargo test (factory-dispatcher, failpoints feature)". Under `set -euo pipefail`, `out="$(cargo test … --test bc_1_18_011_b2_migration_crash_injection_test 2>&1)"`: if the second run fails, errexit stops on the assignment, so the printf and ::error:: never run. Fix: `out="$(… 2>&1)" || { printf '%s\n' "$out"; exit 1; }` or set +e around the capture. Routing: devops-engineer.

F-S2508-L5-004 — LOW — the story contradicts itself on pass-4 status. Anchor: S-25.08 §Adversarial Review "Local pass 4" table and its "Convergence status (v1.7)" paragraph. The L4-001 row says "CHANGELOG bullet owed (T-43)" while the frontmatter round 3 says T-43 DONE at a7fc5930; the convergence paragraph says L4-007 is owed (T-42) and cites tip db671fea while the L4-007 row says CLOSED and the tip is a7fc5930; AC-031 and the L4-001 row repeat the "not observable" claim. Routing: story-writer.

Deferred: D-L4-1 (architectural, phase-5) carried forward unchanged; D-L3-1 stays deferred to the wave gate; no new deferred items.

[process-gap] Twice in a row, a pass-N fix to a version-citing document (CHANGELOG) was made stale by spec bumps in the same burst (L4-005, then L5-002). Release-notes / CHANGELOG version pins have no same-burst re-sync step. Cheapest structural fix: ban "current versions" pins in CHANGELOG entries (POLICY 19 style). Routing: orchestrator to codify.

Scope confirmations: the story claims no S-25.09+ ACs; the BinaryIntegrityFailure carrier for state_integrity is correct for S-25.08; the coordinator's run_bc_index_migration still uses !lock_path.exists() in shard_manager.rs, which is coordinator scope outside AC-031 and outcome-equivalent.
