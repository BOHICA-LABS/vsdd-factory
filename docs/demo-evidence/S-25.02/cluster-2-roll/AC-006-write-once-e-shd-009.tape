# VHS Demo Tape — S-25.02 Phase F4 cluster-2 "roll"
# AC-006 (write-once sealed shards, NON-EMPTY destination):
#   publish_sealed_shard refuses to overwrite an already-sealed,
#   NON-EMPTY shard file at the same <stem>.<seq:04>.md path. A
#   pre-existing seq=1 file holds content Y (non-empty); an attempt to
#   publish different content Z at that same path fails loud with
#   ShardRollError::SealedShardAlreadyExists (E-SHD-009), and the
#   pre-existing file is left byte-for-byte unchanged (never
#   overwritten).
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by
# grepping for the literal `SealedShardAlreadyExists {` variant
# definition, NOT by pinned file.rs:NNN line numbers.

Output AC-006-write-once-e-shd-009.gif
Output AC-006-write-once-e-shd-009.webm

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

Type "echo 'BC-1.18.006 AC-006: write-once guard -- NON-EMPTY collision at <stem>.<seq>.md refuses (E-SHD-009)'"
Enter
Sleep 1.5s

Type "grep -n 'SealedShardAlreadyExists {' crates/factory-dispatcher/src/shard_manager.rs | head -3"
Enter
Sleep 3s

Type "cargo test -p factory-dispatcher --test bc_1_18_006_roll_test test_BC_1_18_006_FC2P4_002_publish_sealed_shard_refuses_to_overwrite_existing_seq -- --exact --nocapture"
Enter
Sleep 5s

Sleep 2s
