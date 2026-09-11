# VHS Demo Tape — S-25.02 Phase F4 cluster-3 "backfill-split"
# Full cluster-3 test suite (BC-1.18.008 v1.8 mechanism-A one-time
# backfill-split): all 59 tests, all GREEN. This is the baseline clip —
# every other clip in this cluster records one of the same 59 tests in
# isolation with --nocapture for readable output.
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers are
# used anywhere in this tape.

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

Type "echo 'BC-1.18.008 cluster-3 backfill-split: full test suite, 59 tests'"
Enter
Sleep 1.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test"
Enter
Sleep 6s

Sleep 2s
