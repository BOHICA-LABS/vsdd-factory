# Demo Evidence — S-25.02 Phase F4 BC-cluster 1 "cap+trigger" (BC-1.18.005)

**Branch:** `feature/S-25.02-cap-trigger`, as of this PR's final commit (see the PR's commit
history for the exact tip SHA — a literal SHA pin here goes stale on every subsequent fix-burst
commit, per n3, PR #818 fix-burst finding; the recordings below were captured against the SHA that
was current at recording time and remain valid evidence regardless of later commits)
**Status:** LOCAL-adversary-converged (3-CLEAN) as of the recording pass; additional PR-review
fix-burst commits may have landed since — see the PR's commit history for current HEAD
**Story spec:** `.factory/stories/S-25.02-artifact-sharding-layer2.md` (AC-001..AC-005, AC-023)
**Behavioral contract:** BC-1.18.005 v1.12 — Byte-Size-Denominated Shard-Cap Formula and Native
Deterministic Size-Trigger (Provisional Constants Pending F4 Harness Calibration)

## Why a VHS terminal recording, not a browser demo

BC-1.18.005 is a **native (non-WASM) dispatcher PreToolUse gate** —
`crates/factory-dispatcher/src/shard_manager.rs`'s `shard_cap_gate_check` function, invoked from
`executor.rs::shard_cap_precheck` before the registry-driven WASM plugin tier loop. It has no
user-facing UI: its only observable surface is a `HookResult` (`Continue` / `Block` / `Error`)
returned to the dispatcher's own control flow. A browser/Playwright demo would have nothing to
capture. A VHS terminal recording of the real `cargo test` suite driving the actual gate function
(and, for two clips, the actual `execute_tiers` -> `shard_cap_precheck` -> `ShardRegistry::load` ->
`shard_cap_gate_check` dispatcher stack end-to-end) is the faithful, proportionate evidence format
for this kind of product. `vhs 0.11.0` was available in this environment; no fallback to
plain-text capture was needed.

**No output was manufactured or hand-typed as text.** Every recording runs the real, unmodified
test suite that already exists in this branch (`crates/factory-dispatcher/tests/
bc_1_18_005_shard_cap_trigger_test.rs` for the three full-stack `execute_tiers`-driven clips;
`crates/factory-dispatcher/src/shard_manager.rs`'s `#[cfg(test)] mod tests` for the two
gate/formula-level clips) and shows the real terminal output, including PASS results. Per the
demo-recorder's constraints, **no source or test file was modified** to produce these
recordings — each clip is a `sed` excerpt of the exact, already-existing assertion plus a live
`cargo test <name> -- --exact --nocapture` run of that exact test.

**A note on the `tracing::warn!` advisory (AC-002b clip).** This codebase does not wire up a
`tracing_subscriber` anywhere in the workspace today (confirmed: `grep -rl tracing_subscriber
crates/` returns zero hits) — `tracing::*!` calls are structured-but-currently-unobserved
telemetry plumbing, consistent with this project's "no `println!`, use `tracing::*!`" convention.
The AC-002b clip therefore cannot show the warn line printed to a terminal (nothing in this
codebase prints it yet); instead it shows the real source of the exact `tracing::warn!` call site
plus the exact `HookResult::Continue` hand-off that follows it, paired with the real passing test
that proves the boolean decision (`size_trigger_fires(50_000, 49_152) == true`) which sends
execution down that branch. This is the most faithful evidence obtainable without adding new
plumbing to the target project, which is out of scope for a demo-recording pass.

## AC/EC -> Clip Mapping

| AC / EC | Behavior | Clip | Test(s) exercised | Result |
|---------|----------|------|--------------------|--------|
| AC-002 (under-cap leg) | `Write`, `projected_size = len(content) = 5,000B <= shard_cap_bytes = 49,152B` -> `HookResult::Continue`. Full-stack: real `.factory/shard-config.toml`, real target file, driven through `execute_tiers`. | `AC-002a-under-cap-write-continue.{gif,webm}` | `test_BC_1_18_005_INV1_write_with_real_shard_config_reaches_native_gate` (`tests/bc_1_18_005_shard_cap_trigger_test.rs`) | PASS (`exit_code == 0`, `block_intent == false`) |
| AC-002 (over-cap leg) | `Write`, `projected_size = 50,000B > shard_cap_bytes = 49,152B` -> trigger fires. Roll/Block outcome is BC-1.18.006's (later cluster, not yet landed); this cluster's own contract is a non-fatal `tracing::warn!` advisory + honest `HookResult::Continue` hand-off (never a fabricated `Block`). | `AC-002b-over-cap-trigger-fires.{gif,webm}` | `test_BC_1_18_005_vector_write_50000_bytes_over_cap_triggers` (`src/shard_manager.rs` unit test) + source excerpt of the `tracing::warn!`/`HookResult::Continue` arm | PASS (`size_trigger_fires(50_000, 49_152) == true`) |
| AC-023 / EC-009, EC-011, EC-013, EC-015, EC-017 | A `[[shard]]` entry declares `shard_cap_bytes` (100,000) GREATER than its own formula-derived ceiling `compute_shard_cap_bytes(..)` (50,640) -> `ShardRegistry`/`validate_entry` fails loud, `HookResult::Error` propagates all the way to a non-zero dispatch `exit_code`, and the block_reason names BOTH the offending `artifact_stem` (`"over-cap-log"`) AND the failure kind (`EC-013`). Full-stack, driven through `execute_tiers`. | `AC-023-malformed-config-fail-loud.{gif,webm}` | `test_BC_1_18_005_P2001_cap_exceeds_ceiling_block_reason_names_artifact_stem_and_failure_kind` (`tests/bc_1_18_005_shard_cap_trigger_test.rs`) | PASS (`exit_code != 0`; `per_plugin_results` debug output contains both `"over-cap-log"` and `"EC-013"`) |
| EC-018 (BC-1.18.005 v1.12 MATCH-FIRST, Postcondition 1 "Blast-radius scoping") | A SIBLING `[[shard]]` entry (`"lessons"`) omits `shape` entirely (would fail EC-009 if matched), but the current dispatch's target (`foo.rs`) matches NO entry at all -> `Continue`. `find_matching_entry` resolves (or fails to resolve) BEFORE `validate_entry` ever runs, so the malformed sibling is never validated for THIS dispatch. Full-stack, driven through `execute_tiers`. | `EC-018-match-first-blast-radius.{gif,webm}` | `test_BC_1_18_005_EC_018_malformed_sibling_unmatched_target_continues` (`tests/bc_1_18_005_shard_cap_trigger_test.rs`) | PASS (`exit_code == 0`, `block_intent == false`, despite the malformed sibling entry existing in the same config file) |
| EC-014 / AC-005 (Postcondition 8 missing-file/first-write) | A `"frontmatter-changelog-array"`-shaped target (`BC-INDEX.md`) does not yet exist on disk — a legitimate first-ever `Write` CREATING it. `read_changelog_item_count` maps `io::ErrorKind::NotFound -> Ok(0)`, so `current_item_count + 1 = 1 <= N=50` -> `Continue`. The legitimate create is NEVER hard-blocked as `HookResult::Error`. | `EC-014-missing-changelog-first-write.{gif,webm}` | `test_BC_1_18_005_EC_014_shard_cap_gate_check_create_path_missing_file_continues` (`src/shard_manager.rs` unit test, calls the public `shard_cap_gate_check` gate function directly) | PASS (`result == HookResult::Continue`) |

## Files

| File | Content |
|------|---------|
| `AC-002a-under-cap-write-continue.{tape,gif,webm}` | VHS script + recording — under-cap Write -> Continue |
| `AC-002b-over-cap-trigger-fires.{tape,gif,webm}` | VHS script + recording — over-cap trigger fires (warn + Continue interim) |
| `AC-023-malformed-config-fail-loud.{tape,gif,webm}` | VHS script + recording — cap-exceeds-formula-ceiling fail-loud, artifact-named |
| `EC-018-match-first-blast-radius.{tape,gif,webm}` | VHS script + recording — malformed sibling entry + unmatched target -> Continue |
| `EC-014-missing-changelog-first-write.{tape,gif,webm}` | VHS script + recording — missing changelog-array file, first-write create path -> Continue |

## Reproduction

Any operator can reproduce every clip by running its `Test(s) exercised` command directly, or by
re-running the `.tape` script with `vhs <file>.tape` **from inside this directory**
(`docs/demo-evidence/S-25.02/cluster-1-cap-trigger/`) — each tape's `Output` directive is a bare
filename (relative to wherever `vhs` itself is invoked from), while the RECORDED shell session
inside the tape separately self-locates to the repo root via `cd $(git rev-parse
--show-toplevel)` before running `sed`/`cargo test` (so the recorded commands are portable across
checkouts and survive this story's worktree being cleaned up post-merge). Prerequisites: Rust
toolchain (`cargo 1.95.0` used here) and `vhs` (`0.11.0` used here; `brew install vhs`).

```bash
cd docs/demo-evidence/S-25.02/cluster-1-cap-trigger/
vhs AC-002a-under-cap-write-continue.tape
vhs AC-002b-over-cap-trigger-fires.tape
vhs AC-023-malformed-config-fail-loud.tape
vhs EC-018-match-first-blast-radius.tape
vhs EC-014-missing-changelog-first-write.tape
```

**Path-doubling pitfall (fixed):** an earlier render pass invoked `vhs` from this same directory
while each tape's `Output` directive ALSO carried the full `docs/demo-evidence/S-25.02/
cluster-1-cap-trigger/...` path, which doubled the path
(`.../cluster-1-cap-trigger/docs/demo-evidence/S-25.02/cluster-1-cap-trigger/*.gif`). The `Output`
directives were corrected to bare filenames (this README's reproduction command above is now the
single source of truth for the render cwd), the misplaced files were moved up to the flat location
below, and the stray nested `docs/` subtree was deleted.

```bash
# AC-002a
cargo test -p factory-dispatcher --test bc_1_18_005_shard_cap_trigger_test \
  test_BC_1_18_005_INV1_write_with_real_shard_config_reaches_native_gate -- --exact --nocapture

# AC-002b
cargo test -p factory-dispatcher --lib \
  shard_manager::tests::test_BC_1_18_005_vector_write_50000_bytes_over_cap_triggers -- --exact --nocapture

# AC-023
cargo test -p factory-dispatcher --test bc_1_18_005_shard_cap_trigger_test \
  test_BC_1_18_005_P2001_cap_exceeds_ceiling_block_reason_names_artifact_stem_and_failure_kind -- --exact --nocapture

# EC-018
cargo test -p factory-dispatcher --test bc_1_18_005_shard_cap_trigger_test \
  test_BC_1_18_005_EC_018_malformed_sibling_unmatched_target_continues -- --exact --nocapture

# EC-014
cargo test -p factory-dispatcher --lib \
  shard_manager::tests::test_BC_1_18_005_EC_014_shard_cap_gate_check_create_path_missing_file_continues -- --exact --nocapture
```
