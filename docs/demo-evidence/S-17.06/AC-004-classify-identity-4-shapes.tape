# VHS Demo — S-17.06 AC-004
# AC-004: classify_identity_resolution 4-shape rule:
#   shape 1 — Err(_)             → Failed("exec error: ...")
#   shape 2 — Ok, exit != 0      → Failed("exit N: ...")
#   shape 3 — Ok, exit 0, empty  → Failed("empty identity")
#   shape 4 — Ok, exit 0, nonempty → Resolved(email)
#
# Usage: vhs AC-004-classify-identity-4-shapes.tape  (run from worktree root)
# Output: AC-004-classify-identity-4-shapes.gif + .webm

Output docs/demo-evidence/S-17.06/AC-004-classify-identity-4-shapes.gif
Output docs/demo-evidence/S-17.06/AC-004-classify-identity-4-shapes.webm

Require cargo

Set Shell "zsh"
Set FontSize 14
Set Width 1200
Set Height 640
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 8
Set Padding 20
Set Framerate 30
Set TypingSpeed 40ms

Hide
Type "cd /Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-17.06"
Enter
Sleep 3s
Type "clear"
Enter
Sleep 1s
Show

Type "# S-17.06 AC-004 — classify_identity_resolution 4-shape rule (ADR-046 Decision 2)"
Enter
Sleep 400ms

Type "cargo test -p factory-lock test_classify_identity_resolution 2>&1 | grep -E 'test |test result'"
Enter
Sleep 10s
