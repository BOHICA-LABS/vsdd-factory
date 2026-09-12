# VHS Demo Tape — S-25.02 cluster-4 AC-015 CTV#1 happy-path
#
# Demonstrates the B1 rotation happy-path (BC-1.18.009 AC-015):
#   BC-INDEX.md at exactly N=50 changelog: items -> Edit dispatch ->
#   rotate_changelog_at trims live sequence to low_water_mark=25 and
#   appends 25 overflow items to the single evergreen archive
#   BC-INDEX-changelog-archive.md -> gate returns HookResult::Block
#   (retry instruction, block_intent=true, NOT HookResult::Error).
#
# Tests: CTV1 filter matches two tests:
#   test_BC_1_18_009_AC015_CTV1_over_n_edit_dispatch_rotates_and_blocks
#   test_BC_1_18_009_AC015_CTV1_over_n_write_dispatch_rotates_and_blocks
#
# Assertions verified by the tests:
#   - exit_code != 0, block_intent = true (HookResult::Block)
#   - Block reason names "BC-INDEX-changelog-archive.md"
#   - Block reason states low_water_mark=25
#   - Archive file exists after rotation
#   - Live sequence trimmed to low_water_mark=25 (NEVER to N-1=49)
#   - Archive contains exactly 25 overflow items (no history loss, VP-125)
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers.

Output AC-015-b1-rotation-happy-path.gif
Output AC-015-b1-rotation-happy-path.webm

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

Type "echo 'AC-015/CTV#1: over-N dispatch -> Block + rotation (happy path)'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_009_b1_rotate_test CTV1 -- --nocapture"
Enter
Sleep 20s

Sleep 2s
