# VHS Demo Tape — S-25.02 Phase F4 cluster-3 "backfill-split"
# AC-014 (Invariant 5, EC-013, happy-path read-back, BC-1.18.008 v1.8):
# the ORDINARY, non-recovery, first-time canonical-truncate write also
# gets its own mandatory post-hoc disk read-back verification — a clean
# positive control succeeds, and a corruption race on that read-back
# aborts loud with `E-SHD-013`.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by grepping
# for the literal `pub fn mechanism_a_write_and_verify_sealed_shard`
# signature, NOT by pinned file.rs:NNN line numbers.

Output AC-014-happy-path-canonical-readback-e-shd-013.gif
Output AC-014-happy-path-canonical-readback-e-shd-013.webm

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

Type "echo 'BC-1.18.008 AC-014 INV5 EC-013: happy-path canonical write read-back, clean control + E-SHD-013 on corruption race'"
Enter
Sleep 1.5s

Type "grep -A 8 'pub fn mechanism_a_write_and_verify_sealed_shard' crates/factory-dispatcher/src/shard_manager.rs"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test FC3P8002_EC013 -- --nocapture"
Enter
Sleep 5s

Sleep 2s
