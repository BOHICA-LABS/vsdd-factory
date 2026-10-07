# VHS Demo Tape — S-25.02 Phase F4 cluster-3 "backfill-split"
# AC-014 (Recovery-Confirmation Rule, DANGEROUS disposition heal,
# BC-1.18.008 v1.8): the heal path derives its offset from the Backfill
# Recovery Manifest (NEVER from the shard-index's own `bytes_at_seal`),
# the heal write receives its own mandatory disk read-back verification,
# and a slice-verification failure aborts loud with `E-SHD-012`
# (SliceVerificationFailed). Three tests: offset-derivation,
# disk-read-back-verification, and the slice-verification-failure case.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by grepping
# for the literal `SliceVerificationFailed {` enum-variant declaration,
# NOT by pinned file.rs:NNN line numbers.

Output AC-014-dangerous-heal-manifest-slice-verify.gif
Output AC-014-dangerous-heal-manifest-slice-verify.webm

Require cargo

Set Shell "zsh"
Set FontFamily "Menlo"
Set FontSize 18
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 8
Set Padding 20
Set Framerate 30
Set TypingSpeed 50ms

Hide
Type "cd $(git rev-parse --show-toplevel) && clear"
Enter
Sleep 1.5s
Show

Type "echo 'BC-1.18.008 AC-014 DANGEROUS heal: offset from Manifest, own read-back verify, E-SHD-012 on slice-verify failure'"
Enter
Sleep 1.5s

Type "grep -A 6 'SliceVerificationFailed {' crates/factory-dispatcher/src/shard_manager.rs"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_008_backfill_split_test FC3P7001 -- --nocapture"
Enter
Sleep 5s

Sleep 2s
