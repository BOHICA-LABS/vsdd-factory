# VHS Demo — S-17.07 AC-005
# AC-005: Malformed arm (primary) + absent-lock 0th case (secondary)
#
# Primary — Malformed arm (BC-7.07.001 PC3 case 1 / EC-004 / Invariant 3 step 3):
#   Err(LockError::Malformed(msg)) → host::log_warn is MANDATORY (SHALL).
#   resolve_identity NOT invoked (call counter == 0). expires_at NOT written.
#   Flush proceeds unblocked.
#
# Secondary — 0th case (BC-7.07.001 PC3 0th case / EC-009):
#   Ok((RenewOutcome::NoOp, None)) absent/null factory_lock →
#   resolve_identity NOT invoked, no event emitted, flush proceeds unblocked.
#
# Usage: vhs docs/demo-evidence/S-17.07/AC-005-malformed-lock-emits-log-warn-no-exec.tape
# Output: AC-005-malformed-lock-emits-log-warn-no-exec.gif + .webm

Output docs/demo-evidence/S-17.07/AC-005-malformed-lock-emits-log-warn-no-exec.gif
Output docs/demo-evidence/S-17.07/AC-005-malformed-lock-emits-log-warn-no-exec.webm

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

Type "# S-17.07 AC-005 — Malformed: mandatory log_warn no exec; 0th case absent lock (BC-7.07.001 PC3 case 1 + EC-009)"
Enter
Sleep 400ms

Type "cargo test -p precompact-flush step4_tests::test_precompact_flush_step4_malformed_lock_emits_log_warn_no_exec -- --nocapture"
Enter
Sleep 15s
