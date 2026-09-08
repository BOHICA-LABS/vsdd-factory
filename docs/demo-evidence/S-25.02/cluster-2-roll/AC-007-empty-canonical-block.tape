# VHS Demo Tape — S-25.02 Phase F4 cluster-2 "roll"
# AC-007 (empty-canonical Block template, Case B1):
#   When the canonical file is ALREADY 0 bytes and the incoming call's
#   own payload alone exceeds the cap, execute_roll's Ok(None)
#   short-circuit fires -- no roll of any kind occurs (no shard
#   published, no index entry appended) -- and the gate emits the
#   DISTINCT empty-canonical retry template (never the unified
#   rotate-and-retry template AC-006 uses), naming the artifact, the
#   payload's own byte count <N>, and the cap, with "recompute or
#   split your payload" guidance instead of "retry against the
#   current post-roll state". Verified table-driven over both a Write
#   and an Edit{replace_all:true} case, pinned VERBATIM against the
#   BC's own Postcondition 2 template text.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by
# grepping for the literal `fn build_empty_roll_retry_block_reason`
# signature, NOT by pinned file.rs:NNN line numbers.

Output AC-007-empty-canonical-block.gif
Output AC-007-empty-canonical-block.webm

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

Type "echo 'BC-1.18.006 AC-007: empty-canonical over-cap payload -> distinct Block template (never an error)'"
Enter
Sleep 1.5s

Type "grep -A 10 'fn build_empty_roll_retry_block_reason' crates/factory-dispatcher/src/shard_manager.rs"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_006_roll_test test_BC_1_18_006_FC2P4_003_empty_canonical_block_message_matches_verbatim_template -- --exact --nocapture"
Enter
Sleep 5s

Sleep 2s
