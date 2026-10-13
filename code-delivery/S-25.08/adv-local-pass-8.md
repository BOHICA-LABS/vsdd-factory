---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 8 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-13T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 8
reviewed_head: "feature/S-25.08 @ 2d1fb0c0"
verdict: NOT CLEAN
finding_count: { total_numbered: 5, critical: 0, high: 0, medium: 1, low: 4, deferred: 0 }
streak_3_clean: "0/3"
novelty: "LOW to moderate"
traces_to: S-25.08
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back, session 2026-10-11/13; reviewed HEAD feature/S-25.08 @ 2d1fb0c0.

# S-25.08 local adversarial review, pass 8

## Identity and inputs
- Identity tuple: worktree `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08`, branch `feature/S-25.08`, HEAD `2d1fb0c0`, story S-25.08, canonical root `/Users/zious/Documents/GITHUB/vsdd-factory`.
- Preflight: the worktree basename `S-25.08` matches the story ID. All code was read from the worktree. All specs were read from the canonical `.factory/`.
- Spec versions: story v1.9 (uncommitted working tree), BC-1.18.013 v1.15, BC-1.18.011 v1.23, ADR-052 v1.27 (only §5a "Factory-root lookup mapping" and "Release-leg evaluation order" were read), VP-147 v1.5.
- SHA check: I can't run git, so I read the worktree reflog at `.git/worktrees/S-25.08/logs/HEAD`. Every SHA the story cites for passes 6 and 7 is on the branch, in this order: `fbbb0d91` → `faa9d9ef` → `f34e349c` → `016a192f` → `4adeaee6` → `66d4faeb` → `6f49d911` → `2d1fb0c0`.
- Not done: I could not run `cargo test`. T-49 hands the green-run check to "the pass-8 verifier", and I could not do it, so whether the suite is green at `2d1fb0c0` has still not been independently checked.

## Verdict: NOT CLEAN
- Findings: 0 CRITICAL, 0 HIGH, 1 MEDIUM, 4 LOW. One [process-gap] tag.
- Novelty: LOW to moderate. Everything is documentation or traceability. I found no behavioral, error-path or silent-failure defect in the delivered code.
- Streak after this pass: 0/3.

## Pass-7 fix verification
- **F-S2508-L7-001: partially closed.** The four named module docs were rewritten. Two stale claims remain in S-25.08-authored files, one of them in a file L7-001 named (see F-S2508-L8-001).
- **F-S2508-L7-002: closed.** `bc_1_18_013_release_stat_error_test.rs` now names `migration.admission_advisory` with reason `reservation_release_failed`, credited to S-25.09. It names the function `bc_index_migration_reservation_release`, and its doc points to `s2508_diagnostics_test.rs` for the in-process observation.
- **F-S2508-L7-003: closed, with residue.** Story v1.9 ticks T-49, adds T-50..T-54 and the file-change rows for the pass-6 and pass-7 commits, and lists the four L6_004 tests on the AC-011 and AC-031 Test: lines. The remaining items are F-S2508-L8-004 and L8-005.
- **Observation, ADR version pins: closed for ADR-052.** `grep -E "ADR-[0-9]+ v[0-9]+\.[0-9]+"` over `crates/factory-dispatcher` finds no ADR-052 pin. The only hits are ADR-024, ADR-035 and ADR-048 pins in files S-25.08 did not author; I checked they are also on the main checkout. Two problems the ADR-only regex could not catch are F-S2508-L8-002 and L8-003.
- **Observation, in-function valid-id control: closed.** `assert_release_valid_id_control` runs under each L6_004 test's own fixture and expects exactly one WARN. That is correct for both fixtures:
  - ELOOP `migration-state`: the probe returns `Err`, which produces one warn.
  - ELOOP `.factory`: `resolve_factory_root` returns `Err` (ELOOP is outside `is_factory_root_absent_error`), which produces one warn.
- **`2d1fb0c0` runtime strings: no runtime change.** Two runtime strings were reworded:
  - the `const _: () = assert!` message in `shard_manager.rs`, which only appears at compile time;
  - the reservation-timestamp fallback `tracing::warn!` message.

  Nothing pins their text. No test in the worktree matches "timestamp fallback", "ambiguous timestamps" or the TTL assert text; `s2508_diagnostics_test.rs` checks only the `reason` field. In BC-1.18.011 and BC-1.18.013 the phrase "reservation-timestamp fallback" appears only as vector-description text, not as message text.

## Findings

### F-S2508-L8-001: MEDIUM (HIGH confidence) — the stale-claim sweep (L5-001 / L6-003 / L7-001) missed two S-25.08-authored doc comments
Severity follows the sibling rule: a fix was applied, and a sibling file named in the original finding was not updated.

- **Site 1:** `tests/s2508_admission_blackbox_test.rs`, doc comment on `test_BC_1_18_013_L4_001_release_with_eloop_migration_state_exits_0_deletes_nothing_blackbox`.
  - It says "the warn-vs-silent distinction is not observable in S-25.08 -- see `bc_1_18_013_release_stat_error_test.rs`".
  - L5-001 found that claim wrong. The two `test_BC_1_18_013_L5_001_*` tests in `s2508_diagnostics_test.rs` observe the WARN in-process, and AC-031 and the helper-file doc were corrected to say so.
  - The same comment ends "Green today; guards the fix against …", which is red-gate-era wording; the fix `675fcf77` is delivered.
- **Site 2:** `tests/s2508_registry_exit_code_test.rs` (L7-001 named this file).
  - The module header still opens with "S-25.08 RED-GATE:".
  - The section comment above `run_binary` still says "real-binary pins (GREEN today; they lock the wired behavior the new function must preserve)", although lines 6–10 now say `registry_error_exit_code` is implemented and delegated to.
- **Failure scenario:** someone maintaining the black-box suite is told the release-leg WARN can't be observed and is pointed only to the helper-classification file. They miss the in-process pins, and the claim the story already retracted comes back. This class has now been missed for three passes in a row.
- **Routing:** test-writer. Rewrite both comments to the delivered state and point the black-box comment at `s2508_diagnostics_test.rs`. Then run a case-insensitive grep across all S-25.08 test files for `not observable`, `GREEN today`, `today`, `RED-GATE`, `new function` and `the fix`.

### F-S2508-L8-002: LOW (HIGH confidence) — mis-anchor: "VP-147 v1.18" cites a VP version that does not exist
Mis-anchors block convergence.

- **Where:** `src/shard_manager/obl1_kani_proofs.rs`, in all of these places:
  - the section headers for Harness 1b, 4b and 6b, which read "VP-147 v1.18 extension";
  - the doc comment that begins "VP-147 v1.18 extension (BC-1.18.011 Postcondition 9; …";
  - the step comment "(3) VP-147 v1.18: the FinalizeFromTerminalRecord event."
- **Evidence:**
  - VP-147's frontmatter says `version: "1.5"`.
  - "v1.18" is the ADR-052 v1.18 delta that the AC-010 re-baseline extends, as the module doc itself says.
  - The same wording is in the S-25.09 worktree, which still has the text from before `2d1fb0c0`, so it is original S-25.08 text.
  - The `2d1fb0c0` sweep's `ADR-NNN vX.Y` regex could not match it because the "ADR-052" token was never there.
- **Failure scenario:** a formal-verifier tracing Harness 1b, 4b or 6b to its spec looks for a VP-147 v1.18 that doesn't exist, and the harness-to-delta link is ambiguous.
- **Routing:** implementer or formal-verifier, comment-only change. Replace with "ADR-052 §5a re-baseline (S-25.08 AC-010)" to match the module doc.

### F-S2508-L8-003: LOW (MEDIUM confidence) — pending intent verification: the version-pin cleanup removed ADR pins but left BC, VP and error-taxonomy pins in the same S-25.08-added comments
This is a sibling of the pass-7 observation.

- **Examples on S-25.08-added lines:**

  | File | Pin(s) |
  |---|---|
  | `executor.rs`, doc of `admission_error` | "error-taxonomy v1.37"; the taxonomy is at v1.42 per the story |
  | `admission.rs`, docs of `e_maintenance_block_message` and the as-given alias | "error-taxonomy v1.35", "BC-1.18.013 v1.9" |
  | `shard_manager.rs` | "VP-146 v1.2 / VP-147 v1.1", "BC-1.18.011 v1.13 Precondition", "BC-1.18.011 v1.16 EC-031" |
  | `s2508_admission_blackbox_test.rs` "Message format" header | "BC-1.18.013 v1.6 Pre 6(b); error-taxonomy v1.35" |
  | `s2508_registry_exit_code_test.rs` header | "BC-7.06.001 v1.13 … BC-1.08.001 v1.4" |
  | `s2508_pure_core_and_ttl_test.rs` | 7 BC/VP pins |

- **Counts:** grep for `BC-1\.18\.01[13] v[0-9]|error-taxonomy v[0-9]|VP-147 v[0-9]` returns 58 hits across 8 files. Some are in files S-25.08 did not author, but the `src/` hits in `admission.rs` and the S-25.08 Kani sections are all S-25.08's.
- **Why "pending intent":** POLICY 19's text covers ADR cites, and T-54(c) was scoped to ADR only. The volatility argument that drove the ADR sweep and the CHANGELOG [process-gap] ("version pins go stale in the same burst") applies equally to these pins. They are already stale: v1.6 against v1.15, and v1.35/v1.37 against v1.42.
- **Routing:** orchestrator decides whether POLICY 19 covers BC/VP/taxonomy pins in code comments. If it does, implementer and test-writer convert them to §-anchors.

### F-S2508-L8-004: LOW (HIGH confidence) [process-gap] — pass-review bookkeeping in story v1.9 does not match canonical `.factory/`
- **T-47 is stale.** It is still "[ ] OPEN" ("persist `code-delivery/S-25.08/adv-local-pass-5.md`"), but `.factory/code-delivery/S-25.08/adv-local-pass-5.md` exists (glob of the canonical path). This is the L5-004 / L6-002 stale-status class again.
- **Pass-6 and pass-7 review files are missing.** No `adv-local-pass-6.md` or `adv-local-pass-7.md` exists under `.factory/code-delivery/S-25.08/`, yet T-48, T-54 and the "Convergence status (v1.8)" paragraph say both passes were "registered by the state-manager burst". No task tracks persisting either file, as T-42 and T-47 did for passes 4 and 5.
- **Failure scenario:** the pass-6 and pass-7 findings have no durable record. The story's convergence history for those passes lives only in prose summaries, and the next reviewer or PR gate can't reconcile against an authoritative file.
- **Process gap:** the story has fallen behind on review-file persistence for several passes (passes 4, 5, 6 and 7). Persistence is the state-manager's job, but no step makes sure it happens before the next pass is dispatched.
- **Routing:** state-manager persists pass 6 and pass 7 verbatim. Story-writer ticks T-47 and adds persistence tasks for passes 6, 7 and 8.

### F-S2508-L8-005: LOW (HIGH confidence) — story tasks put paraphrases in quotation marks as if they were commit subjects
- **Examples, compared against the reflog:**

  | Task and SHA | Story's quoted "subject" | Actual commit subject |
  |---|---|---|
  | T-49, `faa9d9ef` | "test: red vectors for L6-004" | "test(S-25.08): red vectors — invalid or absent tool_use_id is silent before any stat on release (F-S2508-L6-004, T-49)" |
  | T-49, `f34e349c` | "fix: release leg checks tool_use_id before any stat (L6-004)" | "...; invalid or absent id is silent (F-S2508-L6-004)" |
  | T-52, `fbbb0d91` | "docs: test module docs in past tense (L6-003)" | "docs(S-25.08): test module docs describe the fixed release leg in past tense (F-S2508-L6-003)" |
  | T-54, `66d4faeb` | "test module docs updated to the delivered state" | "docs(S-25.08): sweep S-25.08 test module docs to the delivered state (F-S2508-L7-001, F-S2508-L7-002)" |

- **Why it matters:** earlier tasks (T-41, T-43..T-46) quote subjects verbatim and say they were verified with `git show`. Quoted paraphrases look like the same kind of evidence when they aren't (POLICY 22, report fidelity). The SHAs themselves are correct.
- **Routing:** story-writer replaces each quote with the verbatim subject, or drops the quotation marks.

## Fresh sweep: checked, no finding
- **Release-leg order** (`executor::bc_index_migration_reservation_release`) matches ADR-052 §5a "Release-leg evaluation order" steps 1–5:
  - a completion-event check first;
  - then `tool_use_id` read as a string and checked with `is_valid_tool_use_id`, with no filesystem access;
  - then `resolve_factory_root`, then `probe_release_migration_state_dir`, then the unlink.

  Absent or invalid ids return silently. Admission keeps the opposite order (AC-030, L4-002), as the ADR rules.
- **L6_004 test strength:** vectors (1)–(3) would catch a mutant that probes `migration-state` before checking the id. Vector (4) would catch one that resolves `.factory` first. Each has a live in-function control. No tautology.
- **Delegation and helpers:** `resolve_factory_root` classifies three ways and never collapses an OS error into "absent". `classify_tool_use_id` and the release guard use the same grammar. `admission_error` caps and sanitizes its detail text.
- **CI failpoints step** (`4adeaee6`): per-target floors of 39 and 1, counts parsed from libtest output at run time, output printed on failure, `set -euo pipefail`. Only two test files carry `feature = "failpoints"`, and both are covered.
- **CHANGELOG** S-25.08 entry: no version pins. Every EC and AC is credited to its owner. EC-039 belongs to S-25.08 (BC-1.18.011 v1.20, F-S2508-L3-001). The release-leg evaluation-order bullet is accurate.
- **AC-011, AC-012, AC-030 and AC-031** Test: lines all resolve to existing test functions, including the `RULING2 *_with_exact_keyed_reason` pair and `payload::tests::non_string_tool_name_maps_to_empty`.
- Leftover present-tense claims in `bc_1_18_011_b2_migration_test.rs` ("`read_active_txn_record` today …", "today ignores the intent log") are also on the main checkout. They belong to S-25.02 and are out of this story's scope.

## Deferred (carried, unchanged)
- D-L4-1 → phase-5.
- D-L3-1 → wave gate.
- No new deferrals.

## Relevant paths
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/tests/s2508_admission_blackbox_test.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/tests/s2508_registry_exit_code_test.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/shard_manager/obl1_kani_proofs.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/executor.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/shard_manager/admission.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/stories/S-25.08-shared-admission-core-b2-conformance-vp147-rebaseline.md`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/code-delivery/S-25.08/`
