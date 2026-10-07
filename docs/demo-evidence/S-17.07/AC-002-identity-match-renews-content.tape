# VHS Demo — S-17.07 AC-002
# AC-002: Identity match — flushed STATE.md content uses renewed expires_at
#   When resolve_identity returns Resolved(holder), renew_lock_if_holder
#   returns RenewOutcome::Renewed(new_content). Flushed content MUST use
#   new_content (expires_at advanced by TTL_SECONDS), not original content.
#   Traces to: BC-7.07.001 postcondition PC3 (identity-match arm)
#
# Usage: vhs docs/demo-evidence/S-17.07/AC-002-identity-match-renews-content.tape
# Output: AC-002-identity-match-renews-content.gif + .webm

Output docs/demo-evidence/S-17.07/AC-002-identity-match-renews-content.gif
Output docs/demo-evidence/S-17.07/AC-002-identity-match-renews-content.webm

Require cargo

Set Shell "zsh"
Set FontSize 14
Set FontFamily "Menlo"
Set Width 1200
Set Height 640
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 8
Set Padding 20
Set Framerate 30
Set TypingSpeed 40ms

Hide
Type "cd $(git rev-parse --show-toplevel)"
Enter
Sleep 2s
Type "clear"
Enter
Sleep 500ms
Show

Type "# S-17.07 AC-002 — Identity match renews content (BC-7.07.001 PC3)"
Enter
Sleep 400ms

Type "cargo test -p precompact-flush step4_tests::test_precompact_flush_step4_identity_match_renews_content -- --nocapture"
Enter
Sleep 15s
