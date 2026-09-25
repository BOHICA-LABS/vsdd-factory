# VHS Demo Tape — S-25.02 cluster-5 AC-017 baseline
#
# Demonstrates BC-1.18.010 (AC-017, B2 end-state addressing) — the full
# 22-test integration suite for shard_manager.rs's addressing surface, all
# green. Precondition every focused AC-017 clip in this directory is drawn
# from.
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers.

Output AC-017-suite-all-green.gif
Output AC-017-suite-all-green.webm

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

Type "echo 'AC-017: BC-1.18.010 B2 end-state addressing — full suite'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_010_b2_addressing_test"
Enter
Sleep 6s
