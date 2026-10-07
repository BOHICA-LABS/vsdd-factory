# VHS Demo — S-17.06 AC-003
# AC-003: SkipReason::IdentityResolutionFailed carries exactly 4 struct fields:
#         reason, holder, locked_at, expires_at — all from parsed LockState.
#
# Usage: vhs AC-003-skip-reason-four-fields.tape  (run from worktree root)
# Output: AC-003-skip-reason-four-fields.gif + .webm

Output docs/demo-evidence/S-17.06/AC-003-skip-reason-four-fields.gif
Output docs/demo-evidence/S-17.06/AC-003-skip-reason-four-fields.webm

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

Type "# S-17.06 AC-003 — SkipReason::IdentityResolutionFailed has 4 fields from LockState"
Enter
Sleep 400ms

Type "cargo test -p factory-lock test_skip_reason_identity_resolution_failed_carries_four_fields 2>&1 | grep -E 'test |test result'"
Enter
Sleep 10s
