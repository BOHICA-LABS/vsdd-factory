# VHS Demo Tape — S-25.02 Phase F4 cluster-4 "B1 rotation"
# Full cluster-4 test suite (BC-1.18.009 mechanism-B1 rotation):
# 10 integration tests (bc_1_18_009_b1_rotate_test.rs) + 2 library
# unit tests (bc_1_18_009_rotate_at_test.rs) = 12 tests, all GREEN.
# This is the baseline clip — every other clip in this cluster records
# one of the same tests in isolation.
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers
# are used anywhere in this tape.

Output suite-all-green.gif
Output suite-all-green.webm

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

Type "echo 'BC-1.18.009 cluster-4 B1 rotation: full test suite'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_009_b1_rotate_test"
Enter
Sleep 30s

Type "cargo test -p last-amended-migrate --test bc_1_18_009_rotate_at_test"
Enter
Sleep 10s

Sleep 2s
