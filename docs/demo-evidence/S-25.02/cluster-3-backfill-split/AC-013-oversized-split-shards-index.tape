# VHS Demo Tape — S-25.02 Phase F4 cluster-3 "backfill-split"
# AC-013 (BC-1.18.008 v1.8 Postconditions 2-3): an oversized pre-existing
# artifact is partitioned via deterministic greedy boundary-preserving
# packing into ceil(bytes/cap)-lower-bound sealed shards plus a fresh
# current file, and the full shard index is published in the SAME
# operation.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by grepping
# for the literal `pub fn run_mechanism_a_backfill_split` signature, NOT
# by pinned file.rs:NNN line numbers.

Output AC-013-oversized-split-shards-index.gif
Output AC-013-oversized-split-shards-index.webm

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

Type "echo 'BC-1.18.008 AC-013: oversized artifact -> sealed shards + fresh current + shard index'"
Enter
Sleep 1.5s

Type "grep -A 6 'pub fn run_mechanism_a_backfill_split' crates/factory-dispatcher/src/shard_manager.rs"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test test_BC_1_18_008_PC2_PC3_run_backfill_split_oversized_artifact_produces_sealed_shards_and_fresh_current -- --exact --nocapture"
Enter
Sleep 5s

Sleep 2s
