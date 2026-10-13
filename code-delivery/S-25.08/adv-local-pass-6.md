---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 6 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-13T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 6
reviewed_head: "feature/S-25.08 @ 169df502"
verdict: NOT CLEAN
finding_count: { total_numbered: 5, critical: 0, high: 0, medium: 1, low: 4, deferred: 0 }
streak_3_clean: "0/3"
novelty: "LOW-MEDIUM"
traces_to: S-25.08
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back, session 2026-10-11/13; reviewed HEAD feature/S-25.08 @ 169df502.

# S-25.08 LOCAL Adversary — Pass 6

Identity tuple: worktree `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08` (the basename matches S-25.08), reviewed HEAD `169df502`, story S-25.08, canonical root `/Users/zious/Documents/GITHUB/vsdd-factory`. I read code only from the worktree and specs only from canonical `.factory/`. Git is not runnable for me, so I read the code at the worktree tip.

Spec versions read: story v1.8, BC-1.18.013 v1.14 (frontmatter `version: "1.14"`), BC-1.18.011 (EC rows only; judged at the story-cited v1.22), and ADR-052 (the §5a release-leg ruling and the "Factory-root lookup mapping" table only, read by grep). I read only Part A of the prior review, `adv-local-pass-5.md`.

**Verdict: NOT CLEAN.** 0 CRITICAL, 0 HIGH, 1 MEDIUM, 4 LOW. Novelty: LOW-MEDIUM. **Streak after this pass: 0/3.** The production code in the admission core and the release leg is correct. The findings are a CHANGELOG mis-anchor, stale wording in the story and a test doc, one unruled evaluation-order question on the release leg, and partial CI positive coverage.

## Pass-5 fix verification

- **F-S2508-L5-001: closed, and the fix holds.**
  - The two tests are `test_BC_1_18_013_L5_001_release_stat_error_emits_exactly_one_warn` and `test_BC_1_18_013_L5_001_release_enoent_migration_state_emits_no_warn` in `tests/s2508_diagnostics_test.rs`. Both call the real `pub` `executor::bc_index_migration_reservation_release` under the `Capture` subscriber. The first runs for both PostToolUse and PostToolUseFailure, using a self-referential `migration-state` symlink (ELOOP). It requires exactly one WARN on target `bc_1_18_011_migration`, an `error_kind` that is not `NotFound`, and a `path` field containing `migration-state`.
  - Mutant analysis against the release function body:
    - (a) Reverting to `if !migration_state_dir.exists() { return; }`: ELOOP makes `exists()` false, the function returns silently, 0 WARNs, and the test fails.
    - (b) Changing the arm to `Err(_) => return`: 0 WARNs, and the test fails.
    - (c) Keeping the warn but dropping the `return`: `release_bc_index_writer_reservation` then fails with ELOOP and emits a second WARN, so the count is 2 and the test fails. The "exactly one" assertion is what catches this.
    - (d) Warning on ENOENT: the ENOENT control test fails.
  - The payload deserializes correctly (`hook_event_name` alias; `tool_use_id` lands in the `#[serde(flatten)] extra` map).
  - The AC-031 wording and Test: line are corrected.
- **F-S2508-L5-002: partly closed.** The version pins are gone. The S-25.09-only tokens (`txn_record_malformed`, `live_coordinator`) are gone from the S-25.08 entry (grep hits only in older release sections). The `exclusive.lock` sentence now correctly calls it a hygiene change. However, the §-anchor header of the same entry mis-cites two anchors (F-S2508-L6-001).
- **F-S2508-L5-003: closed.** In the failpoints test step of `.github/workflows/ci.yml`, the capture is now `out="$(… 2>&1)" || { printf '%s\n' "$out"; echo "::error::…"; exit 1; }`. An assignment on the left of `||` does not trigger errexit, so on failure the output and the annotation are printed before `exit 1`. The `MIN_TESTS=39` floor still equals the 39 `#[test]` functions in `bc_1_18_011_b2_migration_crash_injection_test.rs` (40 matches = 39 `#[test]` + 1 `#![cfg]`).
- **F-S2508-L5-004: closed for the two named items.** The "Local pass 4" L4-001 row and the "Convergence status (v1.7)" paragraph now agree with the frontmatter, T-42 and T-43. Sibling statements of the same stale class remain elsewhere in the story (F-S2508-L6-002).

## Findings

**F-S2508-L6-001 — MEDIUM (HIGH confidence) — the S-25.08 CHANGELOG entry mis-anchors two citations in the §-anchor header that the L5-002 fix produced.**
- Anchor: `CHANGELOG.md` `[Unreleased] > Fixed`, the lead bullet of the S-25.08 entry (its parenthetical contract list).
- Citation 1: the lead bullet cites "BC-1.18.013 AC-011 and EC-021". BCs have no ACs: a grep for `AC-011` in canonical `BC-1.18.013.md` returns 0 matches. AC-011 is a story AC.
- Citation 2: it cites "BC-1.18.011 §Edge Cases (EC-039, EC-045)".
  - BC-1.18.011 EC-039 is correct: the known minimal STAGING record without `generation_id`, which mirrors BC-1.18.013 EC-045.
  - BC-1.18.011 EC-045 is the Precondition 6(f)(iii) strict-presence-key vector, one fixture per missing key of the 12-key set. The story's own BC table assigns that to S-25.09: "Precondition 6(f) `migrate-bc-index` recovery leg … EC-043..EC-050 — is S-25.09 AC-008..AC-011, NOT this story".
  - The "Two-tier txn-record interpretation" bullet in the same entry says "(EC-045; txn bytes unchanged)". It means BC-1.18.013 EC-045, the known minimal STAGING case. So the entry contradicts itself about which EC-045 it means.
- Failure scenario: a reader or release-note consumer who follows the header lands on a BC-1.18.011 edge case that S-25.08 does not deliver (S-25.09 scope), and looks for an AC that does not exist in BC-1.18.013. This is the same "CHANGELOG cites S-25.09 scope" class that L5-002 was meant to close.
- Mis-anchoring blocks convergence (POLICY 4).
- Fix: cite "BC-1.18.013 EC-021, EC-045" and "BC-1.18.011 EC-039", or qualify every EC with its BC, and drop the AC token or label it "S-25.08 AC-011 / AC-031".
- Routing: implementer.

**F-S2508-L6-002 — LOW (HIGH confidence) — the F-S2508-L5-004 sweep missed sibling stale-status statements in the story.**
- Anchor: S-25.08 story v1.8, the AC-031 `Test:` line and the T-38 body.
- The AC-031 Test: line still says the fix `675fcf77` is "observed on the local `feature/S-25.08` branch (unreviewed by an adversary; see T-37 and the pass-4 section)". Pass 5 reviewed it and recorded "L4-001 release leg — code fixed".
- T-38 still says "The CHANGELOG obligation for AC-030 / AC-031 … is OPEN: T-37 is ticked but the CHANGELOG bullet is not yet written — T-43". T-43 has been DONE since `a7fc5930`.
- Failure scenario: an implementer reading AC-031 or T-38 concludes that work is still owed and re-opens or duplicates it. This is the same contradiction class as L5-004 (TD-VSDD-060 sibling sweep).
- Fix: update both statements to the current state.
- Routing: story-writer.

**F-S2508-L6-003 — LOW (HIGH confidence) — a test module doc makes a present-tense claim about production code that is false at HEAD.**
- Anchor: module doc of `crates/factory-dispatcher/tests/bc_1_18_013_release_stat_error_test.rs`. `fb2479d0` edited this doc (+10/-5) and left the claim in place.
- The doc says "`bc_index_migration_reservation_release` currently guards with `if !migration_state_dir.exists() { return; }`". It also still frames the helper as one "the implementer must introduce", with "RED at authoring time = compile failure".
- In fact, `executor::bc_index_migration_reservation_release` now branches on `probe_release_migration_state_dir`, and the helper exists.
- Failure scenario: a maintainer reading the test believes the release leg still collapses stat errors. That inverts what the file pins.
- Fix: rewrite in past tense ("the pre-fix code guarded with …; fixed by `probe_release_migration_state_dir`").
- Routing: test-writer.

**F-S2508-L6-004 — LOW (MEDIUM confidence; pending intent verification) — the release leg's evaluation order puts the stat probes before the `tool_use_id` grammar check, so an invalid or absent id is not a "silent no-op" when a stat fails.**
- Anchor: `executor::bc_index_migration_reservation_release`. The order is: `resolve_factory_root` (warns on an unstatable `.factory`), then `probe_release_migration_state_dir` (warns on a non-ENOENT error), then the `tool_use_id` lookup. The grammar check happens even later, inside `release_bc_index_writer_reservation`'s `is_valid_tool_use_id`.
- Spec text:
  - Story AC-011, BC-1.18.013 EC-021 and Precondition 6(c), and ADR-052 §5a (the release ruling) all say an id that "fails the grammar … is a silent no-op on release".
  - The ADR-052 §5a "Factory-root lookup mapping" table says an unstatable `.factory` on release gives one advisory, without qualifying the id.
  - So the spec does not say which rule wins on the release leg. For the admission leg, the analogous ordering question was ruled explicitly (L4-002, ADR §Error Code Semantics, AC-030).
- Failure scenario: a PostToolUse or PostToolUseFailure envelope with `tool_use_id: "../x"`, or with no id, under a self-looping `migration-state` emits a WARN. The contract says such an id is silent.
  - In S-25.08 this is a `tracing` event only (no subscriber is installed), so the effect is limited to diagnostics.
  - In S-25.09 the same path becomes a `migration.admission_advisory` InternalLog event per completion event.
  - No test covers an invalid or absent id combined with a stat error. The L5-001 tests use the valid id `toolu_l5_001`.
- Fix: either get an architect ruling for the release-leg order (mirroring the AC-030 admission ruling), or move the id presence and grammar check ahead of both stat probes. Then add an invalid-id + ELOOP zero-WARN vector.
- Routing: architect (ruling), then implementer and test-writer.

**F-S2508-L6-005 — LOW (HIGH confidence) — the failpoints CI step asserts positive coverage for only one of the two failpoints-gated targets it says it covers.**
- Anchor: `.github/workflows/ci.yml`, cargo-host step "cargo test (factory-dispatcher, failpoints feature)".
- The step comment names both `bc_1_18_011_b2_migration_crash_injection_test` and `bc_1_18_011_b2_migration_obl1_failpoint_smoke_test`. The `MIN_TESTS` re-run checks only the crash-injection target.
- The smoke target is gated by `#![cfg(feature = "failpoints")]` plus a `#[cfg(feature = "failpoints")]` single test, so it can compile to zero tests and exit 0.
- Failure scenario: someone deletes, renames or re-gates the smoke target (for example, a different feature name on that file only). CI stays green with 0 smoke tests run. This is a partial POLICY-11 / POL-11 positive-coverage gap.
- Fix: add a second re-run with `--test bc_1_18_011_b2_migration_obl1_failpoint_smoke_test` and a floor of 1, or sum the per-binary `test result` counts against a combined floor of 40.
- Routing: devops-engineer.

## Deferred

- D-L4-1 (architectural, phase-5) carried forward unchanged.
- D-L3-1 stays deferred to the wave gate.
- No new deferred items.

## Observations (non-blocking)

- The story's "Local pass 5" table labels F-S2508-L5-002 as "LOW [process-gap] documentation". The review file lists the [process-gap] as a separate item and L5-002 as plain LOW. T-47 (the state-manager reconciliation) is already owed and should align this.
- The F-S2508-L5-002 [process-gap] (no version pins in CHANGELOG) worked for versions, but the §-anchor replacement introduced unqualified or mis-qualified EC/AC tokens (L6-001). A codified rule should require every EC/AC token in a CHANGELOG to be BC-qualified and to resolve to a row in that BC.
- The production code is sound:
  - `probe_release_migration_state_dir` maps only ENOENT to `Ok(false)`.
  - The `exclusive.lock` sibling in `reconcile_stale_admission_gate` uses `try_exists()?` and maps errors to `Io`.
  - The CHANGELOG's "E-MAINTENANCE-002 evaluation order" sentence matches `bc_index_migration_admission`: input guards, then `resolve_factory_root`, then `resolve_target_path`/`classify_target`, then `classify_tool_use_id`.
  - No subscriber is installed in production `src/`, so the "no operator-visible channel" claim is accurate.
  - Kani `EXPECTED_PROOFS=10` matches the VP-147 bullet.
- No volatile line pins found in the reviewed story or test text (TD-VSDD-091).

## Files relevant to this pass

- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/CHANGELOG.md`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/executor.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/tests/s2508_diagnostics_test.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/tests/bc_1_18_013_release_stat_error_test.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/.github/workflows/ci.yml`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/stories/S-25.08-shared-admission-core-b2-conformance-vp147-rebaseline.md`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/specs/behavioral-contracts/ss-01/BC-1.18.013.md`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md`
