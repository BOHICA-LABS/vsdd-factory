# VHS Demo Tape — S-25.02 Phase F4 cluster-2 "roll"
# AC-007 (Case B2 double-fire empty-canonical template, EC-026):
#   A pre-existing, orphaned, over-cap canonical (60,000 bytes, never
#   rolled) is retroactively sealed+truncated by the Write-arm
#   backstop (catch point ii) BEFORE this dispatch's own trigger is
#   even evaluated. This SAME Write's own payload (55,000 bytes) then
#   ALSO exceeds the cap against the now-freshly-emptied canonical --
#   Postcondition 1's Ok(None) short-circuit fires a SECOND time in
#   the SAME dispatch. Because a roll DID occur earlier in this
#   dispatch (the backstop's own retroactive seal), the gate selects
#   the THIRD, distinct Case B2 template -- dropping Case B1's "no
#   roll was performed... remains exactly as it was before this call"
#   claim, which would be FALSE here -- while keeping the identical,
#   actionable split-payload guidance.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by
# grepping for the literal `preceded_by_backstop_roll` parameter name,
# NOT by pinned file.rs:NNN line numbers.

Output AC-007-double-fire-case-b2.gif
Output AC-007-double-fire-case-b2.webm

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

Type "echo 'BC-1.18.006 AC-007 (EC-026): double-fire dispatch emits Case B2, never Case B1 wording'"
Enter
Sleep 1.5s

Type "grep -n 'preceded_by_backstop_roll' crates/factory-dispatcher/src/shard_manager.rs | head -6"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_006_roll_test test_BC_1_18_006_EC026_FC2P8_004_double_fire_emits_case_b2_not_case_b1 -- --exact --nocapture"
Enter
Sleep 5s

Sleep 2s
