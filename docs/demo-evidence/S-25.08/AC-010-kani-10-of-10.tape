# VHS Demo -- S-25.08 AC-010
# VP-147 v1.4 re-baseline: Kani 10/10 OBL-1 harnesses verified, vacuity gates pass.
# The live solve takes ~15 min (h4b alone ~10 min), far beyond a 15 s recording, so this tape
# shows the captured summary of a real full run (same command + gates as CI kani.yml; see
# AC-010-kani-summary.txt, full log AC-010-kani-output.log.gz) and the harness count pin.
# Traces to: BC-1.18.011 Pre 5/6(b)/6(c)/6(d)/6(e), Post 9, Inv 3; VP-147 v1.4; ADR-052 Decision 12

Output docs/demo-evidence/S-25.08/AC-010-kani-10-of-10.gif
Output docs/demo-evidence/S-25.08/AC-010-kani-10-of-10.webm

Require grep

Set Shell "zsh"
Set FontSize 13
Set FontFamily "Menlo"
Set Width 1400
Set Height 780
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 8
Set Padding 20
Set Framerate 30
Set TypingSpeed 25ms

Hide
Type "setopt interactive_comments"
Enter
Type "cd $(git rev-parse --show-toplevel)"
Enter
Type "clear"
Enter
Show

Type "# S-25.08 AC-010 -- Kani OBL-1: 10 of 10 harnesses, vacuity gates clean"
Enter
Sleep 600ms
Type "grep -c '^#\[kani::proof\]' crates/factory-dispatcher/src/shard_manager/obl1_kani_proofs.rs"
Enter
Sleep 1500ms
Type "sed -n '/per-harness verdicts/,$p' docs/demo-evidence/S-25.08/AC-010-kani-summary.txt"
Enter
Sleep 6s
