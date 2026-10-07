# VHS Demo Tape — S-25.02 cluster-5 AC-017 §Reader Integration
#
# Demonstrates BC-1.18.010 §Reader Integration: how a reader detects and
# survives an in-flight B2 migration. `detect_migration_read_state` reports
# NotStarted / Committing / Completed from the migration-state directory's
# marker files (Completed takes precedence over an in-flight Committing
# marker), and `open_bc_index_path_during_migration` opens the staged
# generation path when present and falls back to the canonical pre-migration
# path on ENOENT -- a reader is never blocked by an in-flight migration.
#
# Tests: `READERINT` filter matches 6 tests:
#   test_BC_1_18_010_READERINT_detect_migration_read_state_completed_when_marker_present
#   test_BC_1_18_010_READERINT_detect_migration_read_state_committing_when_current_json_present
#   test_BC_1_18_010_READERINT_detect_migration_read_state_not_started_when_neither_marker_present
#   test_BC_1_18_010_READERINT_detect_migration_read_state_completed_takes_precedence_over_current
#   test_BC_1_18_010_READERINT_open_bc_index_path_during_migration_opens_gen_path_when_present
#   test_BC_1_18_010_READERINT_open_bc_index_path_during_migration_falls_back_to_canonical_on_enoent
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers.

Output AC-017-reader-integration.gif
Output AC-017-reader-integration.webm

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

Type "echo 'AC-017/§Reader Integration: migration-state-aware reads, never blocked'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_010_b2_addressing_test READERINT -- --nocapture"
Enter
Sleep 6s
