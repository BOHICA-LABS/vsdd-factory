---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 11 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-13T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 11
reviewed_head: "feature/S-25.08 @ 9e936d4c"
verdict: NOT CLEAN
finding_count: { total_numbered: 1, critical: 0, high: 0, medium: 0, low: 1, deferred: 0 }
streak_3_clean: "0/3"
novelty: "LOW"
traces_to: S-25.08
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back, session 2026-10-13; reviewed HEAD feature/S-25.08 @ 9e936d4c.

# S-25.08 local adversarial review, pass 11

## Identity and inputs
- **Identity tuple:** worktree `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08`, branch `feature/S-25.08`, reviewed HEAD `9e936d4c`, story S-25.08, canonical root `/Users/zious/Documents/GITHUB/vsdd-factory`.
- **Preflight passed:**
  - The tuple was present, and the worktree basename `S-25.08` matches the story ID.
  - The worktree reflog `.git/worktrees/S-25.08/logs/HEAD` ends at `1bdd86fc → 9e936d4c`, so there is no code change since pass 10.
  - All code was read from the worktree; all specs from canonical `.factory/`.
- **Spec versions:**
  - Story v1.11.
  - BC-1.18.013 v1.16 and BC-1.18.011 v1.24, judged only on the clauses the story assigns to S-25.08: the release-leg evaluation order, the admission core, B2 conformance and the VP-147 re-baseline.
  - ADR-052 v1.28: I read §5a "Factory-root lookup mapping", §5a "Release-leg evaluation order", and §Error Code Semantics "Factory-root stat failure".
- **Read only one prior review:** `.factory/code-delivery/S-25.08/adv-local-pass-10.md`.
- **Limits:** I cannot run git or cargo. I checked SHA reachability against the reflogs of both worktrees (`.git/worktrees/S-25.08/logs/HEAD` and `.git/worktrees/S-25.09/logs/HEAD`). For the test, Kani, bats and pin-sweep results I relied on the orchestrator's literal-shell gate evidence.

## Verdict: NOT CLEAN
- **Findings:** 0 CRITICAL, 0 HIGH, 0 MEDIUM, 1 LOW.
- **Novelty: LOW.** The one finding is a provenance-label error that the pass-10 fix itself introduced, in the same evidence-fidelity class as L10-001. I found no defect in the code, CI, CHANGELOG, error paths or test strength.
- **Streak after this pass:** 0/3.

## Pass-10 fix verification
- **F-S2508-L10-001: CLOSED for reachability; one labelling defect remains (L11-001).**
  - T-1 (story line ~1142) and T-2 (~1154) now cite `5ee2fe87` and `09507051` with the range `ce2421be..HEAD`. The old SHAs `c5e24633` and `1b28017e` are labelled pre-rebase.
  - Reflog lines 21–22 confirm both new SHAs are `rebase (pick)` re-picks with identical subjects, inside the branch's linear post-rebase history.
  - I listed every backtick SHA in the story and checked it against the S-25.08 reflog. These SHAs are unreachable:
    - the pre-rebase SHAs (reflog lines 3–19);
    - the amended-away SHAs `2ada9e04` and `b783d2d2`;
    - the commits removed by the reset to `5091f88f` (reflog line 76).
  - Outside the historical `last_amended` text, none of them appears unlabelled. `0c64cb35` appears only in T-7's explicit pre-rebase explanation.
  - The S-25.09 SHAs (`5fe9bc0f`, `c0a2c73e`, `0813826a`, `a33c5b41`, `1d9f69be`, `5fe27f59`, `31204a3d`, `f5e8bd3e`, `c632c29c`, `390c852b`, `be8bfa9a`) all appear in the S-25.09 worktree reflog after its last `rebase (finish)` (line 225), so they are on `feature/S-25.09` (tip `788e05a2`).
- **F-S2508-L10-002: CLOSED.**
  - The convergence paragraph (line 1660) now says pass 10 was NOT CLEAN with 0 MEDIUM + 2 LOW, that T-61 is DONE (D-1259), that ten review files `adv-local-pass-1.md` .. `-10.md` are persisted, and that pass 11 is next on `9e936d4c`.
  - A glob of canonical `code-delivery/S-25.08/` returns exactly those ten files.
  - T-58's tail is now past tense ("Pass 9 then ran on `1bdd86fc`…"), and T-59 and T-61 are ticked and match the persisted files.

## Findings

### F-S2508-L11-001 — LOW (HIGH confidence) — three S-25.09-native commits are labelled "pre-split", but they were created after the split
- **Where:**
  - T-31 (line 1319): "`a33c5b41` (pre-split `37df1ebd`)".
  - T-34 (line 1322): "S-25.09 `1d9f69be` (pre-split `da52d067`; …) / `5fe27f59` (pre-split `6c0fb5a4`; …)".
  - The v1.11 `last_amended` item (1) lists `a33c5b41`, `1d9f69be` and `5fe27f59` among "the S-25.09-moved SHAs … labelled 'pre-split'".
- **Evidence:**
  - The split is the `reset: moving to 5091f88f` at S-25.08 reflog line 76 (timestamp 1791398191). The moved commits are `dbca5754` .. `39e89c59` (lines 65–75).
  - `37df1ebd` is a fresh `commit: docs(S-25.09): CHANGELOG unstatable-release advisory…` at S-25.09 reflog line 57 (timestamp 1791415757).
  - `da52d067` and `6c0fb5a4` are `rebase (pick)` SHAs of `test(S-25.09): …` commits at S-25.09 reflog lines 149–150 (timestamp 1791487015). Their originals are `ef9a2ec8` and `d88e2f55` at lines 95–96.
  - None of these commits was ever on `feature/S-25.08` or in the split range. They are pre-rebase SHAs of commits authored on S-25.09 after the split.
  - The other "pre-split" labels (`e7a490cd`, `07fccafd`, `0e90e101`, `67fc6437`, `41acb16a`, `f8726c34`, `39e89c59`, `dbca5754`) are correct.
- **Failure scenario:** an auditor following the label looks for `37df1ebd`, `da52d067` or `6c0fb5a4` in the S-25.08 split range `5091f88f..39e89c59` and does not find them. The point of L10-001's fix was that each label states the SHA's true provenance; these three misstate it. Reachability is not affected, because the reachable S-25.09 SHA is given next to each one. That is why this is LOW.
- **Routing:** story-writer.
  - Relabel the three as "pre-rebase, on `feature/S-25.09`" in T-31 and T-34.
  - Correct the v1.11 `last_amended` item (1) classification in a new version entry; the history itself stays as written.

## [process-gap]
- **The reachability sweep checks only whether a label exists, not whether it is true.**
  - The pass-10 codification asks for `git merge-base --is-ancestor` on every cited SHA that is "not labelled". A wrong label therefore passes the gate.
  - **Proposed:** for every SHA labelled pre-split, require it to be in `git rev-list 5091f88f..39e89c59` (from the reflog). For every SHA labelled pre-rebase, require a later `rebase (pick)` reflog entry with the same subject. Capture the literal shell output.
  - **Routing:** orchestrator, then state-manager (codification).

## Fresh sweep: checked, no finding
- **Release leg** (`executor::bc_index_migration_reservation_release`, executor.rs ~616–685) matches ADR-052 §5a "Release-leg evaluation order" steps 1–5 exactly:
  1. `is_tool_completion_event`.
  2. String id plus `is_valid_tool_use_id`, silent, with no filesystem access.
  3. `resolve_factory_root`: `Ok(None)` is silent; `Err` gives one warn and no verdict.
  4. `probe_release_migration_state_dir`: only ENOENT gives `Ok(false)`; any other stat error gives one warn and is never treated as absent.
  5. `release_bc_index_writer_reservation`, warning on error.
- **Admission** (`bc_index_migration_admission`):
  - Order: input guards, then `resolve_factory_root` (an `Err` gives `admission_error`, i.e. `E-MAINTENANCE-002 (io)`), then the `CLAUDE_PROJECT_DIR` lexical alias, then `resolve_target_path` and `classify_target`, then `classify_tool_use_id`, then `admit_protected_write`. This is the deliberate L4-002 admission order the ADR keeps.
  - `admission_error` puts only the `<cause>` token in the verdict. The detail goes through `sanitize_diagnostic` with a 256-character cap.
- **`resolve_factory_root`** (admission.rs ~408–438): one `metadata`. A non-directory gives `None`; `NotFound` and `NotADirectory` are the closed Absent set; every other error gives `Err(Io { path: <root>/.factory })`. This matches the ADR's Factory-root lookup mapping and the §Error Code Semantics "Factory-root stat failure" ruling.
- **`main.rs` wiring:**
  - Admission and release run immediately after the payload parse and `resolve_project_cwd`, before `CLAUDE_PLUGIN_ROOT` tiering, `resolve_registry_path` and `Registry::load`.
  - A verdict returns before the registry is loaded.
  - Release-on-block is driven by `code == 2` through `release_reservation_file`, which is idempotent on NotFound and warns otherwise.
  - The `run` → `Err` → exit 0 path leaves the reservation for PostToolUse, which is correct because the tool is admitted.
  - Registry exit codes go through `registry_error_exit_code` with an explicit variant list.
- **Tests:**
  - The four `test_BC_1_18_013_L6_004_*` tests in `s2508_diagnostics_test.rs` cover vectors (1)–(4), on both completion events.
  - Each test runs `assert_release_valid_id_control` under the same ELOOP fixture and requires exactly one WARN, so the zero-WARN assertions cannot pass vacuously.
  - The ELOOP fixtures behave the same when running as root.
- **CHANGELOG S-25.08 entry** (lines 21–33):
  - No version pins.
  - The §-anchors resolve: §5a, §7e and §Error Code Semantics, including "Factory-root stat failure" for the unstatable-`.factory` bullet.
  - The release-order, release-leg stat, two-tier txn, TTL and registry exit-code bullets match the code.
  - The "no operator-visible channel until S-25.09" qualifiers are accurate.
- **Version-pin policy:**
  - The orchestrator's literal-shell sweeps at `9e936d4c` cover the whole diff.
  - The `admission.rs` "pre-v1.11 code" provenance line is the accepted exception.
  - Test-function identifiers remain the open human question [D-1257-POLICY19-SCOPE]; I did not count them.

## Observations (non-blocking)
- In the convergence paragraph (line 1660), the two `**` emphasis spans pair wrongly: the bold opened at "Pass 8" closes at "Pass 10", and the final `**` after "`9e936d4c`." opens a span that never closes. Its heading still reads "Convergence status (v1.8)" although the content now runs through v1.11. Both are cosmetic; the content is accurate. The story-writer can fix them with L11-001.

## Deferred (carried, unchanged)
- D-L4-1 → phase-5 (architectural).
- D-L3-1 → wave gate.
- No new deferrals.

## Relevant paths
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/stories/S-25.08-shared-admission-core-b2-conformance-vp147-rebaseline.md`: T-1/T-2 (~1141–1154), T-31 (1319), T-34 (1322), T-58/T-59/T-61 (1347–1349), the convergence paragraph (1660), frontmatter `last_amended` v1.11 item (1).
- `/Users/zious/Documents/GITHUB/vsdd-factory/.git/worktrees/S-25.08/logs/HEAD`: rebase re-picks (lines 21–38), the split reset (line 76).
- `/Users/zious/Documents/GITHUB/vsdd-factory/.git/worktrees/S-25.09/logs/HEAD`: `37df1ebd` (line 57), `da52d067`/`6c0fb5a4` (149–150), the last rebase finish (225).
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/executor.rs`: `bc_index_migration_admission`, `bc_index_migration_reservation_release`, `probe_release_migration_state_dir`.
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/shard_manager/admission.rs`: `resolve_factory_root`, `release_reservation_file`.
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/main.rs`: `run`, `main` release-on-block.
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/tests/s2508_diagnostics_test.rs`: the L6_004 tests.
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/CHANGELOG.md`: lines 21–33.
