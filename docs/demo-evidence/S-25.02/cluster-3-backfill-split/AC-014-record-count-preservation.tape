# VHS Demo Tape — S-25.02 Phase F4 cluster-3 "backfill-split"
# AC-014 (Postcondition 5, BC-1.18.008 v1.8): exactly-once record
# accounting — the sum of every partition's record_count must equal the
# original record count. Three tests: sum-matches (true), a dropped
# record (false), and a duplicated record (false).
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by grepping
# for the literal
# `pub fn mechanism_a_verify_backfill_record_counts_preserved` signature,
# NOT by pinned file.rs:NNN line numbers.

Output AC-014-record-count-preservation.gif
Output AC-014-record-count-preservation.webm

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

Type "echo 'BC-1.18.008 AC-014 PC5: exactly-once record accounting, success + dropped + duplicated'"
Enter
Sleep 1.5s

Type "grep -A 8 'pub fn mechanism_a_verify_backfill_record_counts_preserved' crates/factory-dispatcher/src/shard_manager.rs"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test PC6b_verify_record_counts_preserved -- --nocapture"
Enter
Sleep 5s

Sleep 2s
