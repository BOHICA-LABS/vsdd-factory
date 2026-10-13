---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 9 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-13T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 9
reviewed_head: "feature/S-25.08 @ 1bdd86fc"
verdict: NOT CLEAN
finding_count: { total_numbered: 4, critical: 0, high: 0, medium: 1, low: 3, deferred: 0 }
streak_3_clean: "0/3"
novelty: "LOW to moderate"
traces_to: S-25.08
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back, session 2026-10-13; reviewed HEAD feature/S-25.08 @ 1bdd86fc.

# S-25.08 local adversarial review, pass 9

## Identity and inputs
- **Identity tuple:**
  - worktree `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08`
  - branch `feature/S-25.08`, reviewed HEAD `1bdd86fc`
  - story S-25.08
  - canonical root `/Users/zious/Documents/GITHUB/vsdd-factory`
- **Preflight passed.** The tuple was present. The worktree basename `S-25.08` matches the story ID. All code was read from the worktree and all specs from the canonical `.factory/`.
- **Spec versions:** story v1.10 (canonical working tree), BC-1.18.013 v1.15, BC-1.18.011 v1.23, ADR-052 v1.27 (I searched it and read §5a "Factory-root lookup mapping" and "Release-leg evaluation order"), VP-147 v1.5.
- **SHA chain.** I read the worktree reflog at `.git/worktrees/S-25.08/logs/HEAD`. The commits run in order `2d1fb0c0` → `ee9b2741` → `17941cc4` → `1bdd86fc`.
- **Baseline.** The main checkout's `develop` ref is `ce2421be` (`.git/refs/heads/develop`). I used the main checkout as the develop baseline for line comparisons. I assumed it has no uncommitted changes; I can't check that without git.
- **What I could not do:**
  - Run `git diff` or `cargo test`. Whether the suite is green at `1bdd86fc` has still not been independently checked; T-49 hands that to a verifier.
  - Prove that the three pass-8 commits change only comments or assertion text. I checked this indirectly (see "Runtime-behavior check").

## Verdict: NOT CLEAN
- **Findings:** 0 CRITICAL, 0 HIGH, 1 MEDIUM, 3 LOW. One [process-gap].
- **Novelty:** LOW to moderate. All four findings are leftovers of the pass-8 classes (a mis-anchor, version pins, stale red-gate wording, quoted commit subjects) at sites the pass-8 sweeps did not reach. I found no behavioral, error-path or silent-failure defect in the delivered code.
- **Streak after this pass:** 0/3.

## Pass-8 fix verification
- **F-S2508-L8-001: closed in comments, one sibling left in assertion messages (F-S2508-L9-003).**
  - The black-box L4_001 doc now points to the `test_BC_1_18_013_L5_001_*` tests in `s2508_diagnostics_test.rs`.
  - The `s2508_registry_exit_code_test.rs` header and the `run_binary` section comment state the delivered state.
  - A case-insensitive search of every S-25.08 test file for `today|red[- ]gate|not observable|new function|not yet|TODO|currently|stub|unimplemented|planned|pending` finds only past-tense history ("green at Red Gate", "were red-first") in comments.
  - The exception is two assertion messages, covered by L9-003.
- **F-S2508-L8-002: closed in `src/`, NOT closed repo-wide (F-S2508-L9-001).**
  - `obl1_kani_proofs.rs` now reads "ADR-052 §5a re-baseline (S-25.08 AC-010)" at Harness 1b, 4b and 6b, and no version token is left in that file.
  - The same "VP-147 v1.18" mis-anchor survives in `.github/workflows/kani.yml`.
- **F-S2508-L8-003: not closed for the whole diff.**
  - Under `crates/factory-dispatcher`, the prescribed regex `(BC-…|VP-…|error-taxonomy|ADR-…) vX.Y` gives identical per-file counts in the worktree and on develop (333 hits in 26 files each).
  - Line-for-line, the hits in `executor.rs`, `main.rs`, `registry.rs` and `shard_manager.rs` are identical to develop, so every remaining hit is a pre-existing line.
  - The S-25.08 test files, `admission.rs` and `obl1_kani_proofs.rs` have zero hits.
  - Over the whole S-25.08 diff, though, the regex is not empty: `.github/workflows/kani.yml` line 29 matches (L9-001).
  - A bare-version sweep (`\bv1\.[0-9]+`) finds one more S-25.08-added comment token (L9-002).
- **F-S2508-L8-004: closed.** `code-delivery/S-25.08/adv-local-pass-6.md`, `-7.md` and `-8.md` exist (I globbed the canonical path). T-47 and T-55..T-57 are ticked, and the "Convergence status (v1.8)" paragraph matches the pass-8 frontmatter (1 MEDIUM + 4 LOW).
- **F-S2508-L8-005: closed for T-49..T-58, NOT story-wide (F-S2508-L9-004).** Every quoted subject in T-49..T-58 matches the reflog exactly: `faa9d9ef`, `f34e349c`, `016a192f`, `fbbb0d91`, `4adeaee6`, `66d4faeb`, `6f49d911`, `2d1fb0c0`, `ee9b2741`, `17941cc4` and `1bdd86fc`. Two older quotes in the story do not match.

## Runtime-behavior check of the comment-only commits
I could not diff the commits, so I checked indirectly:
- Every string literal in `src/` that names a spec anchor (BC-1.18.011/013, ADR-052, VP-146/147, error-taxonomy) is one of three things:
  - the compile-time TTL assert message;
  - the reservation-timestamp-fallback `tracing::warn!` text, which pass 8 already confirmed no test or spec pins;
  - the `total_bcs` `HookResult` message, which is byte-identical to develop.
- The version-token parity above shows the pre-existing comment lines in the shared `src/` files are unchanged.
- No test asserts on any changed assertion-message text; those messages are failure output only.

I found no sign of a runtime change. This is not a proof without a diff.

## Findings

### F-S2508-L9-001 — MEDIUM (HIGH confidence) — the "VP-147 v1.18" mis-anchor (L8-002) survives in the Kani CI workflow
Mis-anchors block convergence. Severity follows the sibling rule: the fix was applied to the main file and one sibling file was not updated.

- **Site:** `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/.github/workflows/kani.yml`.
  - The `kani` job comment reads "The ten OBL-1 harnesses (VP-147 v1.18 re-baseline, S-25.08 AC-010)."
  - The comment above `EXPECTED_PROOFS=10` reads "and the v1.18 re-baseline adds h1/h4/h6 harnesses".
- **Added by S-25.08:** develop's `kani.yml` has neither line (it has `EXPECTED_PROOFS=7` and no `v1.18`).
- **Why it is wrong:** VP-147 is at v1.5. "v1.18" is the ADR-052 delta. `ee9b2741` fixed exactly this text in `obl1_kani_proofs.rs` but only swept `src/`, and the L8-003 regex check was run only over `crates/`. The prescribed check (the regex over the S-25.08 diff must be empty) fails on line 29.
- **Failure scenario:** a formal-verifier or CI maintainer tracing the `EXPECTED_PROOFS=10` pin to its spec looks for a VP-147 v1.18 that does not exist. The CI comment also contradicts the module doc it describes.
- **Routing:** devops-engineer, comment-only change. Use the module-doc form on both lines, "ADR-052 §5a re-baseline (S-25.08 AC-010)". Then run the regex (and the bare-version sweep in the [process-gap] below) over every S-25.08-touched path: `.github/`, `CHANGELOG.md` and `crates/`.

### F-S2508-L9-002 — LOW (HIGH confidence) — an S-25.08-added comment cites a bare, version-only anchor, "(v1.18 B2-3)"
- **Site:** `shard_manager.rs`, the step-1 comment inside `drain_bc_index_writers`: "falling back to file mtime ONLY when the field is absent or unparseable (v1.18 B2-3)."
- **Added by S-25.08:** develop's copy of this comment ends after "already exited by the time any drain step runs)." with no staleness sentence. That sentence is the only bare-version difference between the worktree and develop across the whole file.
- **Why it is wrong:**
  - The token names no document.
  - B2-3 is an ADR-052 defect ID (ADR-052 §Verification Strategy, and VP-147 row "Precondition 6(c) TTL …").
  - BC-1.18.011 also had a v1.18, so the anchor is ambiguous.
  - The policy applied for this pass requires S-25.08-added comments to cite stable anchors with no version tokens. The doc-prefixed regex can't catch a bare token, which is the same escape route L8-002 found.
- **Failure scenario:** a reader can't tell which document "v1.18" refers to. The pin also goes stale as ADR-052 moves on (it is now at v1.27).
- **Routing:** implementer, comment only. Replace with "(ADR-052 §5a 'Reservation timestamp rules'; defect B2-3)".
- **Note:** the neighboring "v1.9 H1" tokens are on develop and out of scope. The `admission.rs` phrase "written by pre-v1.11 code" repeats BC-1.18.011's own provenance text, so I did not raise it.

### F-S2508-L9-003 — LOW (HIGH confidence) — red-gate-era wording left in two S-25.08 assertion messages
- **Site:** `s2508_pure_core_and_ttl_test.rs`, `test_BC_1_18_011_EC009_production_ttl_constant_floor_and_created_at_judgment`. Both `catch_unwind` arms of vector (b) push "production TTL validator panicked (unimplemented) for {secs}s".
- **Why it is wrong:** `validate_production_reservation_ttl` is implemented; the same file's module doc says "all four are now implemented". A panic today would be a real defect, but the message blames a missing implementation.
- **Why the sweeps missed it:** `17941cc4` swept comments for stale tense and status claims. `1bdd86fc` swept assertion messages only for spec version pins. Neither looked for status wording in assertion messages.
- **Failure scenario:** a regression that makes the validator panic is reported as "unimplemented", and the person triaging it goes looking for a missing stub instead of the panic.
- **Routing:** test-writer. Reword to "production TTL validator panicked for {secs}s". Re-run the stale-status keyword search over assertion-message string literals in all S-25.08 test files.

### F-S2508-L9-004 — LOW (HIGH confidence) — the story still quotes two commit subjects that do not match the commits (L8-005 sibling)
Both were checked against the reflog.

| Task and SHA | Story's quoted text | Actual commit subject |
|---|---|---|
| T-34 "Root cause", `ae74c039` | "feat(S-25.08): shared admission core - reserve-then-verify, **Branch** A/B/C reconciliation..." | "...reserve-then-verify, **Branches** A/B/C reconciliation, path-family keyed E-MAINTENANCE-001 (AC-005/AC-007/AC-009)" |
| T-9 "Original note", `81bbb897` | "resolve_target_path (T_real,T_lex); admission anchored to session factory_root; component-wise case-insensitive classification (F-002/F-003/...)" | The quote silently drops the middle clauses "; tool_use_id validity -> E-MAINTENANCE-002; non-string migration_id -> state_integrity" and the ID list "F-006/F-009". It has no elision marker where the text was cut. |

- **Why it matters:** T-58(c) says the L8-005 fix replaced quoted subjects with verbatim ones, but it was scoped to T-49..T-54. Quotes elsewhere in the story still present paraphrase as evidence (POLICY 22, report fidelity). The SHAs themselves are correct.
- **Routing:** story-writer. Correct both quotes to the verbatim subject, or mark the cut with "...". Do a story-wide pass over every `` `<sha>` "<subject>" `` pair.

## [process-gap]
- **The L8-003 gate is too narrow to prove its own claim.** Two gaps:
  - Its regex requires a document prefix, so bare tokens like "(v1.18 B2-3)" and "the v1.18 re-baseline" slip through.
  - It was run only over `crates/`, while S-25.08 also changed `.github/workflows/kani.yml`, `ci.yml` and `CHANGELOG.md`.
- **Effect:** "zero version pins on S-25.08-added lines" was declared closed twice (passes 7 and 8) while pins survived outside the scanned path or form.
- **Proposed codification:** the pin gate must run over every path in `git diff --name-only ce2421be..HEAD`. It must use both the prefixed regex and a bare `\bv[0-9]+\.[0-9]+\b` sweep restricted to `^\+` lines, with literal shell output captured (D-449(a) / POLICY 22).
- **Routing:** orchestrator, then state-manager (codification).

## Observation (pending intent verification; not counted)
- Test-function identifiers carry version tokens: the 11 `test_BC_1_18_013_ADR052_v123_*` tests and `test_BC_1_18_011_EC025_reservation_is_stale_v120_timestamp_table`.
- The policy as applied covers comments and assertion messages, not identifiers. Renaming them would change the AC Test: lines and S-25.09's references.
- The orchestrator decides whether [D-1257-POLICY19-SCOPE] extends to identifiers.

## Fresh sweep: checked, no finding
- **Release leg** (`executor::bc_index_migration_reservation_release`) follows ADR-052 §5a "Release-leg evaluation order" steps 1–5 exactly:
  - `is_tool_completion_event`;
  - then the `tool_use_id` read as a string and checked with `is_valid_tool_use_id`, silent with no filesystem access;
  - then `resolve_factory_root`, then `probe_release_migration_state_dir`, then `release_bc_index_writer_reservation`.
  - JSON `null` and non-string ids are silent. ENOENT gives `Ok(false)` and stays silent; any other stat error gives one non-fatal warn.
- **Admission** (`bc_index_migration_admission`): the order is input guards, then `resolve_factory_root`, then target classification, then `classify_tool_use_id`, which matches the CHANGELOG's stated `E-MAINTENANCE-002` order. `resolve_factory_root` treats only NotFound and NotADirectory as absent and never collapses an OS error into "absent", as ADR-052 §5a (b)/(c) requires. `admission_error` sanitizes and caps its detail text.
- **`main.rs` wiring:**
  - Admission and release run before the `CLAUDE_PLUGIN_ROOT` tiering and `Registry::load`.
  - Release-on-block applies only on final exit 2.
  - The `run` → `Err` → exit 0 path (`resolve_registry_path()?`) admits the tool, so PostToolUse releases the reservation. No leak beyond TTL.
- **L6_004 tests:** vectors (1)–(4) plus the in-function `assert_release_valid_id_control` are not tautological. The control expects exactly one WARN under each fixture, which holds for both the ELOOP `migration-state` and the ELOOP `.factory` cases.
- **`registry_error_exit_code`:** the variant list is explicit with no wildcard. The test table's 7 variants match the enum.
- **CI:**
  - Failpoint floors: 39 and 1 equal the `#[test]` counts in the two gated targets, with no `#[ignore]` and the count parsed from the run output.
  - Kani: `EXPECTED_PROOFS=10` equals the 10 `^#[kani::proof]` lines in `obl1_kani_proofs.rs`, with a declared-count guard.
- **CHANGELOG** S-25.08 entry: no version pins, owners credited. The "no tracing subscriber installed" claim is true (no `set_global_default` or `tracing_subscriber` in `src/`). The release-order and unstatable-`.factory` bullets match the code.

## Deferred (carried, unchanged)
- D-L4-1 → phase-5.
- D-L3-1 → wave gate.
- No new deferrals.

## Relevant paths
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/.github/workflows/kani.yml`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/shard_manager.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/tests/s2508_pure_core_and_ttl_test.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/executor.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/shard_manager/admission.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/stories/S-25.08-shared-admission-core-b2-conformance-vp147-rebaseline.md`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.git/worktrees/S-25.08/logs/HEAD`
