# VHS Demo -- S-25.08 AC-011/031
# Release-leg order: invalid/absent id silent; valid id + unstatable migration-state = exactly one warn
# Traces to: BC-1.18.013 EC-021/EC-042; BC-1.18.011 EC-016/EC-035; ADR-052 5a
# Usage: vhs docs/demo-evidence/S-25.08/AC-011-031-release-leg-evaluation-order.tape   (from the repo/worktree root)
# Real `factory-dispatcher` built from this worktree; fixtures live in fresh temp dirs
# (never the repo's own .factory/).

Output docs/demo-evidence/S-25.08/AC-011-031-release-leg-evaluation-order.gif
Output docs/demo-evidence/S-25.08/AC-011-031-release-leg-evaluation-order.webm

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

Type "# S-25.08 AC-011/031 -- Release-leg order: invalid/absent id silent; valid id + unstatable migration-state = exactly one warn"
Enter
Sleep 600ms
Type "python3 docs/demo-evidence/S-25.08/demo-driver.py release-order"
Enter
Wait+Screen@120s /tape that follows/
Sleep 1500ms
Type "cargo test -p factory-dispatcher --test s2508_diagnostics_test -- L5_001 L6_004 2>&1 | grep -E '^(test |test result)|passed'"
Enter
Sleep 6s
Sleep 1500ms
Type "cargo test -p factory-dispatcher --test bc_1_18_013_release_stat_error_test 2>&1 | grep -E 'test result'"
Enter
Sleep 6s
Sleep 1500ms
Sleep 3s
