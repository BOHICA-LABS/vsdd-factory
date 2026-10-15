# VHS Demo -- S-25.08 AC-005/006
# Reserve-then-verify interleavings (mechanism-A + B2) and release-on-block: real-binary suite
# Traces to: BC-1.18.013 Pre 6(c) EC-017/EC-018; BC-1.18.011 EC-007/EC-008
# Usage: vhs docs/demo-evidence/S-25.08/AC-005-006-reserve-then-verify-and-release-on-block.tape   (from the repo/worktree root)
# Real `factory-dispatcher` built from this worktree; fixtures live in fresh temp dirs
# (never the repo's own .factory/).

Output docs/demo-evidence/S-25.08/AC-005-006-reserve-then-verify-and-release-on-block.gif
Output docs/demo-evidence/S-25.08/AC-005-006-reserve-then-verify-and-release-on-block.webm

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

Type "# S-25.08 AC-005/006 -- Reserve-then-verify interleavings (mechanism-A + B2) and release-on-block: real-binary suite"
Enter
Sleep 600ms
Type "cargo test -p factory-dispatcher --test s2508_admission_blackbox_test -- reserve_then_verify EC018 2>&1 | grep -E '^test |test result'"
Enter
Wait+Screen@120s /test result/
Sleep 1500ms
Sleep 3s
