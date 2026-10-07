# VHS Demo — S-17.06 AC-006
# AC-006: doc-comment corrections in crates/factory-lock/src/lib.rs (3 loci):
#   1. renew_lock_with_now algorithm doc
#   2. Ok(None) arm inline comment in renew_lock_with_now body
#   3. parse_lock doc
# All reflect post-F-P56-001 semantics:
#   Ok(None) ONLY for absent-or-fully-null block;
#   empty/absent holder with siblings → Err(MalformedLockBlock), not Ok(None).
#
# Usage: vhs AC-006-doc-comment-corrections.tape  (run from worktree root)
# Output: AC-006-doc-comment-corrections.gif + .webm

Output docs/demo-evidence/S-17.06/AC-006-doc-comment-corrections.gif
Output docs/demo-evidence/S-17.06/AC-006-doc-comment-corrections.webm

Require grep

Set Shell "zsh"
Set FontSize 13
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

Type "# S-17.06 AC-006 — corrected doc-comments (post-F-P56-001 semantics)"
Enter
Sleep 400ms

Type "# Locus 1+3: renew_lock_with_now + parse_lock docs"
Enter
Sleep 300ms
Type "grep -n 'F-P56-001' crates/factory-lock/src/lib.rs"
Enter
Sleep 3s

Type "# Locus 2: Ok(None) arm inline comment"
Enter
Sleep 300ms
Type "grep -A1 'Block absent or fully-null' crates/factory-lock/src/lib.rs"
Enter
Sleep 2s
