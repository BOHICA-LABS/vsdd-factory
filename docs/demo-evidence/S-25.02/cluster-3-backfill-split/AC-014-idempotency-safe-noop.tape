# VHS Demo Tape — S-25.02 Phase F4 cluster-3 "backfill-split"
# AC-014 (Invariant 3, SAFE disposition, BC-1.18.008 v1.8): the
# Recovery-Confirmation Rule's SAFE arm — a second invocation against an
# already-migrated artifact resolves via
# mechanism_a_backfill_already_migrated and is a strict no-op: matching
# (final_bytes, final_sha256), no write issued.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by grepping
# for the literal `pub fn mechanism_a_backfill_already_migrated`
# signature, NOT by pinned file.rs:NNN line numbers.

Output AC-014-idempotency-safe-noop.gif
Output AC-014-idempotency-safe-noop.webm

Require cargo

Set Shell "zsh"
Set FontFamily "Menlo"
Set FontSize 18
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 8
Set Padding 20
Set Framerate 30
Set TypingSpeed 50ms

Hide
Type "cd $(git rev-parse --show-toplevel) && clear"
Enter
Sleep 1.5s
Show

Type "echo 'BC-1.18.008 AC-014 INV3 SAFE: re-invocation against an already-migrated artifact is a no-op'"
Enter
Sleep 1.5s

Type "grep -A 8 'pub fn mechanism_a_backfill_already_migrated' crates/factory-dispatcher/src/shard_manager.rs"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test test_BC_1_18_008_INV3_run_backfill_split_idempotent_second_run_returns_already_migrated -- --exact --nocapture"
Enter
Sleep 5s

Sleep 2s
