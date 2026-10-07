# VHS Demo — S-17.07 AC-004
# AC-004: IdentityResolutionFailed — emits factory.lock.renewal_indeterminate event + log_warn
#   When resolve_identity returns IdentityResolution::Failed("test error"),
#   the plugin MUST emit a factory.lock.renewal_indeterminate event with 5 payload
#   fields (plugin, holder, locked_at, expires_at, resolution_error) AND emit
#   host::log_warn. Flush proceeds unblocked with un-renewed content.
#   Traces to: BC-7.07.001 invariant 3b (IdentityResolutionFailed arm)
#
# Usage: vhs docs/demo-evidence/S-17.07/AC-004-resolution-failed-emits-event-and-logs.tape
# Output: AC-004-resolution-failed-emits-event-and-logs.gif + .webm

Output docs/demo-evidence/S-17.07/AC-004-resolution-failed-emits-event-and-logs.gif
Output docs/demo-evidence/S-17.07/AC-004-resolution-failed-emits-event-and-logs.webm

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

Type "# S-17.07 AC-004 — IdentityResolutionFailed: emit event + log_warn (BC-7.07.001 Invariant 3b)"
Enter
Sleep 400ms

Type "cargo test -p precompact-flush step4_tests::test_precompact_flush_step4_resolution_failed_emits_event_and_logs -- --nocapture"
Enter
Sleep 15s
