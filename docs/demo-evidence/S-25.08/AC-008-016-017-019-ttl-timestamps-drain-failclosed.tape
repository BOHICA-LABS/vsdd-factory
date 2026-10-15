# VHS Demo -- S-25.08 AC-008/016/017/019
# TTL floor, reservation_is_stale timestamp rules, RESERVATION_TTL_BELOW_FLOOR, drain fail-closed
# Traces to: BC-1.18.011 EC-009/EC-025/PC6c; BC-1.18.013 EC-019/EC-031
# Usage: vhs docs/demo-evidence/S-25.08/AC-008-016-017-019-ttl-timestamps-drain-failclosed.tape   (from the repo/worktree root)
# Real `factory-dispatcher` built from this worktree; fixtures live in fresh temp dirs
# (never the repo's own .factory/).

Output docs/demo-evidence/S-25.08/AC-008-016-017-019-ttl-timestamps-drain-failclosed.gif
Output docs/demo-evidence/S-25.08/AC-008-016-017-019-ttl-timestamps-drain-failclosed.webm

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

Type "# S-25.08 AC-008/016/017/019 -- TTL floor, reservation_is_stale timestamp rules, RESERVATION_TTL_BELOW_FLOOR, drain fail-closed"
Enter
Sleep 600ms
Type "cargo test -p factory-dispatcher --test s2508_pure_core_and_ttl_test 2>&1 | grep -E '^test |test result'"
Enter
Wait+Screen@120s /test result/
Sleep 1500ms
Sleep 3s
