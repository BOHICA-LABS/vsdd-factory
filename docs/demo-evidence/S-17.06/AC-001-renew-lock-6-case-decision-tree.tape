# VHS Demo — S-17.06 AC-001
# AC-001: renew_lock_if_holder 6-case decision tree
#   case 0 → NoOp (absent block)
#   case 1 → Err::Malformed (bad block, no resolve_identity call)
#   case 2 → NoOp + SkipReason::AlreadyExpired (no resolve_identity call)
#   case 3 → NoOp + SkipReason::NotHolder
#   case 4 → NoOp + SkipReason::IdentityResolutionFailed
#   case 5 → Renewed(new_content)
#
# Usage: vhs AC-001-renew-lock-6-case-decision-tree.tape  (run from worktree root)
# Output: AC-001-renew-lock-6-case-decision-tree.gif + .webm

Output docs/demo-evidence/S-17.06/AC-001-renew-lock-6-case-decision-tree.gif
Output docs/demo-evidence/S-17.06/AC-001-renew-lock-6-case-decision-tree.webm

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

Type "# S-17.06 AC-001 — renew_lock_if_holder 6-case decision tree (BC-4.17.001 PC2)"
Enter
Sleep 400ms

Type "cargo test -p factory-lock test_renew_lock_if_holder 2>&1 | grep -E 'test |test result'"
Enter
Sleep 12s
