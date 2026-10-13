# VHS Demo -- S-25.08 AC-007/009/015/020
# B2 (BC-INDEX) coordinator conformance: reconcile branches A/B, foreign txn, terminal-record read
# Traces to: BC-1.18.011 Pre 6(d), EC-010/EC-022..025; BC-1.18.013 Post 5a(c)
# Usage: vhs docs/demo-evidence/S-25.08/AC-007-009-015-020-b2-coordinator-conformance.tape   (from the repo/worktree root)
# Real `factory-dispatcher` built from this worktree; fixtures live in fresh temp dirs
# (never the repo's own .factory/).

Output docs/demo-evidence/S-25.08/AC-007-009-015-020-b2-coordinator-conformance.gif
Output docs/demo-evidence/S-25.08/AC-007-009-015-020-b2-coordinator-conformance.webm

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

Type "# S-25.08 AC-007/009/015/020 -- B2 (BC-INDEX) coordinator conformance: reconcile branches A/B, foreign txn, terminal-record read"
Enter
Sleep 600ms
Type "python3 docs/demo-evidence/S-25.08/demo-driver.py b2"
Enter
Wait+Screen@120s /AC-015/
Sleep 1500ms
Type "cargo test -p factory-dispatcher --test s2508_admission_blackbox_test -- PC6d_ PC6c_b2 2>&1 | grep -E '^test result'"
Enter
Sleep 6s
Sleep 1500ms
Sleep 3s
