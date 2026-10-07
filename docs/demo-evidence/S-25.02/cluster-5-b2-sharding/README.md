# Demo Evidence — S-25.02 cluster-5 "B2 BC-INDEX sharding" (BC-1.18.010 + BC-1.18.011)

**Branch:** `feature/S-25.02-b2-sharding`, HEAD at recording time: `814bccd0` — a literal SHA pin
here goes stale on every subsequent commit; the recordings were captured against the SHA current at
recording time and remain valid evidence regardless of later commits. See the branch's commit
history for current HEAD.

**Story spec:** `.factory/stories/S-25.02-artifact-sharding-layer2.md`, task T-10 (AC-017) and
task T-11 (AC-018) — cluster-5 "shard-b2" in the story's own task breakdown (mirroring cluster-4's
README convention for `.factory/stories/S-25.02-artifact-sharding-layer2.md`'s AC references; the
`.factory/` worktree is not mounted in this story worktree, so the spec file itself is cited by path
rather than quoted here).

**Behavioral contracts:**
- **BC-1.18.010 — B2 end-state addressing** (AC-017). Zero-lookup, pure-function first-level
  addressing (`first_level_shard_path`) from an in-memory `SubsystemPrefixSnapshot` (the ARCH-INDEX
  prefix map), manifest-keyed second-level addressing (`resolve_bc_shard_path`) for over-cap
  sub-sharded subsystems (SS-05/SS-06), a three-way ARCH-INDEX parity check
  (`check_arch_index_parity`) that fails closed on any SHA divergence between the compiled binary,
  the prefix-map snapshot, and live `ARCH-INDEX.md`, and §Reader Integration
  (`detect_migration_read_state` / `open_bc_index_path_during_migration`) so a reader is never
  blocked by an in-flight migration.
- **BC-1.18.011 — B2 one-time governed migration** (AC-018). `run_bc_index_migration`'s crash-atomic
  txn state machine: content-preservation (per-BC-row equivalence between staged shards and source),
  an independent census gate (re-scan-based, does not trust the migration's own bookkeeping),
  crash-atomicity via a durable txn record + WAL-ordered intent log with a single sole commit point
  (the canonical pointer-swap), rollback/discard on any pre-commit crash, and second-level sub-shard
  splitting (SS-05/SS-06) performed IN the same operation as the first-level split.

## Why a VHS terminal recording, not a browser demo

Like clusters 1-4, BC-1.18.010 and BC-1.18.011 are **Rust dispatcher-internal mechanisms with no web
UI** — the addressing surface and the migration engine both live in
`crates/factory-dispatcher/src/shard_manager.rs`. The only observable surface is the real `cargo
test` suite's pass/fail output plus the resulting filesystem state its own assertions check. There is
no UI; a browser/Playwright demo would have nothing to capture. Per the demo-recording skill's
library/test-harness mode and following clusters 1-4's own precedent, a VHS terminal recording of the
real, unmodified `cargo test` suite driving the real production code is the faithful, proportionate
evidence format. `vhs 0.11.0` was available in this environment; no fallback to plain-text capture was
needed for the AC-facet clips below.

**Substitution note (2 items, captured as `.txt` instead of `.gif`/`.webm`):** the 30-test
fault-injection suite (`bc_1_18_011_b2_migration_crash_injection_test.rs`, `--features
factory-dispatcher/failpoints`) and the 7-harness Kani model-checking proof suite
(`obl1_kani_proofs.rs`) are both cited by the task brief as supplementary crash-consistency evidence
rather than as one of the two headline AC demos. Both were actually RUN locally in this environment
(not merely cited) and their real captured stdout is committed as `AC-018-crash-injection-30-of-30.txt`
and `AC-018-kani-proofs-7-of-7.txt`. They were captured as text rather than VHS clips for two reasons:
(1) both runs exceed VHS's ~15s-per-clip guidance by a wide margin (the crash-injection suite needs
`--test-threads=1` — global `fail::cfg` failpoint state is process-wide, so parallel test threads
interfere with each other's injected failpoints, a test-isolation property of the `fail` crate, not a
product defect — and runs ~13s; the Kani proof suite is an ~85s CBMC solve across 7 harnesses), and
(2) neither has meaningful *interactive terminal motion* to show beyond "the command is running, then
it finishes" — the evidentiary content is the pass/fail summary line, which a captured-stdout text
file preserves exactly as faithfully as a recording would, per the demo-recorder's explicit allowance
for captured-stdout evidence on backend/library features.

**No output was manufactured or hand-typed as text, and no new source file was added to enable this
demo.** Every recording and every captured-stdout file runs the real, unmodified test suite that
already exists on this branch (`bc_1_18_010_b2_addressing_test.rs`, `bc_1_18_011_b2_migration_test.rs`,
`bc_1_18_011_b2_subshard_chunking_test.rs`, `bc_1_18_011_b2_migration_crash_injection_test.rs`) and the
real, unmodified `obl1_kani_proofs.rs` Kani harnesses, and shows the real terminal output including
PASS/VERIFICATION results. Per the demo-recorder's constraints, no source or test file was modified to
produce these recordings.

## Scope note — synthetic fixtures, real activation deferred

**The real one-time B2 migration is NOT yet activated.** The armed-activation-manifest system that
would trigger `run_bc_index_migration` against this repository's own live
`.factory/specs/behavioral-contracts/BC-INDEX.md` is activation-boundary scope, not part of this
story. Every AC-018 clip and text-capture below therefore demonstrates `run_bc_index_migration`
running against SYNTHETIC temp-dir BC-INDEX fixtures — exactly the harness the crate's own integration
test suite already uses (`bc_1_18_011_b2_migration_test.rs`,
`bc_1_18_011_b2_subshard_chunking_test.rs`, `bc_1_18_011_b2_migration_crash_injection_test.rs`). This
is real production code (`run_bc_index_migration`, `verify_content_preservation`,
`compute_independent_census`, the txn state machine, the intent log, `chunk_subsystem_rows_into_sub_shards`)
exercised through its real test harness against fixture data, not a mock or a simulation written for
this demo.

## AC -> Clip Mapping

### AC-017 (BC-1.18.010) — B2 end-state addressing

| AC / VP / facet | Behavior | Clip | Test(s) exercised | Result |
|---|---|---|---|---|
| Baseline (all AC-017 clips) | Full 22-test addressing suite, all green. Precondition every focused AC-017 clip below is drawn from. | `AC-017-suite-all-green.{gif,webm}` | `cargo test -p factory-dispatcher --test bc_1_18_010_b2_addressing_test` | PASS (`22 passed; 0 failed`) |
| AC-017 / VP-127 — zero-lookup first-level addressing | `first_level_shard_path(bc_id, prefixes)` computes a subsystem's shard path from the in-memory `SubsystemPrefixSnapshot` alone (no filesystem I/O); returns a named `UnmappedPrefix` error (INV-1) for an unknown subsystem major rather than guessing. | `AC-017-zero-lookup-first-level.{gif,webm}` | Filter `first_level_shard_path` — 2 tests | PASS (`2 passed; 0 failed`) |
| AC-017 / VP-128 pt.1 — manifest-keyed second-level addressing | `resolve_bc_shard_path` delegates to first-level for non-sub-sharded subsystems (SS-01, SS-07) but, for an over-cap sub-sharded subsystem (SS-05), consults `BC-INDEX-SS-05.manifest.toml` to find the single authoritative sub-shard range; returns `SubShardRangeNotFound` (never a silent guess) when uncovered. | `AC-017-manifest-subshard-resolution.{gif,webm}` | Filter `resolve_bc_shard_path` — 4 tests | PASS (`4 passed; 0 failed`) |
| AC-017 / VP-128 pt.2 / INV-2 — three-way ARCH-INDEX parity | `check_arch_index_parity` compares the compiled binary's SHA, the prefix-map snapshot's recorded SHA, and live `ARCH-INDEX.md`'s SHA; passes only when all three agree, fails closed on any single divergence (stale binary vs. current manifest, or drifted manifest vs. `ARCH-INDEX.md`). | `AC-017-three-way-parity-fail-closed.{gif,webm}` | Filter `INV2` — 4 tests | PASS (`4 passed; 0 failed`) |
| AC-017 / §Reader Integration | `detect_migration_read_state` reports NotStarted / Committing / Completed from migration-state marker files (Completed takes precedence over an in-flight Committing marker); `open_bc_index_path_during_migration` opens the staged generation path when present, falls back to canonical on ENOENT. A reader is never blocked by an in-flight migration. | `AC-017-reader-integration.{gif,webm}` | Filter `READERINT` — 6 tests | PASS (`6 passed; 0 failed`) |

### AC-018 (BC-1.18.011) — B2 one-time governed migration

| AC / VP / facet | Behavior | Clip / file | Test(s) exercised | Result |
|---|---|---|---|---|
| Baseline (all AC-018 clips) | Full 55-test migration suite, all green (crash-atomicity + intent log + content-preservation + census + admission-gate wiring). Precondition every focused AC-018 clip below is drawn from. Runs against synthetic temp-dir fixtures. | `AC-018-suite-all-green-migration.{gif,webm}` | `cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test` | PASS (`55 passed; 0 failed`) |
| AC-018 / PC1 — fresh split + content-preservation | A FRESH `run_bc_index_migration` run against a synthetic multi-subsystem fixture splits every BC row into its correct first-level shard, preserves the frontmatter summary/invariants verbatim (`FC5P1001`); the content-preservation gate (`verify_content_preservation`) proves per-BC-row equivalence, correctly IGNORES the newly-added subsystem-shard-manifest section, and catches a genuine mismatch as a `ContentPreservationAbort`. | `AC-018-fresh-split-content-preservation.{gif,webm}` | Filter `FC5P1001` (1 test) then filter `content_preservation` (3 tests) | PASS (`1 passed` then `3 passed; 0 failed`) |
| AC-018 / PC2 — independent census | `compute_independent_census` re-scans staged shards directly (does not trust migration bookkeeping) and asserts enumerated cardinality matches `total_bcs`; `verify_independent_census` asserts every id appears in EXACTLY ONE shard — a duplicated row, a dropped row, or a staged body that still retains rows post-split are all caught as `CensusMismatchAbort`. Includes the SS-05 sub-split scoped count. | `AC-018-independent-census.{gif,webm}` | Filter `census` — 8 tests | PASS (`8 passed; 0 failed`) |
| AC-018 — crash-atomicity: txn state machine + WAL-ordered intent log | A durable txn record (`write_txn_record`/`read_active_txn_record`) round-trips and always distinguishes a live txn from a stale terminal one; the WAL-ordered intent log records each canonical move BEFORE it happens, a torn trailing record (partial crash-mid-append) is treated as ABSENT never partial, and `decide_recovery` reads the log to decide treat-done / redo-rename / fail-closed-on-absent / fail-closed-on-ambiguous, never guessing. | `AC-018-crash-atomicity-txn-intent-log.{gif,webm}` | Filter `INV1` (2 tests) then filter `intent_log` (6 tests) | PASS (`2 passed` then `6 passed; 0 failed`) |
| AC-018 — rollback / discard on crash | A crash mid-staging (pre-commit) leaves the ORIGINAL `BC-INDEX.md` completely untouched and a restart DISCARDS the partial staged generation (`EC-059`). Resume-from-staging re-checks content-preservation/census: a staged generation that now fails is aborted, a still-valid one resumes forward to completion (`EC-060`). | `AC-018-rollback-discard-on-crash.{gif,webm}` | Filter `EC059` (1 test) then filter `EC060` (2 tests) | PASS (`1 passed` then `2 passed; 0 failed`) |
| AC-018 / PC6 — sub-shard split within the same operation | A single `run_bc_index_migration` run performs SECOND-LEVEL sub-sharding for over-cap subsystems (SS-05, SS-06) in the SAME operation as the first-level split — not a separate follow-up pass. Over-cap rows are greedily packed into `BC-INDEX-SS-0N.a/.b/...` sub-shards plus a `BC-INDEX-SS-0N.manifest.toml` sub-manifest; the SS-05+SS-06 row count independently reconciles against the census gate. | `AC-018-subshard-split-same-operation.{gif,webm}` | Filter `run_bc_index_migration` on `bc_1_18_011_b2_subshard_chunking_test.rs` — 2 tests | PASS (`2 passed; 0 failed`) |
| AC-018 — crash-consistency, 30/30 fault-injection (supplementary, captured as text) | 30 OBL-1 fault-injection tests systematically crash the migration at every I/O reach point (`write_temp`, `fsync_file`, `fsync_dir`, `append`, `rename`, `remove`, plus graceful-error injections for each) and assert the post-restart state always converges to a correct outcome — clean restart, discard, resume, or forward-recovery, never a silent false-success or a stuck/ambiguous state. Requires `--test-threads=1`: `fail::cfg`'s failpoint state is process-global, so parallel threads interfere with each other's injected failpoints (a test-isolation property of the `fail` crate, not a product defect). | `AC-018-crash-injection-30-of-30.txt` | `cargo test -p factory-dispatcher --features factory-dispatcher/failpoints --test bc_1_18_011_b2_migration_crash_injection_test -- --test-threads=1` | PASS (`30 passed; 0 failed; finished in 13.10s`) |
| AC-018 — crash-consistency, 7/7 Kani formal proofs (supplementary, captured as text) | Seven `#[kani::proof]` bounded-model-checking harnesses (OBL-1 / D-1232-OBL-1, ADR-052 §Decision 4e/5a/7a/7b/7c) exhaustively prove, over their bounded input domains: recovery totality (h1), recovery safety (h2), state-machine transition boundedness + inductive step (h3 x2), the admission-gate invariant (h4), pointer-swap crash-atomicity — the sole commit point is atomic across a crash at every modeled interruption (h5) — and recovery idempotence (h6). This is the same harness set `.github/workflows/kani.yml` runs in CI on every PR to develop/main. | `AC-018-kani-proofs-7-of-7.txt` | `cargo kani -p factory-dispatcher --harness proof_obl1` | PASS (`Complete - 7 successfully verified harnesses, 0 failures, 7 total.`) |

## Files

| File | Content |
|------|---------|
| `AC-017-suite-all-green.{tape,gif,webm}` | BC-1.18.010 full 22-test addressing suite baseline |
| `AC-017-zero-lookup-first-level.{tape,gif,webm}` | VP-127 zero-lookup pure-function first-level addressing |
| `AC-017-manifest-subshard-resolution.{tape,gif,webm}` | VP-128 pt.1 manifest-keyed second-level sub-shard resolution (SS-05) |
| `AC-017-three-way-parity-fail-closed.{tape,gif,webm}` | VP-128 pt.2 / INV-2 three-way ARCH-INDEX parity, fail-closed |
| `AC-017-reader-integration.{tape,gif,webm}` | §Reader Integration — migration-state-aware reads |
| `AC-018-suite-all-green-migration.{tape,gif,webm}` | BC-1.18.011 full 55-test migration suite baseline |
| `AC-018-fresh-split-content-preservation.{tape,gif,webm}` | PC1 fresh split + content-preservation gate (per-BC-row equivalence) |
| `AC-018-independent-census.{tape,gif,webm}` | PC2 independent, re-scan-based census gate |
| `AC-018-crash-atomicity-txn-intent-log.{tape,gif,webm}` | Durable txn record + WAL-ordered intent log + fail-closed recovery decisions |
| `AC-018-rollback-discard-on-crash.{tape,gif,webm}` | EC-059/EC-060 rollback on pre-commit crash + resume-from-staging re-check |
| `AC-018-subshard-split-same-operation.{tape,gif,webm}` | PC6 SS-05/SS-06 sub-shard split within the same migration operation |
| `AC-018-crash-injection-30-of-30.txt` | Captured stdout — 30/30 OBL-1 fault-injection suite (`--features factory-dispatcher/failpoints`) |
| `AC-018-kani-proofs-7-of-7.txt` | Captured stdout excerpt — 7/7 OBL-1 Kani formal-verification harnesses |

## Reproduction

Any operator can reproduce every VHS clip by running its `Test(s) exercised` command directly, or by
re-running the `.tape` script with `vhs <file>.tape` **from inside this directory**
(`docs/demo-evidence/S-25.02/cluster-5-b2-sharding/`) — each tape's `Output` directive is a bare
filename (relative to wherever `vhs` itself is invoked from), while the recorded shell session inside
the tape self-locates to the repo root via `cd $(git rev-parse --show-toplevel)` before running
`cargo test` (portable across checkouts, survives post-merge worktree cleanup). Prerequisites: Rust
toolchain (`cargo 1.95.0` used here) and `vhs` (`0.11.0` used here; `brew install vhs`).

```bash
cd docs/demo-evidence/S-25.02/cluster-5-b2-sharding/
vhs AC-017-suite-all-green.tape
vhs AC-017-zero-lookup-first-level.tape
vhs AC-017-manifest-subshard-resolution.tape
vhs AC-017-three-way-parity-fail-closed.tape
vhs AC-017-reader-integration.tape
vhs AC-018-suite-all-green-migration.tape
vhs AC-018-fresh-split-content-preservation.tape
vhs AC-018-independent-census.tape
vhs AC-018-crash-atomicity-txn-intent-log.tape
vhs AC-018-rollback-discard-on-crash.tape
vhs AC-018-subshard-split-same-operation.tape
```

```bash
# AC-017 facets
cargo test -p factory-dispatcher --test bc_1_18_010_b2_addressing_test
cargo test -p factory-dispatcher --test bc_1_18_010_b2_addressing_test first_level_shard_path -- --nocapture
cargo test -p factory-dispatcher --test bc_1_18_010_b2_addressing_test resolve_bc_shard_path -- --nocapture
cargo test -p factory-dispatcher --test bc_1_18_010_b2_addressing_test INV2 -- --nocapture
cargo test -p factory-dispatcher --test bc_1_18_010_b2_addressing_test READERINT -- --nocapture

# AC-018 facets
cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test
cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test FC5P1001 -- --nocapture
cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test content_preservation -- --nocapture
cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test census -- --nocapture
cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test INV1 -- --nocapture
cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test intent_log -- --nocapture
cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test EC059 -- --nocapture
cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test EC060 -- --nocapture
cargo test -p factory-dispatcher --test bc_1_18_011_b2_subshard_chunking_test run_bc_index_migration -- --nocapture

# AC-018 supplementary crash-consistency evidence (captured as .txt, not VHS)
cargo test -p factory-dispatcher --features factory-dispatcher/failpoints --test bc_1_18_011_b2_migration_crash_injection_test -- --test-threads=1
cargo install --locked kani-verifier@0.68.0 && cargo kani setup   # one-time
cargo kani -p factory-dispatcher --harness proof_obl1
```

## Scope note — tests not separately recorded

The AC-017 suite (22 tests) and AC-018 suite (55 tests + 7 sub-shard-chunking tests) cover additional
behaviors beyond the facets recorded above, all GREEN in the respective `suite-all-green` clips:

- **AC-017**: `parse_bc_id` well-formed/malformed identifier parsing (`PC2`, 2 tests);
  `load_shard_manifest`/`load_sub_shard_manifest` valid-TOML parsing (`PC3`/`PC4`, 2 tests);
  `From<BcIndexAddressingError> for HookResult` mapping for malformed-id and parity-mismatch errors
  (2 tests).
- **AC-018**: `stage_new_generation`/`commit_current_generation_pointer`/`write_completed_record` (PC3,
  3 tests); pre-commit fingerprint recheck (PC3a, 2 tests); migration-lock acquire/already-held
  (PRECOND6A, 2 tests); admission-gate open/closed across every txn state (PRECOND6BC, 4 tests);
  gate-precedence ordering vs. `shard_cap_precheck` (PC6 RULING1, 5 tests); admission precheck
  no-op cases for non-migration-dir / Bash-tool / PostToolUse (3 tests); admission reservation
  creation (1 test); idempotent no-op when `completed.json` already present (EC-061, 1 test); process
  exit-code mapping (4 tests); Postcondition 7 / VP-134 no-new-Cohort-B-dependency static check (1
  test); the FC5P2 resume/discard-incomplete-staging suite (3 tests); and the chunking algorithm's own
  unit-test coverage in `bc_1_18_011_b2_subshard_chunking_test.rs` — deterministic/reproducible
  packing, cap-inclusive boundary, greedy-pack boundary correctness, letter-exhaustion (base-26
  extension), and lone-oversized-row handling (5 tests, VP-142).

All items above are GREEN in the full suite runs (`AC-017-suite-all-green` clip: `22 passed; 0
failed`; `AC-018-suite-all-green-migration` clip: `55 passed; 0 failed`; the sub-shard-chunking file's
own full run: `7 passed; 0 failed`, verified directly — see `AC-018-subshard-split-same-operation`'s
Test(s) exercised command for the 2 integration tests recorded, the remaining 5 are the unit-level
chunking-algorithm tests).
