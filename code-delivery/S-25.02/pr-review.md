# PR Review — #831 (S-25.02 cluster-3, mechanism-A backfill-split)

**Reviewer:** pr-reviewer (fresh-eyes, final pre-merge gate)
**Head SHA reviewed:** `a5a801038f48850c1f89f19532a80d3ccff1364e`
**Base:** `develop` | **Branch:** `feature/S-25.02-backfill`
**Verdict: APPROVE** — 0 BLOCKING, 2 SUGGESTION, 3 NIT

---

## What I verified (independently, not taken on trust)

This review re-derives its conclusions from the diff, the PR body, and locally executed
evidence — not from the prior review cycle's records.

**Build/test gates, executed locally at this HEAD:**

| Gate | Command | Result |
|------|---------|--------|
| Full crate suite | `cargo test -p factory-dispatcher` | **941 passed, 0 failed** |
| Cluster-3 suites | `cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test --test bc_1_18_007_retention_test` | **62 + 24 passed, 0 failed** |
| Format | `cargo fmt --check --all` | clean (exit 0) |
| Lint | `cargo clippy -p factory-dispatcher --all-targets -- -D warnings` | clean |
| CI rollup | `gh pr view 831 --json statusCheckRollup` | 19 SUCCESS, 2 SKIPPED (release-branch guardrail, expected for a non-release branch) |

**Checklist (8 items):**

1. **Diff coherence — PASS.** All 43 files belong to S-25.02 cluster-3: two production manifests
   (`sha2` workspace dep), `shard_manager.rs`, two new test files, and the per-AC demo-evidence
   directory. No unrelated changes.
2. **Description accuracy — PASS with a NIT** (see NIT-1: the Test Evidence table carries
   pre-EC-014/EC-015 counts).
3. **Test coverage — PASS.** Every new production function is reachable from at least one named
   test. I spot-verified that the highest-risk paths are covered by *load-bearing* assertions, not
   tautologies — see "Deep-dive" below.
4. **Demo evidence — PASS.** `docs/demo-evidence/S-25.02/cluster-3-backfill-split/` contains 12
   per-AC clips as real `.gif` + `.webm` pairs (280 KB–2.5 MB each, not `.txt` placeholders) plus
   `.tape` sources, a 227-line `README.md` with the AC/EC → clip map and reproduction commands, and
   a `suite-all-green` baseline. Error paths are recorded, not just happy paths: `E-SHD-011`
   (AMBIGUOUS), `E-SHD-013` (post-write read-back failure), crash-atomicity restart, and the
   per-shard-cap gate.
5. **Commit quality — PASS.** Conventional format, `S-25.02` scope on every commit. The `wip(...)`
   subjects in the middle of the chain are acceptable given this PR is squash-merged.
6. **Diff size — NOTED (not a finding).** 8,268 additions is far over the 500-line flag threshold,
   but 5,043 of those are tests, ~700 are demo tapes/README, and the 2,429-line production delta is
   dominated by doc comments (this file's established convention). The genuine new logic is roughly
   600 lines. Reviewed in full.
7. **Missing changes — PASS.** AC-013 (mandatory one-time split, greedy boundary-preserving packing,
   preamble handling, retention composition) and AC-014 (content preservation, record-count
   preservation, per-shard-cap gate, crash atomicity, idempotency, DANGEROUS heal, read-back
   verification) are each backed by named tests I observed passing.
8. **Dependency status — PASS.** `#818` and `#824` are both MERGED, and `git log origin/develop`
   confirms `fff5e4cc` (cluster-1) and `0959e34b` (cluster-2) are ancestors of this PR's base.

---

## Deep-dive on the highest-risk logic

This is a *destructive, one-time, source-overwriting migration*. I concentrated on the paths where a
defect is unrecoverable.

**The prior cycle's BLOCKING finding is structurally closed, not paper-fixed.**
`backfill_manifest: Option<BackfillManifest>` is now a real `ShardIndex` field. I confirmed the fix
is structural by reading `publish_shard_index_update`: it does
`load_shard_index(...).unwrap_or_else(|| ShardIndex { ... })`, mutates `index.shards`, and
re-serializes the whole struct — so the Manifest survives every roll and both self-heal paths with
no special-case preservation code. `test_BC_1_18_008_FC3P9004_EC014_backfill_then_roll_then_backfill_manifest_survives_roll`
pins this against a real `execute_roll` and a real on-disk TOML round trip, and would fail under the
retired side-channel design. `EC015` pins the composability direction (roll-before-backfill appends
from `existing_max_seq + 1` and never misreads a manifest-less index as migrated).

**TOML field ordering is safe.** All scalar `ShardIndex` fields are declared before the
`backfill_manifest` table and the `[[shard]]` array-of-tables, so `toml::to_string` cannot hit the
"values must be emitted before tables" failure. `skip_serializing_if = "Option::is_none"` preserves
the pre-v1.9 on-disk shape when no backfill has run; `serde(default)` on the three new
`ShardIndexEntry` fields keeps older indexes loading unchanged.

**Partitioner (`mechanism_a_partition_for_backfill`) — content preservation holds by construction.**
I traced every branch: the three preamble dispositions (fold / EC-007 overflow / EC-008 degenerate),
the EC-002 oversized-record flush, the greedy cap flush, and the trailing flush. Every push is
contiguous with `partition_start`, and `partition_start` advances monotonically from 0 to
`content.len()`, so concatenation reproduces the original exactly. I checked the two subtractions
that could underflow (`rec_end - rec_start`, `first_record_end - offsets[0]`) and both are guarded by
`record_boundary_offsets_are_well_formed`, which is enforced on *both* entry paths (the `pub` fn's
own F-003 guard and `run_mechanism_a_backfill_split`'s upfront MED-C check).

**The `is_preamble_shard: partition.record_count == 0` discriminator is sound.** I verified that a
zero-record partition can only be produced at `i == 0` (after an oversized-record flush,
`partition_start == rec_start` on the next iteration, so the `partition_start < rec_start` guard
suppresses an empty push). No mid-file zero-record partition is reachable.

**The oracle cross-check is genuinely load-bearing.** The set-equality comparison against
`mechanism_a_record_boundary_offsets` (rather than a union or a cardinality check) is the right fix
for the tautology it replaces: under a union, an over-detecting caller list satisfies
`|caller ∪ oracle| == |caller|` and both sides of the PC6(b) comparison stay equal while a spurious
offset physically splits a record. The three disposition arms (empty-caller, unknown-stem,
recognized-stem-no-markers) are each distinguished correctly and each fail loud where they should.

**The DANGEROUS-window heal is safe.** The slice offset is derived solely from the Manifest
(`original_bytes - final_bytes`), never from summing `bytes_at_seal`, and the slice is verified
against `(final_bytes, final_sha256)` *before* any write and again by a fresh disk read-back
*after*. `checked_sub`, `usize::try_from`, and `get(offset..)` all fail loud rather than panic. The
SAFE check is evaluated before the DANGEROUS check, which correctly short-circuits the EC-016 case
where `final_* == original_*` and avoids a spurious heal.

**Error handling meets the project's production-grade bar.** I grepped the entire production delta:
zero `unwrap()`, `expect()`, `panic!`, `todo!`, or `println!` outside doc comments. The SEC-831-02
remediation is visible as the typed `ShardRetentionError::ArchivalIndexEntryVanished` variant
replacing an invariant `.expect()`. `write_atomic_bytes` fails loud on non-UTF-8 rather than
lossily substituting, which is the correct choice given PC6(a)'s byte-for-byte guarantee.

---

## Findings

### SUGGESTION-1 — `retention_count` is silently reset in the roll-before-backfill path

| Field | Value |
|-------|-------|
| Severity | suggestion |
| Category | coherence |
| Location | `crates/factory-dispatcher/src/shard_manager.rs`, `fresh_backfill_shard_index` / `run_mechanism_a_backfill_split` |

In the EC-015 composability path, an index already exists (published by a prior BC-1.18.006 roll) and
therefore already carries its own `retention_count`. `run_mechanism_a_backfill_split` reads
`existing_shards` off that index but then rebuilds the whole index via
`fresh_backfill_shard_index(entry, retention_count, all_shards)`, overwriting the persisted value
with the caller-supplied parameter. BC-1.18.007 Invariant 2 makes `retention_count` the artifact's
own independent value, never a global constant, so an operator who had lowered it for this artifact
would see it silently reverted by the backfill.

Suggested fix — carry the existing value forward when one is present:

```rust
let existing_index = load_shard_index(&index_path)...;
let effective_retention = existing_index
    .as_ref()
    .map(|idx| idx.retention_count)
    .unwrap_or(retention_count);
```

Not blocking: this mechanism has no production caller on this branch (activation is scoped to T-12),
so no live index can be affected before that wiring lands.

### SUGGESTION-2 — partial archival-move failure can orphan shard copies under `archive/`

| Field | Value |
|-------|-------|
| Severity | suggestion |
| Category | coherence |
| Location | `crates/factory-dispatcher/src/shard_manager.rs`, `archive_overflow_shards` (called from `run_mechanism_a_backfill_split` before the index publish) |

The archival loop performs `std::fs::rename` per shard and returns on the first failure. Because the
index publish happens *after* the loop, a mid-loop failure leaves the already-renamed shards sitting
under `archive/<stem>/` with no index entry describing them. A re-run then re-seals fresh shards at
the same `seq` filenames at the cycle root, and a subsequent `WholeCorpusGlobScope::ArchiveInclusive`
scan (POLICY-1's mandatory carve-out) would see both copies — double-counting IDs for append-only /
uniqueness auditing.

Suggested fix: on `ArchivalMoveFailed`, roll back the moves already performed in this pass (rename
back to the sibling location) before returning, or sweep stale `archive/<stem>/` entries that the
loaded index does not reference at the start of a backfill run.

Not blocking: this needs an I/O failure partway through the archival loop, and it falls in the same
crash-recovery-robustness class as the SUGGESTION-2/3/4 items already adjudicated to T-12 on the
grounds that the mechanism has no production caller yet. Flagging it so T-12 inherits it explicitly
rather than rediscovering it.

### NIT-1 — PR body's Test Evidence table carries stale counts

The Test Evidence table states `59/59` for the cluster-3 test file and `938/938` for the full crate
suite. Actual at this HEAD: **62** and **941** (the EC-014/EC-015 additions). The PR body's own
opening paragraph already says "941/941 tests green", so the body is internally inconsistent.
Worth a one-line edit before merge so the merged description matches reality.

### NIT-2 — demo-evidence README baseline predates EC-014/EC-015

`docs/demo-evidence/S-25.02/cluster-3-backfill-split/README.md` says "all 59" and
"`59 passed; 0 failed`" for the `suite-all-green` baseline. The clips were recorded at `a1328c3f`,
before the two regression tests landed at `1cce59b7`/`a5a80103`, so the README is accurate
*as-recorded* but stale relative to HEAD. Either re-record the baseline clip or add a one-line note
that it predates the EC-014/EC-015 additions. Per-AC coverage itself is complete and unaffected.

### NIT-3 — `sha256_hex` allocates a `String` per byte

`sha256_hex` builds the hex encoding via `.map(|byte| format!("{byte:02x}")).collect()`, allocating
32 short `String`s per call. `write!` into a `String::with_capacity(64)` avoids that. Irrelevant to
correctness and to any realistic hot path here; noted only because the helper is mirrored verbatim
into the test file and will likely be copied again.

---

## Why this is APPROVE and not REQUEST_CHANGES

The one mechanism that could cause irreversible data loss — the destructive canonical-truncate and
its crash-recovery heal — is guarded at every destructive write site by a fresh post-hoc disk
read-back verified against a durably-published, content-addressed Manifest, with a fail-loud
AMBIGUOUS disposition that refuses to guess. The previously-found cross-mechanism defect is closed
structurally (a real struct field carried by ordinary serde round-tripping) rather than by
special-case preservation code, and is pinned by a regression test that exercises the real roll
mechanism end-to-end on disk. The two SUGGESTIONs are narrow, both live behind the same
no-production-caller boundary already adjudicated to T-12, and neither can affect a live artifact
before that activation lands. The three NITs are documentation staleness and one cosmetic
allocation.

**covered_sha:** `a5a801038f48850c1f89f19532a80d3ccff1364e`
