# VHS Demo Tape — S-25.02 Phase F4 cluster-3 "backfill-split"
# AC-014 (EC-003, crash-atomicity, BC-1.18.008 v1.8): a restart after a
# partial prior backfill-split attempt (crash mid-sequence) still
# produces the correct final result — no double-seal, no data loss.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by grepping
# for the literal `pub fn run_mechanism_a_backfill_split` signature, NOT
# by pinned file.rs:NNN line numbers.

Output AC-014-crash-atomicity-restart.gif
Output AC-014-crash-atomicity-restart.webm

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

Type "echo 'BC-1.18.008 AC-014 EC-003: restart after a partial prior attempt still produces the correct result'"
Enter
Sleep 1.5s

Type "grep -A 6 'pub fn run_mechanism_a_backfill_split' crates/factory-dispatcher/src/shard_manager.rs"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test test_BC_1_18_008_EC003_run_backfill_split_restart_after_partial_prior_attempt_produces_correct_result -- --exact --nocapture"
Enter
Sleep 5s

Sleep 2s
