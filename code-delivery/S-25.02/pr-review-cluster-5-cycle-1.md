# PR Review — #842 (feature/S-25.02-b2-sharding → develop)

**Reviewer role:** pr-reviewer (fresh-eyes, PR-diff-level, cluster-5 cycle-1)
**PR:** #842 — feat(S-25.02): cluster 5 — B2 end-state addressing + governed migration (BC-1.18.010/011 v1.10)
**Verdict: APPROVE** — no BLOCKING findings; 2 SUGGESTIONs + 2 NITs.

> Posting note: a formal `gh pr review --approve` is environmentally impossible on this repo — GitHub rejects self-approval ("Can not approve your own pull request") because the PR author and the environment's `gh` identity are the same account (Zious11); `github-ops` shares that identity and hits the same wall. The verdict was therefore submitted as a formal `gh pr review --comment` review event (distinct from a plain `gh pr comment` issue comment) with the APPROVE verdict at the top of the body. Escalated to the team lead for the merge-authority path.

Independently verified the PR body's headline claims against the actual diff (did not trust the narrative). All held up.

## Verified holds

- **`recover()` is total and fail-closed on its face.** `completed.json` short-circuits first; `>1` live records → `Quarantine`; the no-live branch matches `terminal`; the live branch matches exhaustively over all four `BcIndexMigrationTxnState` variants, with `Completed`/`Aborted` (unreachable-by-classification) → defensive `Quarantine`. Every reachable state returns a `RecoveryDecision`; ambiguous/inconsistent → fail-closed. The pass-2 dead-code finding is genuinely closed: `decide_intent_log_recovery` is now wired into `run_bc_index_migration`'s intent-log arm (used ~line 14536).
- **`unsafe extern "C" fcntl` (`atomic_write.rs::sync_file_durable`) is sound.** Correct SAFETY comment; correct zero-varargs call to variadic `fcntl` for `F_FULLFSYNC`; fd borrowed from a `&File` that outlives the call; `-1` → `last_os_error()`. Correctly confined to `last-amended-migrate` (no `#![deny(unsafe_code)]`).
- **Kani harness count is correct.** Exactly 7 real `#[kani::proof]` attributes (the two extra grep hits are comments); all `proof_obl1`-prefixed; `EXPECTED_PROOFS=7` matches. The `kani.yml` positive-coverage assertion correctly guards the substring-filter false-green (F-C5-P3-001).
- **Tests assert real properties.** Kani `proof_obl1_h2_recovery_safety_predicate` drives `kani::any()` inputs through the real `recover()` and asserts independently-recomputed safety predicates — non-tautological. Crash-injection suite: ~95 assertions with content-level invariant helpers (`assert_genuinely_fully_migrated`, `assert_recovery_is_idempotent`, `assert_admission_blocked`).
- **Migration write-side paths are traversal-safe.** Write-side `ss_id` flows from `parse_ss_heading_line` (strict `SS-NN`); `sub_shard_id` is machine-generated. Pre-existing SEC-002 guard on `artifact_stem` intact.
- **"Dormant" claim accurate.** `run_bc_index_migration` has no production caller. The newly-wired `bc_index_migration_admission_precheck` (main.rs dispatch) fails safe / no-ops when `.factory/migration-state/` is absent. `first_level_shard_path` has no production caller yet.
- **`Fs` seam clean.** `fail` is optional behind an off-by-default `failpoints` feature; `StdFs` production methods are durable + error-mapped.

## SUGGESTION-1 — WAL append durability is asymmetric with staging writes; comment overclaims parity

`shard_manager/migration_fs.rs` — `StdFs::append` (WAL/intent-log) fsyncs via plain `File::sync_all()` (plain `fsync(2)`), while `StdFs::write_temp` → `migration_durable_write` → `write_atomic_strict_durable` uses `F_FULLFSYNC` on macOS. The `append` comment claims "the same discipline `write_temp`'s bundled durable-write primitive already provides" — inaccurate: on macOS/APFS the WAL (the authority `recover()` trusts) is strictly less power-loss-durable than the staging data it authorizes. Only bites on true macOS power loss (process-abort is fully covered), which is D-1232-OBL-2's declared scope — **not a merge-blocker** — but the comment shouldn't claim a guarantee the code doesn't provide. Fix: route the WAL append through the `F_FULLFSYNC` primitive, or correct the comment.

## SUGGESTION-2 — config-sourced `ss_id` lacks the SEC-002 traversal guard its sibling `artifact_stem` has

`shard_manager.rs::first_level_shard_path` interpolates `entry.ss_id` (free `String` from the `subsystem_prefixes` TOML snapshot, no format validation) into `shards/BC-INDEX-{ss_id}.md` with no `/ \ .. NUL` rejection. The sibling `artifact_stem` field carries exactly that SEC-002 guard for this CWE-22 threat model. Mitigated (write-side `ss_id` is `SS-NN`-strict; `first_level_shard_path` has no production caller yet), so not blocking — but restore parity by validating `ss_id` shape (ideally assert `SS-NN`) at snapshot load or point-of-use.

## NIT-1 — `.expect()` in non-test code

`close_sub_shard_chunk` uses `.expect("checked non-empty above")` twice for `range_start`/`range_end`. Provably safe (guarded by `is_empty()` early return), but CLAUDE.md discourages `expect` on critical paths; a `let-else` on `.first()/.last()` would eliminate it.

## NIT-2 — `kani.yml` runs `cargo kani setup` unconditionally on cache hit

The version-check `if` guards only `cargo install`, not `cargo kani setup`, so the cached path still pays for setup. Idempotent/harmless; minor CI-time waste.
