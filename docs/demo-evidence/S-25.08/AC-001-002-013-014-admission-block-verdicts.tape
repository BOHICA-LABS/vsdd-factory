# VHS Demo -- S-25.08 AC-001/002/013/014
# Protected-path admission: E-MAINTENANCE-001 blocks vs admitted paths
# Traces to: BC-1.18.013 Pre 6(b); BC-1.18.011 Pre 6(b); EC-013/EC-023/EC-024
# Usage: vhs docs/demo-evidence/S-25.08/AC-001-002-013-014-admission-block-verdicts.tape   (from the repo/worktree root)
# Real `factory-dispatcher` built from this worktree; fixtures live in fresh temp dirs
# (never the repo's own .factory/).

Output docs/demo-evidence/S-25.08/AC-001-002-013-014-admission-block-verdicts.gif
Output docs/demo-evidence/S-25.08/AC-001-002-013-014-admission-block-verdicts.webm

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

Type "# S-25.08 AC-001/002/013/014 -- Protected-path admission: E-MAINTENANCE-001 blocks vs admitted paths"
Enter
Sleep 600ms
Type "python3 docs/demo-evidence/S-25.08/demo-driver.py verdicts"
Enter
Wait+Screen@120s /no txn/
Sleep 1500ms
Sleep 3s
