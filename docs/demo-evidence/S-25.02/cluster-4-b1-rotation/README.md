# Demo Evidence — S-25.02 Phase F4 BC-cluster 4 "B1 rotation" (BC-1.18.009)

**Branch:** `feature/S-25.02-b1-rotation`, HEAD at recording time: `32350e2c` — a literal SHA pin
here goes stale on every subsequent fix-burst commit; the recordings were captured against the SHA
current at recording time and remain valid evidence regardless of later commits. See the branch's
commit history for current HEAD.

**Story spec:** `.factory/stories/S-25.02-artifact-sharding-layer2.md` (AC-015, AC-016, EC-008)

**Behavioral contract:** BC-1.18.009 v1.7 — Mechanism-B1 FrontmatterChangelogArray rotate-and-retry
for `BC-INDEX.md`. The B1 gate intercepts `Edit`/`Write` PreToolUse dispatches targeting `BC-INDEX.md`
when the live `changelog:` sequence reaches `N=50` items, rotates the oldest entries to a single
evergreen archive file (`BC-INDEX-changelog-archive.md`), and returns a `HookResult::Block` with a
retry instruction — or `HookResult::Error(E-SHD-014)` when the rotation reports `mutated=false`
(counter-divergence guard, EC-008/INV-5 v1.7 hardening).

## Why a VHS terminal recording, not a browser demo

Like clusters 1, 2, and 3, BC-1.18.009 is a **Rust dispatcher-internal PreToolUse gate mechanism
with no web UI** — the `FrontmatterChangelogArray` trigger-fired branch lives in
`crates/factory-dispatcher/src/shard_manager.rs`, and `rotate_changelog_at` lives in
`crates/last-amended-migrate/src/rotate.rs`. The only observable surface is the
`execute_tiers` / `shard_cap_precheck` stack's returned `TierExecutionSummary` (exit_code,
block_intent, per_plugin_results) plus the resulting filesystem state (trimmed `BC-INDEX.md`
frontmatter + created/appended `BC-INDEX-changelog-archive.md`). There is no UI. A browser/Playwright
demo would have nothing to capture. Per the demo-recording skill's library/test-harness mode and
following clusters 1-3's own precedent exactly, a VHS terminal recording of the real, unmodified
`cargo test` suite driving the real production code is the faithful, proportionate evidence format.
`vhs 0.11.0` was available in this environment; no fallback to plain-text capture was needed.

**No output was manufactured or hand-typed as text, and no new source file was added to enable this
demo.** Every recording runs the real, unmodified test suite that already exists on this branch
(`crates/factory-dispatcher/tests/bc_1_18_009_b1_rotate_test.rs` and
`crates/last-amended-migrate/tests/bc_1_18_009_rotate_at_test.rs`) and shows the real terminal
output, including PASS results plus the real, byte-for-byte filesystem and behavioral assertions each
test makes. Per the demo-recorder's constraints, no source or test file was modified to produce these
recordings.

## AC/EC -> Clip Mapping

| AC / EC | Behavior | Clip | Test(s) exercised | Result |
|---------|----------|------|--------------------|--------|
| Baseline (all clips) | Full cluster-4 test suite — 10 integration tests (`bc_1_18_009_b1_rotate_test.rs`) + 2 library unit tests (`bc_1_18_009_rotate_at_test.rs`) = 12 tests, all green. Precondition every individual clip is drawn from. | `suite-all-green.{gif,webm}` | `cargo test -p factory-dispatcher --test bc_1_18_009_b1_rotate_test` + `cargo test -p last-amended-migrate --test bc_1_18_009_rotate_at_test` | PASS (`10 passed; 0 failed` + `2 passed; 0 failed`) |
| AC-015 / EC-001 / CTV#1 (happy-path Block+rotation) | `BC-INDEX.md` at exactly N=50 `changelog:` items; Edit/Write dispatch triggers rotation; `rotate_changelog_at` trims live sequence to `low_water_mark=25` and appends 25 overflow items to the single evergreen archive `BC-INDEX-changelog-archive.md`; gate returns `HookResult::Block` (retry instruction, `block_intent=true`) — NOT `HookResult::Error`. Live sequence trimmed to low_water_mark=25 (NEVER N-1=49, the withdrawn framing). Archive accumulates all overflow items (VP-125 no-history-loss). | `AC-015-b1-rotation-happy-path.{gif,webm}` | Filter `CTV1` — matches exactly 2 tests: `test_BC_1_18_009_AC015_CTV1_over_n_edit_dispatch_rotates_and_blocks`, `test_BC_1_18_009_AC015_CTV1_over_n_write_dispatch_rotates_and_blocks` | PASS (both Edit and Write variants: `block_intent=true`, Block reason names `BC-INDEX-changelog-archive.md` and `low_water_mark=25`, archive created, live count=25, archive count=25) |
| AC-015 / EC-008 / INV-5 (counter-divergence BLOCKING guard) | Item-count trigger fires (serde count=50 >= N=50) BUT `rotate_changelog_at` returns `Ok(report)` with `report.mutated==false` (YAML inline-sequence fixture diverges serde count from line-scan count). Gate MUST return `HookResult::Error` carrying `"E-SHD-014: ..."` — a BLOCKING fail-loud error (`block_intent=true`, `exit_code=2`, `on_error=Block`). MUST NEVER return `HookResult::Block` (the retry variant) — emitting Block on `mutated=false` would cause a permanent self-DoS block+retry loop on `BC-INDEX.md`. Frontmatter byte-identical to pre-attempt state; no archive file created. | `AC-015-ec008-counter-divergence.{gif,webm}` | Filter `EC008_INV5` — matches exactly 1 test: `test_BC_1_18_009_EC008_INV5_mutated_false_returns_e_shd_014_error_variant_still_blocking` | PASS (`exit_code==2`, `block_intent=true`, `"outcome":"error"` present, `"outcome":"block"` absent, error message starts `"E-SHD-014:"`, frontmatter unchanged, archive NOT created) |

## Files

| File | Content |
|------|---------|
| `suite-all-green.{tape,gif,webm}` | VHS script + recording — full 12-test cluster-4 suite (10 integration + 2 library), all green (baseline evidence) |
| `AC-015-b1-rotation-happy-path.{tape,gif,webm}` | CTV#1 happy-path: over-N dispatch -> rotate -> Block with retry instruction (Edit + Write variants, VP-125 no-history-loss) |
| `AC-015-ec008-counter-divergence.{tape,gif,webm}` | EC-008/INV-5 counter-divergence guard: trigger fires but mutated=false -> E-SHD-014 BLOCKING Error (not Block retry variant) |

## Reproduction

Any operator can reproduce every clip by running its `Test(s) exercised` command directly, or by
re-running the `.tape` script with `vhs <file>.tape` **from inside this directory**
(`docs/demo-evidence/S-25.02/cluster-4-b1-rotation/`) — each tape's `Output` directive is a bare
filename (relative to wherever `vhs` itself is invoked from), while the recorded shell session
inside the tape self-locates to the repo root via `cd $(git rev-parse --show-toplevel)` before
running `cargo test` (portable across checkouts, survives post-merge worktree cleanup).
Prerequisites: Rust toolchain (`cargo 1.95.0` used here) and `vhs` (`0.11.0` used here; `brew install vhs`).

```bash
cd docs/demo-evidence/S-25.02/cluster-4-b1-rotation/
vhs suite-all-green.tape
vhs AC-015-b1-rotation-happy-path.tape
vhs AC-015-ec008-counter-divergence.tape
```

```bash
# suite-all-green (10 integration tests)
cargo test -p factory-dispatcher --test bc_1_18_009_b1_rotate_test

# suite-all-green (2 library unit tests)
cargo test -p last-amended-migrate --test bc_1_18_009_rotate_at_test

# AC-015-b1-rotation-happy-path (CTV#1 Edit + Write)
cargo test -p factory-dispatcher --test bc_1_18_009_b1_rotate_test CTV1 -- --nocapture

# AC-015-ec008-counter-divergence (EC-008/INV-5)
cargo test -p factory-dispatcher --test bc_1_18_009_b1_rotate_test EC008_INV5 -- --nocapture
```

## Scope note — AC-016 and other tests not separately recorded

The 12-test suite covers additional behaviors beyond the two recorded clips:

- **AC-016 / VP-131 / CTV#4** (`test_BC_1_18_009_AC016_VP131_CTV4_rotation_failure_returns_e_shd_004_state_preserved`):
  rotation failure (archive path pre-created as a directory → EISDIR) returns `HookResult::Error`
  with `E-SHD-004:`, frontmatter byte-identical to pre-rotation state (fail-loud, VP-131).
- **AC-016 / VP-125 / CTV#6** (`test_BC_1_18_009_AC016_VP125_single_evergreen_archive_no_history_loss`):
  two successive rotations both APPEND to the SAME single evergreen archive — second rotation's
  content follows first rotation's content verbatim; no overwrite or truncation (VP-125 no-history-loss).
- **AC-015 / EC-002 / CTV#3** (`test_BC_1_18_009_AC015_EC002_CTV3_below_n_continues_without_rotation`):
  below-threshold dispatch (10 items, N=50) returns `HookResult::Continue`, no archive created.
- **AC-015 / EC-007 / CTV#2** (`test_BC_1_18_009_AC015_EC007_CTV2_amortized_cadence_24_continues_then_retriggers`):
  amortized rotation cadence: first rotation trims to 25; 24 subsequent writes return Continue;
  file back at 50 items triggers Block again (EC-007 is NOT a defect).
- **AC-015 / CTV#5** (`test_BC_1_18_009_AC015_CTV5_post_rotation_correct_retry_continues`):
  correctly-retried write at low_water_mark+1=26 items returns Continue.
- **AC-015 / INV-1 / VP-126** (`test_BC_1_18_009_AC015_INV1_VP126_zero_prepend_changelog_item_callsites_in_shard_manager`):
  static source scan — `prepend_changelog_item` appears ZERO times in `shard_manager.rs`.
- **AC-015 / build_b1_block_reason verbatim pin** (`test_BC_1_18_009_AC015_build_b1_block_reason_format_pinned_verbatim`):
  pure string-template test, no branching; asserts the exact BC-1.18.009 Postcondition 2 step-3
  retry-instruction text verbatim (L-BB-D1179 verbatim-pin lesson).
- **Library unit tests** (`bc_1_18_009_rotate_at_test.rs`): `rotate_changelog_at` uses the
  explicit sibling archive path (not cycle-derived); single evergreen archive accumulates across
  rotations in the library function itself.

All items above are GREEN in the full suite run (`suite-all-green` clip, `10 passed; 0 failed` +
`2 passed; 0 failed`). The two recorded clips (AC-015 happy-path and EC-008 counter-divergence)
cover the two specific behaviors called out in the cluster-4 dispatch as the primary demonstration
targets.
