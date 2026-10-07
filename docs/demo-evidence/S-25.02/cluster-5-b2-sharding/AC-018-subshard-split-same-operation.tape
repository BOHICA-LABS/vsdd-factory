# VHS Demo Tape — S-25.02 cluster-5 AC-018 sub-shard split within the same operation
#
# Demonstrates BC-1.18.011 Postcondition 6: a single `run_bc_index_migration`
# run performs SECOND-LEVEL sub-sharding for over-cap subsystems (SS-05,
# SS-06) in the SAME operation as the first-level split -- not a separate
# follow-up pass. Over-cap subsystem rows are greedily packed into
# `BC-INDEX-SS-0N.a/.b/...` sub-shard files plus a
# `BC-INDEX-SS-0N.manifest.toml` sub-manifest recording each sub-shard's
# covered id range, and the SS-05+SS-06 sub-split row count independently
# reconciles against the same census gate AC-018's census clip exercises
# (no row lost or double-counted across the sub-split boundary).
#
# Tests: `run_bc_index_migration` filter (run against the subshard-chunking
# test file) matches exactly 2 integration tests:
#   test_BC_1_18_011_PC6_run_bc_index_migration_produces_sub_shard_files_and_manifest_for_over_cap_ss
#   test_BC_1_18_011_PC6_EC004_run_bc_index_migration_ss05_ss06_row_count_sum_matches_independent_census
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers.

Output AC-018-subshard-split-same-operation.gif
Output AC-018-subshard-split-same-operation.webm

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

Type "echo 'AC-018/PC6: SS-05/SS-06 sub-shard split happens IN the same migration run'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_011_b2_subshard_chunking_test run_bc_index_migration -- --nocapture"
Enter
Sleep 6s
