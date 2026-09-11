# VHS Demo Tape — S-25.02 Phase F4 cluster-3 "backfill-split"
# AC-013 (BLOCKER-1, BC-1.18.008 v1.8): a leading preamble (frontmatter /
# title / table-header content preceding the first record boundary) is
# treated as a single atomic packing unit and its bytes are preserved
# byte-for-byte end-to-end through run_mechanism_a_backfill_split.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by grepping
# for the literal `pub fn mechanism_a_partition_for_backfill` signature,
# NOT by pinned file.rs:NNN line numbers.

Output AC-013-preamble-preserved-end-to-end.gif
Output AC-013-preamble-preserved-end-to-end.webm

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

Type "echo 'BC-1.18.008 AC-013 BLOCKER-1: leading preamble is a single atomic packing unit, preserved end-to-end'"
Enter
Sleep 1.5s

Type "grep -A 6 'pub fn mechanism_a_partition_for_backfill' crates/factory-dispatcher/src/shard_manager.rs"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test test_BC_1_18_008_BLOCKER1_run_backfill_split_preserves_preamble_bytes_end_to_end -- --exact --nocapture"
Enter
Sleep 5s

Sleep 2s
