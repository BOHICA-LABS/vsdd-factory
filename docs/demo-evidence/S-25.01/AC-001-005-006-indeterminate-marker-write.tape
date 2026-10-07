# S-25.01 — Fuel-exhaustion INDETERMINATE classification + durable marker write
# AC-001 (fuel cause classification), AC-005 (atomic 6-field marker write),
# AC-006 (plugin.indeterminate Event 8), BC-1.18.001, BC-3.08.001 Event 8 + Event 10.
#
# Drives the REAL production execute_tiers dispatch path (factory-dispatcher crate)
# against a real compiled WASM fixture whose fuel is exhausted mid-execution, and
# prints the resulting marker TOML file plus the durable dispatcher-internal JSONL
# audit lines -- not a reconstruction, the actual wire evidence.
#
# NOTE: the locally installed `vhs` (v0.11.0) `Wait`/`Wait+Line` command hangs
# indefinitely against this shell's PS1 in this environment (reproduced with a
# minimal repro tape — Sleep-based timing works, Wait does not). Sleep durations
# below are sized generously against measured command runtimes.

Output docs/demo-evidence/S-25.01/AC-001-005-006-indeterminate-marker-write.gif
Output docs/demo-evidence/S-25.01/AC-001-005-006-indeterminate-marker-write.webm

Require cargo

Set Shell "bash"
Set FontFamily "Menlo"
Set FontSize 18
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 8
Set Padding 20
Set Framerate 30
Set TypingSpeed 40ms

Hide
Type "cd $(git rev-parse --show-toplevel)"
Enter
Sleep 1s
Type "cargo build -q -p factory-dispatcher --example demo_s2501_marker_lifecycle"
Enter
Sleep 3s
Type "clear"
Enter
Sleep 500ms
Show

Type "./target/debug/examples/demo_s2501_marker_lifecycle fuel-indeterminate"
Enter
Sleep 3s

Sleep 2s
