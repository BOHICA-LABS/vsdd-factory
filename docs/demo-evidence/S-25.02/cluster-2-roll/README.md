# Demo Evidence — S-25.02 Phase F4 BC-cluster 2 "roll" (BC-1.18.006)

**Branch:** `feature/S-25.02-roll`, as of this recording pass's HEAD (`39369cc6`) — a literal SHA
pin here goes stale on every subsequent fix-burst commit, per the cluster-1 README's own n3/PR
#818 precedent; the recordings below were captured against the SHA current at recording time and
remain valid evidence regardless of later commits. See the PR's commit history for current HEAD.
**Status:** recorded against the branch tip described above; see the PR's commit history for any
additional fix-burst commits landed since.
**Story spec:** `.factory/stories/S-25.02-artifact-sharding-layer2.md` (AC-006, AC-007)
**Behavioral contract:** BC-1.18.006 v1.11 — Staged, Crash-Recoverable Roll-Before-Write Sequence
(Copy-Then-Atomic-Truncate, Never Rename-Away)

## Why a VHS terminal recording, not a browser demo

BC-1.18.006 is, like cluster-1's BC-1.18.005, a **native (non-WASM) dispatcher PreToolUse gate**
— the roll sequence lives in `crates/factory-dispatcher/src/shard_manager.rs`'s `execute_roll`
function (staged read-canonical / publish-sealed-shard / truncate-canonical / publish-index
sequence), invoked from `executor.rs`'s `shard_cap_precheck` -> `execute_tiers` dispatch stack
before the registry-driven WASM plugin tier loop. It has no user-facing UI: its only observable
surface is a `HookResult` (`Continue` / `Block` / `Error`) plus the resulting filesystem state
(sealed shard file, truncated canonical, shard-index TOML). A browser/Playwright demo would have
nothing to capture. A VHS terminal recording of the real `cargo test` suite driving the actual
roll logic (and, for four of the five clips, the actual `execute_tiers` -> `shard_cap_precheck`
-> `execute_roll` dispatcher stack end-to-end) is the faithful, proportionate evidence format for
this kind of product. `vhs 0.11.0` was available in this environment; no fallback to plain-text
capture was needed.

**No output was manufactured or hand-typed as text.** Every recording runs the real, unmodified
test suite that already exists in this branch (`crates/factory-dispatcher/tests/
bc_1_18_006_roll_test.rs` for the four dispatch-driven clips; `shard_manager.rs`'s own
`#[cfg(test)] mod bc_1_18_006_roll_tests` for the one direct `execute_roll`-level clip, EC-025's
0-byte reclaim) and shows the real terminal output, including PASS results plus the real,
byte-for-byte filesystem assertions each test makes (sealed-shard content equality, canonical
truncation to 0 bytes, shard-index entry counts). Per the demo-recorder's constraints, **no source
or test file was modified** to produce these recordings — each clip is a `grep` excerpt of the
exact, already-existing source anchor (function signature, error variant, or doc-comment header)
plus a live `cargo test <name> -- --exact --nocapture` run of that exact test. All five tests were
independently re-run and confirmed `PASS` immediately before recording (see Reproduction below).

## AC/EC -> Clip Mapping

| AC / EC | Behavior | Clip | Test(s) exercised | Result |
|---------|----------|------|--------------------|--------|
| AC-006 (roll + seal + truncate, Postcondition 1 steps a-d) | `Write`, `projected_size = 50,000B > shard_cap_bytes = 49,152B` -> the fired trigger runs `execute_roll`'s full four-step sequence BEFORE any `HookResult` returns: (a) read pre-roll canonical content (3,000B), (b) publish `decision-log.0001.md` as a byte-for-byte copy of that pre-roll content, (c) atomically truncate the canonical to exactly 0 bytes, (d) publish the shard-index. The blocked Write's own 50,000-byte payload is NEVER applied. Full-stack, driven through `execute_tiers`. | `AC-006-roll-seal-truncate.{gif,webm}` | `test_BC_1_18_006_AC006_over_cap_write_dispatch_stages_full_roll_sequence` (`tests/bc_1_18_006_roll_test.rs`) | PASS (`exit_code != 0`, `block_intent == true`; sealed shard == pre-roll content; canonical == 0 bytes) |
| AC-006 (write-once guard, NON-EMPTY destination, Postcondition 8 / EC-024) | A sealed shard, once published, is immutable: `publish_sealed_shard` refuses to overwrite an already-sealed, NON-EMPTY `decision-log.0001.md` with different content — fails loud with `ShardRollError::SealedShardAlreadyExists` (`E-SHD-009`), and the pre-existing file is left byte-for-byte UNCHANGED (never overwritten). | `AC-006-write-once-e-shd-009.{gif,webm}` | `test_BC_1_18_006_FC2P4_002_publish_sealed_shard_refuses_to_overwrite_existing_seq` (`tests/bc_1_18_006_roll_test.rs`) | PASS (`Err(SealedShardAlreadyExists)`; error Display contains `"E-SHD-009"`; on-disk content unchanged) |
| AC-006 (bounded 0-byte-destination reclaim, EC-025) | Unlike the NON-EMPTY case, a 0-byte file already occupying the next-expected seq path is structurally NEVER genuine sealed history (this BC's own write paths only ever seal non-empty content) — `publish_sealed_shard` reclaims it: `stat()`s once, unlinks the 0-byte file, retries `write_exclusive` exactly ONCE (never a loop). The roll then SUCCEEDS: the reclaimed seq (1) is reused (never skipped past), the real 50,000-byte pre-roll content is durably sealed at that exact path, and the shard-index advances by exactly one entry — never `E-SHD-009`. | `AC-006-0-byte-reclaim.{gif,webm}` | `test_BC_1_18_006_EC025_FC2P8_002_execute_roll_reclaims_zero_byte_destination_and_succeeds` (`shard_manager.rs` unit test, drives the full `execute_roll` staged sequence directly) | PASS (`Ok(Some(_))`; reclaimed seq == 1; sealed content == pre-roll bytes; index has exactly 1 entry; canonical == 0 bytes) |
| AC-007 (empty-canonical Block template, Case B1, Postcondition 2 / Invariant 4) | When the canonical is ALREADY 0 bytes and the incoming payload alone exceeds the cap, `execute_roll`'s `Ok(None)` short-circuit fires — no roll of any kind occurs (no shard published, no index entry appended) — and the gate emits the DISTINCT empty-canonical retry template (never the unified rotate-and-retry template AC-006's clip uses), naming the artifact, the payload's own byte count `<N>`, and the cap, with "recompute or split your payload" guidance. Verified table-driven over both a `Write` and an `Edit{replace_all:true}` case, pinned VERBATIM against BC-1.18.006 v1.8 Postcondition 2's template text. | `AC-007-empty-canonical-block.{gif,webm}` | `test_BC_1_18_006_FC2P4_003_empty_canonical_block_message_matches_verbatim_template` (`tests/bc_1_18_006_roll_test.rs`) | PASS (`exit_code != 0` for both cases; block reason matches the verbatim Case B1 template exactly, `<N>` = 60,000) |
| AC-007 (Case B2 double-fire template, EC-026, Invariant 4 rescoped to 3 templates) | A pre-existing, orphaned, over-cap canonical (60,000B, never rolled) is retroactively sealed+truncated by the Write-arm backstop (catch point ii) BEFORE this dispatch's own trigger is evaluated. This SAME Write's own 55,000-byte payload then ALSO exceeds the cap against the now-freshly-emptied canonical — the `Ok(None)` short-circuit fires a SECOND time in the SAME dispatch. Because a roll DID occur earlier in this dispatch, the gate selects the THIRD, distinct Case B2 template — dropping Case B1's "no roll was performed... remains exactly as it was before this call" claim (which would be FALSE here) — while keeping the identical split-payload guidance. Full-stack, driven through `execute_tiers`. | `AC-007-double-fire-case-b2.{gif,webm}` | `test_BC_1_18_006_EC026_FC2P8_004_double_fire_emits_case_b2_not_case_b1` (`tests/bc_1_18_006_roll_test.rs`) | PASS (`exit_code != 0`; index has exactly 1 entry — the backstop's own retroactive seal; block reason matches Case B2's verbatim template exactly, never Case B1's "no roll was performed" wording) |

## Files

| File | Content |
|------|---------|
| `AC-006-roll-seal-truncate.{tape,gif,webm}` | VHS script + recording — over-cap Write -> full 4-step roll sequence (seal + truncate + index) -> Block |
| `AC-006-write-once-e-shd-009.{tape,gif,webm}` | VHS script + recording — NON-EMPTY sealed-shard collision -> fail-loud `E-SHD-009`, pre-existing content unchanged |
| `AC-006-0-byte-reclaim.{tape,gif,webm}` | VHS script + recording — 0-byte pre-existing destination -> bounded single-retry reclaim -> roll succeeds |
| `AC-007-empty-canonical-block.{tape,gif,webm}` | VHS script + recording — over-cap payload against an already-empty canonical -> distinct Case B1 Block template (never an error) |
| `AC-007-double-fire-case-b2.{tape,gif,webm}` | VHS script + recording — backstop retroactive-seal + same-dispatch re-fire -> distinct Case B2 Block template |

## Reproduction

Any operator can reproduce every clip by running its `Test(s) exercised` command directly, or by
re-running the `.tape` script with `vhs <file>.tape` **from inside this directory**
(`docs/demo-evidence/S-25.02/cluster-2-roll/`) — each tape's `Output` directive is a bare filename
(relative to wherever `vhs` itself is invoked from), while the RECORDED shell session inside the
tape separately self-locates to the repo root via `cd $(git rev-parse --show-toplevel)` before
running `grep`/`cargo test` (so the recorded commands are portable across checkouts and survive
this story's worktree being cleaned up post-merge). Prerequisites: Rust toolchain (`cargo 1.95.0`
used here) and `vhs` (`0.11.0` used here; `brew install vhs`).

```bash
cd docs/demo-evidence/S-25.02/cluster-2-roll/
vhs AC-006-roll-seal-truncate.tape
vhs AC-006-write-once-e-shd-009.tape
vhs AC-006-0-byte-reclaim.tape
vhs AC-007-empty-canonical-block.tape
vhs AC-007-double-fire-case-b2.tape
```

```bash
# AC-006-roll-seal-truncate
cargo test -p factory-dispatcher --test bc_1_18_006_roll_test \
  test_BC_1_18_006_AC006_over_cap_write_dispatch_stages_full_roll_sequence -- --exact --nocapture

# AC-006-write-once-e-shd-009
cargo test -p factory-dispatcher --test bc_1_18_006_roll_test \
  test_BC_1_18_006_FC2P4_002_publish_sealed_shard_refuses_to_overwrite_existing_seq -- --exact --nocapture

# AC-006-0-byte-reclaim
cargo test -p factory-dispatcher --lib \
  shard_manager::bc_1_18_006_roll_tests::test_BC_1_18_006_EC025_FC2P8_002_execute_roll_reclaims_zero_byte_destination_and_succeeds \
  -- --exact --nocapture

# AC-007-empty-canonical-block
cargo test -p factory-dispatcher --test bc_1_18_006_roll_test \
  test_BC_1_18_006_FC2P4_003_empty_canonical_block_message_matches_verbatim_template -- --exact --nocapture

# AC-007-double-fire-case-b2
cargo test -p factory-dispatcher --test bc_1_18_006_roll_test \
  test_BC_1_18_006_EC026_FC2P8_004_double_fire_emits_case_b2_not_case_b1 -- --exact --nocapture
```

## Scope note — what this cluster does NOT demo

This cluster covers AC-006 and AC-007 only, per the demo-recorder dispatch scope for cluster-2
"roll." AC-008 (same-invocation shard+index atomicity) and AC-009 (shard-index schema) are
exercised by this same test file's `test_BC_1_18_006_AC008_*` and `test_BC_1_18_006_AC009_*` tests
but are out of scope for this recording pass — they belong to a separate cluster-2 demo pass if
and when dispatched. The self-heal recovery paths (`E-SHD-006`/`E-SHD-007`), the leading-probe and
Write-arm backstop catch points (AC-024/AC-025), and the non-UTF-8 byte-level read/seal guard are
likewise covered by this branch's test suite (all GREEN, confirmed by the full `cargo test -p
factory-dispatcher` run this recording pass also ran as a precondition) but are not separately
recorded here — AC-006's write-once/0-byte-reclaim clips above and AC-007's Case B1/B2 clips are
the acceptance-criteria-facing behaviors this cluster's dispatch explicitly named.
