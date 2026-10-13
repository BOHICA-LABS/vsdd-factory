---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 10 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-13T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 10
reviewed_head: "feature/S-25.08 @ 9e936d4c"
verdict: NOT CLEAN
finding_count: { total_numbered: 2, critical: 0, high: 0, medium: 0, low: 2, deferred: 0 }
streak_3_clean: "0/3"
novelty: "LOW"
traces_to: S-25.08
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back, session 2026-10-13; reviewed HEAD feature/S-25.08 @ 9e936d4c.

# S-25.08 local adversarial review, pass 10

## Identity and inputs
- **Identity tuple:** worktree `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08`, branch `feature/S-25.08`, reviewed HEAD `9e936d4c`, story S-25.08, canonical root `/Users/zious/Documents/GITHUB/vsdd-factory`.
- **Preflight passed:**
  - The tuple was present, and the worktree basename `S-25.08` matches the story ID.
  - `.git/worktrees/S-25.08/HEAD` points to `refs/heads/feature/S-25.08`. The last reflog entry is `1bdd86fc → 9e936d4c`.
  - The main checkout is on `develop`. I used it as the `ce2421be` baseline for comparing lines against develop.
  - All code was read from the worktree; all specs from canonical `.factory/`.
- **Spec versions:**
  - Story v1.10 (canonical).
  - BC-1.18.013 v1.15 and BC-1.18.011 v1.23, for the clauses the story assigns to S-25.08. I did not run `git show`; no finding depends on the uncommitted v1.16 / v1.24 text.
  - ADR-052: I read §5a "Factory-root lookup mapping" (line ~964), "Release-leg evaluation order" (~992) and "Reservation timestamp rules" (~1261, inside §5a, which spans lines 516–1715).
- **Limits:** I cannot run git or cargo. I checked SHA reachability and commit subjects against the worktree reflog `.git/worktrees/S-25.08/logs/HEAD`. For test, Kani, bats and pin-gate results I relied on the orchestrator's literal-shell gate evidence.

## Verdict: NOT CLEAN
- **Findings:** 0 CRITICAL, 0 HIGH, 0 MEDIUM, 2 LOW. One [process-gap].
- **Novelty: LOW.** Both findings are story-vs-delivery bookkeeping leftovers of known classes (L5-004 / L8-004 stale status; L8-005 / L9-004 evidence fidelity). I found no defect in code, CI, CHANGELOG, error paths or test strength.
- **Streak after this pass:** 0/3.

## Pass-9 fix verification
- **F-S2508-L9-001: CLOSED.**
  - `kani.yml` line 29 now reads "The ten OBL-1 harnesses (ADR-052 §5a re-baseline, S-25.08 AC-010)".
  - Lines 98–101 read "the ADR-052 §5a re-baseline (S-25.08 AC-010) adds h1/h4/h6 harnesses".
  - A grep for `v1\.` in `kani.yml` returns nothing. `EXPECTED_PROOFS=10` matches the 10 pinned harnesses, and the declared-count and vacuity guards are intact.
- **F-S2508-L9-002: CLOSED.**
  - The drain step-1 comment in `shard_manager.rs::drain_bc_index_writers` now ends "(ADR-052 §5a "Reservation timestamp rules"; defect B2-3)".
  - That heading exists inside ADR-052 §5a, so the anchor resolves.
  - The neighbouring "v1.9 H1" tokens are byte-identical to develop, so they are out of scope.
- **F-S2508-L9-003: CLOSED.**
  - Both `catch_unwind` arms in `test_BC_1_18_011_EC009_production_ttl_constant_floor_and_created_at_judgment` now read "production TTL validator panicked for {secs}s".
  - A case-insensitive search for stale status (`not yet|TODO|todo!(|unimplemented|planned|pending|currently|red gate|stub`) across `s2508_*.rs` and `bc_1_18_013_release_stat_error_test.rs` finds only past-tense history and JSON field names.
- **F-S2508-L9-004: CLOSED for subjects, story-wide.**
  - I extracted every `` `<sha>` "<subject>" `` pair in the story (49 pairs) plus the line-wrapped quotes (`c5e24633`, `1b28017e`, `b0f53cb3`, `24633c5f`, `2c753d7a` at line 788, `0bdcf4c6` in the pass-3 table) and compared each against the reflog.
  - Every subject is verbatim, including T-34 `ae74c039` ("Branches A/B/C…") and T-9 `81bbb897` (the full subject, ending with F-002/F-003/F-006/F-009).
  - The same sweep found one evidence-fidelity defect that subject matching cannot catch: unreachable SHAs (L10-001).

## Findings

### F-S2508-L10-001 — LOW (HIGH confidence) — T-1 and T-2 cite red-gate SHAs that are not reachable from `feature/S-25.08`
- **What is wrong:**
  - T-1 says "`git log --oneline 117dafdf..HEAD` = `c5e24633` … and `1b28017e` …".
  - T-2 says "same red-gate commits `1b28017e` + `c5e24633`".
- **Evidence:**
  - The reflog shows both are the pre-rebase SHAs, with parent chain `117dafdf → 1b28017e → c5e24633`.
  - The rebase onto `ce2421be` re-picked them as `5ee2fe87` ("test(S-25.08): red gate — shared admission core black-box suite, TTL/pure-core tests, B2 fixtures") and `09507051` ("test(S-25.08): align red gate with BC-1.18.013 v1.6 / BC-1.18.011 v1.13").
  - The branch history after the rebase is linear, and no ref in `packed-refs`, `refs/**` or `logs/refs/**` names either old SHA, so neither is an ancestor of `9e936d4c`. The range base `117dafdf` is the pre-rebase develop.
- **Sibling inconsistency:**
  - T-7 in the same story documents exactly this class for `0c64cb35` ("the PRE-REBASE SHA … unreachable from every branch") and substitutes the real SHA `24633c5f`.
  - T-3 and T-4 likewise say "post-rebase SHAs". T-1 and T-2 were not swept.
- **Failure scenario:** the Red Gate evidence for the story's foundational TDD tasks names commits that disappear when the reflog expires or the repo is garbage-collected. After the squash-merge, `git show 1b28017e` stops resolving, and an auditor cannot confirm that the red-first gate was satisfied. This is report fidelity (POLICY 22): the evidence has to be resolvable.
- **Where it lives:** the T-1 and T-2 **DONE** evidence lines. The frontmatter `last_amended` history entry that mentions them is historical and can stay.
- **Routing:** story-writer.
  - Cite `5ee2fe87` and `09507051`, and the range `ce2421be..HEAD`. Keep the old SHAs only with a "pre-rebase" label, as T-7 does.
  - Then sweep every cited SHA in the story for reachability, for example `git merge-base --is-ancestor <sha> feature/S-25.08`.

### F-S2508-L10-002 — LOW (HIGH confidence) — the "Convergence status (v1.8)" paragraph contradicts T-59 and the persisted review files
- **What the paragraph says** (§Adversarial Review, the bold tail):
  - "its review file is T-59 (OPEN, state-manager), and pass 10 is next, on `9e936d4c`".
  - "All eight review files `adv-local-pass-1.md` .. `adv-local-pass-8.md` are persisted".
- **What the evidence shows:**
  - T-59 is ticked: "[x] T-59 (NEW v1.10; DONE by the D-1258 state-manager burst: `code-delivery/S-25.08/adv-local-pass-9.md` persisted verbatim …)".
  - The file exists at canonical `.factory/code-delivery/S-25.08/adv-local-pass-9.md`; I read it as this pass's input. So nine review files are persisted, not eight, and T-59 is not OPEN.
  - T-58's ticked text also still reads "Pass 9 is NEXT, on `1bdd86fc`", which is present-tense status in a closed task.
- **Class:** this repeats L5-004 and L8-004: the convergence paragraph and the task list disagree about delivered state. The D-1257 rule cited in T-59 requires story status to match delivered work before each adversary pass.
- **Failure scenario:** a reader or orchestrator resuming from the story sees pass 9's review file listed as owed and may re-dispatch the persist burst, or conclude the pass-9 record is missing.
- **Routing:** story-writer.
  - Change the paragraph to "T-59 DONE (D-1258); nine review files `adv-local-pass-1.md` .. `-9.md` persisted".
  - Reword T-58's tail to past tense ("pass 9 then ran on `1bdd86fc`").

## [process-gap]
- **The story-reconciliation step checks commit subjects but not SHA reachability, and the convergence paragraph is not re-synced after the state-manager ticks its persist task.**
  - The v1.10 verbatim sweep (T-60) compared `git log -1 --format=%s <sha>` for each quote. That proves the subject but not that the SHA belongs to the branch (L10-001).
  - The story-writer writes the convergence paragraph before the state-manager burst ticks T-59, and nobody re-reads the paragraph afterwards (L10-002).
- **Proposed codification:**
  - The story-reconciliation gate runs `git merge-base --is-ancestor <sha> feature/<story>` for every cited SHA not labeled "pre-rebase", with literal shell output captured.
  - The state-manager's persist burst re-syncs the convergence paragraph whenever it ticks a review-persist task, or that paragraph is written only after the burst.
- **Routing:** orchestrator, then state-manager (codification).

## Fresh sweep: checked, no finding
- **Release leg** (`executor::bc_index_migration_reservation_release`) follows ADR-052 §5a "Release-leg evaluation order" steps 1–5 exactly:
  1. `is_tool_completion_event`.
  2. String `tool_use_id` plus `is_valid_tool_use_id`, silent, with no filesystem access.
  3. `resolve_factory_root`: `Ok(None)` is silent; `Err` gives one warn and no verdict.
  4. `probe_release_migration_state_dir`: ENOENT gives `Ok(false)` and stays silent; any other stat error gives one non-fatal warn.
  5. `release_bc_index_writer_reservation`, warning on error.
- **Admission** (`executor::bc_index_migration_admission`):
  - Order: input guards, then `resolve_factory_root`, then the lexical-alias union, then `resolve_target_path` and `classify_target`, then `classify_tool_use_id`, then `admit_protected_write`. This matches CHANGELOG line 29 and ADR §5a.
  - `resolve_factory_root` follows the three-way mapping exactly: a non-directory gives `None`; `NotFound` and `NotADirectory` are the closed Absent set; every other error gives `Err(Io { path: <root>/.factory })`.
  - `admission_error` puts only the `<cause>` token in the verdict. Its detail goes through `sanitize_diagnostic` with a 256-character cap.
- **`main.rs` wiring:**
  - Admission and release run immediately after the payload parse and `resolve_project_cwd`, before `CLAUDE_PLUGIN_ROOT` tiering, `resolve_registry_path` and `Registry::load`.
  - A verdict returns before the registry is loaded.
  - Release-on-block is driven by `code == 2` in `main` through `release_reservation_file`, which is idempotent on NotFound and warns otherwise.
  - The `run` → `Err` → exit 0 path leaves the reservation for PostToolUse to release (an admitted dispatch).
- **Drain** (`drain_bc_index_writers`):
  - The GC uses the single pure `reservation_is_stale`, which matches the documented rules: a `created_at` beyond now + 300 s is untrusted; the basis is `created_at`, falling back to mtime; neither available means retained; `age > ttl` is strict.
  - The poll step fails closed on any non-ENOENT `read_dir` error, replacing develop's `unwrap_or(true)`.
  - The GC's `let _ = remove_file` matches develop. A failed unlink keeps the reservation, which surfaces as `DrainTimeoutAbort` (fail-closed), so it is not a silent pass.
- **Tests:**
  - The `bc_1_18_013_release_stat_error_test.rs` probes cover ENOENT, present, ELOOP (root-safe) and EACCES (skipped when running as root). The module doc's observability statement is accurate.
  - The TTL test vectors are not tautological: each arm is reported separately and the reject/accept sets straddle 1,799/1,800.
- **CI:** `kani.yml` is covered above. I did not re-derive the ci.yml failpoints floors; pass 9 verified 39 and 1, and the orchestrator's run confirms 39/39 and 1/1.
- **CHANGELOG S-25.08 entry** (lines 21–33):
  - No version pins.
  - The release-order, unstatable-`.factory`, two-tier txn, TTL, and registry exit-code bullets match the code.
  - The "no operator-visible channel until S-25.09" qualifiers are accurate.
- **Version-pin policy:**
  - The orchestrator's two literal-shell sweeps cover the whole `ce2421be..HEAD` diff, which closes the pass-9 process-gap.
  - The `admission.rs` "pre-v1.11 code" token is BC provenance text, accepted in pass 9.
  - Test-function identifiers with version tokens remain the open human question [D-1257-POLICY19-SCOPE], not counted.

## Deferred (carried, unchanged)
- D-L4-1 → phase-5.
- D-L3-1 → wave gate.
- No new deferrals.

## Relevant paths
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/stories/S-25.08-shared-admission-core-b2-conformance-vp147-rebaseline.md` — T-1/T-2 (around lines 1141–1154), T-58/T-59 (around 1347–1348), "Convergence status (v1.8)" (around line 1659)
- `/Users/zious/Documents/GITHUB/vsdd-factory/.git/worktrees/S-25.08/logs/HEAD` — reflog evidence for the rebase re-picks `1b28017e → 5ee2fe87` and `c5e24633 → 09507051`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/.github/workflows/kani.yml`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/shard_manager.rs` (`drain_bc_index_writers`, `reservation_is_stale`)
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/executor.rs` (`bc_index_migration_admission`, `bc_index_migration_reservation_release`, `probe_release_migration_state_dir`)
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/shard_manager/admission.rs` (`resolve_factory_root`, `release_reservation_file`)
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/main.rs` (`run`, `main` release-on-block)
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/tests/s2508_pure_core_and_ttl_test.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/CHANGELOG.md`
