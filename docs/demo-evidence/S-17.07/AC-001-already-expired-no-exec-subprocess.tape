# VHS Demo — S-17.07 AC-001
# AC-001: AlreadyExpired path — no exec subprocess, flush proceeds unblocked
#   When factory_lock.expires_at < now, renew_lock_if_holder returns
#   SkipReason::AlreadyExpired. resolve_identity closure is NOT called
#   (call counter asserted == 0). Flushed content uses original expires_at.
#   Traces to: BC-7.07.001 postcondition PC3 (already-expired arm)
#
# Usage: vhs docs/demo-evidence/S-17.07/AC-001-already-expired-no-exec-subprocess.tape
# Output: AC-001-already-expired-no-exec-subprocess.gif + .webm

Output docs/demo-evidence/S-17.07/AC-001-already-expired-no-exec-subprocess.gif
Output docs/demo-evidence/S-17.07/AC-001-already-expired-no-exec-subprocess.webm

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

Type "# S-17.07 AC-001 — AlreadyExpired: no exec subprocess (BC-7.07.001 PC3)"
Enter
Sleep 400ms

Type "cargo test -p precompact-flush step4_tests::test_precompact_flush_step4_already_expired_no_exec_subprocess -- --nocapture"
Enter
Sleep 15s
