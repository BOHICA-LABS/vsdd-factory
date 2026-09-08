# VHS Demo Tape — S-25.02 Phase F4 cluster-2 "roll"
# AC-006 (bounded 0-byte-destination reclaim, EC-025):
#   A 0-byte file already occupies the next-expected seq path
#   (decision-log.0001.md) -- an external anomaly (a stray touch, a
#   botched manual recovery), never a genuine sealed shard (this BC's
#   own write paths only ever seal non-empty content). Unlike the
#   NON-EMPTY case (E-SHD-009 refusal, see the sibling
#   write-once clip), a 0-byte pre-existing destination is safe to
#   reclaim: publish_sealed_shard unlinks the 0-byte file and retries
#   write_exclusive exactly ONCE (never a loop), then the roll
#   succeeds -- the reclaimed seq is reused (never skipped), the real
#   pre-roll content is durably sealed at that exact path, and the
#   shard-index advances by exactly one entry.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by
# grepping for the literal "0-byte-destination exception" doc-comment
# anchor above publish_sealed_shard, NOT by pinned file.rs:NNN line
# numbers.

Output AC-006-0-byte-reclaim.gif
Output AC-006-0-byte-reclaim.webm

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

Type "echo 'BC-1.18.006 AC-006 (EC-025): 0-byte pre-existing destination is RECLAIMED, not refused'"
Enter
Sleep 1.5s

Type "grep -A 8 '0-byte-destination exception' crates/factory-dispatcher/src/shard_manager.rs | head -10"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --lib shard_manager::bc_1_18_006_roll_tests::test_BC_1_18_006_EC025_FC2P8_002_execute_roll_reclaims_zero_byte_destination_and_succeeds -- --exact --nocapture"
Enter
Sleep 5s

Sleep 2s
