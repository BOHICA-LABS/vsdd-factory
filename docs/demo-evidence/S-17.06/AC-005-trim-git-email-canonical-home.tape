# VHS Demo — S-17.06 AC-005
# AC-005: trim_git_email single canonical home in crates/factory-lock/src/lib.rs;
#         verify-factory-lock delegates — no re-implementation.
#
# Evidence:
#   1. grep for "pub fn trim_git_email" → exactly one definition in factory-lock
#   2. test_trim_git_email_canonical_in_factory_lock passes
#   3. test_verify_factory_lock_delegates_trim_git_email passes
#
# Usage: vhs AC-005-trim-git-email-canonical-home.tape  (run from worktree root)
# Output: AC-005-trim-git-email-canonical-home.gif + .webm

Output docs/demo-evidence/S-17.06/AC-005-trim-git-email-canonical-home.gif
Output docs/demo-evidence/S-17.06/AC-005-trim-git-email-canonical-home.webm

Require cargo

Set Shell "zsh"
Set FontSize 14
Set Width 1200
Set Height 700
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

Type "# S-17.06 AC-005 — trim_git_email: single canonical home in factory-lock"
Enter
Sleep 400ms

Type "grep -rn '^pub fn trim_git_email' crates/"
Enter
Sleep 2s

Type "# Exactly one definition — in crates/factory-lock/src/lib.rs"
Enter
Sleep 400ms

Type "cargo test -p factory-lock test_trim_git_email_canonical_in_factory_lock 2>&1 | grep -E 'test |test result'"
Enter
Sleep 10s

Type "cargo test -p verify-factory-lock test_verify_factory_lock_delegates_trim_git_email 2>&1 | grep -E 'test |test result'"
Enter
Sleep 10s
