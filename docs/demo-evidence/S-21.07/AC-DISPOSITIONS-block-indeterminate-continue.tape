# VHS Demo — S-21.07 validate-cross-site-correspondence
#
# Demonstrates the gate's three core dispositions against the REAL
# factory-dispatcher binary + REAL compiled WASM plugin (see run-scenario.sh
# for the exact invocation, mirrored from
# plugins/vsdd-factory/tests/validate-cross-site-correspondence.bats):
#
#   1. BLOCK              — AC-001 / BC-5.39.010 PC2b (index-newer-than-primary)
#   2. INDETERMINATE       — AC-028 / PC15b / PC26 (secondary-index UTF-8 decode
#      advisory + Continue    failure — distinct advisory, NOT a misleading
#                              RowAbsent block)
#   3. CONTINUE (pass)     — well-formed, consistent artifacts — silent Continue
#
# NOTE: this environment's installed VHS (0.11.0) does not support
# Wait/Wait+Line reliably (command hangs and aborts the recording even
# though the wait pattern does appear on screen — verified via Screenshot
# probes). Deterministic Sleep durations are used instead; the underlying
# run-scenario.sh invocations complete in well under 0.5s each (measured),
# so the sleeps below are generous safety margin, not guesswork.
#
# Output: AC-DISPOSITIONS-block-indeterminate-continue.gif / .webm

Output docs/demo-evidence/S-21.07/AC-DISPOSITIONS-block-indeterminate-continue.gif
Output docs/demo-evidence/S-21.07/AC-DISPOSITIONS-block-indeterminate-continue.webm

Require bash

Set Shell "bash"
Set FontFamily "Menlo"
Set FontSize 16
Set Width 1200
Set Height 760
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 8
Set Padding 20
Set Framerate 30
Set TypingSpeed 40ms

Hide
Type "clear"
Enter
Sleep 500ms
Show

Sleep 500ms
Type "# validate-cross-site-correspondence -- PostToolUse governance gate (S-21.07)"
Enter
Sleep 1s

Type "./docs/demo-evidence/S-21.07/run-scenario.sh block"
Enter
Sleep 3s

Type "clear"
Enter
Sleep 500ms

Type "./docs/demo-evidence/S-21.07/run-scenario.sh indeterminate"
Enter
Sleep 3s

Type "clear"
Enter
Sleep 500ms

Type "./docs/demo-evidence/S-21.07/run-scenario.sh continue"
Enter
Sleep 3s

Sleep 2s
