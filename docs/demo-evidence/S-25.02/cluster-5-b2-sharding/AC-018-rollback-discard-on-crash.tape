# VHS Demo Tape — S-25.02 cluster-5 AC-018 rollback / discard on crash
#
# Demonstrates BC-1.18.011's rollback path: a crash mid-staging (before the
# sole pointer-swap commit point) leaves the ORIGINAL BC-INDEX.md completely
# untouched, and a restart DISCARDS the partial staged generation rather
# than resuming or completing it -- pre-commit crashes always roll back to
# the pre-migration state. Also covers the two resume-from-staging arms: a
# staged generation that now fails its OWN content-preservation/census
# checks on resume is aborted (never silently completed), while a staged
# generation that is still valid resumes forward to completion.
#
# Tests: `EC059` filter matches 1 test (pre-commit crash rollback); `EC060`
# filter matches 2 tests (resume-from-staging abort vs. resume-and-succeed):
#   test_BC_1_18_011_EC059_crash_mid_staging_leaves_original_untouched_and_restart_discards_partial
#   test_BC_1_18_011_EC060_resume_from_staging_fails_when_staged_content_now_has_a_duplicate
#   test_BC_1_18_011_EC060_resume_from_staging_succeeds_when_staged_content_still_valid
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers.

Output AC-018-rollback-discard-on-crash.gif
Output AC-018-rollback-discard-on-crash.webm

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

Type "echo 'AC-018/EC-059: pre-commit crash -- original untouched, partial staging discarded'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test EC059 -- --nocapture"
Enter
Sleep 5s

Type "echo 'AC-018/EC-060: resume-from-staging -- aborts on re-check failure, else resumes forward'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test EC060 -- --nocapture"
Enter
Sleep 6s
