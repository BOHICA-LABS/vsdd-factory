# VHS Demo Tape — S-25.02 Phase F4 cluster-2 "roll"
# AC-006 (staged, crash-recoverable roll-before-write sequence):
#   Write, projected_size = 50,000 bytes, shard_cap_bytes = 49,152 ->
#   trigger fires -> execute_roll's four-step sequence runs BEFORE any
#   HookResult returns: (a) read canonical (pre-roll: 3,000 bytes),
#   (b) publish sealed shard <stem>.0001.md as a byte-for-byte copy of
#   the pre-roll content, (c) atomically truncate the canonical to
#   0 bytes, (d) publish the shard-index. The blocked Write's own
#   50,000-byte payload is NEVER applied. Driven end-to-end through the
#   real PreToolUse dispatch stack (execute_tiers -> shard_cap_precheck
#   -> execute_roll), not a synthetic function call.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by
# grepping for the literal `pub fn execute_roll` signature, NOT by
# pinned file.rs:NNN line numbers.

Output AC-006-roll-seal-truncate.gif
Output AC-006-roll-seal-truncate.webm

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

Type "echo 'BC-1.18.006 AC-006: over-cap Write -> execute_roll (seal + truncate + index) -> Block'"
Enter
Sleep 1.5s

Type "grep -A 12 'pub fn execute_roll' crates/factory-dispatcher/src/shard_manager.rs"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_006_roll_test test_BC_1_18_006_AC006_over_cap_write_dispatch_stages_full_roll_sequence -- --exact --nocapture"
Enter
Sleep 5s

Sleep 2s
