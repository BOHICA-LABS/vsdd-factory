# VHS Demo Tape — S-25.02 Phase F4 cluster-3 "backfill-split"
# AC-014 (Recovery-Confirmation Rule, AMBIGUOUS disposition, BC-1.18.008
# v1.8): when the exact whole-file (length, SHA-256) of on-disk state
# matches NEITHER the Backfill Recovery Manifest's original nor final
# pair, the disposition is AMBIGUOUS — fails loud with `E-SHD-011`
# (AmbiguousRecoveryState), no write issued.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by grepping
# for the literal `AmbiguousRecoveryState {` enum-variant declaration, NOT
# by pinned file.rs:NNN line numbers.

Output AC-014-ambiguous-e-shd-011.gif
Output AC-014-ambiguous-e-shd-011.webm

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

Type "echo 'BC-1.18.008 AC-014: AMBIGUOUS disposition -> fail loud E-SHD-011, no write issued'"
Enter
Sleep 1.5s

Type "grep -A 9 'AmbiguousRecoveryState {' crates/factory-dispatcher/src/shard_manager.rs"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test test_BC_1_18_008_FC3P6001_EC010_run_backfill_split_ambiguous_on_disk_state_fails_loud_e_shd_011 -- --exact --nocapture"
Enter
Sleep 5s

Sleep 2s
