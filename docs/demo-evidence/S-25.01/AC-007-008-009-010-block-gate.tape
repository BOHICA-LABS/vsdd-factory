# S-25.01 — Two-arm next-advance quarantine gate (validate-unvalidated-mutation-marker)
# AC-007 (Agent arm block message), AC-008 (Bash git commit/push arm block message),
# AC-009 (non-advancing git command passes through the filter unchecked),
# AC-010 (marker absent -> both arms allow), BC-1.18.002, BC-1.18.003 PC3 escape hatch.
#
# Calls the REAL, fully-pub production entry point `on_pre_tool_use` -- the exact
# function the compiled WASM plugin exports as its PreToolUse hook -- so the printed
# block message (including the T1 -> T2 -> T3 recovery-guidance block, which contains
# NO instruction to the agent to run rm) is byte-for-byte what Claude Code receives.
#
# NOTE: the locally installed `vhs` (v0.11.0) `Wait`/`Wait+Line` command hangs
# indefinitely against this shell's PS1 in this environment (reproduced with a
# minimal repro tape — Sleep-based timing works, Wait does not). Sleep durations
# below are sized generously against measured command runtimes.

Output docs/demo-evidence/S-25.01/AC-007-008-009-010-block-gate.gif
Output docs/demo-evidence/S-25.01/AC-007-008-009-010-block-gate.webm

Require cargo

Set Shell "bash"
Set FontFamily "Menlo"
Set FontSize 16
Set Width 1300
Set Height 900
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
Type "cargo build -q -p validate-unvalidated-mutation-marker --example demo_s2501_block_gate"
Enter
Sleep 3s
Type "clear"
Enter
Sleep 500ms
Show

Type "./target/debug/examples/demo_s2501_block_gate"
Enter
Sleep 3s

Sleep 2s
