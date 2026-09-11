# VHS Demo Tape — S-25.02 Phase F4 cluster-3 "backfill-split"
# AC-014 (Postcondition 5, BC-1.18.008 v1.8): byte-for-byte content
# preservation verification — the concatenation of all partitions'
# bytes must reconstruct the original content exactly. Two tests: the
# true (matches) and false (byte-mismatch-detected) cases.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by grepping
# for the literal `pub fn mechanism_a_verify_backfill_content_preserved`
# signature, NOT by pinned file.rs:NNN line numbers.

Output AC-014-content-preservation.gif
Output AC-014-content-preservation.webm

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

Type "echo 'BC-1.18.008 AC-014 PC5: byte-for-byte content-preservation check, success + failure'"
Enter
Sleep 1.5s

Type "grep -A 6 'pub fn mechanism_a_verify_backfill_content_preserved' crates/factory-dispatcher/src/shard_manager.rs"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test PC6a_verify_content_preserved -- --nocapture"
Enter
Sleep 5s

Sleep 2s
