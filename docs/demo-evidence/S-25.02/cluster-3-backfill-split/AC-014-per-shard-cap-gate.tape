# VHS Demo Tape — S-25.02 Phase F4 cluster-3 "backfill-split"
# AC-014 (Invariant 4, BC-1.18.008 v1.8): per-shard-cap enforcement —
# every partition must be within shard_cap_bytes UNLESS it carries the
# oversized_record flag. Three tests: within-cap (true), unflagged shard
# exceeds cap (false), and the oversized_record-flagged exception (true).
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by grepping
# for the literal
# `pub fn mechanism_a_verify_backfill_per_shard_cap_preserved` signature,
# NOT by pinned file.rs:NNN line numbers.

Output AC-014-per-shard-cap-gate.gif
Output AC-014-per-shard-cap-gate.webm

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

Type "echo 'BC-1.18.008 AC-014 INV4: per-shard-cap enforcement, success + failure + oversized_record exception'"
Enter
Sleep 1.5s

Type "grep -A 6 'pub fn mechanism_a_verify_backfill_per_shard_cap_preserved' crates/factory-dispatcher/src/shard_manager.rs"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test PC6c_INV4 -- --nocapture"
Enter
Sleep 5s

Sleep 2s
