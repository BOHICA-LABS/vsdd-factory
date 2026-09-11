# VHS Demo Tape — S-25.02 Phase F4 cluster-3 "backfill-split"
# AC-013 (Postcondition 4, BC-1.18.008 v1.8): the backfill-split composes
# with BC-1.18.007's retention policy immediately when the actual shard
# count produced by this SAME operation exceeds retention_count.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by grepping
# for the literal `pub fn run_mechanism_a_backfill_split` signature, NOT
# by pinned file.rs:NNN line numbers.

Output AC-013-retention-composition.gif
Output AC-013-retention-composition.webm

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

Type "echo 'BC-1.18.008 AC-013 PC4: backfill-split composes with BC-1.18.007 retention when shard count exceeds retention_count'"
Enter
Sleep 1.5s

Type "grep -A 6 'pub fn run_mechanism_a_backfill_split' crates/factory-dispatcher/src/shard_manager.rs"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test test_BC_1_18_008_PC4_run_backfill_split_composes_with_retention_when_shard_count_exceeds_retention_count -- --exact --nocapture"
Enter
Sleep 5s

Sleep 2s
