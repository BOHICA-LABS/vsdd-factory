# Demo Evidence — S-25.02 Phase F4 BC-cluster 3 "backfill-split" (BC-1.18.008)

**Branch:** `feature/S-25.02-backfill`, as of this recording pass's HEAD (`2dd39bbb`) — a literal
SHA pin here goes stale on every subsequent fix-burst commit, per cluster-1's own n3/PR #818 and
cluster-2's precedent; the recordings below were captured against the SHA current at recording time
and remain valid evidence regardless of later commits. See the branch's commit history for current
HEAD.
**Status:** BC-5.39.001 3-CLEAN adversarial convergence achieved for mechanism-A one-time
backfill-split as of the recording pass; additional fix-burst commits may have landed since — see
the branch's commit history for current HEAD.
**Story spec:** `.factory/stories/S-25.02-artifact-sharding-layer2.md` (AC-013, AC-014)
**Behavioral contract:** BC-1.18.008 v1.8 — Mechanism-A One-Time Backfill-Split of Pre-Existing
Oversized Append-Logs. AC-013's Postcondition 4 clip (retention composition) also exercises
BC-1.18.007 (Retention Policy) as its immediate composition partner — the backfill-split's own
freshly-produced shard count feeding straight into the retention archival decision in the same
operation.

## Why a VHS terminal recording, not a browser demo

Like cluster-1's BC-1.18.005 and cluster-2's BC-1.18.006, BC-1.18.008 is a **Rust library
mechanism with no wired production entry point yet** — `run_mechanism_a_backfill_split` and its
supporting functions live in `crates/factory-dispatcher/src/shard_manager.rs`, but F4-activation
wiring (invoking this mechanism from the dispatcher's own PreToolUse gate chain, on the same
`execute_tiers` stack cluster-1 and cluster-2's dispatch-driven clips exercise) is deferred to story
T-12 and has not landed on this branch. Unlike cluster-1/cluster-2, there is therefore no
`execute_tiers -> shard_cap_precheck -> ...` full-stack path to record for this cluster today —
every clip below drives the mechanism's public Rust API directly (via its own test suite), not
through the dispatcher's PreToolUse gate chain. The mechanism's only observable surface is that
public API (`Result<MechanismABackfillOutcome, MechanismABackfillError>` plus the enum/error
variants) and the resulting filesystem state (sealed shard files, a fresh current file, the
shard-index TOML, and — for the happy-path/heal clips — the canonical file's own byte content). It
has no UI. A browser/Playwright demo would have nothing to capture. Per the demo-recording skill's
library/test-harness mode, and following cluster-1's and cluster-2's own precedent exactly, a VHS
terminal recording of the real, unmodified `cargo test` suite driving the real production code is
the faithful, proportionate evidence format for this kind of product. `vhs 0.11.0` was available in
this environment; no fallback to plain-text capture was needed.

**No output was manufactured or hand-typed as text, and no new source file was added to enable this
demo.** Every recording runs the real, unmodified test suite that already exists on this branch
(`crates/factory-dispatcher/tests/bc_1_18_008_backfill_split_test.rs`) and shows the real terminal
output, including PASS results plus the real, byte-for-byte filesystem and API-return assertions
each test makes (reconstructed-content equality, record-count sums, per-shard-cap checks, error
variant matching, `(length, sha256)` manifest comparisons). Per the demo-recorder's constraints, no
source or test file was modified to produce these recordings — each clip is a `grep` excerpt of the
exact, already-existing source anchor (function signature or error-variant declaration — never a
pinned `file.rs:NNN` line number, per TD-VSDD-091) plus a live `cargo test <name-or-filter> --
--nocapture` run of the exact test(s) named below. Every test in the cluster-3 file (all 59) was
independently re-run via `cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test`
and confirmed 59/59 `PASS` immediately before recording, and the FULL crate test suite
(`cargo test -p factory-dispatcher`, all files) was also independently re-run and confirmed green as
a precondition (see Reproduction below).

**A note on one filter-substring correction.** The dispatch for clip 11 (DANGEROUS-heal / manifest
slice-verify) specified filter substring `FC3P7001_run_backfill_split_heal`, expected to match
exactly 3 tests. Verified via `cargo test ... FC3P7001_run_backfill_split_heal -- --list`: this
substring actually matches only 2 of the 3 intended tests
(`..._heal_offset_derived_from_manifest_never_shard_index_bytes_at_seal` and
`..._heal_write_receives_disk_read_back_verification`) — it misses the third
(`..._FC3P7001_EC011_run_backfill_split_heal_aborts_e_shd_012_when_manifest_slice_verification_fails`)
because that test name inserts `EC011_` between `FC3P7001_` and `run_backfill_split_heal`, breaking
the substring match. The broader filter `FC3P7001` matches all three of the intended tests and
nothing else (verified via `--list`), so clip 11 uses `FC3P7001` instead. This is the only deviation
from the literal filter strings named in the dispatch; every other filter was verified via
`--list` to match exactly the intended test set before recording.

## AC/EC -> Clip Mapping

| AC / EC | Behavior | Clip | Test(s) exercised | Result |
|---------|----------|------|--------------------|--------|
| Baseline (all clips) | Full cluster-3 test suite, all 59 tests, all green — the precondition every other clip's individual test is drawn from. | `suite-all-green.{gif,webm}` | `cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test` (full file, 59 tests) | PASS (`59 passed; 0 failed`) |
| AC-013 (Postconditions 2-3) | An oversized pre-existing artifact is partitioned via deterministic greedy boundary-preserving packing into `ceil(bytes/cap)`-lower-bound sealed shards plus a fresh current file, and the full shard index is published in the SAME operation. | `AC-013-oversized-split-shards-index.{gif,webm}` | `test_BC_1_18_008_PC2_PC3_run_backfill_split_oversized_artifact_produces_sealed_shards_and_fresh_current` (`--exact`) | PASS (sealed shard count matches lower bound; fresh current published; shard index has matching entry count) |
| AC-013 (BLOCKER-1, leading preamble atomic unit) | A leading preamble (frontmatter/title/table-header content preceding the first record boundary) is treated as a single atomic packing unit, never split, and its exact bytes are preserved end-to-end through `run_mechanism_a_backfill_split`. | `AC-013-preamble-preserved-end-to-end.{gif,webm}` | `test_BC_1_18_008_BLOCKER1_run_backfill_split_preserves_preamble_bytes_end_to_end` (`--exact`) | PASS (preamble bytes byte-for-byte identical in the reconstructed output) |
| AC-013 (Postcondition 4, retention composition with BC-1.18.007) | The backfill-split composes with BC-1.18.007's retention policy immediately when the actual shard count this SAME operation produces exceeds `retention_count` — archival fires in the same call, not a follow-up one. | `AC-013-retention-composition.{gif,webm}` | `test_BC_1_18_008_PC4_run_backfill_split_composes_with_retention_when_shard_count_exceeds_retention_count` (`--exact`) | PASS (`archived_count` reflects the oldest-shard archival that fired within this same operation) |
| AC-014 (Postcondition 5, content-preservation) | Byte-for-byte reconstruction check: concatenating every partition's bytes must reproduce the original content exactly. Success + failure (deliberate byte mismatch). | `AC-014-content-preservation.{gif,webm}` | Filter `PC6a_verify_content_preserved` — matches exactly 2 tests: `..._true_when_concatenation_matches_original`, `..._false_when_byte_mismatch` | PASS (both: `true` case returns `true`, mismatch case returns `false`) |
| AC-014 (Postcondition 5, record-count preservation) | Exactly-once record accounting: the sum of every partition's `record_count` must equal the original record count. Success + 2 failure modes (a record dropped, a record duplicated). | `AC-014-record-count-preservation.{gif,webm}` | Filter `PC6b_verify_record_counts_preserved` — matches exactly 3 tests: `..._true_when_sum_matches`, `..._false_when_record_dropped`, `..._false_when_record_duplicated` | PASS (sum-matches -> `true`; both drop/duplicate mutations -> `false`) |
| AC-014 (Invariant 4, per-shard-cap enforcement) | Every partition must stay within `shard_cap_bytes` UNLESS it carries the `oversized_record` flag. Success (within cap), failure (an unflagged shard exceeds cap), and the flagged exception (an oversized-record shard exceeding cap is still accepted). | `AC-014-per-shard-cap-gate.{gif,webm}` | Filter `PC6c_INV4` — matches exactly 3 tests: `..._true_when_every_shard_within_cap`, `..._false_when_unflagged_shard_exceeds_cap`, `..._true_when_oversized_record_flagged_exception` | PASS (all 3: gate accepts within-cap and the flagged exception, rejects the unflagged over-cap shard) |
| AC-014 (EC-003, crash-atomicity + restart) | Restarting after a partial prior backfill-split attempt (simulating a crash mid-sequence) still produces the correct final result — no double-sealing, no data loss, no duplicate shard-index entries. | `AC-014-crash-atomicity-restart.{gif,webm}` | `test_BC_1_18_008_EC003_run_backfill_split_restart_after_partial_prior_attempt_produces_correct_result` (`--exact`) | PASS (restart converges to the same correct end state a clean run would produce) |
| AC-014 (Invariant 3, SAFE disposition / idempotency) | The Recovery-Confirmation Rule's SAFE arm: `mechanism_a_backfill_already_migrated` resolves a second invocation against an already-migrated artifact as a strict no-op — matching `(final_bytes, final_sha256)`, no write issued. | `AC-014-idempotency-safe-noop.{gif,webm}` | `test_BC_1_18_008_INV3_run_backfill_split_idempotent_second_run_returns_already_migrated` (`--exact`) | PASS (`Ok(MechanismABackfillOutcome::AlreadyMigrated)`, no filesystem write on the second call) |
| AC-014 (Recovery-Confirmation Rule, AMBIGUOUS disposition, `E-SHD-011`) | When on-disk `(length, SHA-256)` state matches NEITHER the Backfill Recovery Manifest's original nor final pair, the disposition is AMBIGUOUS — fails loud with `MechanismABackfillError::AmbiguousRecoveryState` (`E-SHD-011`), leaving the canonical file untouched, no write issued. | `AC-014-ambiguous-e-shd-011.{gif,webm}` | `test_BC_1_18_008_FC3P6001_EC010_run_backfill_split_ambiguous_on_disk_state_fails_loud_e_shd_011` (`--exact`) | PASS (`Err(AmbiguousRecoveryState { .. })`; error Display contains `E-SHD-011`; canonical file byte-for-byte unchanged) |
| AC-014 (Recovery-Confirmation Rule, DANGEROUS disposition, heal path + `E-SHD-012`) | The DANGEROUS-disposition heal: offset is derived from the Backfill Recovery Manifest (NEVER from the shard-index's own `bytes_at_seal`), the heal write receives its own mandatory disk read-back verification, and a slice-verification failure aborts loud with `MechanismABackfillError::SliceVerificationFailed` (`E-SHD-012`). | `AC-014-dangerous-heal-manifest-slice-verify.{gif,webm}` | Filter `FC3P7001` — matches exactly 3 tests: `..._heal_offset_derived_from_manifest_never_shard_index_bytes_at_seal`, `..._heal_write_receives_disk_read_back_verification`, `..._FC3P7001_EC011_..._heal_aborts_e_shd_012_when_manifest_slice_verification_fails` (see filter-correction note above) | PASS (all 3: offset sourced from Manifest not shard-index; heal write's own read-back verified; slice-verify failure -> `Err(SliceVerificationFailed)` containing `E-SHD-012`) |
| AC-014 (Invariant 5, EC-013, happy-path canonical read-back, `E-SHD-013`) | The ORDINARY, non-recovery, first-time canonical-truncate write ALSO gets its own mandatory post-hoc disk read-back verification — a clean positive control succeeds, and a corruption race on that read-back aborts loud with `E-SHD-013`. | `AC-014-happy-path-canonical-readback-e-shd-013.{gif,webm}` | Filter `FC3P8002_EC013` — matches exactly 2 tests: `..._happy_path_canonical_write_clean_positive_control_succeeds`, `..._happy_path_canonical_write_aborts_e_shd_013_on_disk_corruption_race` | PASS (clean control succeeds; corruption-race case fails loud, error Display contains `E-SHD-013`) |

## Files

| File | Content |
|------|---------|
| `suite-all-green.{tape,gif,webm}` | VHS script + recording — the full 59-test cluster-3 suite, all green (baseline evidence) |
| `AC-013-oversized-split-shards-index.{tape,gif,webm}` | Oversized artifact -> deterministic greedy split into sealed shards + fresh current + shard index, same operation |
| `AC-013-preamble-preserved-end-to-end.{tape,gif,webm}` | BLOCKER-1: leading preamble is a single atomic packing unit, bytes preserved end-to-end |
| `AC-013-retention-composition.{tape,gif,webm}` | Backfill-split composing with BC-1.18.007 retention archival when shard count exceeds `retention_count` |
| `AC-014-content-preservation.{tape,gif,webm}` | Byte-for-byte content-preservation check — success + byte-mismatch failure |
| `AC-014-record-count-preservation.{tape,gif,webm}` | Exactly-once record accounting — success + dropped-record + duplicated-record failures |
| `AC-014-per-shard-cap-gate.{tape,gif,webm}` | Per-shard-cap enforcement — success, unflagged-over-cap failure, `oversized_record`-flagged exception |
| `AC-014-crash-atomicity-restart.{tape,gif,webm}` | Restart after a partial prior attempt still converges to the correct result |
| `AC-014-idempotency-safe-noop.{tape,gif,webm}` | SAFE disposition: re-invocation against an already-migrated artifact is a strict no-op |
| `AC-014-ambiguous-e-shd-011.{tape,gif,webm}` | AMBIGUOUS disposition -> fail loud `E-SHD-011`, canonical untouched, no write |
| `AC-014-dangerous-heal-manifest-slice-verify.{tape,gif,webm}` | DANGEROUS-disposition heal: Manifest-derived offset, own read-back verification, `E-SHD-012` on slice-verify failure |
| `AC-014-happy-path-canonical-readback-e-shd-013.{tape,gif,webm}` | Happy-path canonical-truncate write's own post-hoc read-back — clean control + `E-SHD-013` on corruption race |

## Reproduction

Any operator can reproduce every clip by running its `Test(s) exercised` command directly, or by
re-running the `.tape` script with `vhs <file>.tape` **from inside this directory**
(`docs/demo-evidence/S-25.02/cluster-3-backfill-split/`) — each tape's `Output` directive is a bare
filename (relative to wherever `vhs` itself is invoked from), while the RECORDED shell session
inside the tape separately self-locates to the repo root via `cd $(git rev-parse
--show-toplevel)` before running `grep`/`cargo test` (so the recorded commands are portable across
checkouts and survive this story's worktree being cleaned up post-merge). Prerequisites: Rust
toolchain (`cargo 1.95.0` used here) and `vhs` (`0.11.0` used here; `brew install vhs`).

```bash
cd docs/demo-evidence/S-25.02/cluster-3-backfill-split/
vhs suite-all-green.tape
vhs AC-013-oversized-split-shards-index.tape
vhs AC-013-preamble-preserved-end-to-end.tape
vhs AC-013-retention-composition.tape
vhs AC-014-content-preservation.tape
vhs AC-014-record-count-preservation.tape
vhs AC-014-per-shard-cap-gate.tape
vhs AC-014-crash-atomicity-restart.tape
vhs AC-014-idempotency-safe-noop.tape
vhs AC-014-ambiguous-e-shd-011.tape
vhs AC-014-dangerous-heal-manifest-slice-verify.tape
vhs AC-014-happy-path-canonical-readback-e-shd-013.tape
```

```bash
# suite-all-green
cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test

# AC-013-oversized-split-shards-index
cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test \
  test_BC_1_18_008_PC2_PC3_run_backfill_split_oversized_artifact_produces_sealed_shards_and_fresh_current -- --exact --nocapture

# AC-013-preamble-preserved-end-to-end
cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test \
  test_BC_1_18_008_BLOCKER1_run_backfill_split_preserves_preamble_bytes_end_to_end -- --exact --nocapture

# AC-013-retention-composition
cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test \
  test_BC_1_18_008_PC4_run_backfill_split_composes_with_retention_when_shard_count_exceeds_retention_count -- --exact --nocapture

# AC-014-content-preservation
cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test PC6a_verify_content_preserved -- --nocapture

# AC-014-record-count-preservation
cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test PC6b_verify_record_counts_preserved -- --nocapture

# AC-014-per-shard-cap-gate
cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test PC6c_INV4 -- --nocapture

# AC-014-crash-atomicity-restart
cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test \
  test_BC_1_18_008_EC003_run_backfill_split_restart_after_partial_prior_attempt_produces_correct_result -- --exact --nocapture

# AC-014-idempotency-safe-noop
cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test \
  test_BC_1_18_008_INV3_run_backfill_split_idempotent_second_run_returns_already_migrated -- --exact --nocapture

# AC-014-ambiguous-e-shd-011
cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test \
  test_BC_1_18_008_FC3P6001_EC010_run_backfill_split_ambiguous_on_disk_state_fails_loud_e_shd_011 -- --exact --nocapture

# AC-014-dangerous-heal-manifest-slice-verify
cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test FC3P7001 -- --nocapture

# AC-014-happy-path-canonical-readback-e-shd-013
cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test FC3P8002_EC013 -- --nocapture
```

## Scope note — what this cluster does NOT demo

This cluster covers AC-013 and AC-014 only, per the demo-recorder dispatch scope for cluster-3
"backfill-split." The following are exercised by this same test file's suite (all GREEN, confirmed
by the full `cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test` run this
recording pass also ran as a precondition — see `suite-all-green` above) but are NOT separately
recorded in an individual clip:

- **Partition-level unit tests** underlying the end-to-end clips above: `mechanism_a_partition_for_backfill`'s
  own direct tests (`test_BC_1_18_008_PC2_partition_for_backfill_never_splits_mid_record_and_stays_under_cap`,
  `test_BC_1_18_008_P2003_partition_for_backfill_first_partition_counts_preamble_bytes_toward_cap`,
  `test_BC_1_18_008_P3001_EC007_partition_for_backfill_preamble_overflow_seals_as_own_zero_record_partition`,
  `test_BC_1_18_008_P3001_EC008_partition_for_backfill_preamble_alone_exceeds_cap_sealed_whole_oversized`,
  `test_BC_1_18_008_EC002_EC017_partition_for_backfill_oversized_single_record_flagged_not_split`,
  `test_BC_1_18_008_EC016_partition_for_backfill_undersized_content_yields_single_partition`) — the
  corresponding end-to-end `run_backfill_split_*` tests for the preamble-overflow and preamble-alone
  cases (`P3001_EC007`/`P3001_EC008` end-to-end variants) and the oversized-single-record case
  (`EC017_run_backfill_split_oversized_record_sealed_whole_never_split_mid_record`) and the
  undersized-content case (`EC016_run_backfill_split_undersized_artifact_creates_zero_shard_index_no_sealing`)
  are likewise GREEN but not separately recorded here.
- **Caller-offset validation guards** (F001, P2001, P3002, P5001): malformed/spurious/empty/
  unrecognized-stem caller-supplied `record_boundary_offsets` abort cases
  (`F001_run_backfill_split_aborts_when_supplied_offsets_miss_a_real_boundary`,
  `P2001_run_backfill_split_aborts_when_supplied_offsets_add_a_spurious_mid_record_boundary`,
  `P3002_run_backfill_split_aborts_when_artifact_stem_unrecognized_with_caller_offsets`,
  `P5001_run_backfill_split_aborts_when_caller_offsets_empty_but_oracle_finds_real_boundaries`,
  `P5001_run_backfill_split_empty_content_and_empty_offsets_still_succeeds`) — covered by the suite,
  not individually recorded.
- **`record_boundary_offsets` oracle functions themselves** (F002, F004, FC3P6003, FC3P7002,
  INV2, MED3, P3003, PC2_MT) — the per-artifact-type (decision-log / lessons / burst-log /
  session-checkpoints) H2/H3 boundary-detection oracles' own unit tests, including the canonical
  vector regression tests (F004) and word-boundary/nested-block/appendix-subclause edge cases —
  these are a large, self-contained sub-suite exercising boundary *detection* (upstream of the
  *splitting* behavior this cluster's AC-013/AC-014 clips demonstrate) and are out of scope for
  this recording pass.
- **`mechanism_a_backfill_already_migrated`'s own direct unit tests** (`INV3_backfill_already_migrated_true_when_shard_index_already_exists`,
  `INV3_backfill_already_migrated_false_when_no_shard_index_exists`) — the idempotency-safe-noop
  clip above exercises this function indirectly through the full `run_mechanism_a_backfill_split`
  path; these two direct unit tests are GREEN but not separately recorded.
- **`mechanism_a_write_and_verify_sealed_shard`'s own direct unit tests** (`FC3P6002_mechanism_a_write_and_verify_sealed_shard_round_trips_uncorrupted_write`,
  `FC3P6002_mechanism_a_write_and_verify_sealed_shard_aborts_fail_loud_on_disk_corruption_race`) —
  sealed-shard-write disk read-back is covered structurally by the AC-013-oversized-split clip's
  own PC2/PC3 assertions (which depend on successful sealed-shard writes); these two direct
  round-trip/corruption-race unit tests for that same function are GREEN but not separately
  recorded, per the dispatch's own optional-13th-clip note.
- **Second-invocation / repeated-prefix healing tests** (`FC3P6001_run_backfill_split_second_invocation_heals_genuine_crash_window`,
  `FC3P6001_EC009_run_backfill_split_second_invocation_preserves_repeated_prefix_content`,
  `FC3P6001_PC3_run_backfill_split_publishes_backfill_recovery_manifest_fields`) — closely related
  to the AMBIGUOUS/DANGEROUS disposition clips above but not individually recorded.
- **F4-activation / dispatcher-gate wiring** — as noted above, this mechanism has no wired
  `execute_tiers`-driven PreToolUse gate invocation on this branch; that wiring, and any demo of it,
  is deferred to story T-12.

Every item above is GREEN in the full suite run (`suite-all-green` clip, `59 passed; 0 failed`) and
in the full-crate run (`cargo test -p factory-dispatcher`, also confirmed as a precondition for this
recording pass); none is a known gap, only an out-of-scope-for-this-recording-pass item per the
cluster-3 dispatch's own explicit 12-clip scope.
