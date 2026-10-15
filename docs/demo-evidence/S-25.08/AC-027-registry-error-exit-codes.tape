# VHS Demo -- S-25.08 AC-027
# Registry error -> exit code mapping (fail-open operational errors, fail-closed schema errors)
# Traces to: BC-7.06.001 Inv 7; BC-1.08.001 Inv 1/2; E-REG-001/002/003
# Usage: vhs docs/demo-evidence/S-25.08/AC-027-registry-error-exit-codes.tape   (from the repo/worktree root)
# Real `factory-dispatcher` built from this worktree; fixtures live in fresh temp dirs
# (never the repo's own .factory/).

Output docs/demo-evidence/S-25.08/AC-027-registry-error-exit-codes.gif
Output docs/demo-evidence/S-25.08/AC-027-registry-error-exit-codes.webm

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

Type "# S-25.08 AC-027 -- Registry error -> exit code mapping (fail-open operational errors, fail-closed schema errors)"
Enter
Sleep 600ms
Type "python3 docs/demo-evidence/S-25.08/demo-driver.py registry"
Enter
Wait+Screen@120s /empty registry/
Sleep 1500ms
Type "cargo test -p factory-dispatcher --test s2508_registry_exit_code_test 2>&1 | grep -E '^test result'"
Enter
Sleep 6s
Sleep 1500ms
Sleep 3s
