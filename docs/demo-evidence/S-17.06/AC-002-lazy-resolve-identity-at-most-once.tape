# VHS Demo — S-17.06 AC-002
# AC-002: resolve_identity called AT MOST ONCE per renew_lock_if_holder call;
#         NOT called for cases 0, 1, or 2.
#
# Usage: vhs AC-002-lazy-resolve-identity-at-most-once.tape  (run from worktree root)
# Output: AC-002-lazy-resolve-identity-at-most-once.gif + .webm

Output docs/demo-evidence/S-17.06/AC-002-lazy-resolve-identity-at-most-once.gif
Output docs/demo-evidence/S-17.06/AC-002-lazy-resolve-identity-at-most-once.webm

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

Type "# S-17.06 AC-002 — lazy identity: called at most once, never on cases 0/1/2"
Enter
Sleep 400ms

Type "cargo test -p factory-lock test_resolve_identity_called_at_most_once 2>&1 | grep -E 'test |test result'"
Enter
Sleep 10s
