---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 7 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-13T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 7
reviewed_head: "feature/S-25.08 @ 4adeaee6"
verdict: NOT CLEAN
finding_count: { total_numbered: 3, critical: 0, high: 0, medium: 1, low: 2, deferred: 0 }
streak_3_clean: "0/3"
novelty: "LOW-MEDIUM"
traces_to: S-25.08
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back, session 2026-10-11/13; reviewed HEAD feature/S-25.08 @ 4adeaee6.

# S-25.08 Local Adversarial Review, Pass 7 (Step 4.5, BC-5.39.001 3-CLEAN)

## Identity and inputs
- **Identity tuple:** the tuple was present. Worktree `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08`. Its basename `S-25.08` matches the story-id. Branch `feature/S-25.08`, reviewed HEAD `4adeaee6`, canonical root `/Users/zious/Documents/GITHUB/vsdd-factory`. All code was read from the worktree and all specs from canonical `.factory/`.
- **Spec versions read:**
  - Story S-25.08 v1.9
  - BC-1.18.013 v1.15
  - BC-1.18.011 v1.23
  - ADR-052 v1.27: §5a "Factory-root lookup mapping" release row and "Release-leg evaluation order"

## Verdict: NOT CLEAN
- **Counts:** 0 CRITICAL, 0 HIGH, 1 MEDIUM, 2 LOW.
- **Novelty:** LOW to MEDIUM. All three findings are about documentation and record-keeping, none about behavior. Two of them are leftovers of pass-6 fixes that were not swept across sibling files (L6-003, L6-002), not new defect classes. Production code matches the spec.
- **Streak after this pass:** 0/3.

## Pass-6 fix verification
- **L6-001: CLOSED.** The CHANGELOG `[Unreleased] > Fixed` S-25.08 lead bullet now attaches every EC/AC to its owner. Each one resolves to a real S-25.08-scope row:
  - BC-1.18.011 EC-039: KNOWN minimal STAGING without `generation_id`.
  - BC-1.18.013 EC-021: release on `PostToolUseFailure`, plus the v1.15 order clause.
  - BC-1.18.013 EC-045: KNOWN minimal STAGING without `generation_id`, the `state_integrity` case.
  - S-25.08 AC-011 / AC-031.
  
  No unqualified `AC-NNN` and no S-25.09-scope EC token remains. The new release-order bullet cites `ADR-052 §5a "Release-leg evaluation order"` and BC-1.18.013 EC-021, which carries the v1.15 order text.
- **L6-002: CLOSED for the two cited lines.** AC-031's Test: line now says "reviewed by local adversary pass 5". T-38 records that the CHANGELOG obligation was done by T-43. The same class of problem has come back for the pass-6 fixes themselves; see F-S2508-L7-003.
- **L6-003: CLOSED for the cited file only.** The module doc of `tests/bc_1_18_013_release_stat_error_test.rs` now describes the `exists()` guard in the past tense. The sibling sweep was not done; see F-S2508-L7-001 and F-S2508-L7-002.
- **L6-004: CLOSED, and the tests are load-bearing.**
  - `executor::bc_index_migration_reservation_release` now runs in the ADR order: `is_tool_completion_event`, then `extra["tool_use_id"]` with `as_str` and `is_valid_tool_use_id` (return on failure), then `resolve_factory_root`, then `probe_release_migration_state_dir`, then `release_bc_index_writer_reservation`.
  - Mutation reasoning for the four `test_BC_1_18_013_L6_004_*` tests in `tests/s2508_diagnostics_test.rs`:
    - If the id check moves after the probe, vectors (1)–(3) fail: an ELOOP `migration-state` produces the probe WARN.
    - If it moves between `resolve_factory_root` and the probe, vector (4) fails: an ELOOP `.factory` produces the "unstatable" WARN.
  - The zero-WARN proxy is not vacuous. `with_default` builds a new `Dispatch`, which rebuilds callsite interest. The valid-id + ELOOP control `test_BC_1_18_013_L5_001_release_stat_error_emits_exactly_one_warn` now has to pass the moved id check, which also proves `tool_use_id` is still read from the flattened `extra`.
- **L6-005: CLOSED, and the floor really fails on 0.**
  - In `ci.yml` (`cargo-host`, ubuntu and macos), `assert_floor` re-runs each target and parses `^test result: ok\. N passed` (`set -euo pipefail`; no color env, so no ANSI codes).
  - It fails on an empty or short count, so "0 passed" is less than the floor. A cargo error, such as a renamed target, fails the `||` branch and prints the captured output.
  - Floors are 39 for `bc_1_18_011_b2_migration_crash_injection_test` and 1 for `bc_1_18_011_b2_migration_obl1_failpoint_smoke_test`. Both match the actual `#[test]` counts (39 and 1). Neither target has `#[ignore]` tests.
  - These are the only two `#![cfg(feature = "failpoints")]` test targets on the branch.

## Findings

**F-S2508-L7-001 — MEDIUM (HIGH confidence): the L6-003 fix was applied to one test module doc, but four sibling S-25.08 test-module docs still describe the red-gate state in the present tense, which contradicts the code.**
- **Severity note:** the partial-fix rubric would put a 2+ file blast radius at HIGH. I rated it MEDIUM because it is documentation only and the original L6-003 was LOW. Orchestrator to adjudicate.
- **Anchors (each checked against the source):**
  - `tests/s2508_diagnostics_test.rs` module doc "# Stub surface (BC-5.38.001)": says `as_given_factory_root_spelling` "is added … as a `todo!()` stub: the merged `executor.rs` inlines the `CLAUDE_PROJECT_DIR` alias derivation (filtering only the EMPTY case)". In reality `shard_manager::admission::as_given_factory_root_spelling` is implemented (it rejects empty and non-absolute input), and `executor::bc_index_migration_admission` calls it. The same doc's scope list names only the pass-2 findings L2-008/L2-007/L2-010. It omits the L5-001 and L6-004 sections, even though this file was edited by `faa9d9ef` in the pass-6 fix set.
  - `tests/s2508_registry_exit_code_test.rs` module doc: "Merged `main.rs::run` maps a registry load error with an inline `match` that ends in `_ => 0`" and "test-writer added it as a `todo!()` stub". In reality `main.rs::run` calls `factory_dispatcher::registry::registry_error_exit_code(&e)`.
  - `tests/s2508_pure_core_and_ttl_test.rs` module doc: "All four symbols below were added … as minimal compilable stubs with `todo!()` bodies". All four are implemented.
  - `tests/s2508_admission_blackbox_test.rs` module doc "# TEST-ONLY SEAM": "No production code reads it yet (so these tests are RED for the right reason: the seam and the core behavior do not exist)". In reality `shard_manager::admission` has `mod seam`, which reads `VSDD_TEST_ADMISSION_SEAM_DIR`.
- **Failure scenario:** a maintainer or a later reviewer trusts the module doc. They may conclude that the `CLAUDE_PROJECT_DIR` not-absolute rule is unimplemented, or that registry errors still fall through `_ => 0`. That leads to duplicate work or a false "untested / unwired" finding. This is the defect class L6-003 was raised to remove, and the fix did not sweep sibling files (TD-VSDD-060).
- **Routing:** test-writer. Rewrite the four module docs in the past tense with the current state. Add the L5-001 and L6-004 sections to the scope list in `s2508_diagnostics_test.rs`.

**F-S2508-L7-002 — LOW (HIGH confidence): the module doc of `tests/bc_1_18_013_release_stat_error_test.rs` names an event that does not exist (mis-anchor, POLICY 4).**
- **Anchor:** the OBSERVABILITY paragraph says "S-25.09 adds the `migration.release_failed` advisory" and refers to `reservation_release`.
  - Grep found `migration.release_failed` nowhere in canonical `.factory/specs/`; this file is its only occurrence in the worktree.
  - The canonical S-25.09 release-side event is `migration.admission_advisory` with `reason=reservation_release_failed`. Sources: the ADR-052 §5a "Factory-root lookup mapping" release row and BC-1.18.013 EC-042.
  - The function is `bc_index_migration_reservation_release`, not `reservation_release`.
- **Failure scenario:** someone greps for the named event to find the S-25.09 channel or its test and finds nothing. Or they write an assertion against the wrong event name once S-25.09 lands. This paragraph sits in the file the L6-003 fix (`fbbb0d91`) edited and was not corrected.
- **Routing:** test-writer.

**F-S2508-L7-003 — LOW (HIGH confidence): story v1.9 is out of date with the pass-6 fix delivery. This is the same class as L6-002, recurring for a second consecutive pass.**
- **Anchors in the canonical story:**
  - T-49 is still `- [ ]` with "(NEW v1.9; OPEN)".
  - AC-031's "Valid-id-only qualification" paragraph ends "Delivery: T-49 (OPEN)", although the red `faa9d9ef`, the fix `f34e349c` and the CHANGELOG bullet T-49 required (in `016a192f`) are all on the branch.
  - The AC-011 and AC-031 `- **Test:**` lines and the file-change table row for `tests/s2508_diagnostics_test.rs` list no `test_BC_1_18_013_L6_004_*` test.
  - No task records the L6-003 (`fbbb0d91`) or L6-005 (`4adeaee6`) deliveries.
  - Grep of the story for `L6_004|f34e349c|faa9d9ef|fbbb0d91|016a192f|4adeaee6` returns 0 matches.
- **Failure scenario:** the PR or a wave-gate reviewer reads the story as the delivery record. They conclude that AC-031's required vectors (1)–(5) are undelivered, or they cannot trace the four new tests to an AC (traceability gap).
- **[process-gap] candidate:** two consecutive passes have found story task and status lines not reconciled after implementer commits land. The story-writer has no mandatory post-fix "tick and trace" step tied to fix commits. One more recurrence meets the 3+ codification threshold.
- **Routing:** story-writer, to tick T-49 with commit evidence, update the AC-011/AC-031 Test lines and the file table, and add task records for L6-003 and L6-005. Route the process-gap to the orchestrator.

## Observations (not blocking)
- **Version-pinned ADR citations in S-25.08 code and test comments (outside POLICY 19's scope).** These are the same decaying-pin class T-45 removed from the CHANGELOG:
  - `executor::bc_index_migration_reservation_release` cites "ADR-052 v1.20 §5a F-001" and "ADR-052 v1.23 §5a".
  - `main.rs::run` and the release-on-block comment cite "ADR-052 v1.18/v1.20 §5a".
  - The new L6-004 section header in `s2508_diagnostics_test.rs` cites "ADR-052 v1.27 5a".
  
  The ADR is now v1.27, so the older pins are already stale. Recommend §-anchor-only citations.
- **No valid-id control inside the L6-004 tests.** They rely on `test_BC_1_18_013_L5_001_release_stat_error_emits_exactly_one_warn` in the same binary. This is acceptable and stated in the section comment; an in-function control would make each test self-proving.
- **No black-box vector for the release order.** It is pinned only in-process. This is consistent with T-49's "in-process" instruction, and the black-box ELOOP release test still pins exit 0 and that nothing is deleted.

## Deferred (carried, unchanged)
- **D-L4-1:** architectural, routed to phase-5. A protected subtree that is itself a symlink out of the root is not gated through its resolved location.
- **D-L3-1:** routed to the wave gate.
- No new deferrals this pass.

## Key paths
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/executor.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/tests/s2508_diagnostics_test.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/tests/bc_1_18_013_release_stat_error_test.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/tests/s2508_registry_exit_code_test.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/tests/s2508_pure_core_and_ttl_test.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/tests/s2508_admission_blackbox_test.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/.github/workflows/ci.yml`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/CHANGELOG.md`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/stories/S-25.08-shared-admission-core-b2-conformance-vp147-rebaseline.md`
