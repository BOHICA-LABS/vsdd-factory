# VHS Demo -- S-25.08 AC-018/028/030
# E-MAINTENANCE-002 cause tokens and first-failure-wins evaluation order
# Traces to: BC-1.18.013 Pre 6(c), EC-032/EC-041; ADR-052 error semantics
# Usage: vhs docs/demo-evidence/S-25.08/AC-018-028-030-emaintenance-002-cause-tokens.tape   (from the repo/worktree root)
# Real `factory-dispatcher` built from this worktree; fixtures live in fresh temp dirs
# (never the repo's own .factory/).

Output docs/demo-evidence/S-25.08/AC-018-028-030-emaintenance-002-cause-tokens.gif
Output docs/demo-evidence/S-25.08/AC-018-028-030-emaintenance-002-cause-tokens.webm

Require cargo
Require python3

Set Shell "zsh"
Set FontSize 13
Set FontFamily "Menlo"
Set Width 1400
Set Height 780
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 8
Set Padding 20
Set Framerate 30
Set TypingSpeed 25ms

Hide
Type "setopt interactive_comments"
Enter
Type "cd $(git rev-parse --show-toplevel)"
Enter
Type "cargo build -q -p factory-dispatcher 2>&1 | tail -1"
Enter
Sleep 3s
Type "clear"
Enter
Show

Type "# S-25.08 AC-018/028/030 -- E-MAINTENANCE-002 cause tokens and first-failure-wins evaluation order"
Enter
Sleep 600ms
Type "python3 docs/demo-evidence/S-25.08/demo-driver.py causes"
Enter
Wait+Screen@120s /AC-028/
Sleep 1500ms
Sleep 3s
