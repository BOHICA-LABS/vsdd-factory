---
document_type: formal-verification-evidence
level: ops
title: "S-25.08 Pre-Merge Formal Re-Verification Evidence (verbatim)"
producer: formal-verifier
persisted_by: state-manager
timestamp: 2026-10-13T00:00:00Z
phase: per-story-step-formal-reverification
story: S-25.08
head: "feature/S-25.08 @ 9e936d4c"
date: 2026-10-13
traces_to: S-25.08
---

> **Provenance.** Produced by the formal-verifier (session 2026-10-13) against `feature/S-25.08` @ `9e936d4c`; relayed verbatim by the orchestrator via the session scratchpad `s2508-formal-evidence.md` and persisted unchanged by the state-manager (D-1263). The body below is the verbatim file content. The raw logs it cites live in the session scratchpad and are not persisted here. T-6, T-11 and T-14 await the PR's CI result; T-12 awaits the durable runner commit.

# S-25.08 pre-merge formal re-verification evidence (formal-verifier, 2026-10-13)

Subject: `feature/S-25.08` @ `9e936d4c6aeda608f72105c5653bbe5b30b4f516` (worktree `.worktrees/S-25.08`).
`develop` = `ce2421be`; `git merge-base --is-ancestor develop HEAD` → `ANCESTOR_OK`, so no rebase is needed.
Toolchain: `Kani Rust Verifier 0.68.0 (cargo plugin)`, `CBMC 6.11.0` (the `kani.yml` pin).
Proof file SHA-256 (`obl1_kani_proofs.rs` @ 9e936d4c): `1363201fd5995c363155eac44aea5934bbdd66781a8e5b8b8091902b042e456f`.

Worktree hygiene: the tree was clean at the start. During the run another agent created an untracked
`docs/demo-evidence/S-25.08/` directory (8 `.tape` files + `demo-driver.py`, mtimes 14:14–14:16 CDT;
probably a concurrent demo-recorder). This agent did not create it. The tracked sources at HEAD are unchanged
(`git status --porcelain` shows only `?? docs/demo-evidence/S-25.08/`), and the directory does not affect any
build or test below. No commit, push or `.factory/` edit was made.

Raw logs (scratchpad):
- Kani: `kani/ci-step.sh` (the exact `kani.yml` step script; the ONLY change is the log path, shown by `diff`), `kani/kani-output.log`, `kani/ci-step.stdout`
- Tests: `tests/run-tests.sh`, `tests/summary.txt`, `tests/{workspace,failpoints,suite-*,floor-*,fmt,clippy*}.log`
- Mutants: `vp147-mutants/run-mutant.sh` (v1), `vp147-mutants/run-mutant.v2.sh` (verdict-classifier fix), `vp147-mutants/<M>.log`, `<M>.kani.log`, `<M>.test.log`

---

## T-11(d) / T-12 — VP-147 v1.4: ten-harness re-run from the exact CI step script — PASS

T-11(d) asks to re-run all ten harnesses from the exact CI step script, with `EXPECTED_PROOFS` staying at 10,
INV-GATE-TXN UNSAT-for-violation and every cover REACHABLE. The pre-v1.4 local 10/10 (HEAD 292ffed5) is NOT
re-claimed. This run is at 9e936d4c.

Step-script provenance: it was extracted verbatim from `.github/workflows/kani.yml`. `diff` against the
extraction shows only the `kani-output.log` path rewritten to the scratchpad.

Gate tail (`ci-step.stdout`):
```
cargo kani exit code: 0
Successful proofs:    10 (expected 10)
Failed/undetermined:  0
PASS: all 10 OBL-1 harnesses verified successfully, all covers satisfied.
STEP_EXIT=0
```
Per harness (`kani-output.log`):
```
proof_obl1_h6_terminal_reconcile_idempotence       8 of 8 covers   SUCCESSFUL   1.7 s
proof_obl1_h6_recovery_idempotence                 1 of 1          SUCCESSFUL   3.0 s
proof_obl1_h5_pointer_swap_crash_atomicity         2 of 2          SUCCESSFUL   3.2 s
proof_obl1_h4_reservation_quiescence_and_selfheal 20 of 20         SUCCESSFUL 991.1 s
proof_obl1_h4_admission_gate_invariant             2 of 2          SUCCESSFUL  23.1 s
proof_obl1_h3_transition_bounded_sequence          2 of 2          SUCCESSFUL   4.6 s
proof_obl1_h3_transition_inductive_step            3 of 3          SUCCESSFUL   0.8 s
proof_obl1_h2_recovery_safety_predicate            2 of 2          SUCCESSFUL  78.3 s
proof_obl1_h1_terminal_reconcile_totality          6 of 6          SUCCESSFUL   1.1 s
proof_obl1_h1_recover_totality                     2 of 2          SUCCESSFUL  96.5 s
Complete - 10 successfully verified harnesses, 0 failures, 10 total.
```
Vacuity gates:
- Gate 1: 10 cover summaries for 10 harnesses, and every summary has satisfied == total (48/48 covers).
- Gate 2: 102 `Status: UNREACHABLE` checks in the log, 0 of them located in `obl1_kani_proofs.rs`. All 102
  are in the Kani library (`kani_lib.c` `__rust_realloc` preconditions).

Wall time 1222.9 s; peak RSS 12.9 GB. The h4b peak was taken while 6 mutant builds ran in parallel; CI runs
alone, inside the job's 45-min budget.

INV-GATE-TXN is UNSAT for violation: h4 and h4b SUCCESSFUL means no counterexample exists within the bounds.
The model is shown to be non-vacuous by the S3 refused-by-txn-half and lost-gate covers among the 20/20 h4b
covers. Gate 1 confirms every cover is SATISFIED.

Scope note: this is a LOCAL run. The CI `kani` job result on the PR is still required for T-6/T-14 ("green IN CI").

## D-1252-OWED remainder — VP-147 mutants M1–M5: PASS (all killed)

The mutant list comes from VP-147 (v1.3 status: M1 foreign-migration check, M2 mtime precedence, M3 admission
admits STAGING, M4-model and M4-production admit without reservation) plus M5 from commit `25e5464e` ("M5:
both-unusable treated as stale, killed by S10").

Procedure, for each mutant:
1. `git worktree add --detach <scratchpad>/vp147-mutants/wt-<M> 9e936d4c`.
2. Apply exactly one perl substitution. It must hit exactly once, otherwise the run aborts as INVALID. All
   hit once (`substitutions=1`).
3. Run the killing harness(es) with `cargo kani -p factory-dispatcher --exact --harness ...`, or the
   black-box suites.
4. `git worktree remove --force`. The final `git worktree list | grep -c wt-M` returned `0`.

| Mutant | Mutation (diff) | Killer run | Result | Failed property (literal) |
|---|---|---|---|---|
| M1 | `decide_terminal_record_reconciliation`: removed `if !inputs.txn_migration_known { return RefuseForeignMigration; }` | h1-terminal + h6-terminal | **KILLED** (kani exit 1; both FAILED) | "H1b row 2: RefuseForeignMigration iff lock acquired, live txn, migration_id not in K"; "H1b rows 1/3: NoOp iff ..."; "H6b: Branch B iff lock + known-migration null-generation STAGING + its terminal record absent (no gate condition)" |
| M2 | `reservation_is_stale`: `trusted_created_at.or(mtime)` → `mtime.or(trusted_created_at)` | h4b | **KILLED** (FAILED) | "H4b/S8: a trusted created_at decides staleness; the mtime is ignored"; "H4b/S10: a future created_at within the skew tolerance ages as 0 (never stale)" |
| M3 | `is_bc_index_admission_open`: `Completed \| Aborted` → `Completed \| Aborted \| Staging` | h4 + h4b | **KILLED** by h4b (FAILED); h4 SUCCESSFUL (does NOT kill it) | "H4b/S3: no writer is admitted while a txn is STAGING/COMMITTING" |
| M4-model | h4b W1: `present: true` → `present: false` (writer advances without its reservation) | h4b | **KILLED** (FAILED; 16/20 covers) | "H4b/S3: admission only after the admitter's own reservation exists (reserve-then-verify)"; "H4b/S2: no in-flight writer holds an OPEN gate observation at the snapshot" |
| M4-production | `admit_protected_write`: W1 `create_writer_reservation(...)?` skipped, reservation = `None` | `cargo test` on `s2508_admission_blackbox_test`, `s2508_pure_core_and_ttl_test`, `bc_1_18_011_b2_migration_test` | **KILLED** (exit 101; 24 tests failed: b2_migration 52/4, admission_blackbox 28/20; 0 rustc errors) | e.g. `test_BC_1_18_013_PC6c_reservation_created_on_pretooluse_removed_on_posttooluse_blackbox`, `test_BC_1_18_013_EC020_concurrent_first_ever_admissions_all_succeed_blackbox`, `test_BC_1_18_011_PRECOND6_admit_or_block_bc_index_writer_creates_reservation_on_admission` |
| M5 | `reservation_is_stale`: `None => false` → `None => true` | h4b | **KILLED** (FAILED; 19/20 covers) | "H4b/S10: a reservation of unknown age (no usable created_at, no mtime) is never reclaimed" |

Runner defect, found and fixed in this session:
- The v1 runner's compile-error regex `^error(\[E..\])?: ` matched cargo's `error: test failed` lines, so it
  printed `VERDICT M4-production: INVALID` although the log shows 0 rustc errors and 24 failing tests.
- `run-mutant.v2.sh` restricts the regex to `^error\[E..\]|^error: could not compile`.
- M4-production was re-run with v2 and printed `VERDICT M4-production: KILLED`. The v1 log is kept as
  `M4-production.v1.log`.
- The Kani verdicts (M1–M3, M4-model, M5) did not depend on that branch, because `FAILED` was ≥ 1 in each.

Observation (not a failure): the pre-v1.18 harness `proof_obl1_h4_admission_gate_invariant` alone does not
kill M3. Only h4b S3 does, which matches `b66a460c` ("the admits-STAGING mutant is now killed by h4b S3").
The pinned 10-harness job kills it.

Durability gap (T-12 v1.5 text): T-12 asks for the runner to be authored "under `.github/` or `crates/`
tooling that survives the session". The dispatch forbade editing the S-25.08 worktree, so the runner exists
only in the scratchpad. `run-mutant.v2.sh` is ready to commit; the orchestrator should route it
(devops-engineer or implementer) to e.g. `.github/scripts/vp147-mutants.sh`. Until then the T-12
"survives the session" clause is OPEN.

## T-14 / T-6 — re-verification gate: local legs PASS; CI leg pending

Literal results (`tests/summary.txt`, CI-mirroring flags, `CI_REQUIRE_ARTIFACTS=1`, `VSDD_CORPUS_ROOT=<repo>/.factory`):
```
### 1. cargo test --workspace --all-targets            EXIT workspace=0   (3393 passed, 0 failed across all targets)
### 2. cargo test -p factory-dispatcher --features failpoints --all-targets   EXIT failpoints=0   (1173 passed, 0 failed)
FLOOR bc_1_18_011_b2_migration_crash_injection_test: exit=0 passed=39 floor=39
FLOOR bc_1_18_011_b2_migration_obl1_failpoint_smoke_test: exit=0 passed=1 floor=1
### 3. S-25.08 targeted suites
SUITE s2508_admission_blackbox_test: exit=0 :: test result: ok. 48 passed; 0 failed
SUITE s2508_pure_core_and_ttl_test:  exit=0 :: test result: ok. 16 passed; 0 failed
SUITE s2508_diagnostics_test:        exit=0 :: test result: ok. 9 passed; 0 failed
SUITE s2508_registry_exit_code_test: exit=0 :: test result: ok. 5 passed; 0 failed
SUITE bc_1_18_011_b2_migration_test: exit=0 :: test result: ok. 56 passed; 0 failed
### 4. EXIT fmt=0   EXIT clippy=0   EXIT clippy-failpoints=0
```
- AC-011..AC-020 suites: green. They live in `s2508_admission_blackbox_test` (AC-011..AC-015, AC-018) and
  `s2508_pure_core_and_ttl_test` (AC-015 pure core, AC-016, AC-017, AC-019, AC-020) per the story's file table.
- VP-133 facets 6/7: green (`s2508_admission_blackbox_test`, `bc_1_18_011_b2_migration_test`).
- VP-143 D1/D5: green (`s2508_admission_blackbox_test`).
- VP-147 re-run: PASS (above).
- Still open, not something this agent can discharge:
  - (i) T-11 green IN CI: the CI `kani` job on the PR.
  - (ii) T-14 local adversary 3-CLEAN: the story records streak 0/3; the human accepted convergence on
    2026-10-13 per this dispatch, and that acceptance needs to be recorded by state-manager.
- So T-14/T-6 is LOCAL PASS / CI PENDING. Neither should be ticked until the PR's `kani` and `cargo-host`
  checks are green.

## D-1244-REVERIFY

- **B2 obl1 Kani (VP-147): PASS.** 10/10 above.
- **VP-143: PASS for the S-25.08 share.**
  - D1/D5 black-box: `s2508_admission_blackbox_test` 48/48.
  - The B2 fault-injection prong it pairs with: `bc_1_18_011_b2_migration_crash_injection_test` 39/39 under
    `--features failpoints`, plus the obl1 failpoint smoke test 1/1.
  - VP-143 (b)/D2/D3/D4 belong to S-25.06 (VP-143 v1.2 anchor split) and are not on this branch.
- **VP-146: NOT APPLICABLE to the S-25.08 merge. Evidence:**
  - `git ls-files | grep -c append_log_kani_proofs` on 9e936d4c → `0`, and `git grep proof_obl_a -- crates .github` → no hits.
  - VP-146 v1.2: "S-25.06 keeps ALL of VP-146: harnesses a1..a6 in `append_log_kani_proofs.rs`... S-25.08
    delivers the shared admission core the a4 model abstracts, so VP-146 is re-verified only after S-25.08
    lands (S-25.06 `depends_on: [S-25.08]`)."
  - The shared pure cores that VP-146 a1/a4/a6 will model (`decide_terminal_record_reconciliation`,
    `is_bc_index_admission_open`, `reservation_is_stale`) are the REAL functions proven here by VP-147 h1-terminal / h4b / h6-terminal.
  - D-1244-REVERIFY therefore stays OPEN for the VP-146 leg, as an S-25.06 merge gate (T-8a). The STATE.md
    row reads "before merge of S-25.08 / S-25.06"; state-manager should record that the VP-146 leg binds
    S-25.06 only.

---

## Claim-boundary text for the PR (T-12)

> **VP-147 v1.4 re-verification (S-25.08, local, HEAD 9e936d4c, Kani 0.68.0 / CBMC 6.11.0).** All ten pinned
> `proof_obl1_*` harnesses report VERIFICATION SUCCESSFUL via the exact `kani.yml` step script
> (`EXPECTED_PROOFS=10`, `Complete - 10 successfully verified harnesses, 0 failures`). 48/48 `kani::cover!`
> are satisfied, and no check in `obl1_kani_proofs.rs` is UNREACHABLE. The v1.4 deltas are covered:
> h1-terminal decision rows 1–5 (BC-1.18.011 Precondition 6(d)), the `txn_migration_known` input
> (`migration_id ∈ K`), h4b S7 (a live txn whose `migration_id ∉ K` is never finalized, aborted or reconciled
> over) and h4b S10 (through the REAL 4-argument `reservation_is_stale` with the shipped 300 s skew: unknown
> age never reclaimed; a `created_at` beyond the skew falls back to the mtime; a within-skew future
> `created_at` ages as 0). Mutants M1 (foreign check dropped), M2 (mtime precedence), M3 (admission admits
> STAGING), M4-model and M5 (unknown age treated as stale) are each KILLED by a named Kani assertion.
> M4-production (admission without reservation) is KILLED by 24 black-box tests.
>
> **What this does NOT prove.** The claim form is: under abstract filesystem model M, every modeled op/crash
> sequence of length ≤ N preserves the old-or-new invariant and recovery is total and fail-closed. The
> following are outside Kani's model and are discharged by tests:
> - Path classification and resolution (`factory_root` anchoring, `resolve_target_path` aliases/case, out-of-root no-op).
> - The `E-MAINTENANCE-002` causes and their evaluation order.
> - `PostToolUse`/`PostToolUseFailure` delivery and release.
> - Registry-independent evaluation before `Registry::load`.
> - The real `create_dir_all`, `flock`, `rename(2)`, fsync, page-cache, torn-write and power-loss behaviour.
> - Txn-record Tier 0/Tier 1 parsing.
> - That the dispatcher actually calls the shared core on the production path.
>
> These are carried by the AC-011..AC-020/AC-027..AC-031 black-box and integration suites (VP-133 facets 6/7,
> VP-143 D1/D5) and the 39-test B2 crash-injection suite. The Kani bounds are ≤ 10 scheduler steps, 2
> writers and 2 reservation slots in h4b, and the single-character hash domain. This is a LOCAL result: it
> becomes PROVED/merged only when the PR's `kani` CI job is green. `verification_lock` stays false until the
> lock ceremony runs on the merged tree.
