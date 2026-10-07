# VHS Demo — S-17.07 AC-003
# AC-003: NotHolder — flush proceeds with un-renewed content, expires_at byte-identical
#   When resolve_identity returns Resolved(other@example.com) but holder is
#   me@example.com, renew_lock_if_holder returns SkipReason::NotHolder.
#   Flush proceeds unblocked; expires_at is byte-identical to pre-renewal value.
#   Traces to: BC-7.07.001 invariant 3 (not-holder arm)
#
# Usage: vhs docs/demo-evidence/S-17.07/AC-003-not-holder-no-renewal.tape
# Output: AC-003-not-holder-no-renewal.gif + .webm

Output docs/demo-evidence/S-17.07/AC-003-not-holder-no-renewal.gif
Output docs/demo-evidence/S-17.07/AC-003-not-holder-no-renewal.webm

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

Type "# S-17.07 AC-003 — NotHolder: flush proceeds, expires_at byte-identical (BC-7.07.001 Invariant 3)"
Enter
Sleep 400ms

Type "cargo test -p precompact-flush step4_tests::test_precompact_flush_step4_not_holder_no_renewal -- --nocapture"
Enter
Sleep 15s
